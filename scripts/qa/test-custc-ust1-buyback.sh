#!/usr/bin/env bash
# Static checks for the $200 cUSTC → UST1 hourly buyback script (no chain).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "custc-ust1-buyback static checks"
python3 scripts/lib/custc-ust1-buyback-math.py self-test
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
bash -n scripts/mint-swap-custc-ust1.sh
bash -n scripts/lib/custc-ust1-buyback-defaults.sh
rg -q 'route/solve/best' scripts/mint-swap-custc-ust1.sh
rg -q 'window-redeem' docs/runbooks/mint-swap-custc-ust1.md
rg -q 'CUSTC_UST1_DEST:=cmm' scripts/lib/custc-ust1-buyback-defaults.sh
rg -q 'CUSTC_UST1_MINT_USD:=200' scripts/lib/custc-ust1-buyback-defaults.sh
rg -q 'refresh_cmm_vfdusd' scripts/mint-swap-custc-ust1.sh
rg -q 'CMM vFDUSD' docs/runbooks/mint-swap-custc-ust1.md
rg -q 'third-party' docs/runbooks/mint-swap-custc-ust1.md
rg -q 'fdusd_per_vfdusd' scripts/mint-swap-custc-ust1.sh
rg -q 'mint-raw-usd' scripts/lib/custc-ust1-buyback-math.py
rg -q 'Never unwrap. Never window-redeem' skills/AGENTS_MINT_SWAP_CUSTC_UST1.md
rg -q 'mint-swap-custc-ust1' AGENTS.md
echo "OK"
