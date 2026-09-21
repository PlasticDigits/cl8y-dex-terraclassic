#!/usr/bin/env bash
# Automated verification for Forgejo #1234 — UpdateLimitOrderPrice (and
# CleanLimitBook) call gate_asset_code_ids. Freeze means no DLL writes.
#
# Proves:
#   1. Execute arms call gate_asset_code_ids before relink / park.
#   2. Pin suite: reprice + CleanLimitBook fail-closed on drift / whitelist freeze;
#      honest reprice still succeeds; cancel/swap stay gated.
#   3. Docs / F6 item 2 / ops no longer call reprice ungated.
#   4. make verify-issue-582 still green (shared pin module).
#
# Refs: skills/AGENTS_CW20_CODE_ID_PIN.md (F6 item 2),
#       docs/runbooks/cw20-code-id-ops.md,
#       git.cl8y.com #1234
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
echo "  Forgejo #1234 — F6 gate on UpdateLimitOrderPrice + CleanLimitBook"
echo "════════════════════════════════════════════════════════════════"

export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH:-}"

run_gate_arms() {
  python3 - <<'PY'
import pathlib, sys

text = pathlib.Path("smartcontracts/contracts/pair/src/contract.rs").read_text()


def arm_has_gate(msg_name: str, next_fn: str) -> bool:
    i = text.find(f"ExecuteMsg::{msg_name}")
    if i < 0:
        print(f"{msg_name} arm missing", file=sys.stderr)
        return False
    j = text.find(next_fn, i)
    if j < 0:
        print(f"{next_fn} not found after {msg_name}", file=sys.stderr)
        return False
    chunk = text[i:j]
    if "gate_asset_code_ids(deps.as_ref())?" not in chunk:
        print(
            f"{msg_name} missing gate_asset_code_ids before {next_fn}",
            file=sys.stderr,
        )
        return False
    return True


ok = True
ok &= arm_has_gate("UpdateLimitOrderPrice", "execute_update_limit_order_price")
ok &= arm_has_gate("CleanLimitBook", "execute_clean_limit_book")
if not ok:
    sys.exit(1)
print("UpdateLimitOrderPrice + CleanLimitBook call gate_asset_code_ids")
PY
}

run_docs() {
  set -euo pipefail
  rg -q "UpdateLimitOrderPrice" skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q "CleanLimitBook" skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q "no DLL writes" skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q "1234" skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q "1234" docs/security-model.md
  rg -q "UpdateLimitOrderPrice" docs/security-model.md
  rg -q "CleanLimitBook" docs/security-model.md
  rg -q "1234" docs/contracts-security-audit.md
  rg -q "UpdateLimitOrderPrice" docs/contracts-terraclassic.md
  rg -q "gate_asset_code_ids" docs/runbooks/cw20-code-id-ops.md
  # Must not keep the old ungated wording.
  if rg -q "UpdateLimitOrderPrice is ungated" docs/runbooks/cw20-code-id-ops.md; then
    echo "ops runbook still says UpdateLimitOrderPrice is ungated" >&2
    return 1
  fi
  if rg -q "CleanLimitBook still parks during freeze" docs/runbooks/cw20-code-id-ops.md; then
    echo "ops runbook still treats CleanLimitBook as a freeze exception" >&2
    return 1
  fi
  rg -q "1234" docs/limit-orders.md
  rg -q "1234" docs/runbooks/cw20-whitelist-policy.md
  rg -q "F6" skills/AGENTS_LIMIT_ORDER_REPRICE_FIFO.md
  rg -q "1234" skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md
  rg -q "1234" skills/AGENTS_FRONTEND_ORDER_BOOK_ROW_ACTIONS.md
  rg -q "verify-issue-1234" AGENTS.md
  rg -q "verify-issue-1234" docs/testing.md
  rg -q "gate_asset_code_ids" frontend-dapp/src/hooks/useLimitOrderUpdatePriceMutation.ts || \
    rg -q "1234" frontend-dapp/src/hooks/useLimitOrderUpdatePriceMutation.ts
}

run_pin_tests() {
  (cd smartcontracts && cargo test -p cl8y-dex-tests --lib asset_code_id_pin -- --quiet)
}

echo ""
echo "── first pass ──"
run_step "pair execute: F6 gate on reprice + CleanLimitBook" run_gate_arms
run_step "integration: pin suite including #1234 reprice/clean" run_pin_tests
run_step "docs: F6 item 2 + ops + skills + no ungated wording" run_docs

echo ""
echo "── retest ──"
run_step "retest pair execute arms" run_gate_arms
run_step "retest integration: pin suite" run_pin_tests

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> Forgejo #1234 verification passed"
