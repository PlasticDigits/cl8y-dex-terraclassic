#!/usr/bin/env bash
# Automated verification for GitLab #245 — off-chain `trader` forwarding for CL8Y
# fee-discount quote parity (frontend preflight + indexer route solve).
#
# Layers:
#   1. Frontend unit tests (pair, swapRoutePreflight, indexer client)
#   2. Indexer unit + api_route_solve integration tests
#   3. Optional live LocalTerra stack via verify-issue-238.sh (on-chain #238 + indexer
#      route/solve with `trader`; proves quote >= undiscounted on deployed wasm)
#
# Refs: docs/contracts-security-audit.md (L8), docs/indexer-invariants.md,
#       docs/integrators.md, skills/AGENTS_{HYBRID_QUOTING,FEE_DISCOUNT_TIERS,INDEXER_HYBRID_BEST_EXECUTION}.md
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
echo "  GitLab #245 — off-chain trader quote parity"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend unit tests (pair, preflight, indexer client)" \
  bash -c 'cd frontend-dapp && { test -d node_modules || npm ci --silent; } && npx vitest run \
    src/services/terraclassic/__tests__/pair.test.ts \
    src/services/terraclassic/__tests__/swapRoutePreflight.test.ts \
    src/services/indexer/__tests__/client.test.ts'

run_step "indexer hybrid_cache_key unit test" \
  bash -c 'cd indexer && cargo test hybrid_cache_key_includes_trader --lib -- --quiet'

run_step "indexer api_route_solve integration tests" \
  bash -c 'cd indexer && cargo test --test api_route_solve -- --test-threads=1 --quiet'

# Live stack: same script as #238 (pair/router/indexer accept `trader` on deployed wasm).
IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"
if ! docker compose ps -q localterra 2>/dev/null | head -1 | grep -q .; then
  echo ""
  echo "[live] SKIP — localterra not running (unit tests only). For full checklist:"
  echo "       make start-qa && make verify-issue-245"
  RESULTS+=("SKIP  live stack (localterra not running)")
elif [ ! -f "$IDX_ENV" ] || [ ! -f "$FE_ENV" ]; then
  echo ""
  echo "[live] SKIP — indexer/.env or frontend-dapp/.env.local missing (gitignored; run from primary checkout after make deploy-local)."
  RESULTS+=("SKIP  live stack (deploy env files missing in this worktree)")
elif bash "$REPO_ROOT/scripts/qa/verify-issue-238.sh"; then
  ok "live stack: verify-issue-238 (trader on pair/router/indexer route/solve)"
else
  bad "live stack: verify-issue-238 (see output above; run make deploy-local if wasm stale)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
