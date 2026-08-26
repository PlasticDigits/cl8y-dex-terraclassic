#!/usr/bin/env bash
# Verification for GitLab #646 — GeckoTerminal /gt/ Integration API.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env).
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
echo "  GitLab #646 — GeckoTerminal /gt/ adapters"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "routes + form pack + scripts/geckoterminal" \
  bash -c '
    grep -qE "/gt/latest-block" indexer/src/api/mod.rs && \
    grep -qE "/gt/asset" indexer/src/api/mod.rs && \
    grep -qE "/gt/pair" indexer/src/api/mod.rs && \
    grep -qE "/gt/events" indexer/src/api/mod.rs && \
    grep -qE "dexKey|DEX_KEY" indexer/src/api/gt.rs && \
    grep -qE "indexer.dex.cl8y.com/gt" docs/listings/forms/geckoterminal.md && \
    test -f scripts/geckoterminal/README.md && \
    grep -qE "/gt/latest-block" scripts/geckoterminal/README.md
  '

run_step "cargo test --test api_gt (unit + integration)" \
  bash -c 'cd indexer && cargo test --test api_gt --lib gt -- --test-threads=1'

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
