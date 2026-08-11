#!/usr/bin/env bash
# Verification for GitLab #515: ticker-scoped external oracle (ustc/lunc).
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
echo "  GitLab #515 — external oracle ticker routes (ustc / lunc)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: runbook + skill + invariants crosslinks" \
  bash -c '
    set -euo pipefail
    test -f docs/runbooks/indexer-external-oracle.md
    test -f skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "External oracle tickers (#515)" docs/indexer-invariants.md
    grep -q "AGENTS_INDEXER_EXTERNAL_ORACLE" AGENTS.md
    grep -q "X1" docs/runbooks/indexer-external-oracle.md
    grep -q "/api/v1/oracle/price/{ticker}" docs/runbooks/indexer-external-oracle.md
  '

run_step "indexer lib: oracle ticker symbols + coingecko parse" \
  bash -c 'cd indexer && cargo test --lib oracle -- --quiet'

run_step "indexer integration: api_oracle" \
  bash -c 'cd indexer && cargo test --test api_oracle -- --test-threads=1 --quiet'

run_step "frontend: Protocol + Trade oracle mocks / types" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx src/pages/TradePage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
