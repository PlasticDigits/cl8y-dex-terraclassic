#!/usr/bin/env bash
# Automated verification for GitLab #710 — greedy pause / blacklist / AfterSwap L7 /
# community-tax extra-debit (leftover of !480 / #708).
#
# Layers (no LocalTerra required):
#   1. community-tax greedy extra-debit (pair-direct + router trader)
#   2. cl8y-dex-tests greedy_book_first_708 pause / L7 / G1 / A14
#   3. cl8y-dex-tests greedy_blacklist maker skip + taker reject
#   4. Docs/skills tax / pause / L7 / blacklist
#   5. Retest tax + blacklist filters
#
# Refs: skills/AGENTS_GREEDY_BOOK_FIRST.md, skills/AGENTS_COMMUNITY_TAX_ROUTER.md, GitLab #710
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
echo "  GitLab #710 — greedy tax / pause / blacklist / AfterSwap L7"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "tax: greedy extra-debit pair-direct + router trader (T592-13)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-community-tax-token greedy -- --test-threads=1'

run_step "multitest: greedy_book_first_708 (pause L6, AfterSwap L7, A14 spoof)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests greedy_book_first_708 -- --test-threads=1'

run_step "blacklist: greedy_blacklist maker skip + taker reject (L19)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests greedy_blacklist -- --test-threads=1'

run_step "skill: AGENTS_GREEDY_BOOK_FIRST tax/pause/L7/blacklist" \
  grep -qE 'community-tax extra-debit|T592-13' skills/AGENTS_GREEDY_BOOK_FIRST.md

run_step "skill: AGENTS_COMMUNITY_TAX_ROUTER greedy Send+Swap" \
  grep -qE 'greedy|#710' skills/AGENTS_COMMUNITY_TAX_ROUTER.md

run_step "AGENTS.md verify-issue-710" \
  grep -qE 'verify-issue-710' AGENTS.md

echo ""
echo "── retest (tax greedy + blacklist greedy) ──"
run_step "retest: community-tax greedy" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-community-tax-token greedy -- --test-threads=1'
run_step "retest: greedy_blacklist" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests greedy_blacklist -- --test-threads=1'

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
