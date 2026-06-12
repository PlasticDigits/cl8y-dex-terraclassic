#!/usr/bin/env bash
# Verification for GitLab #369: DB-hybrid route/solve must not 502 when a candidate path
# touches a zero-reserve pair while a direct healthy route exists.
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
echo "  GitLab #369 — skip zero-reserve path candidates (DB hybrid)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "indexer integration: zero-reserve path skip (api_route_solve_db_hybrid)" \
  bash -c 'cd indexer && cargo test --test api_route_solve_db_hybrid route_solve_db_hybrid_skips_zero_reserve_path_candidate -- --test-threads=1 --quiet'

run_step "indexer lib: concurrent_solve (fail-fast preserved)" \
  bash -c 'cd indexer && cargo test concurrent_solve --lib -- --quiet'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
