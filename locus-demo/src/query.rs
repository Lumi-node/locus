//! similar + show-attestation handlers.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file, signature::Signature};
use tracing::info;

use locus_client::LocusClient;

pub struct SimilarArgs {
    pub wallet: Pubkey,
    pub k: usize,
    pub rpc: String,
    pub helius_key: Option<String>,
    pub arms: String,
    pub program_id: Option<Pubkey>,
    pub keypair: Option<PathBuf>,
}

pub struct ShowArgs {
    pub signature: Option<Signature>,
    pub pda: Option<Pubkey>,
    pub rpc: String,
    pub program_id: Option<Pubkey>,
    pub keypair: Option<PathBuf>,
}

pub async fn similar(args: SimilarArgs) -> Result<()> {
    let program_id = args
        .program_id
        .ok_or_else(|| anyhow!("--program-id required"))?;
    let keypair_path = args
        .keypair
        .clone()
        .ok_or_else(|| anyhow!("--keypair required"))?;
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow!("read keypair {:?}: {}", keypair_path, e))?;
    let owner_pk = solana_sdk::signer::Signer::pubkey(&payer);

    info!(?args.wallet, k = args.k, "similar: looking up embedding from ARMS");

    let http = reqwest::Client::new();
    let body = serde_json::json!({ "id": args.wallet.to_string() });
    let me: serde_json::Value = http
        .post(format!("{}/get", args.arms.trim_end_matches('/')))
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("wallet {} not in ARMS — run `ingest` first", args.wallet))?
        .json()
        .await?;
    let embedding: Vec<f32> = serde_json::from_value(me["coord"].clone())
        .map_err(|e| anyhow!("decode coord: {e}"))?;

    let client = LocusClient::new(&args.rpc, None, &args.arms, program_id, payer);
    let result = client
        .query_with_attestation(&owner_pk, &embedding, args.k)
        .await
        .context("query_with_attestation")?;

    println!("\n=== top {} behaviorally similar wallets to {} ===", args.k, args.wallet);
    for (i, n) in result.neighbors.iter().enumerate() {
        let summary = n
            .payload
            .as_ref()
            .and_then(|p| p.get("summary"))
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .unwrap_or_default();
        println!(
            "  {:>2}. {:<45} dist={:<8.4} {}",
            i + 1,
            n.id,
            n.distance,
            summary
        );
    }

    println!("\n=== on-chain attestation ===");
    println!("  signature : {}", result.attestation_signature);
    println!("  pda       : {}", result.attestation_pda);
    println!("  query_hash : {}", hex::encode(result.query_hash));
    println!("  result_hash: {}", hex::encode(result.result_hash));
    println!("  version    : {}", result.version);
    println!("  nonce      : {}", result.nonce);
    println!();
    println!(
        "  solscan tx : https://solscan.io/tx/{}?cluster=devnet",
        result.attestation_signature
    );
    println!(
        "  solscan pda: https://solscan.io/account/{}?cluster=devnet",
        result.attestation_pda
    );
    Ok(())
}

pub async fn show_attestation(args: ShowArgs) -> Result<()> {
    let program_id = args
        .program_id
        .ok_or_else(|| anyhow!("--program-id required"))?;
    let pda = args
        .pda
        .ok_or_else(|| anyhow!("--pda required (signature-only lookup not yet supported)"))?;
    let keypair_path = args
        .keypair
        .clone()
        .ok_or_else(|| anyhow!("--keypair required (for client construction)"))?;
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow!("read keypair {:?}: {}", keypair_path, e))?;

    let client = LocusClient::new(&args.rpc, None, "", program_id, payer);
    let att = client.fetch_attestation(pda).await?;

    println!("\n=== RetrievalAttestation @ {} ===", pda);
    println!("  agent        : {}", att.agent);
    println!("  requester    : {}", att.requester);
    println!("  version      : {}", att.version);
    println!("  nonce        : {}", att.nonce);
    println!("  timestamp    : {}", att.timestamp);
    println!("  memory_root  : {}", hex::encode(att.memory_root));
    println!("  query_hash   : {}", hex::encode(att.query_hash));
    println!("  result_hash  : {}", hex::encode(att.result_hash));
    println!("\n  solscan: https://solscan.io/account/{}?cluster=devnet", pda);
    if let Some(sig) = args.signature {
        println!("  tx     : https://solscan.io/tx/{}?cluster=devnet", sig);
    }
    Ok(())
}
