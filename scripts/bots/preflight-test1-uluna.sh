#!/usr/bin/env bash
# Query test1 uluna via LCD; WARN when below BOTS_ULUNA_WARN_FLOOR (default 100k LUNC).
# GitLab #372 — used by launch-swarm.sh before spawning workers.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEST1_ADDR="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"
WARN_FLOOR="${BOTS_ULUNA_WARN_FLOOR:-100000000000}"

# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"

LCD="${TERRA_LCD_URL:-http://127.0.0.1:1317}"
LCD="${LCD%/}"
port="${DEX_TERRA_LCD_PORT:-}"
if [[ -n "$port" && "$LCD" == "http://127.0.0.1:1317" ]]; then
  LCD="http://127.0.0.1:${port}"
fi

raw="$(localterra_lcd_curl "$LCD" "/cosmos/bank/v1beta1/balances/${TEST1_ADDR}" 2>/dev/null || true)"
if [[ -z "$raw" ]]; then
  echo "WARN: could not query test1 uluna; swarm may hit insufficient fee errors." >&2
  exit 0
fi

amount="$(echo "$raw" | jq -r '.balances[]? | select(.denom=="uluna") | .amount' | head -1)"
if [[ -z "$amount" || "$amount" == "null" ]]; then
  amount="0"
fi

if [[ "$amount" =~ ^[0-9]+$ ]] && ((10#$amount < 10#$WARN_FLOOR)); then
  echo "WARN: test1 uluna=${amount} (< ${WARN_FLOOR} floor). Run \`make reset && make deploy-local\` or reduce swarm workers. (GitLab #372)" >&2
else
  echo "preflight-test1-uluna: test1 uluna=${amount} (warn floor ${WARN_FLOOR})"
fi
