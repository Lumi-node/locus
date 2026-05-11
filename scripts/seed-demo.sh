#!/usr/bin/env bash
# Run the full demo end-to-end:
#   1) start arms-service
#   2) run ingest on the seed wallet list
#   3) start the relayer
#   4) query similar wallets for a known DEX address
set -euo pipefail

cd "$(dirname "$0")/.."

ARMS_DIM="${ARMS_DIM:-64}"
ARMS_URL="${ARMS_URL:-http://localhost:8080}"
WALLETS="${WALLETS:-data/wallets.json}"
LIMIT="${LIMIT:-1000}"

echo ">> building workspace"
cargo build --release -p arms-service -p locus-relayer -p locus-demo

echo ">> starting arms-service (dim=$ARMS_DIM) in background"
./target/release/arms-service --dim "$ARMS_DIM" &
ARMS_PID=$!
trap 'kill $ARMS_PID 2>/dev/null || true' EXIT
sleep 2

echo ">> ingesting $LIMIT wallets from $WALLETS"
./target/release/locus-demo ingest --wallets "$WALLETS" --limit "$LIMIT" --dim "$ARMS_DIM"

echo ""
echo ">> done. seed-demo finished."
echo "   query example:  ./target/release/locus-demo similar --wallet <pubkey> --k 10"
