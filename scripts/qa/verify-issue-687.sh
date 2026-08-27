#!/usr/bin/env bash
# Verification for GitLab #687: DeFiLlama fees headline partial SUM + adapter start.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #687 — DeFiLlama fees headline + adapter start / 404"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: L687 skill + invariants + no Coolify-draft leftover" \
  bash -c '
    set -euo pipefail
    grep -q "L687-1" skills/AGENTS_DEFILLAMA.md
    grep -q "L687-8" skills/AGENTS_DEFILLAMA.md
    grep -q "DeFiLlama fees headline (#687)" docs/indexer-invariants.md
    grep -q "1786924800" docs/DEFILLAMA.md
    grep -q "2026-08-17" docs/DEFILLAMA.md
    grep -q "verify-issue-687" docs/testing.md
    grep -q "verify-issue-687" AGENTS.md
    grep -q "L687" AGENTS.md
    if grep -n "draft until Coolify" docs/DEFILLAMA.md scripts/defillama/README.md; then
      echo "Coolify daily route is live — do not document #8987 as waiting on the route" >&2
      exit 1
    fi
    if grep -nE "1777593600|2026-05-01 00:00" docs/DEFILLAMA.md scripts/defillama/README.md scripts/defillama/gems.js indexer/src/indexer/defillama.rs; then
      echo "adapter start must be first 200 UTC day, not 2026-05-01" >&2
      exit 1
    fi
  '

run_step "source: headline partial SUM; per-source fail-closed; no GET scan" \
  bash -c '
    set -euo pipefail
    grep -q "fn daily_headline_usd" indexer/src/indexer/defillama.rs
    grep -q "daily_headline_usd(fee_events, &fee_usd)" indexer/src/api/defillama.rs
    grep -q "ADAPTER_START_UTC_DAY: i64 = 1_786_924_800" indexer/src/indexer/defillama.rs
    grep -q "const ADAPTER_START = 1786924800" scripts/defillama/gems.js
    grep -q "ADAPTER_START_ISO = '\''2026-08-17'\''" scripts/defillama/gems.js
    grep -q "feeResidual" scripts/defillama/dimensions/mapDaily.js
    grep -q "requirePricedUsd" scripts/defillama/fees/index.ts
    grep -q "ADAPTER_START_ISO" scripts/defillama/fees/index.ts
    grep -q "ADAPTER_START_ISO" scripts/defillama/dexs/index.ts
    if grep -nE "FROM swap_events|FROM protocol_fee_events|FROM limit_order_fills" \
         indexer/src/api/defillama.rs 2>/dev/null; then
      echo "GET handler must not scan event tables" >&2
      exit 1
    fi
    if grep -n "dailyFees.addUSDValue(mapped.dailyFees)" scripts/defillama/fees/index.ts; then
      echo "fees adapter must not add headline then labels (A16 double-count)" >&2
      exit 1
    fi
    if grep -nE "liquidity_in_usd|total_liquidity_usd" \
         scripts/defillama/tvl/tvlCore.js \
         scripts/defillama/dimensions/mapDaily.js 2>/dev/null; then
      echo "TVL/volume helpers must not read indexer/CG USD fields" >&2
      exit 1
    fi
  '

run_step "lib: headline vs per-source fail-closed + start pin" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --lib defillama -- --test-threads=1
  '

run_step "integration: partial SUM / idle zero / start 200 / NULL-only flip" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --test indexer_defillama -- --test-threads=1
  '

run_step "node: mapper residual + adapter start pin" \
  bash -c '
    set -euo pipefail
    node --test scripts/defillama/dimensions/mapDaily.test.js
  '

if [[ "${VERIFY_ISSUE_687_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 631/683] skipped (VERIFY_ISSUE_687_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-631" \
    bash -c 'make verify-issue-631'
  run_step "related: verify-issue-683" \
    bash -c 'VERIFY_ISSUE_683_SKIP_RELATED=1 make verify-issue-683'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
