#!/usr/bin/env bash
# Automated verification for #1228 — SendFrom listed-pair Sell allowance = TaxPreview.debit.
#
# Proves (unit + docs, twice):
#   1. SendFrom allowance rows (amount reverts; debit succeeds; TransferFrom 1:1)
#   2. Existing owner Send extra-debit + greedy extra-debit stay green
#   3. execute_send_from deducts preview.debit; TransferFrom still deducts amount
#   4. Docs/skills A-allow / T592-2; pair/router/factory wasm untouched
#
# Refs: skills/AGENTS_COMMUNITY_TAX_CW20.md (T592-2 / A-allow)
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
echo "  #1228 — SendFrom listed-pair Sell allowance covers economic debit"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-1228-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi
if [[ ! -e "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-1228-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_send_from() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token send_from -- --test-threads=1)
}

run_transfer_from_allowance() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token transfer_from_listed_pair_allowance_is_amount -- --test-threads=1)
}

run_owner_sell() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token sell_extra_debit_on_swap_send -- --test-threads=1)
}

run_greedy() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token greedy -- --test-threads=1)
}

run_docs() {
  set -euo pipefail
  rg -q "send_from_listed_sell_allowance_amount_reverts" \
    smartcontracts/contracts/community-tax-token/src/multitest.rs
  rg -q "send_from_listed_sell_allowance_debit_succeeds" \
    smartcontracts/contracts/community-tax-token/src/multitest.rs
  rg -q "send_from_listed_sell_allowance_debit_minus_one_reverts" \
    smartcontracts/contracts/community-tax-token/src/multitest.rs
  rg -q "A-allow" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "A-allow" docs/contracts-security-audit.md
  rg -q "SendFrom" docs/contracts-terraclassic.md
  rg -q "verify-issue-1228" AGENTS.md
  rg -q "verify-issue-1228" docs/testing.md
  rg -q "preview.debit" smartcontracts/contracts/community-tax-token/src/contract.rs
  # TransferFrom must stay 1:1 (deduct declared amount, no Swap hook).
  rg -n "fn execute_transfer_from" -A 20 \
    smartcontracts/contracts/community-tax-token/src/contract.rs \
    | rg -q "deduct_allowance\(deps.storage, &owner_addr, &info.sender, &env.block, amount\)"
  # Pair / router / factory wasm must not change on this ticket.
  if git diff --name-only -- \
      smartcontracts/contracts/pair \
      smartcontracts/contracts/router \
      smartcontracts/contracts/factory \
      | rg -q .; then
    echo "FAIL: pair/router/factory files changed" >&2
    git diff --name-only -- \
      smartcontracts/contracts/pair \
      smartcontracts/contracts/router \
      smartcontracts/contracts/factory
    return 1
  fi
}

echo ""
echo "── first pass ──"
run_step "crates: SendFrom allowance rows (#1228)" run_send_from
run_step "crates: TransferFrom listed pair 1:1 allowance" run_transfer_from_allowance
run_step "crates: owner Send extra-debit (AC9)" run_owner_sell
run_step "crates: greedy extra-debit (#710)" run_greedy
run_step "docs: A-allow + preview.debit + pair/router untouched" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: SendFrom allowance rows" run_send_from
run_step "retest crates: TransferFrom listed pair 1:1" run_transfer_from_allowance
run_step "retest crates: owner Send extra-debit" run_owner_sell
run_step "retest docs: A-allow + preview.debit + pair/router untouched" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> #1228 verification passed"
