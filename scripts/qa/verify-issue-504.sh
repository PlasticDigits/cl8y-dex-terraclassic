#!/usr/bin/env bash
# Verification for GitLab #504 — ExpiredLimitParkReason on parked refund rows.
#
# Contract multi-tests cover all four park reasons + legacy decode (no chain).
# Optional LocalTerra LCD smoke: set VERIFY504_LCD=1 (needs deploy with #504 wasm).
#
# Refs: docs/limit-orders.md#expired-limit-park-reason-gitlab-504,
#       docs/contracts-security-audit.md (L22),
#       skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md
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
echo "  GitLab #504 — expired-limit park reason discriminator"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "dex-common: expired_limit_park_reason unit tests" \
  bash -c 'cd smartcontracts && cargo test -p dex-common expired_limit_park_reason -- --quiet'

run_step "pair unit: match_bid_dust_remainder" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair --lib match_bid_dust_remainder -- --quiet'

run_step "integration: match_dust_flush (DustFilled)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests --lib match_dust_flush -- --quiet'

run_step "integration: clean_limit_book TTL Expired reason" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests --lib clean_limit_book_parks_expired_head_default_config -- --quiet'

run_step "integration: clean_limit_book ForceCleaned reason" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests --lib clean_limit_book_force_dust_bid_then_claim_refunds -- --quiet'

run_step "integration: blacklist Blacklisted reason" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests --lib blacklisted_maker_resting -- --quiet'

run_step "integration: order_status parked after CleanLimitBook reason" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests --lib order_status_parked_after_clean_limit_book -- --quiet'

run_step "docs: park-reason anchors + skill present" \
  bash -c '
    set -euo pipefail
    rg -q "expired-limit-park-reason-gitlab-504" docs/limit-orders.md
    rg -q "expired-limit-park-reason-gitlab-504" docs/integrators.md
    rg -q "L22" docs/contracts-security-audit.md
    rg -q "ExpiredLimitParkReason" docs/contracts-security-audit.md
    test -f skills/AGENTS_EXPIRED_LIMIT_PARK_REASON.md
    rg -q "AGENTS_EXPIRED_LIMIT_PARK_REASON" AGENTS.md
    rg -q "AGENTS_EXPIRED_LIMIT_PARK_REASON" skills/AGENTS_ORDER_STATUS_QUERY.md
  '

if [[ "${VERIFY504_LCD:-0}" == "1" ]]; then
  echo ""
  echo "[optional LCD] LocalTerra dust-flush -> reason=DustFilled"
  chmod +x "$REPO_ROOT/scripts/qa/verify-issue-504-lcd.sh"
  if "$REPO_ROOT/scripts/qa/verify-issue-504-lcd.sh"; then
    ok "LCD dust-flush reason=DustFilled"
  else
    bad "LCD dust-flush reason=DustFilled"
  fi
else
  echo ""
  echo "[skip] VERIFY504_LCD unset — contract tests are the primary gate; set VERIFY504_LCD=1 after deploy-local"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
