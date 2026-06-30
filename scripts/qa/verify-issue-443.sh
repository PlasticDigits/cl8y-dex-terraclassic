#!/usr/bin/env bash
# Automated verification for GitLab #443 — wasm migration rollback limitations (SEC-H05).
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
echo "  GitLab #443 — wasm migration rollback limitations (SEC-H05)"
echo "════════════════════════════════════════════════════════════════"

run_step "wasm migration rollback doc invariant" \
  make check-wasm-migration-rollback-docs

run_step "rollback section covers contract reversal path" \
  grep -q 'Contract migration reversal' docs/runbooks/wasm-admin-migration.md

run_step "rollback section documents irrecoverable cases" \
  grep -q 'Irrecoverable cases' docs/runbooks/wasm-admin-migration.md && \
  grep -q 'Admin cleared' docs/runbooks/wasm-admin-migration.md

run_step "rollback section references indexer revert down.sql" \
  grep -q 'indexer/migrations/revert/' docs/runbooks/wasm-admin-migration.md

run_step "rollback section covers partial migration recovery" \
  grep -q 'Partial migration recovery' docs/runbooks/wasm-admin-migration.md

run_step "launch runbook rollback cross-links migration limitations" \
  grep -q 'rollback-and-limitations-sec-h05' docs/runbooks/launch-checklist.md

run_step "agent skill present" \
  test -f skills/AGENTS_WASM_MIGRATION_ROLLBACK.md

run_step "migration tests still pass (SEC-C14 rehearsal)" \
  make test-contracts

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
