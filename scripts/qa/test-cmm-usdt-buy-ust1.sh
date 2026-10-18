#!/usr/bin/env bash
# Static checks for the CMM USDT → UST1 20-minute drip (no chain).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "cmm-usdt-buy-ust1 static checks"
python3 scripts/lib/ust1-clunc-buyback-math.py self-test
bash -n scripts/cmm-usdt-buy-ust1.sh
bash -n scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_QUOTE_RETRIES' scripts/cmm-usdt-buy-ust1.sh
rg -q 'route/solve/best' scripts/cmm-usdt-buy-ust1.sh
rg -q 'instant_withdraw_cw20' scripts/cmm-usdt-buy-ust1.sh
rg -q 'set_cw20_spender' scripts/cmm-usdt-buy-ust1.sh
rg -q 'cw20_spender_limit' scripts/cmm-usdt-buy-ust1.sh
rg -q 'CMM_USDT_BUY_USDT_PER_TICK:=50' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_FLOOR_USDT:=500' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_ADMIN_KEY:=cl8y2_admin' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_ADMIN_ADDR:=terra1xsecn4' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_INTERVAL_SEC:=1200' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_TREASURY:=terra16j5u6' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM_USDT_BUY_USDT:=terra1z0xe7t5' scripts/lib/cmm-usdt-buy-ust1-defaults.sh
rg -q 'CMM USDT floor reached' scripts/cmm-usdt-buy-ust1.sh
rg -q 'cmm-usdt-buy-ust1' docs/runbooks/cmm-usdt-buy-ust1.md
rg -q -- '--setup-spender' docs/runbooks/cmm-usdt-buy-ust1.md
echo "OK"
