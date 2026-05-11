//! Feature extraction per wallet, per build brief §7.
//!
//! Target dimensionality: 64 (default) or 128.
//! Layout (64-dim default):
//!    0..32   top-32 program-interaction frequency (counts, log-normalized)
//!    32      DEX flag (any SWAP / Jupiter / Raydium / Orca)
//!    33      NFT flag (any NFT_* type)
//!    34      stake flag (any STAKE / unstake)
//!    35      transfer flag (any TRANSFER)
//!    36      token-burn / mint flag
//!    37      total tx count (log + clip 0..1)
//!    38      unique SPL mints touched (log + clip)
//!    39      mean SOL value transferred (log + clip)
//!    40      tx span days (log + clip — proxy for account age)
//!    41      avg tx fee (log + clip)
//!    42..64  reserved / behavioral hash buckets (zero for now)
//!
//! All features normalized to [0, 1] via log1p / clip.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

use crate::ingest::HeliusTx;

#[allow(dead_code)]
fn _value_unused(_v: &Value) {}

const KNOWN_DEX: &[&str] = &[
    "JUPITER", "JUP", "RAYDIUM", "ORCA", "METEORA", "PHOENIX", "OPENBOOK", "LIFINITY",
    "WHIRLPOOL", "SABER",
];
const KNOWN_LENDING: &[&str] = &[
    "KAMINO", "MARGINFI", "SOLEND", "MANGO", "DRIFT", "PORT",
];
const KNOWN_STAKE: &[&str] = &[
    "STAKE_PROGRAM", "MARINADE", "JITO", "LIDO",
];
const NFT_TYPES: &[&str] = &[
    "NFT_SALE", "NFT_MINT", "NFT_LISTING", "NFT_BID", "NFT_CANCEL_LISTING",
    "NFT_GLOBAL_BID", "NFT_AUCTION_CREATED",
];

pub fn extract_features(txs: &[HeliusTx], dim: usize) -> Vec<f32> {
    let mut feat = vec![0.0f32; dim];
    if txs.is_empty() {
        return feat;
    }

    // 0..32 — top-32 program ID frequencies (truncated, log-normalized).
    let mut prog_counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut sources: BTreeMap<String, u32> = BTreeMap::new();
    let mut tx_types: BTreeMap<String, u32> = BTreeMap::new();
    let mut mints: BTreeSet<String> = BTreeSet::new();
    let mut total_native: f64 = 0.0;
    let mut total_fee: u64 = 0;
    let mut timestamps: Vec<i64> = Vec::new();

    for tx in txs {
        for ix in &tx.instructions {
            if !ix.program_id.is_empty() {
                *prog_counts.entry(ix.program_id.clone()).or_insert(0) += 1;
            }
        }
        if !tx.source.is_empty() {
            *sources.entry(tx.source.to_uppercase()).or_insert(0) += 1;
        }
        if !tx.tx_type.is_empty() {
            *tx_types.entry(tx.tx_type.to_uppercase()).or_insert(0) += 1;
        }
        for t in &tx.token_transfers {
            if !t.mint.is_empty() {
                mints.insert(t.mint.clone());
            }
        }
        for n in &tx.native_transfers {
            total_native += (n.amount.abs() as f64) / 1.0e9;
        }
        total_fee = total_fee.saturating_add(tx.fee);
        if let Some(ts) = tx.timestamp {
            timestamps.push(ts);
        }
    }

    // Stable-hash each program ID into one of 32 buckets for dim 0..32.
    for (pid, c) in &prog_counts {
        let bucket = stable_bucket(pid, 32);
        feat[bucket] += (*c as f32).ln_1p();
    }
    // Normalize 0..32 to [0,1] by max.
    if let Some(&mx) = feat[..32]
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    {
        if mx > 0.0 {
            for v in &mut feat[..32] {
                *v /= mx;
            }
        }
    }

    // 32 DEX flag
    let dex_hit = sources.keys().any(|s| KNOWN_DEX.iter().any(|d| s.contains(d)))
        || tx_types.contains_key("SWAP");
    feat[32] = if dex_hit { 1.0 } else { 0.0 };

    // 33 NFT flag
    let nft_hit = NFT_TYPES.iter().any(|t| tx_types.contains_key(*t));
    feat[33] = if nft_hit { 1.0 } else { 0.0 };

    // 34 stake flag
    let stake_hit = KNOWN_STAKE.iter().any(|s| sources.contains_key(*s))
        || tx_types.contains_key("STAKE")
        || tx_types.contains_key("DELEGATE");
    feat[34] = if stake_hit { 1.0 } else { 0.0 };

    // 35 transfer flag
    feat[35] = if tx_types.contains_key("TRANSFER") { 1.0 } else { 0.0 };

    // 36 lending flag
    let lending_hit = sources.keys().any(|s| KNOWN_LENDING.iter().any(|d| s.contains(d)));
    feat[36] = if lending_hit { 1.0 } else { 0.0 };

    // 37 total tx count, log-normalized
    feat[37] = norm_log(txs.len() as f64, 10000.0);
    // 38 unique mints
    feat[38] = norm_log(mints.len() as f64, 500.0);
    // 39 total SOL value transferred
    feat[39] = norm_log(total_native, 10000.0);
    // 40 tx span days
    let span_days = match (timestamps.iter().min(), timestamps.iter().max()) {
        (Some(&lo), Some(&hi)) if hi > lo => ((hi - lo) as f64) / 86400.0,
        _ => 0.0,
    };
    feat[40] = norm_log(span_days, 1000.0);
    // 41 avg fee
    feat[41] = norm_log(total_fee as f64 / txs.len() as f64, 1.0e6);

    // 42..dim — leave as zero (reserved / pad).
    feat
}

pub fn summarize(txs: &[HeliusTx]) -> Value {
    let mut sources: BTreeMap<String, u32> = BTreeMap::new();
    let mut tx_types: BTreeMap<String, u32> = BTreeMap::new();
    for tx in txs {
        if !tx.source.is_empty() {
            *sources.entry(tx.source.clone()).or_insert(0) += 1;
        }
        if !tx.tx_type.is_empty() {
            *tx_types.entry(tx.tx_type.clone()).or_insert(0) += 1;
        }
    }
    json!({
        "tx_count": txs.len(),
        "top_sources": top_map(&sources, 5),
        "top_types": top_map(&tx_types, 5),
    })
}

fn top_map(m: &BTreeMap<String, u32>, k: usize) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = m.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.truncate(k);
    v
}

fn stable_bucket(s: &str, n: usize) -> usize {
    let h = Sha256::digest(s.as_bytes());
    let v = u32::from_le_bytes([h[0], h[1], h[2], h[3]]);
    (v as usize) % n
}

fn norm_log(x: f64, cap: f64) -> f32 {
    let v = (x.max(0.0).ln_1p() / cap.ln_1p()).clamp(0.0, 1.0);
    v as f32
}
