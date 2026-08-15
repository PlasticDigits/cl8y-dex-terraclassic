#!/usr/bin/env bash
# Move leftover cl8ydeploy powers to the DEX 2-of-3 multisig (columbus-5).
#
# Does NOT touch the UST1 / wrap stack (cl8y2_admin, invariant O6).
# Extra CW20 minters (faucet, cl8y-bridge-v2) are left in place.
#
# Usage:
#   DRY_RUN=1 ./scripts/handoff-cl8ydeploy-to-multisig.sh
#   ./scripts/handoff-cl8ydeploy-to-multisig.sh --verify
#   ./scripts/handoff-cl8ydeploy-to-multisig.sh              # all phases
#   ./scripts/handoff-cl8ydeploy-to-multisig.sh faucet gem-minters gem-pairs cl8y
#
# Unlock file keyring once:
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"
# shellcheck source=lib/governance-multisig.sh
source "$SCRIPT_DIR/lib/governance-multisig.sh"

ADDRESSES_ENV="${ADDRESSES_ENV:-$REPO_ROOT/deployments/mainnet-soft-launch/addresses.env}"
if [[ -f "$ADDRESSES_ENV" ]]; then
  # shellcheck disable=SC1090
  source "$ADDRESSES_ENV"
fi

TERRAD_HOST_KEY="${TERRAD_HOST_KEY:-cl8ydeploy}"
TERRAD_HOST_EXPECTED_ADDR="${TERRAD_HOST_EXPECTED_ADDR:-terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv}"
TERRAD_HOST_CHAIN_ID="${TERRAD_HOST_CHAIN_ID:-columbus-5}"
TERRAD_HOST_NODE="${TERRAD_HOST_NODE:-https://terra-classic-rpc.publicnode.com:443}"
LCD_URL="${LCD_URL:-${TERRA_LCD_URL:-https://terra-classic-lcd.publicnode.com}}"
LCD_URL="${LCD_URL%/}"

MSIG="${HANDOFF_MULTISIG:-${GOVERNANCE_ADDRESS:-$GOVERNANCE_MULTISIG_ADDR}}"
DEPLOY="${TERRAD_HOST_EXPECTED_ADDR}"

FAUCET="${FAUCET_ADDRESS:-terra1388y0ppe2c3dy4nrmnpqp7e4ggukkrnmpzfjadfeu0pu2rm9cvkslfzcen}"
CL8Y="${CL8Y_TOKEN_ADDRESS:-terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3}"
CL8Y_BRIDGE="${CL8Y_BRIDGE_ADDRESS:-terra18m02l2f43c2dagqnz3kfccpgz9pzzz5hk9l5mh5wvr6dcvv47zfqdfs7la}"

GEM_MINTABLES=(
  "EMBER:${TOKEN_EMBER_ADDRESS:-terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94}"
  "CORAL:${TOKEN_CORAL_ADDRESS:-terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena}"
  "JADE:${TOKEN_JADE_ADDRESS:-terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr}"
  "ONYX:${TOKEN_ONYX_ADDRESS:-terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9}"
  "RUBY:${TOKEN_RUBY_ADDRESS:-terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc}"
  "TOPAZ:${TOKEN_TOPAZ_ADDRESS:-terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme}"
)

GEM_PAIRS=(
  "EMBER/CORAL:${PAIR_EMBER_CORAL_ADDRESS:-terra1klwuxas6x7p6fjde60kq70t0hu86wvt3fvyr2vgs0nn32fnv0q4qwznwp4}"
  "EMBER/JADE:${PAIR_EMBER_JADE_ADDRESS:-terra1y5xxv980jn0qu7n7y3slhtjehta6nlpqjkgcxl80uetdx84dxa4qegjhtx}"
  "EMBER/ONYX:${PAIR_EMBER_ONYX_ADDRESS:-terra16827w2c7zcvetck9xz8d6ds3379v77gelwra6jdafqkx9q9u0r8qvkluu0}"
  "CORAL/RUBY:${PAIR_CORAL_RUBY_ADDRESS:-terra1ra7cugjhchr45kdupxe2al5fna0zxu6syl8xhpanfk8dsvkq9lksf6fm9l}"
  "JADE/TOPAZ:${PAIR_JADE_TOPAZ_ADDRESS:-terra1nqjvd2xatac5ydcs6nstw7zp2yjc20p632ycxtevtf3rr2554fqswstx0n}"
  "ONYX/QUARTZ:${PAIR_ONYX_QUARTZ_ADDRESS:-terra1havxdjfyphjazc342r3cj2n3kslsptac2eunvw8uzayusywg9t4shtuz7v}"
  "RUBY/PEARL:${PAIR_RUBY_PEARL_ADDRESS:-terra1p0sd0t2ggm9ye43gp0ryadx3wwkz5hzn99hnz93ve99397xvuufsvsmw73}"
  "EMBER/QUARTZ:${PAIR_EMBER_QUARTZ_ADDRESS:-terra1mp72n97rzwmqwudzycjj0e4jveetjnp622gnprv6ugqt3hfxg60sr5gkjm}"
  "CORAL/PEARL:${PAIR_CORAL_PEARL_ADDRESS:-terra16k6huf87gzvnlgpvf85f8xfgawl6y2l5d4d5qdpyhknqaran9s5qx63c3r}"
  "JADE/ONYX:${PAIR_JADE_ONYX_ADDRESS:-terra1pc7dvcucrl9sr4r2nhr2rv3ywhthskqqtvtvaerx9cff7l8gesdskjgmn6}"
)

die() { echo "ERROR: $*" >&2; exit 1; }

who() {
  case "$1" in
    "$MSIG") echo "DEX_MULTISIG" ;;
    "$DEPLOY") echo "cl8ydeploy" ;;
    "$FAUCET") echo "faucet" ;;
    "$CL8Y_BRIDGE") echo "cl8y-bridge-v2" ;;
    *) echo "$1" ;;
  esac
}

lcd_get() {
  curl -sf --connect-timeout 8 --max-time 25 \
    -H 'Accept: application/json' -H 'User-Agent: cl8y-handoff' \
    "$1"
}

wasm_admin() {
  lcd_get "$LCD_URL/cosmwasm/wasm/v1/contract/$1" | jq -r '.contract_info.admin // empty'
}

smart() {
  local addr="$1" msg="$2" b64
  b64="$(printf '%s' "$msg" | base64 -w0 2>/dev/null || printf '%s' "$msg" | base64)"
  lcd_get "$LCD_URL/cosmwasm/wasm/v1/contract/${addr}/smart/${b64}" | jq -c '.data // .'
}

broadcast_and_wait() {
  local label="$1"
  shift
  echo "  → $label" >&2
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "    DRY_RUN: terrad tx $* --from $TERRAD_HOST_KEY" >&2
    return 0
  fi
  local out tx_hash
  out="$(terrad_host_tx "$@")"
  tx_hash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
  [[ -n "$tx_hash" ]] || {
    echo "ERROR: no txhash from: $label" >&2
    printf '%s\n' "$out" >&2
    exit 1
  }
  echo "    tx: $tx_hash" >&2
  terrad_host_wait_tx_inclusion "$tx_hash"
}

exec_if_needed() {
  local label="$1" current="$2" want="$3"
  shift 3
  if [[ "$current" == "$want" ]]; then
    echo "  skip $label (already $(who "$want"))" >&2
    return 0
  fi
  broadcast_and_wait "$label" "$@"
}

verify() {
  local fail=0
  echo "=== verify (LCD $LCD_URL) ==="
  local a
  a="$(wasm_admin "$FAUCET")"
  echo "faucet wasm admin: $(who "$a")"
  [[ "$a" == "$MSIG" ]] || { echo "  FAIL expected multisig"; fail=1; }
  a="$(smart "$FAUCET" '{"config":{}}' | jq -r '.admin // empty')"
  echo "faucet config.admin: $(who "$a")"
  [[ "$a" == "$MSIG" ]] || { echo "  FAIL expected multisig"; fail=1; }

  local entry sym addr minter extras
  for entry in "${GEM_MINTABLES[@]}"; do
    sym="${entry%%:*}"
    addr="${entry#*:}"
    minter="$(smart "$addr" '{"minter":{}}' | jq -r '.minter // empty')"
    extras="$(smart "$addr" '{"minters":{}}' | jq -r '[.minters[]?] | join(",")')"
    echo "$sym primary=$(who "$minter") extras=$extras"
    [[ "$minter" == "$MSIG" ]] || { echo "  FAIL primary minter"; fail=1; }
    [[ "$extras" == *"$FAUCET"* ]] || { echo "  FAIL faucet extra minter missing"; fail=1; }
  done

  for entry in "${GEM_PAIRS[@]}"; do
    sym="${entry%%:*}"
    addr="${entry#*:}"
    a="$(wasm_admin "$addr")"
    echo "pair $sym wasm admin: $(who "$a")"
    [[ "$a" == "$MSIG" ]] || { echo "  FAIL expected multisig"; fail=1; }
  done

  a="$(wasm_admin "$CL8Y")"
  minter="$(smart "$CL8Y" '{"minter":{}}' | jq -r '.minter // empty')"
  extras="$(smart "$CL8Y" '{"minters":{}}' | jq -r '[.minters[]?] | join(",")')"
  local mkt
  mkt="$(smart "$CL8Y" '{"marketing_info":{}}' | jq -r '.marketing // empty')"
  echo "CL8Y wasm admin: $(who "$a")"
  echo "CL8Y primary: $(who "$minter") extras=$extras marketing=$(who "$mkt")"
  [[ "$a" == "$MSIG" ]] || { echo "  FAIL CL8Y wasm admin"; fail=1; }
  [[ "$minter" == "$MSIG" ]] || { echo "  FAIL CL8Y primary minter"; fail=1; }
  [[ "$extras" == *"$CL8Y_BRIDGE"* ]] || { echo "  FAIL CL8Y bridge extra minter missing"; fail=1; }
  [[ "$mkt" == "$MSIG" ]] || { echo "  FAIL CL8Y marketing"; fail=1; }

  if [[ "$fail" -ne 0 ]]; then
    echo "VERIFY FAIL"
    return 1
  fi
  echo "VERIFY PASS"
}

phase_faucet() {
  echo "=== faucet ==="
  local cfg_admin wasm
  cfg_admin="$(smart "$FAUCET" '{"config":{}}' | jq -r '.admin // empty')"
  wasm="$(wasm_admin "$FAUCET")"
  exec_if_needed "faucet update_config.admin" "$cfg_admin" "$MSIG" \
    wasm execute "$FAUCET" "$(jq -nc --arg a "$MSIG" '{update_config:{admin:$a}}')"
  exec_if_needed "faucet set-contract-admin" "$wasm" "$MSIG" \
    wasm set-contract-admin "$FAUCET" "$MSIG"
}

phase_gem_minters() {
  echo "=== gem primary minters ==="
  local entry sym addr minter extras
  for entry in "${GEM_MINTABLES[@]}"; do
    sym="${entry%%:*}"
    addr="${entry#*:}"
    extras="$(smart "$addr" '{"minters":{}}' | jq -r '[.minters[]?] | join(",")')"
    [[ "$extras" == *"$FAUCET"* ]] || die "$sym missing faucet extra minter — abort (do not update_minter)"
    minter="$(smart "$addr" '{"minter":{}}' | jq -r '.minter // empty')"
    exec_if_needed "$sym update_minter" "$minter" "$MSIG" \
      wasm execute "$addr" "$(jq -nc --arg m "$MSIG" '{update_minter:{new_minter:$m}}')"
  done
}

phase_gem_pairs() {
  echo "=== gem pair wasm admins ==="
  local entry sym addr admin
  for entry in "${GEM_PAIRS[@]}"; do
    sym="${entry%%:*}"
    addr="${entry#*:}"
    admin="$(wasm_admin "$addr")"
    exec_if_needed "pair $sym set-contract-admin" "$admin" "$MSIG" \
      wasm set-contract-admin "$addr" "$MSIG"
  done
}

phase_cl8y() {
  echo "=== CL8Y ==="
  local extras minter mkt wasm
  extras="$(smart "$CL8Y" '{"minters":{}}' | jq -r '[.minters[]?] | join(",")')"
  [[ "$extras" == *"$CL8Y_BRIDGE"* ]] || die "CL8Y missing bridge extra minter — abort"
  mkt="$(smart "$CL8Y" '{"marketing_info":{}}' | jq -r '.marketing // empty')"
  minter="$(smart "$CL8Y" '{"minter":{}}' | jq -r '.minter // empty')"
  wasm="$(wasm_admin "$CL8Y")"
  exec_if_needed "CL8Y update_marketing" "$mkt" "$MSIG" \
    wasm execute "$CL8Y" "$(jq -nc --arg m "$MSIG" '{update_marketing:{marketing:$m}}')"
  exec_if_needed "CL8Y update_minter" "$minter" "$MSIG" \
    wasm execute "$CL8Y" "$(jq -nc --arg m "$MSIG" '{update_minter:{new_minter:$m}}')"
  exec_if_needed "CL8Y set-contract-admin" "$wasm" "$MSIG" \
    wasm set-contract-admin "$CL8Y" "$MSIG"
}

PHASES=()
VERIFY_ONLY=0
for arg in "$@"; do
  case "$arg" in
    --verify | verify) VERIFY_ONLY=1 ;;
    faucet | gem-minters | gem-pairs | cl8y | all) PHASES+=("$arg") ;;
    -h | --help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *) die "unknown arg: $arg (faucet|gem-minters|gem-pairs|cl8y|all|--verify)" ;;
  esac
done
[[ ${#PHASES[@]} -gt 0 ]] || PHASES=(all)

echo "handoff cl8ydeploy → $MSIG"
echo "key=$TERRAD_HOST_KEY expected=$DEPLOY chain=$TERRAD_HOST_CHAIN_ID DRY_RUN=${DRY_RUN:-0}"
echo "phases=${PHASES[*]} verify_only=$VERIFY_ONLY"
echo ""

if [[ "$VERIFY_ONLY" -eq 1 ]]; then
  verify
  exit
fi

if [[ "${DRY_RUN:-0}" != "1" ]]; then
  addr="$(terrad_host_key_address)"
  [[ "$addr" == "$DEPLOY" ]] || die "key $TERRAD_HOST_KEY is $addr, expected $DEPLOY"
fi

for p in "${PHASES[@]}"; do
  case "$p" in
    all)
      phase_faucet
      phase_gem_minters
      phase_gem_pairs
      phase_cl8y
      ;;
    faucet) phase_faucet ;;
    gem-minters) phase_gem_minters ;;
    gem-pairs) phase_gem_pairs ;;
    cl8y) phase_cl8y ;;
  esac
done

echo ""
if [[ "${DRY_RUN:-0}" == "1" ]]; then
  echo "DRY_RUN done — re-run without DRY_RUN=1 to broadcast"
else
  verify || true
fi
