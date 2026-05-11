#!/usr/bin/env bash
# Airdrop SOL to the dev wallet (devnet only).
set -euo pipefail

KEYPAIR="${LOCUS_KEYPAIR:-$HOME/.config/solana/locus-dev.json}"
AMOUNT="${1:-5}"

if [[ ! -f "$KEYPAIR" ]]; then
  echo ">> generating new keypair at $KEYPAIR"
  mkdir -p "$(dirname "$KEYPAIR")"
  solana-keygen new --no-bip39-passphrase --outfile "$KEYPAIR"
fi

solana config set --url devnet >/dev/null
solana config set --keypair "$KEYPAIR" >/dev/null

echo ">> wallet:  $(solana address)"
echo ">> before:  $(solana balance)"
solana airdrop "$AMOUNT" || true
echo ">> after:   $(solana balance)"
