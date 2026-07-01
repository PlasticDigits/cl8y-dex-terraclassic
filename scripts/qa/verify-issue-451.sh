#!/usr/bin/env bash
# Automated verification for GitLab #451 — FACTORY_ADDRESS non-empty guard (SEC-I02 H14).
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
echo "  GitLab #451 — FACTORY_ADDRESS non-empty guard (SEC-I02 H14)"
echo "════════════════════════════════════════════════════════════════"

run_step "factory address doc invariant" \
  make check-factory-address-docs

run_step "ConfigError::EmptyFactoryAddress in config.rs" \
  grep -q 'EmptyFactoryAddress' indexer/src/config.rs

run_step "empty_factory_address_rejected_in_dev unit test" \
  bash -c 'export PATH="/usr/local/cargo/bin:$PATH"; cd indexer && cargo test --lib empty_factory_address_rejected_in_dev -- --nocapture'

run_step "indexer startup rejects whitespace FACTORY_ADDRESS" \
  bash -c 'export PATH="/usr/local/cargo/bin:$PATH"; cd indexer && cargo build --bin cl8y-dex-indexer -q && DATABASE_URL=postgres://localhost/db FACTORY_ADDRESS="   " CORS_ORIGINS=http://localhost:5173 ./target/debug/cl8y-dex-indexer 2>&1 | grep -q "FACTORY_ADDRESS must be non-empty"'

run_step "agent skill present" \
  test -f skills/AGENTS_FACTORY_ADDRESS_GUARD.md

run_step "launch runbook Phase 0 FACTORY_ADDRESS step" \
  grep -q 'SEC-I02' docs/runbooks/launch-checklist.md

run_step "qa-verify-deploy asserts FACTORY_ADDRESS present" \
  grep -q 'FACTORY_ADDRESS not found' scripts/qa/verify-deploy.sh

run_step "CI test-indexer-lib covers config guard" \
  grep -q 'test-indexer-lib' .gitlab-ci.yml

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
