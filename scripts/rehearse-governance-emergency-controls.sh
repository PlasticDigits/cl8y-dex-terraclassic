#!/usr/bin/env bash
# LocalTerra / staging dry-run: governance emergency controls via multisig threshold signing (SEC-B09, GitLab #397).
#
# Exercises pause → blacklist wallet → unpause → unblacklist using the same terrad multisig flow
# operators use for production governance wallets (2-of-3 threshold on LocalTerra; mirror on testnet/mainnet).
#
# Prerequisites: make start && make deploy-local (or make setup-cloud-localterra).
#
# Usage:
#   ./scripts/rehearse-governance-emergency-controls.sh
#   ./scripts/rehearse-governance-emergency-controls.sh --output /tmp/rehearsal.md
#
# Post the generated transcript (tx hashes) on GitLab #397 or the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"
# shellcheck source=scripts/lib/terrad-multisig-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-multisig-tx.sh"

OUTPUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) OUTPUT="$2"; shift 2 ;;
    -h|--help)
      echo "Usage: $0 [--output FILE]"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
KEYRING="${KEYRING:-test}"
FEES="${REHEARSAL_FEES:-500000000uluna}"
GAS_ADJ="${REHEARSAL_GAS_ADJ:-1.4}"
MSIG_NAME="${GOV_MSIG_NAME:-gov-rehearsal-msig}"
SIGNER1="${GOV_MSIG_SIGNER1:-gov-rehearsal-1}"
SIGNER2="${GOV_MSIG_SIGNER2:-gov-rehearsal-2}"
SIGNER3="${GOV_MSIG_SIGNER3:-gov-rehearsal-3}"
THRESHOLD="${GOV_MSIG_THRESHOLD:-2}"
BLACKLIST_TARGET="${BLACKLIST_TARGET:-terra1kk96k37mkr89dgzaqtjnd9y7h73mjj63m3naqw}"
DEPLOY_GOV="${DEPLOY_GOV:-terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v}"

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER_NAME" ]]; then
  CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 || true)"
fi

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }

IDX_ENV="$REPO_ROOT/indexer/.env"
FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"
LCD="$(read_env_var "$IDX_ENV" LCD_URLS)"; LCD="${LCD%%,*}"
LCD="${LCD:-http://127.0.0.1:1317}"; LCD="${LCD%/}"

if [[ -z "$CONTAINER_NAME" ]]; then
  echo "ERROR: localterra container not running (make start)." >&2
  exit 1
fi
if [[ -z "$FACTORY" ]]; then
  echo "ERROR: FACTORY_ADDRESS missing in indexer/.env — run make deploy-local." >&2
  exit 1
fi

declare -a TX_LOG=()
declare -a STEP_LOG=()

log_step() { STEP_LOG+=("$1"); echo "  → $1"; }
record_tx() {
  local label="$1"
  local hash="$2"
  TX_LOG+=("$label|$hash")
  echo "    txhash=$hash"
}

terrad_tx_test1() {
  docker exec "$CONTAINER_NAME" terrad tx "$@" \
    --from test1 \
    --keyring-backend "$KEYRING" \
    --chain-id "$CHAIN_ID" \
    --gas auto \
    --gas-adjustment 1.3 \
    --fees "$FEES" \
    --node "$TERRAD_NODE" \
    --broadcast-mode sync \
    -y \
    --output json
}

wait_tx() {
  terrad_wait_tx_inclusion "$CONTAINER_NAME" "$1" "$TERRAD_NODE" 120
}

ensure_signer_key() {
  local name="$1"
  if docker exec "$CONTAINER_NAME" terrad keys show "$name" --keyring-backend "$KEYRING" >/dev/null 2>&1; then
    return 0
  fi
  docker exec "$CONTAINER_NAME" terrad keys add "$name" --keyring-backend "$KEYRING" --no-backup >/dev/null 2>&1
}

setup_multisig() {
  ensure_signer_key "$SIGNER1"
  ensure_signer_key "$SIGNER2"
  ensure_signer_key "$SIGNER3"

  if ! docker exec "$CONTAINER_NAME" terrad keys show "$MSIG_NAME" --keyring-backend "$KEYRING" >/dev/null 2>&1; then
    docker exec "$CONTAINER_NAME" terrad keys add "$MSIG_NAME" \
      --multisig "$SIGNER1,$SIGNER2,$SIGNER3" \
      --multisig-threshold "$THRESHOLD" \
      --keyring-backend "$KEYRING" --no-backup >/dev/null 2>&1
  fi

  MSIG_ADDR="$(docker exec "$CONTAINER_NAME" terrad keys show "$MSIG_NAME" -a --keyring-backend "$KEYRING")"
  echo "$MSIG_ADDR"
}

fund_if_needed() {
  local addr="$1"
  local bal
  bal="$(docker exec "$CONTAINER_NAME" terrad query bank balances "$addr" --node "$TERRAD_NODE" --output json \
    | jq -r '.balances[] | select(.denom=="uluna") | .amount' | head -1)"
  bal="${bal:-0}"
  # Multisig rehearsal runs 4+ wasm executes at ~500M uluna fees each.
  if [[ "$(echo "$bal >= 3000000000" | bc)" != "1" ]]; then
    local tx
    tx="$(terrad_tx_test1 bank send "$DEPLOY_GOV" "$addr" 10000000000uluna | jq -r '.txhash')"
    wait_tx "$tx"
    record_tx "fund multisig" "$tx"
  fi
}

rotate_governance_to_multisig() {
  local msig_addr="$1"
  local cfg
  cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"config":{}}')")"
  local current_gov
  current_gov="$(echo "$cfg" | jq -r '.governance // empty')"
  if [[ "$current_gov" == "$msig_addr" ]]; then
    log_step "factory governance already multisig ($msig_addr)"
    return 0
  fi
  log_step "rotate factory governance to rehearsal multisig"
  local msg
  msg="$(printf '{"update_config":{"governance":"%s"}}' "$msig_addr")"
  local tx
  tx="$(terrad_tx_test1 wasm execute "$FACTORY" "$msg" | jq -r '.txhash')"
  wait_tx "$tx"
  record_tx "UpdateConfig governance→multisig" "$tx"
}

msig_execute() {
  local label="$1"
  local msg_json="$2"
  fund_if_needed "$MSIG_ADDR"
  log_step "$label (multisig ${THRESHOLD}-of-3)"
  local out hash
  out="$(terrad_multisig_wasm_execute "$CONTAINER_NAME" "$TERRAD_NODE" "$CHAIN_ID" \
    "$MSIG_NAME" "$KEYRING" "$FEES" "$GAS_ADJ" "$FACTORY" "$msg_json" "$SIGNER1" "$SIGNER2")"
  hash="$(echo "$out" | jq -r '.txhash // empty')"
  if [[ -z "$hash" ]]; then
    echo "ERROR: multisig broadcast failed for $label" >&2
    echo "$out" >&2
    exit 1
  fi
  wait_tx "$hash"
  record_tx "$label" "$hash"
}

first_pair_addr() {
  local raw doc
  raw="$(lcd_smart_query_raw "$LCD" "$FACTORY" '{"pairs":{"start_after":null,"limit":1}}')"
  doc="$(lcd_decode_smart_data "$raw")"
  echo "$doc" | jq -r '.pairs[0].contract_addr // empty'
}

pair_paused() {
  local pair="$1"
  local raw doc
  raw="$(lcd_smart_query_raw "$LCD" "$pair" '{"is_paused":{}}')"
  doc="$(lcd_decode_smart_data "$raw")"
  echo "$doc" | jq -r '.paused // false'
}

wallet_blacklisted() {
  local wallet="$1"
  local raw doc
  raw="$(lcd_smart_query_raw "$LCD" "$FACTORY" \
    "$(printf '{"blacklist_check":{"wallet":"%s","tokens":[],"pair":null,"pairs":[]}}' "$wallet")")"
  doc="$(lcd_decode_smart_data "$raw")"
  echo "$doc" | jq -r '.wallet_blacklisted // false'
}

echo "════════════════════════════════════════════════════════════════"
echo "  Governance emergency controls rehearsal (SEC-B09)"
echo "  network=$CHAIN_ID factory=$FACTORY"
echo "════════════════════════════════════════════════════════════════"

MSIG_ADDR="$(setup_multisig)"
echo "  multisig=$MSIG_ADDR (${THRESHOLD}-of-3: $SIGNER1,$SIGNER2,$SIGNER3)"

PAIR="$(first_pair_addr)"
if [[ -z "$PAIR" ]]; then
  echo "ERROR: no pair found on factory" >&2
  exit 1
fi
echo "  pair=$PAIR blacklist_target=$BLACKLIST_TARGET"

fund_if_needed "$MSIG_ADDR"
rotate_governance_to_multisig "$MSIG_ADDR"

# Ensure clean starting state so each run exercises all four multisig messages.
if [[ "$(wallet_blacklisted "$BLACKLIST_TARGET")" = "true" ]]; then
  msig_execute "UnblacklistWallet (reset)" \
    "$(printf '{"unblacklist_wallet":{"address":"%s"}}' "$BLACKLIST_TARGET")"
fi
if [[ "$(pair_paused "$PAIR")" = "true" ]]; then
  msig_execute "SetPairPaused paused=false (reset)" \
    "$(printf '{"set_pair_paused":{"pair":"%s","paused":false}}' "$PAIR")"
fi

# 1. Pause
msig_execute "SetPairPaused paused=true" \
  "$(printf '{"set_pair_paused":{"pair":"%s","paused":true}}' "$PAIR")"
if [[ "$(pair_paused "$PAIR")" != "true" ]]; then
  echo "ERROR: pair not paused after SetPairPaused" >&2
  exit 1
fi
log_step "verified pair paused=true"

# 2. Blacklist wallet
msig_execute "BlacklistWallet" \
  "$(printf '{"blacklist_wallet":{"address":"%s"}}' "$BLACKLIST_TARGET")"
if [[ "$(wallet_blacklisted "$BLACKLIST_TARGET")" != "true" ]]; then
  echo "ERROR: wallet not blacklisted" >&2
  exit 1
fi
log_step "verified wallet_blacklisted=true"

# 3. Unpause
msig_execute "SetPairPaused paused=false" \
  "$(printf '{"set_pair_paused":{"pair":"%s","paused":false}}' "$PAIR")"
if [[ "$(pair_paused "$PAIR")" != "false" ]]; then
  echo "ERROR: pair still paused after unpause" >&2
  exit 1
fi
log_step "verified pair paused=false"

# 4. Unblacklist
msig_execute "UnblacklistWallet" \
  "$(printf '{"unblacklist_wallet":{"address":"%s"}}' "$BLACKLIST_TARGET")"
if [[ "$(wallet_blacklisted "$BLACKLIST_TARGET")" != "false" ]]; then
  echo "ERROR: wallet still blacklisted" >&2
  exit 1
fi
log_step "verified wallet_blacklisted=false"

TS_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
TRANSCRIPT="$(mktemp /tmp/governance-emergency-rehearsal-XXXXXX.md)"
cat >"$TRANSCRIPT" <<EOF
# Governance emergency controls rehearsal evidence

| Field | Value |
|-------|-------|
| Checklist | SEC-B09 (GitLab [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)) |
| Network | $CHAIN_ID |
| Factory | \`$FACTORY\` |
| Governance multisig | \`$MSIG_ADDR\` (${THRESHOLD}-of-3: \`$SIGNER1\`, \`$SIGNER2\`, \`$SIGNER3\`) |
| Pair exercised | \`$PAIR\` |
| Blacklist target | \`$BLACKLIST_TARGET\` |
| Rehearsal UTC | $TS_UTC |
| Signing flow | \`terrad tx sign\` + \`terrad tx multisign\` + \`terrad tx broadcast\` (threshold $THRESHOLD) |

## Operations (in order)

| Step | Factory message | Tx hash |
|------|-----------------|---------|
EOF

for entry in "${TX_LOG[@]}"; do
  label="${entry%%|*}"
  hash="${entry#*|}"
  printf '| %s | see on-chain logs | `%s` |\n' "$label" "$hash" >>"$TRANSCRIPT"
done

cat >>"$TRANSCRIPT" <<EOF

## On-chain verification

- Pair pause toggled \`true\` then \`false\` (LCD \`pair config.paused\`).
- Wallet blacklist toggled \`true\` then \`false\` (factory \`blacklist_check.wallet_blacklisted\`).

## Production sign-off

Before mainnet launch, repeat this rehearsal from the **planned production governance multisig** on **testnet or staging** (not mainnet). Attach this table (with real network name + production multisig address) as a comment on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) and link it from Phase 5 in [\`docs/runbooks/launch-checklist.md\`](../../docs/runbooks/launch-checklist.md).

**Automated re-run:** \`make rehearse-governance-emergency\` or \`./scripts/rehearse-governance-emergency-controls.sh\`
EOF

if [[ -n "$OUTPUT" ]]; then
  cp "$TRANSCRIPT" "$OUTPUT"
  echo ""
  echo "Transcript written to $OUTPUT"
else
  echo ""
  echo "── Transcript ──────────────────────────────────────────────────"
  cat "$TRANSCRIPT"
  echo "────────────────────────────────────────────────────────────────"
  echo "Tip: re-run with --output FILE to save for GitLab issue attachment."
fi

echo ""
echo "PASS: governance emergency rehearsal complete (pause, blacklist, unpause, unblacklist)."
