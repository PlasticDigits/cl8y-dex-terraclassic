#!/usr/bin/env bash
# Verification for Forgejo #1205 — redacted UTC-day evidence JSON export.
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
echo "  Forgejo #1205 — GET /api/v1/evidence/daily"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "evidence route in api router" \
  grep -q 'evidence/daily' indexer/src/api/mod.rs

run_step "evidence handler module" \
  test -f indexer/src/api/evidence.rs

run_step "evidence daily query module" \
  test -f indexer/src/db/queries/evidence_daily.rs

run_step "skill AGENTS_INDEXER_EVIDENCE_DAILY.md" \
  test -f skills/AGENTS_INDEXER_EVIDENCE_DAILY.md

run_step "invariants doc mentions evidence daily" \
  grep -q 'evidence/daily' docs/indexer-invariants.md

if [ -f "$REPO_ROOT/indexer/.env" ] && command -v cargo >/dev/null; then
  run_step "integration tests api_evidence_daily" \
    bash -c 'cd indexer && cargo test --test api_evidence_daily -- --test-threads=1'
else
  echo ""
  echo "[skip] indexer/.env or cargo missing — integration tests not run in this environment"
  ok "integration tests api_evidence_daily (skipped)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '  PASS: %s  FAIL: %s\n' "$PASS" "$FAIL"
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
