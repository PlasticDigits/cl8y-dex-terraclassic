#!/usr/bin/env bash
# Automated verification for Forgejo #1219 — named min remaining at place
# + descending ladder_prices (no Overflow: Cannot Sub).
#
# Proves:
#   1. dex-common: descending ladder unit tests + min remaining helper.
#   2. Integration: dust batch / mixed rung / ladder expand / T9 / 18-vs-6 raw 3→1.
#   3. Frontend: expandLimitLadder + escrow gate min-raw.
#   4. Docs / L24 / skill crosslinks.
#
# Refs: smartcontracts/packages/dex-common/src/limit_placement.rs,
#       docs/contracts-security-audit.md (L24),
#       skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md
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
echo "  Forgejo #1219 — named min remaining + descending ladder"
echo "════════════════════════════════════════════════════════════════"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH:-}"

run_step "dex-common: descending ladder + min remaining" \
  bash -c 'cd smartcontracts && cargo test -p dex-common ladder_prices_descending --quiet && cargo test -p dex-common expand_ladder_rejects_dust --quiet && cargo test -p dex-common min_limit_place_remaining --quiet'

run_step "integration: limit_place_min_size_1219" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests limit_place_min_size_1219 -- --test-threads=1 --quiet'

run_step "frontend: limitOrderLadder + escrow min-raw" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run limitOrderLadder limitOrderEscrowBalanceGate

run_step "docs: named error + L24 + skill" \
  bash -c '
    set -euo pipefail
    rg -q "LimitOrderAmountTooSmall" smartcontracts/contracts/pair/src/error.rs
    rg -q "below minimum" smartcontracts/contracts/pair/src/error.rs
    rg -q "min_limit_place_remaining" smartcontracts/packages/dex-common/src/limit_placement.rs
    rg -q "descending" smartcontracts/packages/dex-common/src/limit_placement.rs
    rg -q "ladder_prices_descending_raw_atomics_one_and_three" smartcontracts/packages/dex-common/src/limit_placement.rs
    rg -q "L24" docs/contracts-security-audit.md
    rg -q "S1219-1" skills/AGENTS_LIMIT_ORDER_BATCH_LADDER.md
    rg -q "Minimum size is 10 units" frontend-dapp/src/utils/limitOrderEscrowBalanceGate.ts
    rg -q "Minimum size is 10 units" frontend-dapp/src/utils/limitOrderLadder.ts
    rg -q "verify-issue-1219" AGENTS.md
    rg -q "limit-place-min-remaining" docs/integrators.md
    rg -q "#1219" docs/limit-orders.md
  '

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1219 verification passed"
