#!/usr/bin/env bash
# Automated verification for GitLab #708 — greedy book-first swap (opt-in).
#
# Layers (no LocalTerra required):
#   1. dex-common G1/G5/G11/G14 unit tests
#   2. pair greedy 1-unit beats tests
#   3. cl8y-dex-tests greedy_book_first_708 multitest
#   4. Frontend greedy gas mapping (G13)
#   5. Docs/skills G1–G14 crosslinks
#   6. Retest unit + docs
#
# Pair wasm migrate on columbus-5 is ops follow-up (G14), not this script.
#
# Refs: skills/AGENTS_GREEDY_BOOK_FIRST.md, docs/adr/0001-hybrid-quoting-and-routing.md
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
echo "  GitLab #708 — greedy book-first swap (opt-in)"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "dex-common: greedy_swap_mode_tests (G1/G5/G11/G14)" \
  bash -c 'cd smartcontracts && cargo test -p dex-common greedy_swap -- --nocapture'

run_step "pair: greedy Decimal-rate beats + stop reasons" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair greedy -- --nocapture'

run_step "multitest: greedy_book_first_708 (G1–G11, G14)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests greedy_book_first_708 -- --test-threads=1'

run_step "frontend: greedy gas envelopes (G13)" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/hybridSwapGas.test.ts \
    src/services/terraclassic/__tests__/terraGas.greedy.test.ts

run_step "docs: ADR 0001 #708 greedy note" \
  grep -qE 'GitLab #708|#708' docs/adr/0001-hybrid-quoting-and-routing.md

run_step "docs: limit-orders greedy subsection" \
  grep -qE 'greedy book-first|#708' docs/limit-orders.md

run_step "docs: integrators greedy / G1" \
  grep -qE 'GreedySwapParams|#708' docs/integrators.md

run_step "docs: contracts-security-audit G1–G14" \
  grep -qE '\*\*G1\*\*' docs/contracts-security-audit.md

run_step "skill: AGENTS_GREEDY_BOOK_FIRST G1–G14" \
  grep -qE '\*\*G1\*\*' skills/AGENTS_GREEDY_BOOK_FIRST.md

run_step "skill: AGENTS_HYBRID_QUOTING crosslink #708" \
  grep -qE '#708|greedy book-first' skills/AGENTS_HYBRID_QUOTING.md

run_step "skill: AGENTS_BOOK_MATCH_HINT_SECURITY greedy hint" \
  grep -qE 'greedy|#708' skills/AGENTS_BOOK_MATCH_HINT_SECURITY.md

run_step "skill: AGENTS_TERRACLASSIC_GAS greedy G13" \
  grep -qE 'greedy|#708' skills/AGENTS_TERRACLASSIC_GAS.md

run_step "AGENTS.md verify-issue-708" \
  grep -qE 'verify-issue-708' AGENTS.md

echo ""
echo "── retest (dex-common + greedy gas) ──"
run_step "retest: dex-common greedy_swap" \
  bash -c 'cd smartcontracts && cargo test -p dex-common greedy_swap -- --nocapture'
run_step "retest: greedy gas vitest" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/terraGas.greedy.test.ts

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
