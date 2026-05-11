//! # locus-demo
//!
//! End-to-end CLI for the hackathon demo.
//!
//! Subcommands:
//! - `ingest`            — pull tx history for N wallets, embed, push to ARMS.
//! - `similar`           — query k behaviorally similar wallets with on-chain attestation.
//! - `show-attestation`  — pretty-print an attestation PDA from devnet.

mod embed;
mod ingest;
mod query;
mod seed;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

#[derive(Parser, Debug)]
#[command(name = "locus-demo", version, about = "Locus + ARMS Solana hackathon demo")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Solana RPC URL.
    #[arg(long, default_value = "https://api.devnet.solana.com", env = "SOLANA_RPC", global = true)]
    rpc: String,

    /// Helius API key (for Enhanced Transactions API).
    #[arg(long, env = "HELIUS_API_KEY", global = true)]
    helius_key: Option<String>,

    /// ARMS service base URL.
    #[arg(long, default_value = "http://localhost:8080", env = "ARMS_URL", global = true)]
    arms: String,

    /// Locus program ID.
    #[arg(long, env = "LOCUS_PROGRAM_ID", global = true)]
    program_id: Option<Pubkey>,

    /// Owner keypair JSON.
    #[arg(long, env = "LOCUS_KEYPAIR", global = true)]
    keypair: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Ingest wallets: pull tx history, build feature vectors, push to ARMS, commit root.
    Ingest {
        /// Seed list of wallets to embed (JSON array of pubkey strings).
        #[arg(long, default_value = "data/wallets.json")]
        wallets: PathBuf,

        /// Optional cap (useful when rate-limited).
        #[arg(long)]
        limit: Option<usize>,

        /// Feature vector dimensionality.
        #[arg(long, default_value_t = 64)]
        dim: usize,
    },
    /// Find k behaviorally similar wallets and produce an on-chain attestation.
    Similar {
        /// Wallet address to query.
        #[arg(long)]
        wallet: Pubkey,

        /// k neighbors.
        #[arg(long, default_value_t = 10)]
        k: usize,
    },
    /// Fetch and pretty-print an attestation from devnet.
    ShowAttestation {
        /// Attestation PDA or tx signature.
        #[arg(long)]
        signature: Option<Signature>,

        #[arg(long)]
        pda: Option<Pubkey>,
    },
    /// Derive a wallet seed list from largest-token-account holders.
    SeedWallets {
        #[arg(long, default_value = "data/wallets.json")]
        out: PathBuf,
        /// SPL token mint(s). Defaults: USDC, USDT, JUP, BONK, JTO.
        #[arg(long)]
        mint: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "locus_demo=info,locus_client=info".into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Ingest { wallets, limit, dim } => {
            ingest::run(ingest::Args {
                wallets,
                limit,
                dim,
                rpc: cli.rpc,
                helius_key: cli.helius_key,
                arms: cli.arms,
                program_id: cli.program_id,
                keypair: cli.keypair,
            })
            .await
        }
        Cmd::Similar { wallet, k } => {
            query::similar(query::SimilarArgs {
                wallet,
                k,
                rpc: cli.rpc,
                helius_key: cli.helius_key,
                arms: cli.arms,
                program_id: cli.program_id,
                keypair: cli.keypair,
            })
            .await
        }
        Cmd::ShowAttestation { signature, pda } => {
            query::show_attestation(query::ShowArgs {
                signature,
                pda,
                rpc: cli.rpc,
                program_id: cli.program_id,
                keypair: cli.keypair,
            })
            .await
        }
        Cmd::SeedWallets { out, mint } => {
            let mints = if mint.is_empty() { seed::default_mints() } else { mint };
            let helius_key = cli
                .helius_key
                .ok_or_else(|| anyhow::anyhow!("HELIUS_API_KEY required"))?;
            seed::run(seed::Args { out, helius_key, mints }).await
        }
    }
}
