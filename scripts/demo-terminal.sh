#!/usr/bin/env bash
# Paced terminal demo for the hackathon video.
#
# Start OBS / your screen recorder, then run:
#   ./scripts/demo-terminal.sh
#
# The script "types" each line, pauses, then runs. Total runtime ~2:00.
# After it finishes, switch to the browser and run the Playwright recording
# for the Solscan portion (scripts/demo-solscan.spec.ts).
set -euo pipefail

cd "$(dirname "$0")/.."
source .env
export PATH="/usr/bin:$HOME/.local/share/solana/install/active_release/bin:$PATH"

# ---- pacing helpers -------------------------------------------------------
GREEN='\033[1;32m'
CYAN='\033[1;36m'
DIM='\033[2m'
NC='\033[0m'

# Pretend to type a command at human speed, then run it.
type_and_run() {
    local cmd="$1"
    local pause_after="${2:-1.5}"
    printf "${GREEN}\$${NC} "
    local i=0
    while [ $i -lt ${#cmd} ]; do
        printf "%s" "${cmd:$i:1}"
        sleep 0.018
        i=$((i + 1))
    done
    echo
    sleep 0.4
    eval "$cmd"
    sleep "$pause_after"
}

beat() { sleep "${1:-1.5}"; }
heading() { printf "\n${CYAN}── %s ──${NC}\n\n" "$1"; sleep 1; }

clear
# ---- 0:00–0:15 — show the pitch -------------------------------------------
heading "Locus — verifiable AI memory on Solana"
type_and_run "head -40 README.md" 3

# ---- 0:15–0:30 — kill stragglers, start ARMS service ----------------------
heading "Step 1 — start the ARMS memory service (in-memory, dim=64)"
pkill -f arms-service 2>/dev/null || true
sleep 1
./target/release/arms-service --dim 64 --bind 127.0.0.1:8090 > /tmp/arms-service.log 2>&1 &
ARMS_PID=$!
trap 'kill $ARMS_PID 2>/dev/null || true' EXIT
sleep 2
type_and_run "curl -s http://127.0.0.1:8090/healthz" 1
type_and_run "curl -s http://127.0.0.1:8090/state-root | jq" 2

# ---- 0:30–1:30 — ingest 20 wallets ---------------------------------------
heading "Step 2 — pull Solana wallets via Helius, embed in ARMS, commit root on devnet"
type_and_run "./target/release/locus-demo ingest --wallets data/wallets.json --limit 20 --dim 64" 2

# ---- 1:30–2:00 — attested similarity query --------------------------------
heading "Step 3 — attested similarity query (initialize_agent + attest_retrieval on-chain)"
WALLET="$(python3 -c "import json; print(json.load(open('data/wallets.json'))[0])")"
type_and_run "./target/release/locus-demo similar --wallet $WALLET --k 5" 4

# ---- closing slate --------------------------------------------------------
heading "memory committed → retrieval attested → fee paid — all on Solana"
echo -e "${DIM}  next: switch to the browser tab on Solscan to walk through the on-chain attestation."
echo -e "  run 'npx playwright test scripts/demo-solscan.spec.ts' to record the browser portion.${NC}"
echo
sleep 4
