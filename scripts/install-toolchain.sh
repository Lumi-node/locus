#!/usr/bin/env bash
# Install the Solana + Anchor toolchain needed to build this repo.
#
# Idempotent: skips installers whose binaries are already on PATH.
# Adds the Solana bin dir to ~/.bashrc and ~/.zshrc if it isn't there.
set -euo pipefail

SOLANA_VERSION="v1.18.26"
ANCHOR_VERSION="0.30.1"

solana_bin="$HOME/.local/share/solana/install/active_release/bin"

if ! command -v solana >/dev/null 2>&1 && [[ ! -x "$solana_bin/solana" ]]; then
  echo ">> installing Solana CLI $SOLANA_VERSION (Anza)"
  curl -sSfL "https://release.anza.xyz/$SOLANA_VERSION/install" -o /tmp/anza-install.sh
  sh /tmp/anza-install.sh
else
  echo ">> solana already installed: $(solana --version 2>/dev/null || "$solana_bin/solana" --version)"
fi

# Make sure subsequent shells see Solana on PATH.
for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
  [[ -f "$rc" ]] || continue
  if ! grep -q "solana/install/active_release/bin" "$rc"; then
    echo ">> appending Solana PATH to $rc"
    echo "export PATH=\"\$HOME/.local/share/solana/install/active_release/bin:\$PATH\"" >> "$rc"
  fi
done

export PATH="$solana_bin:$PATH"

if ! command -v avm >/dev/null 2>&1; then
  echo ">> installing Anchor version manager (AVM)"
  cargo install --git https://github.com/coral-xyz/anchor avm --force
fi

if ! command -v anchor >/dev/null 2>&1 || [[ "$(anchor --version | awk '{print $2}')" != "$ANCHOR_VERSION" ]]; then
  echo ">> installing anchor $ANCHOR_VERSION via avm"
  avm install "$ANCHOR_VERSION"
  avm use "$ANCHOR_VERSION"
fi

echo ""
echo ">> toolchain ready:"
echo "   solana   : $(solana --version)"
echo "   anchor   : $(anchor --version)"
echo ""
echo ">> open a new shell (or source ~/.bashrc) so PATH is set globally."
