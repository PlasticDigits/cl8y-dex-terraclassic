#!/usr/bin/env bash
# Automated verification for GitLab #709 — greedy quote=execute mutex,
# remainder_to_pool stop reason, pool_spot Decimal overflow (leftover of !480 / #708).
#
# Layers (no LocalTerra required):
#   1. dex-common G11 mutex + remainder_to_pool wire name
#   2. pair greedy overflow + equal-rate + remainder stop
#   3. cl8y-dex-tests greedy_book_first_708 (query mutex, remainder, filled)
#   4. Docs/skills G7/G11 query + remainder_to_pool
#   5. Retest unit + remainder grep
#
# Refs: skills/AGENTS_GREEDY_BOOK_FIRST.md, GitLab #709
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
echo "  GitLab #709 — greedy query mutex + remainder_to_pool + A7"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "dex-common: greedy_swap_mode_tests (G11 + remainder_to_pool wire)" \
  bash -c 'cd smartcontracts && cargo test -p dex-common greedy_swap -- --nocapture'

run_step "pair: greedy overflow Skip + equal-rate No + remainder stop" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair greedy -- --nocapture'

run_step "multitest: greedy_book_first_708 (query mutex, remainder, filled, G7 stop)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests greedy_book_first_708 -- --test-threads=1'

run_step "docs: integrators G11 query mutex / remainder_to_pool" \
  grep -qE 'remainder_to_pool|#709' docs/integrators.md

run_step "docs: contracts-security-audit G11 query / A7" \
  grep -qE '#709|remainder_to_pool' docs/contracts-security-audit.md

run_step "skill: AGENTS_GREEDY_BOOK_FIRST G11 query + remainder_to_pool" \
  grep -qE 'remainder_to_pool' skills/AGENTS_GREEDY_BOOK_FIRST.md

run_step "skill: pair query uses resolve_swap_hybrid_mode both fields" \
  grep -qE 'resolve_swap_hybrid_mode\(hybrid, greedy\)' skills/AGENTS_GREEDY_BOOK_FIRST.md

run_step "AGENTS.md verify-issue-709" \
  grep -qE 'verify-issue-709' AGENTS.md

echo ""
echo "── retest (dex-common + pair greedy) ──"
run_step "retest: dex-common greedy_swap" \
  bash -c 'cd smartcontracts && cargo test -p dex-common greedy_swap -- --nocapture'
run_step "retest: pair greedy" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair greedy -- --nocapture'

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
