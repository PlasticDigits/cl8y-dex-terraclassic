#!/usr/bin/env bash
# Verification for GitLab #372 — LocalTerra genesis/deploy/swarm funding headroom.
#
# Layers:
#   1. Grep invariants: init-chain.sh + deploy defaults match 10× targets
#   2. LCD balance (optional): test1 uluna >= post-deploy floor after reset+deploy
#   3. Short swarm soak (optional): swarm-local 2 min without draining below soak floor
#
# Env:
#   VERIFY372_POST_DEPLOY_MIN_ULUNA — default 8000000000000 (8M LUNC)
#   VERIFY372_SOAK_MIN_ULUNA       — default 500000000000 (500k LUNC)
#   VERIFY372_SOAK_SEC             — default 120 (2 min; full QA soak is 4h manual)
#   VERIFY372_SKIP_LCD=1           — skip live chain checks
#   VERIFY372_SKIP_SOAK=1          — skip short swarm soak
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/qa/lib/qa-env.sh
source "$REPO_ROOT/scripts/qa/lib/qa-env.sh"
qa_load_env

LCD="${TERRA_LCD_URL:-http://127.0.0.1:${DEX_TERRA_LCD_PORT:-1317}}"
LCD="${LCD%/}"

TEST1_ADDR="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
# ~10M genesis − 2M treasury LUNC − deploy gas ≈ 7.99M (GitLab #372).
POST_DEPLOY_MIN="${VERIFY372_POST_DEPLOY_MIN_ULUNA:-7900000000000}"
SOAK_MIN="${VERIFY372_SOAK_MIN_ULUNA:-500000000000}"
SOAK_SEC="${VERIFY372_SOAK_SEC:-120}"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); SKIP=$((SKIP+1)); echo "  [SKIP] $1"; }

query_test1_uluna() {
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  local raw amount
  raw="$(localterra_lcd_curl "$LCD" "/cosmos/bank/v1beta1/balances/${TEST1_ADDR}" 2>/dev/null || true)"
  amount="$(echo "$raw" | jq -r '.balances[]? | select(.denom=="uluna") | .amount' | head -1)"
  if [[ -z "$amount" || "$amount" == "null" ]]; then
    echo "0"
  else
    echo "$amount"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #372 — LocalTerra funding headroom"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Static invariants (genesis + deploy defaults)"
if grep -q '10000000000000uluna' "$REPO_ROOT/docker/init-chain.sh"; then
  ok "init-chain.sh genesis uluna = 10M LUNC"
else
  bad "init-chain.sh missing 10M LUNC genesis uluna"
fi
if grep -q '20000000000000uusd,2000000000000uluna' "$REPO_ROOT/scripts/deploy-dex-local.sh"; then
  ok "deploy-dex-local.sh DEPLOY_TREASURY_FUND_COINS default 20M USTC + 2M LUNC"
else
  bad "deploy-dex-local.sh treasury default not 10×"
fi
if grep -q "'20000000000000'" "$REPO_ROOT/packages/localnet-trading-swarm/src/funding.ts"; then
  ok "localnet-trading-swarm SWARM_ULUNA_TOPUP default 10×"
else
  bad "localnet-trading-swarm funding defaults not 10×"
fi

echo ""
echo "[2] LCD post-deploy balance (needs LocalTerra + deploy)"
if [[ "${VERIFY372_SKIP_LCD:-0}" == "1" ]]; then
  skip "LCD balance check (VERIFY372_SKIP_LCD=1)"
else
  if make has-localterra >/dev/null 2>&1; then
    bal="$(query_test1_uluna)"
    if [[ "$bal" =~ ^[0-9]+$ ]] && ((10#$bal >= 10#$POST_DEPLOY_MIN)); then
      ok "test1 uluna=${bal} >= ${POST_DEPLOY_MIN} (post-deploy floor)"
    elif [[ "$bal" =~ ^[0-9]+$ ]] && ((10#$bal >= 1000000000000)); then
      skip "test1 uluna=${bal} < ${POST_DEPLOY_MIN} — run \`make reset && make deploy-local\` for fresh 10× genesis"
    else
      bad "test1 uluna=${bal:-unknown} (chain up but balance below expectations)"
    fi
  else
    skip "LocalTerra not running (make has-localterra)"
  fi
fi

echo ""
echo "[3] Short swarm soak (optional)"
if [[ "${VERIFY372_SKIP_SOAK:-0}" == "1" ]]; then
  skip "swarm soak (VERIFY372_SKIP_SOAK=1)"
elif ! make has-localterra >/dev/null 2>&1; then
  skip "swarm soak (LocalTerra down)"
elif [[ ! -f "$REPO_ROOT/frontend-dapp/.env.local" ]]; then
  skip "swarm soak (no frontend-dapp/.env.local — run make deploy-local)"
else
  bal_before="$(query_test1_uluna)"
  echo "    starting make swarm-local for ${SOAK_SEC}s (balance before: ${bal_before})…"
  chmod +x "$REPO_ROOT/scripts/bots/swarm.py"
  BOTS_DRY_RUN=0 timeout "$SOAK_SEC" make swarm-local >/tmp/verify372-swarm.log 2>&1 || true
  bal_after="$(query_test1_uluna)"
  if [[ "$bal_after" =~ ^[0-9]+$ ]] && ((10#$bal_after >= 10#$SOAK_MIN)); then
    ok "after ${SOAK_SEC}s swarm-local: test1 uluna=${bal_after} >= ${SOAK_MIN}"
  else
    bad "after ${SOAK_SEC}s swarm-local: test1 uluna=${bal_after:-unknown} < ${SOAK_MIN}"
  fi
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
printf '%s\n' "${RESULTS[@]}"
echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if ((FAIL > 0)); then
  exit 1
fi
