#!/usr/bin/env bash
# Verification for GitLab #485: distant-pair route solve latency helpers + progress poll.
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
echo "  GitLab #485 — route graph cache, distant TTL, progress poll"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: route-solver constants drift" \
  python3 scripts/check_route_solver_docs.py

run_step "indexer lib: route_solve_progress" \
  bash -c 'cd indexer && cargo test --lib route_solve_progress -- --quiet'

run_step "indexer lib: route_graph + cache_ttl" \
  bash -c 'cd indexer && cargo test --lib route_graph -- --quiet && cargo test --lib cache_ttl -- --quiet'

run_step "indexer lib: hybrid_cache_key + concurrent_solve" \
  bash -c 'cd indexer && cargo test --lib hybrid_cache_key -- --quiet && cargo test --lib concurrent_solve -- --quiet'

run_step "indexer integration: route_solve_progress" \
  bash -c 'cd indexer && cargo test --test api_route_solve route_solve_progress -- --test-threads=1 --quiet'

run_step "indexer integration: route_solve_get_cache" \
  bash -c 'cd indexer && cargo test --test api_route_solve route_solve_get_cache -- --test-threads=1 --quiet'

run_step "frontend: routeSolveProgress + client progress" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/utils/routeSolveProgress.test.ts src/services/indexer/__tests__/client.test.ts'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
