//! Wallet ingest pipeline:
//!   pull Helius Enhanced Tx history -> extract features -> push to ARMS -> commit root.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use tracing::{debug, info, warn};

use crate::embed;
use locus_client::LocusClient;

pub struct Args {
    pub wallets: PathBuf,
    pub limit: Option<usize>,
    pub dim: usize,
    pub rpc: String,
    pub helius_key: Option<String>,
    pub arms: String,
    pub program_id: Option<Pubkey>,
    pub keypair: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletProfile {
    pub address: String,
    pub features: Vec<f32>,
    pub raw_summary: serde_json::Value,
}

pub async fn run(args: Args) -> Result<()> {
    let wallet_list: Vec<String> = serde_json::from_slice(
        &std::fs::read(&args.wallets).with_context(|| format!("read {:?}", args.wallets))?,
    )?;
    let take = args.limit.unwrap_or(wallet_list.len()).min(wallet_list.len());
    info!(wallets = take, dim = args.dim, arms = %args.arms, "ingest starting");

    let key = args
        .helius_key
        .clone()
        .ok_or_else(|| anyhow!("HELIUS_API_KEY not set"))?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut profiles: Vec<WalletProfile> = Vec::with_capacity(take);
    for (i, w) in wallet_list.iter().take(take).enumerate() {
        match helius_pull_txs(&http, &key, w).await {
            Ok(raw) => {
                let features = embed::extract_features(&raw, args.dim);
                profiles.push(WalletProfile {
                    address: w.clone(),
                    features,
                    raw_summary: embed::summarize(&raw),
                });
            }
            Err(e) => warn!(wallet = %w, ?e, "helius pull failed; skipping"),
        }
        if (i + 1) % 20 == 0 {
            info!(done = i + 1, total = take, "progress");
        }
        // Gentle rate-limit pacing for the free tier.
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    info!(count = profiles.len(), "pushing embeddings to ARMS");
    for p in &profiles {
        if let Err(e) = push_to_arms(&http, &args.arms, p).await {
            warn!(wallet = %p.address, ?e, "ARMS push failed");
        }
    }

    let root = fetch_root(&http, &args.arms).await?;
    info!(root = %hex::encode(root), "ARMS root computed");

    if let (Some(program_id), Some(keypair_path)) = (args.program_id, args.keypair.clone()) {
        let payer = read_keypair_file(&keypair_path)
            .map_err(|e| anyhow!("read keypair {:?}: {}", keypair_path, e))?;
        let client = LocusClient::new(&args.rpc, None, &args.arms, program_id, payer);

        // initialize_agent is idempotent for the demo — if already exists, ignore.
        match client.initialize_agent(1000, "ipfs://locus-demo").await {
            Ok(sig) => info!(%sig, "initialize_agent OK"),
            Err(e) => debug!(?e, "initialize_agent skipped (probably already exists)"),
        }

        match client.commit_memory(root).await {
            Ok(sig) => info!(%sig, "commit_memory OK — view on Solscan"),
            Err(e) => warn!(?e, "commit_memory failed (check SOL balance + program ID)"),
        }
    } else {
        warn!("--program-id and --keypair not both set; skipping on-chain commit");
    }

    Ok(())
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeliusTx {
    pub timestamp: Option<i64>,
    #[serde(default)]
    pub fee: u64,
    #[serde(default)]
    pub fee_payer: String,
    #[serde(default)]
    pub source: String,
    #[serde(default, rename = "type")]
    pub tx_type: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub native_transfers: Vec<NativeTransfer>,
    #[serde(default)]
    pub token_transfers: Vec<TokenTransfer>,
    #[serde(default)]
    pub instructions: Vec<Instruction>,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct NativeTransfer {
    #[serde(default)]
    pub from_user_account: String,
    #[serde(default)]
    pub to_user_account: String,
    #[serde(default)]
    pub amount: i64,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TokenTransfer {
    #[serde(default)]
    pub from_user_account: String,
    #[serde(default)]
    pub to_user_account: String,
    #[serde(default)]
    pub token_amount: f64,
    #[serde(default)]
    pub mint: String,
}

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    #[serde(default)]
    pub program_id: String,
}

pub async fn helius_pull_txs(http: &reqwest::Client, api_key: &str, wallet: &str) -> Result<Vec<HeliusTx>> {
    let url = format!(
        "https://api.helius.xyz/v0/addresses/{}/transactions?api-key={}&limit=100",
        wallet, api_key
    );
    let resp = http.get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("helius {} → {}: {}", url, status, body));
    }
    let raw: Value = resp.json().await?;
    // The API returns either an array of txs OR `{ error: ... }` for some wallets.
    if let Some(arr) = raw.as_array() {
        let txs: Vec<HeliusTx> = serde_json::from_value(Value::Array(arr.clone()))
            .context("decode tx array")?;
        Ok(txs)
    } else {
        Err(anyhow!("unexpected helius response: {raw}"))
    }
}

async fn push_to_arms(http: &reqwest::Client, arms: &str, profile: &WalletProfile) -> Result<()> {
    let body = serde_json::json!({
        "id": profile.address,
        "coord": profile.features,
        "payload": {
            "wallet": profile.address,
            "summary": profile.raw_summary,
        }
    });
    http.post(format!("{}/place", arms.trim_end_matches('/')))
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn fetch_root(http: &reqwest::Client, arms: &str) -> Result<[u8; 32]> {
    #[derive(Deserialize)]
    struct R {
        root: String,
    }
    let r: R = http
        .get(format!("{}/state-root", arms.trim_end_matches('/')))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let bytes = hex::decode(&r.root).context("decode root hex")?;
    let mut out = [0u8; 32];
    if bytes.len() < 32 {
        return Err(anyhow!("root too short: {} bytes", bytes.len()));
    }
    out.copy_from_slice(&bytes[..32]);
    Ok(out)
}
