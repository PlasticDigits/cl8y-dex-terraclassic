#!/usr/bin/env bash
# Static checks for mint-clunc-usdt-lp.sh (no chain).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "clunc-usdt-lp static checks"
python3 scripts/lib/clunc-usdt-lp-math.py --self-test
bash -n scripts/mint-clunc-usdt-lp.sh
bash -n scripts/lib/clunc-usdt-lp-defaults.sh
rg -q 'cl8y2_admin' scripts/lib/clunc-usdt-lp-defaults.sh
rg -q 'do not mint USDT' scripts/lib/clunc-usdt-lp-math.py
rg -q 'Never mint USDT' scripts/mint-clunc-usdt-lp.sh
rg -q 'create_pair' scripts/mint-clunc-usdt-lp.sh
rg -q --regexp 'py_ge' scripts/mint-clunc-usdt-lp.sh
rg -q --regexp 'pair not found' scripts/mint-clunc-usdt-lp.sh
rg -q --regexp 'terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4' \
  scripts/lib/clunc-usdt-lp-defaults.sh
# bash 64-bit cannot compare 1e21 USDT raw amounts
if rg -n --regexp 'BAL_USDT.*-lt|USDT_RAW.*-lt|[[:space:]]-lt[[:space:]]+"\$USDT' scripts/mint-clunc-usdt-lp.sh; then
  echo "ERROR: bash -lt on USDT raw amounts overflows int64" >&2
  exit 1
fi
echo "OK"
