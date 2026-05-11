//! # locus-relayer
//!
//! Polls the ARMS HTTP service for state changes and commits the new
//! Merkle root to the Locus program on Solana.

mod merkle;

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use locus_client::LocusClient;
use serde::Deserialize;
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};
use tracing::{error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "locus-relayer", version, about = "ARMS -> Solana bridge")]
struct Args {
    /// Agent owner public key (the PDA seed).
    #[arg(long)]
    agent: Pubkey,

    /// ARMS service base URL.
    #[arg(long, default_value = "http://localhost:8080")]
    arms: String,

    /// Solana RPC URL.
    #[arg(long, default_value = "https://api.devnet.solana.com", env = "SOLANA_RPC")]
    rpc: String,

    /// Path to the owner keypair JSON.
    #[arg(long, env = "LOCUS_KEYPAIR")]
    keypair: PathBuf,

    /// Locus program ID.
    #[arg(long, env = "LOCUS_PROGRAM_ID")]
    program_id: Pubkey,

    /// Poll interval seconds.
    #[arg(long, default_value_t = 30u64)]
    interval: u64,
}

#[derive(Deserialize, Debug)]
struct StateSnapshot {
    /// Merkle root (hex) computed server-side, or raw entries if absent.
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    entries: Vec<StateEntry>,
}

#[derive(Deserialize, Debug)]
struct StateEntry {
    id: String,
    value_hash: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "locus_relayer=info".into()),
        )
        .init();

    let args = Args::parse();
    info!(?args, "starting locus-relayer");

    let payer = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("read keypair {:?}: {}", args.keypair, e))?;

    let client = LocusClient::new(&args.rpc, None, &args.arms, args.program_id, payer);
    let http = reqwest::Client::new();

    let mut last_root: Option<[u8; 32]> = None;
    let mut interval = tokio::time::interval(Duration::from_secs(args.interval));

    loop {
        interval.tick().await;

        match poll_root(&http, &args.arms).await {
            Ok(root) => {
                if Some(root) == last_root {
                    continue;
                }
                info!(new_root = %hex::encode(root), "state changed — committing");
                match client.commit_memory(root).await {
                    Ok(sig) => {
                        last_root = Some(root);
                        info!(%sig, "committed memory root");
                    }
                    Err(e) => error!(?e, "commit_memory failed; will retry next tick"),
                }
            }
            Err(e) => warn!(?e, "failed to read ARMS state; will retry"),
        }
    }
}

async fn poll_root(http: &reqwest::Client, arms: &str) -> Result<[u8; 32]> {
    let snap: StateSnapshot = http
        .get(format!("{}/state-root", arms.trim_end_matches('/')))
        .send()
        .await
        .context("GET /state-root")?
        .error_for_status()?
        .json()
        .await?;

    if let Some(root_hex) = snap.root {
        let bytes = hex::decode(root_hex).context("decode server root hex")?;
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes[..32]);
        return Ok(out);
    }
    Ok(merkle::merkle_root(&snap.entries))
}
