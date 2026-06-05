#!/usr/bin/env bash
# Verification for GitLab #324 follow-up (#335): concurrent solve lib tests + HTTP cache
# tier isolation integration tests (MR !91 integration gap on Cloud Agent VMs).
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
echo "  GitLab #324 — concurrent solve + route cache tier (lib + HTTP)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "indexer lib: concurrent_solve" \
  bash -c 'cd indexer && cargo test concurrent_solve --lib -- --quiet'

run_step "indexer lib: hybrid_cache_key" \
  bash -c 'cd indexer && cargo test hybrid_cache_key --lib -- --quiet'

run_step "indexer integration: route_solve_get_cache (api_route_solve)" \
  bash -c 'cd indexer && cargo test --test api_route_solve route_solve_get_cache -- --test-threads=1 --quiet'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
