#!/usr/bin/env bash
# Static checks for cLUNC/cUSTC LP mint (rebalance + no-rebalance) scripts (no chain).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "clunc-custc-lp static checks"
python3 scripts/lib/clunc-custc-lp-math.py --self-test
bash -n scripts/rebalance-mint-clunc-custc-lp.sh
bash -n scripts/mint-clunc-custc-lp.sh
bash -n scripts/lib/clunc-custc-lp-defaults.sh
rg -q 'CLUNC_LP_SKIP_SWAP' scripts/rebalance-mint-clunc-custc-lp.sh
rg -q 'CLUNC_LP_ADD_USD' scripts/mint-clunc-custc-lp.sh
rg -q 'NO pair swap' scripts/rebalance-mint-clunc-custc-lp.sh
rg -q 'skip_swap' scripts/lib/clunc-custc-lp-math.py
rg -q 'force_add_usd' scripts/lib/clunc-custc-lp-math.py
rg -q 'mint-clunc-custc-lp' docs/runbooks/mint-clunc-custc-lp.md
rg -q 'mint-clunc-custc-lp' skills/AGENTS_MINT_CLUNC_CUSTC_LP.md
rg -q 'mint-clunc-custc-lp' AGENTS.md
echo "OK"
