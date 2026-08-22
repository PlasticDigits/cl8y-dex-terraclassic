#!/usr/bin/env bash
# Automated verification for GitLab #590 — post-merge !394–!396 ops stack.
#
# Proves (docs + children + L7/unwrap classification + optional LocalTerra rungs):
#   1. Q7 / M590-1–M590-8 documented and crosslinked.
#   2. Child make verify-issue-{586,587,589}.
#   3. Hybrid L7 ingest ignores book_commission_amount; unwrap ignores InstantWithdraw tax.
#   4. 8266 stays NO-GO; LAYER_B_LT=1 is not a stub.
#   5. Optional: CODE_ID=8266 LAYER_B_LT=1 when LocalTerra is up (or VERIFY590_REQUIRE_CHAIN=1).
#   6. Optional: Playwright wrap-swap E7/E8 e2e-tx (1 worker).
#
# VERIFY590_SKIP_CHILDREN=1 — docs + classification only.
# VERIFY590_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra / e2e-tx is missing.
# VERIFY590_SKIP_E2E=1 — skip wrap-swap e2e-tx even if chain is up.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_590.md, docs/qa-invariants.md § Q7
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
echo "  GitLab #590 — post-merge !394–!396 ops stack"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31590}"

has_chain() {
  timeout 20 make -s has-localterra >/dev/null 2>&1
}

run_step "docs: Q7 M590-1–M590-8 + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_POST_MERGE_OPS_590.md
    grep -qE "\*\*M590-1\*\*" docs/qa-invariants.md
    grep -qE "\*\*M590-8\*\*" docs/qa-invariants.md
    grep -qE "post-merge-ops-590" docs/qa-invariants.md
    grep -qE "\*\*M590-1" skills/AGENTS_POST_MERGE_OPS_590.md
    grep -qE "make verify-issue-590" skills/AGENTS_POST_MERGE_OPS_590.md
    grep -qE "AGENTS_POST_MERGE_OPS_590" AGENTS.md
    grep -qE "verify-issue-590" AGENTS.md
    grep -qE "verify-issue-590" Makefile
    grep -qE "#590" docs/testing.md
    grep -qE "AGENTS_POST_MERGE_OPS_590" docs/README.md
    grep -qE "#590" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -qE "#590" skills/AGENTS_TERRACLASSIC_GAS.md
    grep -qE "#590" skills/AGENTS_CW20_CODE_ID_AUDIT.md
    grep -qE "layer-a-lcd.sh" skills/AGENTS_CW20_CODE_ID_AUDIT.md
    grep -qE "layer-b-lt.sh" cw20-codeid-audits/harness/README.md
    grep -qE "LAYER_B_LT=1 must not PASS as a stub" skills/AGENTS_POST_MERGE_OPS_590.md
  '

run_step "source: A-lcd/B-lt scripts execute wasm; 589 is not a stub" \
  bash -c '
    set -euo pipefail
    test -x cw20-codeid-audits/scripts/layer-a-lcd.sh || test -f cw20-codeid-audits/scripts/layer-a-lcd.sh
    test -f cw20-codeid-audits/scripts/layer-b-lt.sh
    test -f cw20-codeid-audits/scripts/lib-layer-lt.sh
    grep -q "layer-a-lcd.sh" scripts/qa/verify-issue-589.sh
    grep -q "layer-b-lt.sh" scripts/qa/verify-issue-589.sh
    if grep -n "store/instantiate of LCD wasm is operator-run" scripts/qa/verify-issue-589.sh; then
      echo "verify-issue-589 still stubs LAYER_B_LT=1" >&2
      exit 1
    fi
    grep -q "never columbus-5" cw20-codeid-audits/scripts/lib-layer-lt.sh
    grep -q "NO-GO" cw20-codeid-audits/codeids/8266/REPORT.md
  '

run_step "L7 / PFee-5: hybrid book counted once; unwrap not burn tax" \
  bash -c "
    set -euo pipefail
    grep -q FeeSource::BookTake indexer/src/indexer/parser.rs
    grep -q FeeSource::LimitPlace indexer/src/indexer/parser.rs
    grep -q FeeSource::SwapAmm indexer/src/indexer/parser.rs
    if grep -nF 'wasm_attr_last(attrs, \"book_commission_amount\")' indexer/src/indexer/parser.rs; then
      echo 'parser must not ingest swap book_commission_amount (L7)' >&2
      exit 1
    fi
    grep -q InstantWithdraw indexer/src/indexer/protocol_fees.rs
    grep -q tax_amount indexer/src/indexer/protocol_fees.rs
    grep -q parse_swaps_pool_commission_ignores_book_commission_amount_l7 indexer/src/indexer/parser.rs
    grep -q parse_unwrap_uses_fee_amount_not_tax_amount indexer/src/indexer/protocol_fees.rs
    grep -q hybrid_counts_amm_and_book_once indexer/tests/indexer_protocol_fees.rs
    grep -q 'E7:' frontend-dapp/e2e/wrap-swap.spec.ts
    grep -q 'E8:' frontend-dapp/e2e/wrap-swap.spec.ts
  "

if [[ "${VERIFY590_SKIP_CHILDREN:-0}" = "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY590_SKIP_CHILDREN=1"
  skip "child verify-issue-* (VERIFY590_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-586" make verify-issue-586
  run_step "child: make verify-issue-587" make verify-issue-587
  run_step "child: make verify-issue-589" make verify-issue-589
fi

if has_chain; then
  if [[ "${VERIFY590_SKIP_CHILDREN:-0}" != "1" ]]; then
    run_step "Layer A-lcd + B-lt: CODE_ID=8266 LAYER_B_LT=1" \
      bash -c 'CODE_ID=8266 LAYER_B_LT=1 make verify-issue-589'
  fi
  if [[ "${VERIFY590_SKIP_E2E:-0}" = "1" ]]; then
    skip "playwright wrap-swap E7/E8 e2e-tx (VERIFY590_SKIP_E2E=1)"
  elif [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
    if [[ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
      COMMON="$(git rev-parse --git-common-dir)"
      MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
      if [[ -f "$MAIN_ROOT/frontend-dapp/.env.local" && "$MAIN_ROOT" != "$REPO_ROOT" ]]; then
        cp "$MAIN_ROOT/frontend-dapp/.env.local" "$REPO_ROOT/frontend-dapp/.env.local"
      fi
    fi
    run_step "playwright wrap-swap E7/E8 e2e-tx (1 worker)" \
      bash -c 'CI=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31590}" PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-31590}" bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test --project=e2e-tx --workers=1 e2e/wrap-swap.spec.ts -g "E7:|E8:"'
  else
    if [[ "${VERIFY590_REQUIRE_CHAIN:-0}" = "1" ]]; then
      bad "playwright wrap-swap E7/E8 (Playwright not installed)"
    else
      skip "playwright wrap-swap E7/E8 (no Playwright install)"
    fi
  fi
else
  if [[ "${VERIFY590_REQUIRE_CHAIN:-0}" = "1" ]]; then
    bad "LocalTerra required (VERIFY590_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "Layer A-lcd/B-lt + wrap-swap E7/E8 (make has-localterra). Cloud Agent: make setup-cloud-localterra"
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
echo "==> GitLab #590 verification passed"
