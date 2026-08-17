#!/usr/bin/env bash
# Verification for GitLab #538 — post-#536 follow-ups:
#   1. Factory pointer after migrate (LocalTerra config.discount_registry)
#   2. Dedicated LocalTerra create_pair inherit (no follow-up SetDiscountRegistry)
#   3. dApp getPairDiscountRegistry prefers GetDiscountRegistry (raw fallback)
#
# Columbus-5 ops item 1 is recorded in docs (not re-run here).
#
# Refs: skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
#       skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md
#       docs/contracts-terraclassic.md#factory-discount-registry-snapshot-gitlab-536
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
skip(){ RESULTS+=("SKIP  $1"); SKIP=$((SKIP+1)); echo "  [SKIP] $1"; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #538 — factory inherit check + dApp GetDiscountRegistry"
echo "════════════════════════════════════════════════════════════════"

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/pairDiscountRegistry.test.ts \
    src/utils/__tests__/pairDiscountRegistry.test.ts \
    src/hooks/__tests__/useLimitOrderMakerFeeRates.test.tsx \
    src/pages/PoolPage.feeDiscountRegistryBanner.test.tsx \
    src/pages/SwapPage.feeDiscountRegistryBanner.test.tsx
}

run_smart_query_first() {
  python3 - <<'PY'
from pathlib import Path
p = Path("frontend-dapp/src/services/terraclassic/pairDiscountRegistry.ts").read_text()
fn = p.split("export async function getPairDiscountRegistry", 1)
if len(fn) != 2:
    raise SystemExit("getPairDiscountRegistry not found")
body = fn[1]
i_smart = body.find("get_discount_registry")
i_raw = body.find("queryContractRaw")
if not (0 <= i_smart < i_raw):
    raise SystemExit(f"smart query must precede raw fallback (smart={i_smart} raw={i_raw})")
settings = Path("frontend-dapp/src/services/terraclassic/settings.ts").read_text()
if "getPairDiscountRegistry" not in settings:
    raise SystemExit("settings.ts must re-export getPairDiscountRegistry")
if "from './pairDiscountRegistry'" not in settings and 'from "./pairDiscountRegistry"' not in settings:
    raise SystemExit("settings.ts must re-export from pairDiscountRegistry.ts")
PY
}

run_docs() {
  set -euo pipefail
  rg -q "538" skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
  rg -q "verify-issue-538" skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
  rg -q "F538-1" skills/AGENTS_FACTORY_DISCOUNT_REGISTRY.md
  rg -q "GetDiscountRegistry" skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md
  rg -q "smart-query-first|#538" skills/AGENTS_FRONTEND_PAIR_FEE_DISCOUNT.md
  rg -q "verify-issue-538" AGENTS.md
  rg -q "AGENTS_FACTORY_DISCOUNT_REGISTRY" AGENTS.md
  rg -q "F538-1" docs/frontend.md
  rg -q "F538-3" docs/frontend.md
  rg -q "#538" docs/reference/fee-discount-tiers.md
  rg -q "gitlab-538" docs/contracts-terraclassic.md
  rg -q "verify-issue-538" docs/testing.md
  rg -q "assert_pair_inherited_discount_registry" scripts/deploy-dex-local.sh
  test -f scripts/qa/localterra-create-pair-inherit.sh
  # Dedicated inherit script must never execute SetDiscountRegistry.
  if grep -E '"set_discount_registry"|set_discount_registry[[:space:]]*:' scripts/qa/localterra-create-pair-inherit.sh; then
    echo "inherit script must not execute set_discount_registry" >&2
    exit 1
  fi
}

echo ""
echo "── first pass ──"
run_step "frontend unit: smart-query-first + fee chrome" run_frontend_unit
run_step "code: GetDiscountRegistry before LCD raw; settings re-export" run_smart_query_first
run_step "docs: F538 + skills + AGENTS.md + inherit script" run_docs

echo ""
echo "[LocalTerra create_pair inherit (no SetDiscountRegistry)]"
if make has-localterra >/dev/null 2>&1; then
  set +e
  ./scripts/qa/localterra-create-pair-inherit.sh
  INH_ST=$?
  set -e
  if [[ "$INH_ST" -eq 0 ]]; then
    ok "LocalTerra create_pair inherit (no SetDiscountRegistry)"
  elif [[ "$INH_ST" -eq 2 ]]; then
    skip "LocalTerra inherit (deploy env missing/stale — not a #538 code fail)"
  else
    bad "LocalTerra create_pair inherit (no SetDiscountRegistry)"
  fi
else
  echo "  Probe: make has-localterra failed. Cloud Agent: make setup-cloud-localterra"
  skip "LocalTerra inherit (chain not running — probe ran)"
fi

echo ""
echo "── retest ──"
run_step "retest frontend unit: smart-query-first + fee chrome" run_frontend_unit
run_step "retest code: GetDiscountRegistry before LCD raw" run_smart_query_first

echo ""
echo "[retest LocalTerra inherit]"
if make has-localterra >/dev/null 2>&1; then
  set +e
  ./scripts/qa/localterra-create-pair-inherit.sh
  INH_ST=$?
  set -e
  if [[ "$INH_ST" -eq 0 ]]; then
    ok "retest LocalTerra create_pair inherit"
  elif [[ "$INH_ST" -eq 2 ]]; then
    skip "retest LocalTerra inherit (deploy env missing/stale)"
  else
    bad "retest LocalTerra create_pair inherit"
  fi
else
  skip "retest LocalTerra inherit (chain not running)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #538 results: ${PASS} passed, ${FAIL} failed, ${SKIP} skipped"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
