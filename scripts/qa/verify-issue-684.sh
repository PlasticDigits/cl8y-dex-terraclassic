#!/usr/bin/env bash
# Verification for GitLab #684 — GeckoTerminal /gt/events post-event reserves.
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
echo "  GitLab #684 — /gt/events post-event reserves"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs + skill + migration + CLI" \
  bash -c '
    test -f indexer/migrations/20260827160000_gt_event_post_reserves.sql && \
    test -f skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md && \
    grep -qE "R684-1" skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md && \
    grep -qE "R684-8" skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md && \
    grep -qE "make verify-issue-684" skills/AGENTS_INDEXER_GT_EVENT_RESERVES.md && \
    grep -qE "post-event AMM" docs/indexer-invariants.md && \
    grep -qE "verify-issue-684" docs/testing.md && \
    grep -qE "AGENTS_INDEXER_GT_EVENT_RESERVES" AGENTS.md && \
    grep -qE "backfill-gt-event-reserves" indexer/src/main.rs && \
    grep -qE "post-event" scripts/geckoterminal/README.md && \
    grep -qE "#684" docs/listings/forms/geckoterminal.md
  '

run_step "GET /gt/events must not SELECT pair_reserves" \
  bash -c '
    ! grep -nE "FROM pair_reserves|JOIN pair_reserves|current_reserves" indexer/src/api/gt.rs
  '

run_step "cargo test --lib gt (unit: gt.rs + gt_event_reserves)" \
  bash -c 'cd indexer && cargo test --lib gt -- --test-threads=1 --quiet'

run_step "cargo test --test api_gt --test gt_event_reserves" \
  bash -c 'cd indexer && cargo test --test api_gt --test gt_event_reserves -- --test-threads=1 --quiet'

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
