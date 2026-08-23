#!/usr/bin/env bash
# Automated verification for GitLab #600 — post-merge !400 LocalTerra E9 +
# columbus-5 USTR→USTC unwrap gas (parent #599).
#
# Proves (docs + children + optional LocalTerra E9 + optional columbus-5 LCD):
#   1. Q8 / M600-1–M600-8 documented and crosslinked.
#   2. Child make verify-issue-599 (envelope / inventory / docs).
#   3. Child make verify-issue-587 (wrap combo still one-tx; P599-3).
#   4. Direct mapper unwrap stays 800k; hub combo stays 3,110,000 (P599-4 / P599-5).
#   5. Optional: Playwright wrap-swap E9 (+ E7) e2e-tx (1 worker).
#   6. Optional: VERIFY600_COLUMBUS_TX=<hash> LCD gas_used < gas_wanted (P599-2).
#
# VERIFY600_SKIP_CHILDREN=1 — docs + envelope floors only.
# VERIFY600_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra / e2e-tx is missing.
# VERIFY600_SKIP_E2E=1 — skip wrap-swap e2e-tx even if chain is up.
# VERIFY600_COLUMBUS_TX=<hash> — query columbus-5 LCD for gasWanted/gasUsed.
# VERIFY600_REQUIRE_MAINNET=1 — FAIL when columbus-5 hash is unset / LCD miss.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_600.md, docs/qa-invariants.md § Q8
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

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
echo "  GitLab #600 — post-merge !400 unwrap+≥2hop chain QA"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31600}"

has_chain() {
  timeout 20 make -s has-localterra >/dev/null 2>&1
}

COLUMBUS_LCD="${VERIFY600_COLUMBUS_LCD:-https://terra-classic-lcd.publicnode.com}"

run_step "docs: Q8 M600-1–M600-8 + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_POST_MERGE_OPS_600.md
    grep -qE "\*\*M600-1\*\*" docs/qa-invariants.md
    grep -qE "\*\*M600-8\*\*" docs/qa-invariants.md
    grep -qE "post-merge-ops-600" docs/qa-invariants.md
    grep -qE "\*\*M600-1" skills/AGENTS_POST_MERGE_OPS_600.md
    grep -qE "make verify-issue-600" skills/AGENTS_POST_MERGE_OPS_600.md
    grep -qE "AGENTS_POST_MERGE_OPS_600" AGENTS.md
    grep -qE "verify-issue-600" AGENTS.md
    grep -qE "verify-issue-600" Makefile
    grep -qE "#600" docs/testing.md
    grep -qE "AGENTS_POST_MERGE_OPS_600" docs/README.md
    grep -qE "#600" skills/AGENTS_TERRACLASSIC_GAS.md
    grep -qE "verify-issue-600" skills/AGENTS_TERRACLASSIC_GAS.md
    grep -qE "#600" skills/AGENTS_FRONTEND_HYBRID_ALWAYS_ON.md
    grep -qE "#600" skills/AGENTS_POST_MERGE_OPS_590.md
    grep -qE "E9" frontend-dapp/e2e/wrap-swap.spec.ts
    grep -qE "gas_used < gas_wanted|#600" frontend-dapp/e2e/wrap-swap.spec.ts
    grep -qE "assertSuccessTxGasUsedLtWanted" frontend-dapp/e2e/helpers/chain.ts
    grep -qE "Do \*\*not\*\* reopen" skills/AGENTS_POST_MERGE_OPS_600.md
    grep -qE "UNWRAP_ROUTER_COMBO_OVERHEAD_GAS" docs/frontend.md
    grep -qE "#600" NATIVE_TOKEN_WRAPPING.md
    grep -qE "verify-issue-600" scripts/qa/README.md
  '

run_step "envelope: combo 3.11M; direct unwrap 800k; wrap+2hop 2.71M" \
  bash -c '
    set -euo pipefail
    rg -q "UNWRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000" frontend-dapp/src/utils/constants.ts
    rg -q "UNWRAP_GAS_LIMIT = 800_000" frontend-dapp/src/utils/constants.ts
    rg -q "WRAP_ROUTER_COMBO_OVERHEAD_GAS = 400_000" frontend-dapp/src/utils/constants.ts
    rg -q "send_2hop_unwrap_ustc" frontend-dapp/src/services/terraclassic/terraGasRetailInventory.ts
    rg -q "3,110,000" docs/frontend.md
    rg -q "unwrapRouterComboOverheadGas" frontend-dapp/src/services/terraclassic/terraGas.ts
    # Direct mapper unwrap must stay the 800k named floor — combo is N≥2 only.
    if ! rg -n "unwrapOutput && hops >= 2" frontend-dapp/src/services/terraclassic/terraGas.ts; then
      echo "unwrap combo must gate on hops >= 2" >&2
      exit 1
    fi
    # Do not attach hybrid / book_input to native wrap/unwrap (H596-7).
    rg -q "never copy hybrid" frontend-dapp/src/services/terraclassic/router.ts
  '

if [[ "${VERIFY600_SKIP_CHILDREN:-0}" = "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY600_SKIP_CHILDREN=1"
  skip "child verify-issue-* (VERIFY600_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-599" make verify-issue-599
  run_step "child: make verify-issue-587" make verify-issue-587
fi

if has_chain; then
  if [[ "${VERIFY600_SKIP_E2E:-0}" = "1" ]]; then
    skip "playwright wrap-swap E9/E7 e2e-tx (VERIFY600_SKIP_E2E=1)"
  elif [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
    if [[ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
      COMMON="$(git rev-parse --git-common-dir)"
      MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
      if [[ -f "$MAIN_ROOT/frontend-dapp/.env.local" && "$MAIN_ROOT" != "$REPO_ROOT" ]]; then
        cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
      fi
    fi
    run_step "playwright wrap-swap E9 + E7 e2e-tx (1 worker)" \
      bash -c 'CI=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31600}" PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-31600}" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-tx --workers=1 e2e/wrap-swap.spec.ts -g "E9:|E7:"'
  else
    if [[ "${VERIFY600_REQUIRE_CHAIN:-0}" = "1" ]]; then
      bad "playwright wrap-swap E9 (Playwright not installed)"
    else
      skip "playwright wrap-swap E9 (no Playwright install)"
    fi
  fi
else
  if [[ "${VERIFY600_REQUIRE_CHAIN:-0}" = "1" ]]; then
    bad "LocalTerra required (VERIFY600_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "wrap-swap E9/E7 (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

query_columbus_tx() {
  local hash="$1"
  local url="${COLUMBUS_LCD}/cosmos/tx/v1beta1/txs/${hash}"
  echo "  LCD GET ${url}"
  local body
  if ! body="$(curl -fsS --max-time 30 "${url}")"; then
    echo "columbus-5 LCD fetch failed for ${hash}" >&2
    return 1
  fi
  local wanted used
  wanted="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print(d.get("tx_response",{}).get("gas_wanted",""))' "${body}")"
  used="$(python3 -c 'import json,sys; d=json.loads(sys.argv[1]); print(d.get("tx_response",{}).get("gas_used",""))' "${body}")"
  echo "  gasWanted=${wanted} gasUsed=${used} hash=${hash}"
  python3 -c '
import sys
wanted, used = int(sys.argv[1]), int(sys.argv[2])
if used <= 0 or wanted <= 0:
    raise SystemExit("invalid gas fields")
if used >= wanted:
    raise SystemExit(f"OOG or tight: gasUsed {used} >= gasWanted {wanted}")
if used >= 3_110_000:
    raise SystemExit(f"gasUsed {used} >= 3.11M envelope — open a new #600 follow-up; do not bump UNWRAP_GAS_LIMIT")
print(f"  margin={wanted-used} (used < 3.11M floor ok)")
' "${wanted}" "${used}"
}

if [[ -n "${VERIFY600_COLUMBUS_TX:-}" ]]; then
  run_step "columbus-5 LCD gas_used < gas_wanted (P599-2/P599-5)" \
    query_columbus_tx "${VERIFY600_COLUMBUS_TX}"
else
  if [[ "${VERIFY600_REQUIRE_MAINNET:-0}" = "1" ]]; then
    bad "columbus-5 hash required (VERIFY600_REQUIRE_MAINNET=1) — set VERIFY600_COLUMBUS_TX"
  else
    skip "columbus-5 USTR→USTC (P599-2) — set VERIFY600_COLUMBUS_TX=<hash> after operator swap"
  fi
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
echo "==> GitLab #600 verification passed"
