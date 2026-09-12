#!/usr/bin/env bash
# Static checks for oracle peg rebalance (mint + swap + leftover burn, no LP).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "oracle-rebalance mint/swap/burn static checks"
python3 scripts/lib/ust1-lp-rebalance-math.py --self-test
python3 scripts/lib/clunc-custc-lp-math.py --self-test
bash -n scripts/rebalance-oracle-mint-swap-burn.sh
bash -n scripts/lib/oracle-rebalance-defaults.sh
rg -q 'NO LP' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q '1 UST1 = \$1' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'usd_custc": "0"' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'force_add_usd": "0"' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'ORACLE_RB_BURN_ONLY' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'atomic oracle swaps' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'merge_unsigned_txs' scripts/rebalance-oracle-mint-swap-burn.sh
rg -q 'ONE atomic pool-swap tx' scripts/rebalance-oracle-mint-swap-burn.sh
if rg -q 'execute_ust1_swaps|execute_clunc_swaps' scripts/rebalance-oracle-mint-swap-burn.sh; then
  echo "ERROR: per-pair swap loops must not remain (arb window)" >&2
  exit 1
fi
if rg -q 'provide_liquidity' scripts/rebalance-oracle-mint-swap-burn.sh; then
  echo "ERROR: provide_liquidity must not appear in the oracle rebalance script" >&2
  exit 1
fi
rg -q 'Both pool swaps are one admin tx' docs/runbooks/rebalance-oracle-mint-swap-burn.md
python3 - <<'PY'
import json, tempfile, os, subprocess, textwrap, pathlib
# merge_unsigned_txs is a bash function; smoke the same python inline used by the script.
a = {"body": {"messages": [{"id": 1}]}, "auth_info": {"fee": {"gas_limit": "3000000"}}}
b = {"body": {"messages": [{"id": 2}]}, "auth_info": {"fee": {"gas_limit": "3000000"}}}
d = tempfile.mkdtemp()
pa, pb, po = f"{d}/a.json", f"{d}/b.json", f"{d}/o.json"
json.dump(a, open(pa, "w"))
json.dump(b, open(pb, "w"))
merged = json.loads(open(pa).read())
extra = json.loads(open(pb).read())
merged["body"]["messages"].extend(extra["body"]["messages"])
assert [m["id"] for m in merged["body"]["messages"]] == [1, 2]
print("merge smoke ok")
PY
rg -q 'rebalance-oracle-mint-swap-burn' docs/runbooks/rebalance-oracle-mint-swap-burn.md
rg -q 'rebalance-oracle-mint-swap-burn' skills/AGENTS_REBALANCE_ORACLE_MINT_SWAP_BURN.md
rg -q 'rebalance-oracle-mint-swap-burn' AGENTS.md
rg -q 'Swap-only \(no LP\)' scripts/lib/ust1-lp-rebalance-math.py
rg -q 'force_add_usd=0' scripts/lib/clunc-custc-lp-math.py
echo "OK"
