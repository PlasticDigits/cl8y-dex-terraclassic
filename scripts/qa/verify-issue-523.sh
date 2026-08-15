#!/usr/bin/env bash
# Verification for GitLab #523 — router unwrap_output dual-reads wrap-mapper fees.
#
# Proves (no chain required):
#   1. dex-common ConfigResponse deserializes legacy { fee_bps } and split
#      { fee_wrap_bps, fee_unwrap_bps } (no fee_bps); fail closed on partial.
#   2. Router R3 uses unwrap_fee_bps(); settlement succeeds against post-migrate
#      Config JSON (mock mapper emits no fee_bps).
#   3. Skills/docs say store+migrate router with wrap-mapper migrate.
#
# Refs: skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md
#       skills/AGENTS_ROUTER_MINIMUM_RECEIVE.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #523 — router unwrap_output split-fee Config (R3 / W13)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_dex_common() {
  (cd smartcontracts && cargo test -p dex-common wrap_mapper -- --quiet)
}

run_integration() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib \
    test_unwrap_output_split_fee_config_no_fee_bps -- --quiet) &&
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib \
    test_unwrap_minimum_receive -- --quiet)
}

run_docs() {
  set -euo pipefail
  rg -q "unwrap_fee_bps" smartcontracts/contracts/router/src/contract.rs
  rg -q "fn wrap_mapper_fee_pair" smartcontracts/packages/dex-common/src/wrap_mapper.rs
  rg -q "fee_unwrap_bps" smartcontracts/packages/dex-common/src/wrap_mapper.rs
  rg -q "test_unwrap_output_split_fee_config_no_fee_bps" smartcontracts/tests/src/lib.rs
  rg -q "523" skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md
  rg -q "523" skills/AGENTS_ROUTER_MINIMUM_RECEIVE.md
  rg -q "523" docs/contracts-security-audit.md
  rg -q "verify-issue-523" AGENTS.md
  rg -q "version = \"1.1.0\"" smartcontracts/contracts/router/Cargo.toml
}

echo ""
echo "── first pass ──"
run_step "dex-common: wrap_mapper Config dual-read" run_dex_common
run_step "integration: unwrap_output split Config + R3" run_integration
run_step "docs: #523 dual-read + router 1.1.0" run_docs

echo ""
echo "── retest ──"
run_step "retest dex-common: wrap_mapper Config dual-read" run_dex_common
run_step "retest integration: unwrap_output split Config + R3" run_integration

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #523 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
