//! Simple binary Merkle root over sorted (id, value_hash) entries.

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize, Debug, Clone)]
pub struct StateEntry {
    pub id: String,
    pub value_hash: String,
}

pub fn merkle_root(entries: &[crate::StateEntry]) -> [u8; 32] {
    if entries.is_empty() {
        return [0u8; 32];
    }
    let mut leaves: Vec<[u8; 32]> = entries
        .iter()
        .map(|e| {
            let mut h = Sha256::new();
            h.update(e.id.as_bytes());
            h.update([0u8]);
            let vh = hex::decode(&e.value_hash).unwrap_or_default();
            h.update(&vh);
            h.finalize().into()
        })
        .collect();
    leaves.sort_unstable();

    while leaves.len() > 1 {
        let mut next = Vec::with_capacity((leaves.len() + 1) / 2);
        for pair in leaves.chunks(2) {
            let mut h = Sha256::new();
            h.update(pair[0]);
            if pair.len() == 2 {
                h.update(pair[1]);
            } else {
                h.update(pair[0]);
            }
            next.push(h.finalize().into());
        }
        leaves = next;
    }
    leaves[0]
}
