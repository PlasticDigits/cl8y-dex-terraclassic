#!/usr/bin/env bash
# Static checks for the UST1 → cLUNC hourly buyback script (no chain).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "ust1-clunc-buyback static checks"
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
bash -n scripts/mint-swap-burn-ust1-clunc.sh
bash -n scripts/lib/ust1-clunc-buyback-defaults.sh
rg -q 'route/solve/best' scripts/mint-swap-burn-ust1-clunc.sh
rg -q 'unwraps' docs/runbooks/mint-swap-burn-ust1-clunc.md
rg -q 'UST1_CLUNC_DEST:=burn' scripts/lib/ust1-clunc-buyback-defaults.sh
rg -q 'refresh_cmm_uluna' scripts/mint-swap-burn-ust1-clunc.sh
rg -q 'CMM bank uluna' docs/runbooks/mint-swap-burn-ust1-clunc.md
rg -q 'third-party' docs/runbooks/mint-swap-burn-ust1-clunc.md
rg -q 'attach_book_hop_min_returns' scripts/lib/ust1-clunc-buyback-math.py
rg -q 'sim_hop_returns' scripts/mint-swap-burn-ust1-clunc.sh
rg -q 'mint-swap-burn-ust1-clunc' AGENTS.md
echo "OK"
