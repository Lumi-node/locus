# Locus Demo — what the judges will see

The submission demo is a single command line. There is no UI.

## What the demo proves

1. The Locus Anchor program is deployed on devnet.
2. An agent's memory root can be committed on-chain (`commit_memory`).
3. The relayer keeps that root in sync as ARMS state changes.
4. A behavioral similarity query against 10K Solana wallets returns believable neighbors.
5. Every query produces a `RetrievalAttestation` PDA on devnet with the query hash, result hash, version, and a real fee transfer.

## Demo script — terminal

```bash
# 1. show the program is live
solana program show <PROGRAM_ID>

# 2. start the ARMS HTTP service
cargo run --release -p arms-service -- --dim 64

# 3. ingest wallet behavioral features
cargo run --release -p locus-demo -- ingest --wallets data/wallets.json --limit 10000

# 4. commit shows up on-chain (relayer logs)
cargo run --release -p locus-relayer -- --agent <OWNER> --arms http://localhost:8080 --interval 10

# 5. attested similarity query
cargo run --release -p locus-demo -- similar --wallet <DEX_WALLET> --k 10

# 6. show the attestation on-chain
cargo run --release -p locus-demo -- show-attestation --signature <SIG>
```

## What the technical demo video needs to capture

(per the build brief §14 — keep ≤3 minutes)

1. README header on screen, voiceover hook.
2. Empty ARMS state → first `state-root` call.
3. Ingest progress bar + final commit tx on Solscan.
4. `similar` command returning neighbors + clicking the Solscan attestation link.
5. Solscan view of the `RetrievalAttestation` PDA with all fields decoded.

## Known limitations (be honest in the README + pitch)

- Devnet only. Mainnet is roadmap.
- Brute-force kNN inside `arms-service` for now; HAT plug-in is wired but not exercised end-to-end yet.
- `result_hash` is SHA-256, not a SNARK. Future work.
- Feature extraction is the easy half — feature *quality* matters; we use 12 hand-picked behavioral signals.
