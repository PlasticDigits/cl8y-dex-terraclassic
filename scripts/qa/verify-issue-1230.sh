#!/usr/bin/env bash
# Automated verification for Forgejo #1230 — reject zero / dust-floor belief_price (L9).
#
# Layers (no LocalTerra / no wasm migrate):
#   1. dex-common max_spread unit tests (zero panic, dust floor, happy path)
#   2. pair multitest: execute + HybridSimulation belief_price "0" / dust floor
#   3. existing hybrid belief + pool-only spread regressions
#   4. Frontend serializer + G8 mirror (zero belief is not a floor)
#   5. Docs / skill / L9 crosslinks
#   6. Retest dex-common max_spread
#
# Wasm migrate on columbus-5 is ops follow-up, not this script.
#
# Refs: skills/AGENTS_MAX_SPREAD_HYBRID.md, docs/integrators.md, docs/contracts-security-audit.md L9
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
echo "  Forgejo #1230 — reject zero / dust-floor belief_price (L9)"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "dex-common: max_spread (incl. #1230 zero / dust / commission)" \
  bash -c 'cd smartcontracts && cargo test -p dex-common max_spread -- --nocapture'

run_step "multitest: hybrid_belief_price_max_spread (AC3)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_belief_price_max_spread -- --test-threads=1'

run_step "multitest: test_swap_max_spread (AC4 no-belief)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests test_swap_max_spread -- --test-threads=1'

run_step "multitest: test_swap_belief_price zero / dust / sim (AC1/AC2/AC5/T10)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests test_swap_belief_price -- --test-threads=1'

run_step "frontend: swapMaxSpread G8 zero belief is not a floor" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/swapMaxSpread.test.ts

run_step "frontend: pair swap() rejects belief_price 0" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/pair.test.ts

run_step "docs: integrators invalid zero / dust-floor belief" \
  grep -qE 'strictly positive|expected_return >= 1|#1230' docs/integrators.md

run_step "docs: L9 invalid belief sentence" \
  grep -qE 'expected_return >= 1|#1230' docs/contracts-security-audit.md

run_step "docs: ADR 0001 #1230" \
  grep -qE 'dust-floor|1230' docs/adr/0001-hybrid-quoting-and-routing.md

run_step "skill: AGENTS_MAX_SPREAD_HYBRID #1230" \
  grep -qE 'InvalidBeliefPrice|#1230' skills/AGENTS_MAX_SPREAD_HYBRID.md

run_step "skill: AGENTS_GREEDY_BOOK_FIRST G8 usable belief" \
  grep -qE 'usable.*belief_price|#1230' skills/AGENTS_GREEDY_BOOK_FIRST.md

run_step "AGENTS.md verify-issue-1230" \
  grep -qE 'verify-issue-1230' AGENTS.md

echo ""
echo "── retest (dex-common max_spread) ──"
run_step "retest: dex-common max_spread" \
  bash -c 'cd smartcontracts && cargo test -p dex-common max_spread -- --nocapture'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
