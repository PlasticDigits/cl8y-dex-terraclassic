#!/usr/bin/env bash
# Automated verification for Forgejo #1204 — indexer HTTP pack + OpenAPI gaps.
#
# Proves (docs + ApiDoc unit test; no LocalTerra):
#   I1204-1  pack linked from README and integrators
#   I1204-2  five families in the path table and curls
#   I1204-3  three clocks
#   I1204-4  hooks vs treasury fees vs burn tax; no /burns route
#   I1204-5  raw vs USD and L10
#   I1204-6  health + fee-discount + blacklist-check registered
#   I1204-7  placeholders; LCD-heavy stays in the appendix
#   I1204-8  ApiDoc unit test
#
# Refs: skills/AGENTS_INDEXER_HTTP_PACK.md
#       docs/indexer-http.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  Forgejo #1204 — indexer HTTP pack"
echo "════════════════════════════════════════════════════════════════"

PACK=docs/indexer-http.md

run_step "I1204-1 pack linked from README and integrators" \
  bash -c '
    test -f docs/indexer-http.md && \
    grep -q "indexer-http.md" docs/README.md && \
    grep -q "indexer-http.md" docs/integrators.md && \
    grep -q "I1204-1" skills/AGENTS_INDEXER_HTTP_PACK.md && \
    grep -q "I1204-8" skills/AGENTS_INDEXER_HTTP_PACK.md && \
    grep -q "make verify-issue-1204" skills/AGENTS_INDEXER_HTTP_PACK.md && \
    grep -q "HTTP pack (#1204)" docs/indexer-invariants.md && \
    grep -q "AGENTS_INDEXER_HTTP_PACK" AGENTS.md
  '

run_step "I1204-2 five-surface paths in the pack" \
  bash -c '
    grep -q "/api/v1/pairs" docs/indexer-http.md && \
    grep -q "/api/v1/pairs/{addr}/trades" docs/indexer-http.md && \
    grep -q "/api/v1/pairs/\$PAIR/trades" docs/indexer-http.md && \
    grep -q "/api/v1/traders/\$TRADER/trades" docs/indexer-http.md && \
    grep -q "/api/v1/pairs/\$PAIR/liquidity-events" docs/indexer-http.md && \
    grep -q "/api/v1/pairs/\$PAIR/stats" docs/indexer-http.md && \
    grep -q "/api/v1/hooks" docs/indexer-http.md && \
    grep -q "/api/v1/protocol/fees" docs/indexer-http.md && \
    grep -q "/api/v1/overview" docs/indexer-http.md && \
    grep -q "/api/v1/protocol/volume/daily" docs/indexer-http.md && \
    grep -q "/api/v1/tokens/\$TOKEN" docs/indexer-http.md && \
    grep -q "/api/v1/protocol/fees/daily" docs/indexer-http.md && \
    grep -q "/api/v1/defillama/daily" docs/indexer-http.md
  '

run_step "I1204-3 three clocks labeled" \
  bash -c '
    grep -q "Trailing" docs/indexer-http.md && \
    grep -q "UTC series" docs/indexer-http.md && \
    grep -q "DeFiLlama UTC day" docs/indexer-http.md && \
    grep -q "window=24h" docs/indexer-http.md && \
    grep -q "window=7d" docs/indexer-http.md && \
    grep -q "window=30d" docs/indexer-http.md && \
    grep -q "days=7" docs/indexer-http.md && \
    grep -q "timestamp=1719792000" docs/indexer-http.md
  '

run_step "I1204-4 burns vs fees vs burn tax" \
  bash -c '
    grep -q "after_swap_burn" docs/indexer-http.md && \
    grep -q "Terra Classic burn tax" docs/indexer-http.md && \
    grep -q "no" docs/indexer-http.md && \
    grep -q "/burns" docs/indexer-http.md && \
    ! grep -qE "curl[^\\n]* /burns" docs/indexer-http.md && \
    ! grep -q "GET /burns" docs/indexer-http.md && \
    ! grep -q "GET /api/v1/listings" docs/indexer-http.md
  '

run_step "I1204-5 raw vs USD and L10" \
  bash -c '
    grep -q "L10" docs/indexer-http.md && \
    grep -q "Do not SUM" docs/indexer-http.md && \
    grep -q "limit_order_fills" docs/indexer-http.md && \
    grep -q "volume_quote_24h" docs/indexer-http.md && \
    grep -q "null" docs/indexer-http.md
  '

run_step "I1204-6 health and compliance in ApiDoc source" \
  bash -c '
    grep -q "path = \"/health\"" indexer/src/api/mod.rs && \
    grep -q "path = \"/api/v1/health/fee-discount\"" indexer/src/api/fee_discount_health.rs && \
    grep -q "compliance::blacklist_check" indexer/src/api/mod.rs && \
    grep -q "fee_discount_health::get_fee_discount_health" indexer/src/api/mod.rs
  '

run_step "I1204-7 placeholders and LCD-heavy appendix only" \
  bash -c '
    grep -q "LCD-heavy appendix" docs/indexer-http.md && \
    grep -q "issues/707" docs/indexer-http.md && \
    grep -q -- "--data-urlencode" docs/indexer-http.md && \
    grep -q "api-docs/openapi.json" docs/indexer-http.md && \
    grep -q "/swagger-ui/" docs/indexer-http.md && \
    ! grep -q "DATABASE_URL" docs/indexer-http.md && \
    ! grep -q "Authorization" docs/indexer-http.md && \
    ! grep -qE "format=csv" docs/indexer-http.md || grep -q "prefixed" docs/indexer-http.md
  '

run_step "I1204-8 ApiDoc unit test" \
  bash -c '
    export PATH="/usr/local/cargo/bin:$PATH"
    cd indexer && cargo test --lib openapi_pack_includes_five_surfaces_and_health -- --test-threads=1
  '

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #1204 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for row in "${RESULTS[@]}"; do
  echo "  $row"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
