//! seed-wallets: derive a wallet list from "largest holders" of popular SPL tokens.
//!
//! Uses the standard Solana JSON-RPC `getTokenLargestAccounts` against Helius
//! mainnet, then `getAccountInfo` to resolve each token account back to its
//! owner wallet.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tracing::{info, warn};

pub struct Args {
    pub out: PathBuf,
    pub helius_key: String,
    pub mints: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LargestAccount {
    address: String,
    amount: String,
}

pub async fn run(args: Args) -> Result<()> {
    let rpc = format!(
        "https://mainnet.helius-rpc.com/?api-key={}",
        args.helius_key
    );
    let http = reqwest::Client::new();
    let mut wallets: BTreeSet<String> = BTreeSet::new();

    for mint in &args.mints {
        info!(%mint, "fetching largest token accounts");
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenLargestAccounts",
            "params": [mint, {"commitment": "confirmed"}],
        });
        let resp: Value = http
            .post(&rpc)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let accs = match resp["result"]["value"].as_array() {
            Some(a) => a.clone(),
            None => {
                warn!(?resp, %mint, "no largest-accounts result");
                continue;
            }
        };
        for entry in accs {
            let ta = match entry["address"].as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            // Resolve token account -> owner wallet.
            match resolve_owner(&http, &rpc, &ta).await {
                Ok(Some(owner)) => {
                    wallets.insert(owner);
                }
                Ok(None) => {}
                Err(e) => warn!(%ta, ?e, "owner lookup failed"),
            }
        }
    }

    let list: Vec<String> = wallets.into_iter().collect();
    info!(count = list.len(), out = %args.out.display(), "writing seed list");
    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&args.out, serde_json::to_vec_pretty(&list)?)
        .with_context(|| format!("write {}", args.out.display()))?;
    Ok(())
}

async fn resolve_owner(http: &reqwest::Client, rpc: &str, token_account: &str) -> Result<Option<String>> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [token_account, {"encoding": "jsonParsed", "commitment": "confirmed"}],
    });
    let resp: Value = http
        .post(rpc)
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let owner = resp["result"]["value"]["data"]["parsed"]["info"]["owner"]
        .as_str()
        .map(|s| s.to_string());
    if owner.is_none() {
        return Err(anyhow!("no owner field: {resp}"));
    }
    Ok(owner)
}

/// Default mint set: USDC, USDT, JUP, BONK, JTO — produces ~100 diverse holders.
pub fn default_mints() -> Vec<String> {
    vec![
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(), // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(), // USDT
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN".to_string(), // JUP
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK
        "jtojtomepa8beP8AuQc6eXt5FriJwfFMwQx2v2f9mCL".to_string(), // JTO
    ]
}
