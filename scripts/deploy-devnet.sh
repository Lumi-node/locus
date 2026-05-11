#!/usr/bin/env bash
# Deploy the Locus Anchor program to Solana devnet.
#
# Run twice: the first build generates target/deploy/locus-keypair.json,
# which determines the program ID. We sync that ID into Anchor.toml +
# declare_id! and redeploy.
set -euo pipefail

cd "$(dirname "$0")/.."
export PATH="$HOME/.avm/bin:$HOME/.cargo/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"

KEYPAIR="${LOCUS_KEYPAIR:-$HOME/.config/solana/locus-dev.json}"
PROGRAM_KEYPAIR="target/deploy/locus-keypair.json"

solana config set --url devnet --keypair "$KEYPAIR" >/dev/null
echo ">> wallet:  $(solana address)"
echo ">> balance: $(solana balance)"

if [[ ! -f "$PROGRAM_KEYPAIR" ]]; then
  echo ">> initial anchor build (creates program keypair)"
  anchor build
fi

PROGRAM_ID="$(solana address -k "$PROGRAM_KEYPAIR")"
echo ">> program id: $PROGRAM_ID"

# Sync the program ID into source.
LIB="programs/locus/src/lib.rs"
ANCHOR="Anchor.toml"
sed -i "s/^declare_id!(\".*\");/declare_id!(\"$PROGRAM_ID\");/" "$LIB"
sed -i "s/^locus = \".*\"/locus = \"$PROGRAM_ID\"/" "$ANCHOR"

echo ">> rebuilding with synced program ID"
anchor build

echo ">> deploying"
anchor deploy --provider.cluster devnet

# Persist program ID for the demo.
ENV_FILE=".env"
if [[ -f "$ENV_FILE" ]]; then
  if grep -q "^LOCUS_PROGRAM_ID=" "$ENV_FILE"; then
    sed -i "s|^LOCUS_PROGRAM_ID=.*|LOCUS_PROGRAM_ID=$PROGRAM_ID|" "$ENV_FILE"
  else
    echo "LOCUS_PROGRAM_ID=$PROGRAM_ID" >> "$ENV_FILE"
  fi
fi

echo ""
echo ">> deployed Locus to devnet"
echo "   program id: $PROGRAM_ID"
echo "   solscan:    https://solscan.io/account/$PROGRAM_ID?cluster=devnet"
