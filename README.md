# Locus

> Verifiable AI memory on Solana — built on ARMS.

Solana agents are stateless. The Agent Payments Protocol launched this quarter and agents are trading, voting, and coordinating on-chain — but none of them have persistent, verifiable, queryable memory. **Locus** is the Solana protocol that turns [ARMS](https://github.com/Lumi-node/ARMS), a spatial memory fabric in Rust, into a payable memory network: memory roots committed on-chain, pay-per-query economics, and a cryptographic retrieval attestation for every AI-driven decision.

This repo is the hackathon submission for the Solana Frontier Hackathon (Colosseum).

---

## Live on devnet

Proof artifacts you can click right now:

- **Program ID:** [`C6AJ43ZpzPLtmcwDS1FQP7cQXtWHNwsLty5ijdLTxzmK`](https://solscan.io/account/C6AJ43ZpzPLtmcwDS1FQP7cQXtWHNwsLty5ijdLTxzmK?cluster=devnet)
- **Program deploy tx:** [`2V2Yrot…d2n`](https://solscan.io/tx/2V2YrotUpxWSkeBz63cHUXkffQZZQJzZgFVyJPV4PaJsctJVPUTfYym5PJxiusXP6D191JEu9tMnrZCkCg8qtd2n?cluster=devnet)
- **`initialize_agent` tx:** [`2FPgj7p…RLZ`](https://solscan.io/tx/2FPgj7p5yw3hU4wpTbEZY46kbQavPBp2FAwvK4ZtyKRVocY2kuVyUoW8VkhMtJAEy1p9mkC5Nyo5jUi4D9vCwRLZ?cluster=devnet)
- **`commit_memory` tx** (root `bd8e1419…5f16`): [`3EJotzn…AywF`](https://solscan.io/tx/3EJotznRcuofQmoKy1PgDBqZZYSUywXKth6eGRQPeGxJW6ejfhfQPPwqz9hP6N4eRWAfoxbpxMa9Tgm2RgV4AywF?cluster=devnet)
- **`attest_retrieval` tx:** [`4UxWaB1…j8iX`](https://solscan.io/tx/4UxWaB1TXvJdxwEfchLd7cKnAS17MhXDJ8zcfhNGzg5CLfbzYdSDghvjbQkL59zp5aoGdFGnsTJ2koKYxHxLj8iX?cluster=devnet)
- **`RetrievalAttestation` PDA:** [`9z8SwfDQRu2Nt1wbtQooLitpb3WQXge8PzxdyYebQLVt`](https://solscan.io/account/9z8SwfDQRu2Nt1wbtQooLitpb3WQXge8PzxdyYebQLVt?cluster=devnet)

What the attestation records (all on-chain, all verifiable):

| Field | Value |
| --- | --- |
| `agent` | PDA `[b"agent", owner]` |
| `version` | 1 |
| `memory_root` | `bd8e1419cf6699efd55d6a573f9c6a2f942a51c3c593f22e19fddf5caf8f5f16` |
| `query_hash` | `d5d98ebe29342856fb0f3e6d6412f219122e825b0f8b6164c85b72234587946f` |
| `result_hash` | `4553cb46fa3ec8d8aed75376d154667094cebce6e5c1ce2a3b76ac495cdd11b6` |
| `nonce` | `16070694091605954182` |

Behavioral similarity result (query wallet was Bubblegum-heavy → neighbors are also Bubblegum-heavy):

```
=== top 5 behaviorally similar wallets to 1tjoQzWg21ceMAhyLVs4s2Zc1Pm27mLTcJ9EfEFu7km ===
   1. 1tjoQzWg21ceMAhyLVs4s2Zc1Pm27mLTcJ9EfEFu7km   dist=0.0000
   2. AGkGWK1R669KDT4FCqgDgK7PgahGJPjD4J9xmVjuL9kn  dist=0.8792   bubblegum + compressed NFT
   3. 5oNhn2ESFHgg52xiEKBt93cEEnbsoUoekcxXsWpLbHUJ  dist=0.9779   bubblegum + transfers
   4. 5SybwTvGno6SBApbGinXyLqhnPzs47aaBG4PShyPT8ey  dist=1.1390   high-volume transfers
   5. 9xSi6onCqhLuXTgodd5wB7EXv2WwuYPZyZwDyruGKL6v  dist=1.1918   compressed NFT mint heavy
```

---

## Pitch video — auto-generated, narrated

The 3:00 pitch video lives at [`demo/locus-pitch-final.mp4`](demo/locus-pitch-final.mp4). The whole thing is reproducible from this repo:

```bash
# 1. record the 6-section pitch deck at 1080p (3:02 webm)
npx playwright install chromium
npx playwright test scripts/demo-solscan.spec.ts

# 2. synthesize TTS narration (edge-tts, neural voice, auto-paced per segment)
pip install --user edge-tts
python3 scripts/generate-narration.py

# 3. mux narration onto the recording, output a YouTube-ready MP4
./scripts/mux-demo.sh
```

Each section's narration text + timing window lives in [`demo/narration.json`](demo/narration.json) — edit + re-run to retune voice, pacing, or copy.

## 30-second demo

```bash
# 1) one-time setup
./scripts/airdrop.sh                       # creates dev keypair + airdrops devnet SOL
./scripts/deploy-devnet.sh                 # builds + deploys the Anchor program

# 2) run the demo
./scripts/seed-demo.sh                     # spins up ARMS service + ingests wallets
cargo run --release -p locus-demo -- similar --wallet <pubkey> --k 10
```

That last command finds the 10 most behaviorally similar Solana wallets to the one you pass, calls `attest_retrieval` on devnet, and prints the on-chain attestation signature.

---

## Architecture

```
                +-------------------+
                |    locus-demo     |       CLI: ingest / similar / show-attestation
                +---------+---------+
                          |
                          v
+-------------+   +-------+--------+        Rust SDK: agent ops + attested queries
|   wallet    |-->|  locus-client  |
+-------------+   +-------+--------+
                          |
              +-----------+-----------+
              |                       |
              v                       v
   +------------------+      +------------------+
   |  arms-service    |      |  Locus program   |     Solana devnet
   |  (Axum, wraps    |      |  (Anchor)        |
   |   arms-core)     |      |                  |
   +--------+---------+      +------------------+
            ^                          ^
            |                          |
            |   +------------------+   |
            +---+  locus-relayer   +---+    poll ARMS, merkle, commit_memory
                +------------------+
```

- **`programs/locus/`** — Anchor program: `AgentMemory` + `RetrievalAttestation` accounts, `initialize_agent` / `commit_memory` / `attest_retrieval` / `update_read_fee` / `close_agent` instructions.
- **`locus-client/`** — Rust SDK. Agents/devs talk to Locus + ARMS through one type (`LocusClient`).
- **`locus-relayer/`** — long-running process that recomputes the ARMS state's Merkle root and commits it on a poll interval.
- **`locus-demo/`** — end-to-end CLI: pull Helius tx history, build behavioral feature vectors, push to ARMS, run attested similarity queries.
- **`arms-service/`** — thin Axum HTTP wrapper around the `arms-core` crate (ARMS itself stays untouched).

---

## Build from source

Requirements (`./scripts/install-toolchain.sh` automates these on Linux — review first):
- Rust 1.79+ (`rustup show`)
- Solana CLI 1.18+ (`solana --version`)
- Anchor 0.30.1 (`anchor --version`)
- Yarn or npm (for the TS tests under `tests/`)

```bash
# Build the Solana program (BPF/SBF)
anchor build

# Build all Rust crates (host)
cargo build --release
```

The Locus program declares its ID in `programs/locus/src/lib.rs` via `declare_id!(...)`. After the first `anchor deploy`, copy the printed program ID into:
1. `declare_id!(...)` in [`programs/locus/src/lib.rs`](programs/locus/src/lib.rs)
2. The `[programs.devnet]` entry in [`Anchor.toml`](Anchor.toml)
3. `LOCUS_PROGRAM_ID` in your `.env`

Then rebuild and redeploy once so the on-chain ID matches the embedded ID.

---

## Run from scratch

```bash
# 1. start the ARMS service (in-memory; dim 64)
cargo run --release -p arms-service -- --dim 64

# 2. start the relayer (in another shell)
export LOCUS_KEYPAIR=$HOME/.config/solana/locus-dev.json
export LOCUS_PROGRAM_ID=<your deployed program id>
cargo run --release -p locus-relayer -- \
  --agent <owner pubkey> --arms http://localhost:8080 --interval 30

# 3. ingest wallets + commit root
cargo run --release -p locus-demo -- ingest \
  --wallets data/wallets.json --limit 10000 --dim 64

# 4. attested similarity query
cargo run --release -p locus-demo -- similar \
  --wallet <pubkey> --k 10
```

---

## Background research

Locus is not a hackathon-spawned concept — it productizes pre-existing AI memory research onto Solana.

- **ARMS** — Attention Reasoning Memory Store. Spatial memory fabric in Rust. *Position IS relationship.* → [github.com/Lumi-node/ARMS](https://github.com/Lumi-node/ARMS) · [Zenodo DOI](https://doi.org/10.5281/zenodo.18263613)
- **HAT** — Hierarchical Attention Tree retrieval algorithm. 100% recall, 70× faster build than HNSW. → [github.com/Lumi-node/HAT](https://github.com/Lumi-node/HAT)
- **infinite-context** — productized HAT variant: 11M+ tokens, 0.51 ms latency, 100% recall. → [github.com/Lumi-node/infinite-context](https://github.com/Lumi-node/infinite-context)
- **HuggingFace models** — [AutomateCapture/HAT](https://huggingface.co/AutomateCapture/HAT) · [AutomateCapture/ARMS](https://huggingface.co/AutomateCapture/ARMS)
- **Author** — Andrew Young, independent AI researcher. ORCID: [0009-0002-3685-4289](https://orcid.org/0009-0002-3685-4289)

---

## Roadmap

- Mainnet deployment with a token-incentivized DePIN coordinator for ARMS nodes
- ZK proofs of retrieval (replace the SHA-256 result hash with a SNARK over the actual neighbor set)
- First-class integrations with Solana agent frameworks (Agent Payments Protocol, Jupiter agents, governance agents)
- Onboarding the first 10 agent teams via the Colosseum accelerator

---

## Team

Andrew Young — independent AI researcher (Lumi Node / Automate Capture LLC). [andrew@automate-capture.com](mailto:andrew@automate-capture.com)

## License

MIT — see [LICENSE](LICENSE).
