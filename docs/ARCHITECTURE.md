# Locus — Architecture

## Layering

Locus is a two-layer system. Layer 1 (ARMS) is an existing open-source spatial memory fabric. Layer 2 (Locus) is the Solana protocol that anchors ARMS state and prices retrieval.

```
Layer 2 — Locus (this repo)
  ├─ programs/locus    on-chain accounts + instructions (Anchor)
  ├─ locus-client      Rust SDK
  ├─ locus-relayer     ARMS -> Solana bridge process
  ├─ locus-demo        end-to-end CLI for the hackathon proof
  └─ arms-service      Axum HTTP wrapper around arms-core

Layer 1 — ARMS (external)
  └─ arms-core crate   Point, Proximity, Merge, Place, Near
```

## On-chain data model

### `AgentMemory` (PDA, one per agent)
- seeds: `["agent", owner]`
- fields: `owner`, `memory_root [u8;32]`, `version u64`, `last_updated i64`, `read_fee_lamports u64`, `write_count u64`, `read_count u64`, `metadata_uri String≤200`

### `RetrievalAttestation` (PDA, one per attested read)
- seeds: `["attest", agent_memory, version, nonce]`
- fields: `agent`, `memory_root`, `version`, `query_hash`, `result_hash`, `requester`, `timestamp`, `nonce`

## Trust model

- The agent owner is the only party that can `commit_memory` or `update_read_fee`. Anyone can `attest_retrieval` — they pay the fee.
- An attestation does *not* prove that the off-chain retrieval was correct. It proves: at devnet time T, requester R paid fee F for query hash Q and was returned result hash H against memory root M v. V.
- Replacing the SHA-256 `result_hash` with a SNARK over the actual k-NN computation closes the trust gap. That's roadmap.

## Merkle root over ARMS state

The relayer computes a sorted binary Merkle tree over `SHA256(id || 0x00 || coord_le_bytes)` leaves and posts the root via `commit_memory`. For the demo this is recomputed end-to-end on each tick; production would use an incremental construction.
