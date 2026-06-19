#!/usr/bin/env bash
# LocalTerra rehearsal for GitLab #399 — emergency pause/blacklist command cookbook (SEC-B11).
#
# Executes all eight factory emergency operations against a live LocalTerra deploy and
# confirms each with the post-tx queries documented in docs/runbooks/emergency-commands.md.
#
# Refs: docs/runbooks/emergency-commands.md, skills/AGENTS_EMERGENCY_COMMANDS.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1)"
TEST_ADDRESS="${TEST_ADDRESS:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"
# Dummy wallet for blacklist rehearsal (valid bech32; need not hold funds).
WALLET_FIXTURE="${VERIFY399_WALLET:-terra1f6jlx7d9y408tlzue7r2qcf79plp549n30yzqjajjud8vm7m4vds9nh0dv}"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

IDX_ENV="$REPO_ROOT/indexer/.env"
FE_ENV="$REPO_ROOT/frontend-dapp/.env.local"

FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [[ -z "$FACTORY" && -f "$FE_ENV" ]]; then
  FACTORY="$(read_env_var "$FE_ENV" VITE_FACTORY_ADDRESS)"
  LCD="$(read_env_var "$FE_ENV" VITE_TERRA_LCD_URL)"
  LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"
fi

if [[ -z "$CONTAINER_NAME" ]]; then
  echo "ERROR: localterra container not running (make start)." >&2
  exit 1
fi

if [[ -z "$FACTORY" ]]; then
  echo "ERROR: FACTORY address missing — run make deploy-local." >&2
  exit 1
fi

terrad_tx() {
  docker exec "$CONTAINER_NAME" terrad tx "$@" \
    --from test1 --keyring-backend test --chain-id "$CHAIN_ID" \
    --gas auto --gas-adjustment 1.4 --fees 500000000uluna \
    --node "$TERRAD_NODE" --broadcast-mode sync -y --output json
}

exec_factory() {
  local msg_json="$1"
  local txhash
  txhash="$(terrad_tx wasm execute "$FACTORY" "$msg_json" | jq -r '.txhash')"
  terrad_wait_tx_inclusion "$CONTAINER_NAME" "$txhash" "$TERRAD_NODE" 90
}

pair_is_paused() {
  docker exec "$CONTAINER_NAME" terrad query wasm contract-state smart "$1" \
    '{"is_paused":{}}' --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r 'if (.data.paused | type) == "boolean" then (.data.paused | tostring) else empty end'
}

poll_pair_paused() {
  local pair="$1"
  local expect="$2"
  local tries=12
  local paused=""
  while (( tries > 0 )); do
    paused="$(pair_is_paused "$pair")"
    if [[ "$paused" == "$expect" ]]; then
      echo "$paused"
      return 0
    fi
    sleep 1
    tries=$((tries - 1))
  done
  echo "$paused"
  return 1
}

blacklist_check() {
  local wallet="${1:-}"
  local token="${2:-}"
  local pair="${3:-}"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" \
    "$(jq -nc \
      --arg wallet "$wallet" \
      --arg token "$token" \
      --arg pair "$pair" \
      '{
        blacklist_check: {
          wallet: (if $wallet == "" then null else $wallet end),
          tokens: (if $token == "" then [] else [$token] end),
          pair: (if $pair == "" then null else $pair end),
          pairs: []
        }
      }')")"
}

resolve_pair_and_token() {
  local pairs_doc pair="" t0="" t1=""
  pairs_doc="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":20}}')")"
  while IFS= read -r row; do
    [[ -n "$row" ]] || continue
    t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
    t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
    if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
      pair="$(echo "$row" | jq -r '.contract_addr')"
      PAIR="$pair"
      TOKEN0="$t0"
      TOKEN1="$t1"
      return 0
    fi
  done < <(echo "$pairs_doc" | jq -c '.pairs[]? // empty')

  echo "ERROR: no dual-CW20 pair on factory $FACTORY." >&2
  exit 1
}

run_pause_cycle() {
  echo ""
  echo "[pause / unpause pair]"
  local paused

  exec_factory "$(jq -nc --arg pair "$PAIR" '{set_pair_paused:{pair:$pair,paused:true}}')"
  if paused="$(poll_pair_paused "$PAIR" "true")"; then
    ok "SetPairPaused paused=true confirmed via is_paused"
  else
    bad "SetPairPaused paused=true — is_paused=$paused"
  fi

  exec_factory "$(jq -nc --arg pair "$PAIR" '{set_pair_paused:{pair:$pair,paused:false}}')"
  if paused="$(poll_pair_paused "$PAIR" "false")"; then
    ok "SetPairPaused paused=false confirmed via is_paused"
  else
    bad "SetPairPaused paused=false — is_paused=$paused"
  fi
}

run_wallet_blacklist_cycle() {
  echo ""
  echo "[blacklist / unblacklist wallet]"
  local check blocked wallet_bl

  exec_factory "$(jq -nc --arg address "$WALLET_FIXTURE" '{blacklist_wallet:{address:$address}}')"
  check="$(blacklist_check "$WALLET_FIXTURE" "$TOKEN0" "$PAIR")"
  blocked="$(echo "$check" | jq -r '.blocked')"
  wallet_bl="$(echo "$check" | jq -r '.wallet_blacklisted')"
  if [[ "$blocked" == "true" && "$wallet_bl" == "true" ]]; then
    ok "BlacklistWallet confirmed via blacklist_check"
  else
    bad "BlacklistWallet — blocked=$blocked wallet_blacklisted=$wallet_bl"
  fi

  exec_factory "$(jq -nc --arg address "$WALLET_FIXTURE" '{unblacklist_wallet:{address:$address}}')"
  check="$(blacklist_check "$WALLET_FIXTURE" "$TOKEN0" "$PAIR")"
  wallet_bl="$(echo "$check" | jq -r '.wallet_blacklisted')"
  if [[ "$wallet_bl" == "false" ]]; then
    ok "UnblacklistWallet confirmed via blacklist_check"
  else
    bad "UnblacklistWallet — wallet_blacklisted=$wallet_bl"
  fi
}

run_token_blacklist_cycle() {
  echo ""
  echo "[blacklist / unblacklist token]"
  local check blocked tokens

  exec_factory "$(jq -nc --arg token "$TOKEN0" '{blacklist_token:{token:$token}}')"
  check="$(blacklist_check "" "$TOKEN0" "")"
  blocked="$(echo "$check" | jq -r '.blocked')"
  tokens="$(echo "$check" | jq -r --arg t "$TOKEN0" '[.blacklisted_tokens[]?] | index($t) != null')"
  if [[ "$blocked" == "true" && "$tokens" == "true" ]]; then
    ok "BlacklistToken confirmed via blacklist_check"
  else
    bad "BlacklistToken — blocked=$blocked token_listed=$tokens"
  fi

  exec_factory "$(jq -nc --arg token "$TOKEN0" '{unblacklist_token:{token:$token}}')"
  check="$(blacklist_check "" "$TOKEN0" "")"
  tokens="$(echo "$check" | jq -r --arg t "$TOKEN0" '[.blacklisted_tokens[]?] | index($t) != null')"
  if [[ "$tokens" == "false" ]]; then
    ok "UnblacklistToken confirmed via blacklist_check"
  else
    bad "UnblacklistToken — token still listed"
  fi
}

run_pair_blacklist_cycle() {
  echo ""
  echo "[blacklist / unblacklist pair]"
  local check blocked pair_bl

  exec_factory "$(jq -nc --arg pair "$PAIR" '{blacklist_pair:{pair:$pair}}')"
  check="$(blacklist_check "" "" "$PAIR")"
  blocked="$(echo "$check" | jq -r '.blocked')"
  pair_bl="$(echo "$check" | jq -r '.pair_blacklisted')"
  if [[ "$blocked" == "true" && "$pair_bl" == "true" ]]; then
    ok "BlacklistPair confirmed via blacklist_check"
  else
    bad "BlacklistPair — blocked=$blocked pair_blacklisted=$pair_bl"
  fi

  exec_factory "$(jq -nc --arg pair "$PAIR" '{unblacklist_pair:{pair:$pair}}')"
  check="$(blacklist_check "" "" "$PAIR")"
  pair_bl="$(echo "$check" | jq -r '.pair_blacklisted')"
  if [[ "$pair_bl" == "false" ]]; then
    ok "UnblacklistPair confirmed via blacklist_check"
  else
    bad "UnblacklistPair — pair_blacklisted=$pair_bl"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #399 — emergency command LocalTerra rehearsal (SEC-B11)"
echo "════════════════════════════════════════════════════════════════"
echo "  FACTORY=$FACTORY  LCD=$LCD"
echo "  PAIR/TOKEN resolved from factory pairs query"

resolve_pair_and_token
echo "  PAIR=$PAIR  TOKEN0=$TOKEN0  WALLET_FIXTURE=$WALLET_FIXTURE"

run_pause_cycle
run_wallet_blacklist_cycle
run_token_blacklist_cycle
run_pair_blacklist_cycle

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
