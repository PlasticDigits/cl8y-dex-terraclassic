#!/usr/bin/env bash
# Verify LocalTerra test1 funding headroom after genesis/deploy 10× bump (GitLab #372).
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
TEST1="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
MIN_POST_DEPLOY_ULUNA="${VERIFY_FUNDING_MIN_ULUNA:-8000000000000}"   # 8M LUNC
MIN_AFTER_SOAK_ULUNA="${VERIFY_FUNDING_SOAK_MIN_ULUNA:-500000000000}" # 500k LUNC
SOAK_SEC="${VERIFY_FUNDING_SOAK_SEC:-120}"

lcd_uluna() {
  local lcd="${TERRA_LCD_URL:-http://127.0.0.1:1317}"
  curl -sf "${lcd}/cosmos/bank/v1beta1/balances/${TEST1}" \
    | jq -r '.balances[] | select(.denom=="uluna") | .amount // empty'
}

echo "=== verify-localterra-funding-headroom (#372) ==="

if ! make -C "$REPO_ROOT" has-localterra >/dev/null 2>&1; then
  echo "FAIL: LocalTerra not running (make start && make wait-healthy)" >&2
  exit 1
fi

amount="$(lcd_uluna || true)"
if [[ -z "$amount" ]]; then
  echo "FAIL: could not read test1 uluna from LCD" >&2
  exit 1
fi

echo "test1 uluna now: $amount"
if [[ "$amount" -lt "$MIN_POST_DEPLOY_ULUNA" ]]; then
  echo "FAIL: uluna $amount < post-deploy floor $MIN_POST_DEPLOY_ULUNA (8M LUNC)" >&2
  exit 1
fi
echo "PASS: post-deploy floor (>= 8M LUNC)"

if [[ "${VERIFY_FUNDING_SKIP_SOAK:-0}" == "1" ]]; then
  echo "SKIP: short swarm soak (VERIFY_FUNDING_SKIP_SOAK=1)"
  exit 0
fi

echo "Running swarm soak ${SOAK_SEC}s (set VERIFY_FUNDING_SOAK_SEC for longer)…"
chmod +x "$REPO_ROOT/scripts/bots/launch-swarm.sh" "$REPO_ROOT/scripts/bots/stop-swarm.sh"
BOTS_MEAN_INTERVAL_SEC=15 BOTS_LIMIT_MEAN_INTERVAL_SEC=30 BOTS_LP_MEAN_INTERVAL_SEC=45 \
  "$REPO_ROOT/scripts/bots/launch-swarm.sh"
sleep "$SOAK_SEC"
"$REPO_ROOT/scripts/bots/stop-swarm.sh" || true

amount="$(lcd_uluna || true)"
echo "test1 uluna after soak: $amount"
if [[ -z "$amount" ]]; then
  echo "FAIL: could not read test1 uluna after soak" >&2
  exit 1
fi
if [[ "$amount" -lt "$MIN_AFTER_SOAK_ULUNA" ]]; then
  echo "FAIL: uluna $amount < soak floor $MIN_AFTER_SOAK_ULUNA (500k LUNC)" >&2
  exit 1
fi
echo "PASS: soak floor (>= 500k LUNC after ${SOAK_SEC}s swarm)"
