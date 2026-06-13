#!/usr/bin/env bash
# Verification for GitLab #365 registry outage observability (#375 docs + ladder).
#
# Layers (no LocalTerra required):
#   1. Contract P5: GetDiscount Err → full pair fee (swap still succeeds)
#   2. Indexer: GET /api/v1/health/fee-discount integration tests
#   3. Frontend: feeDiscountRegistryWarning util (unregistered vs unreachable)
#
# Requires: make setup-indexer-postgres when indexer/.env is missing.
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
echo "  GitLab #365 — fee-discount registry outage observability"
echo "════════════════════════════════════════════════════════════════"

postgres_ready() {
  if docker compose exec -T postgres pg_isready -U "${POSTGRES_USER:-cl8y_legal}" >/dev/null 2>&1; then
    return 0
  fi
  if sg docker -c 'docker compose exec -T postgres pg_isready -U cl8y_legal' >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

if [ ! -f "$REPO_ROOT/indexer/.env" ] || ! postgres_ready; then
  echo ""
  echo "[bootstrap] indexer/.env or Postgres missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "contract P5: swap_uses_full_fee_when_discount_registry_query_fails" \
  bash -c 'cd smartcontracts/tests && cargo test swap_uses_full_fee_when_discount_registry_query_fails -- --quiet'

run_step "indexer integration: api_fee_discount_health" \
  bash -c 'cd indexer && cargo test --test api_fee_discount_health -- --test-threads=1 --quiet'

run_step "frontend unit: feeDiscountRegistryWarning" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- src/utils/__tests__/feeDiscountRegistryWarning.test.ts'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
