#!/usr/bin/env bash
# LocalTerra / staging dry-run: governance KEY ROTATION via multisig threshold signing (SEC-D10, GitLab #408).
#
# Rotates the factory CONTRACT WASM ADMIN (migrate authority) round-trip:
#   current admin (test1) -> rehearsal 2-of-3 multisig -> back to current admin
# proving BOTH directions, including a multisig-signed `set-contract-admin` — the real
# production scenario of rotating admin away from an old/compromised multisig. The original
# admin is RESTORED (verified on-chain) so the shared QA deploy is left exactly as found.
#
# Safety on a shared deploy: only the wasm contract-admin (migrate authority) is touched — never
# the factory `governance` pointer. An EXIT/INT/TERM trap restores the original admin on any abort,
# and a re-run auto-recovers if a prior run was killed mid-rotation (admin still on the multisig).
#
# The factory `governance` pointer rotation (`UpdateConfig { governance }`) uses the identical
# multisig flow; it is documented copy-paste in docs/runbooks/governance-key-rotation.md and is
# NOT mutated here (the live governance key is not assumed to be in this keyring).
#
# Prerequisites: make start && make deploy-local (factory admin must be a key in the `test` keyring).
#
# Usage:
#   ./scripts/rehearse-governance-key-rotation.sh
#   ./scripts/rehearse-governance-key-rotation.sh --output /tmp/rotation.md
#
# Post the generated transcript (tx hashes) on GitLab #408 or the launch tracking issue (#391).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/terrad-wait-tx.sh
source "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh"

OUTPUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --output) OUTPUT="$2"; shift 2 ;;
    -h|--help) echo "Usage: $0 [--output FILE]"; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

CHAIN_ID="${CHAIN_ID:-localterra}"
TERRAD_NODE="${TERRAD_NODE:-http://127.0.0.1:26657}"
KEYRING="${KEYRING:-test}"
FEES="${REHEARSAL_FEES:-500000000uluna}"
GAS_ADJ="${REHEARSAL_GAS_ADJ:-1.4}"
MSIG_NAME="${ROT_MSIG_NAME:-gov-rotation-msig}"
SIGNER1="${ROT_MSIG_SIGNER1:-gov-rotation-1}"
SIGNER2="${ROT_MSIG_SIGNER2:-gov-rotation-2}"
SIGNER3="${ROT_MSIG_SIGNER3:-gov-rotation-3}"
THRESHOLD="${ROT_MSIG_THRESHOLD:-2}"
# Keyring name that currently holds the factory wasm admin (the key we sign the forward rotation with).
ADMIN_KEY="${ROTATION_ADMIN_KEY:-test1}"

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER_NAME" ]]; then
  CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 || true)"
fi

read_env_var() { sed -n "s/^${2}=//p" "$1" 2>/dev/null | head -1; }
IDX_ENV="$REPO_ROOT/indexer/.env"
FACTORY="$(read_env_var "$IDX_ENV" FACTORY_ADDRESS)"

if [[ -z "$CONTAINER_NAME" ]]; then
  echo "ERROR: localterra container not running (make start)." >&2; exit 1
fi
if [[ -z "$FACTORY" ]]; then
  echo "ERROR: FACTORY_ADDRESS missing in indexer/.env — run make deploy-local." >&2; exit 1
fi

terrad_q() { docker exec "$CONTAINER_NAME" terrad "$@"; }

contract_admin() {
  terrad_q query wasm contract "$FACTORY" --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '.contract_info.admin // empty'
}

# Read the admin, retrying through transient query failures (shared-node hiccups).
contract_admin_robust() {
  local v tries=0
  v="$(contract_admin || true)"
  while [[ -z "$v" && $tries -lt 5 ]]; do sleep 2; tries=$((tries + 1)); v="$(contract_admin || true)"; done
  printf '%s' "$v"
}

wait_tx() { terrad_wait_tx_inclusion "$CONTAINER_NAME" "$1" "$TERRAD_NODE" 120; }

ensure_signer_key() {
  local name="$1"
  terrad_q keys show "$name" --keyring-backend "$KEYRING" >/dev/null 2>&1 && return 0
  terrad_q keys add "$name" --keyring-backend "$KEYRING" --no-backup >/dev/null 2>&1
}

setup_multisig() {
  ensure_signer_key "$SIGNER1"; ensure_signer_key "$SIGNER2"; ensure_signer_key "$SIGNER3"
  if ! terrad_q keys show "$MSIG_NAME" --keyring-backend "$KEYRING" >/dev/null 2>&1; then
    terrad_q keys add "$MSIG_NAME" --multisig "$SIGNER1,$SIGNER2,$SIGNER3" \
      --multisig-threshold "$THRESHOLD" --keyring-backend "$KEYRING" --no-backup >/dev/null 2>&1
  fi
  terrad_q keys show "$MSIG_NAME" -a --keyring-backend "$KEYRING"
}

fund_addr() { # $1 addr  $2 amount
  local bal tx
  bal="$(terrad_q query bank balances "$1" --node "$TERRAD_NODE" --output json 2>/dev/null \
    | jq -r '[.balances[]|select(.denom=="uluna")|.amount]|first // "0"')"
  bal="${bal:-0}"
  if [[ "$(echo "$bal >= 2000000000" | bc)" != "1" ]]; then
    tx="$(terrad_q tx bank send "$ADMIN_KEY" "$1" "$2" --keyring-backend "$KEYRING" \
      --chain-id "$CHAIN_ID" --gas auto --gas-adjustment 1.3 --fees "$FEES" \
      --node "$TERRAD_NODE" --broadcast-mode sync -y --output json | jq -r '.txhash')"
    wait_tx "$tx"
  fi
}

set_admin_from_key() { # current admin == ADMIN_KEY.  $1 = new admin addr
  terrad_q tx wasm set-contract-admin "$FACTORY" "$1" \
    --from "$ADMIN_KEY" --keyring-backend "$KEYRING" --chain-id "$CHAIN_ID" \
    --gas auto --gas-adjustment 1.3 --fees "$FEES" --node "$TERRAD_NODE" \
    --broadcast-mode sync -y --output json | jq -r '.txhash // empty'
}

set_admin_from_msig() { # current admin == MSIG_ADDR.  $1 = new admin addr
  docker exec "$CONTAINER_NAME" sh -c "
    set -e
    workdir=/tmp/rot-msig-\$\$
    mkdir -p \"\$workdir\"
    terrad tx wasm set-contract-admin '$FACTORY' '$1' \
      --from '$MSIG_NAME' --keyring-backend '$KEYRING' --chain-id '$CHAIN_ID' \
      --gas auto --gas-adjustment '$GAS_ADJ' --fees '$FEES' --node '$TERRAD_NODE' \
      --generate-only --output json > \"\$workdir/unsigned.json\"
    terrad tx sign \"\$workdir/unsigned.json\" --from '$SIGNER1' --multisig '$MSIG_NAME' \
      --sign-mode amino-json --keyring-backend '$KEYRING' --chain-id '$CHAIN_ID' \
      --node '$TERRAD_NODE' --output json > \"\$workdir/sig1.json\"
    terrad tx sign \"\$workdir/unsigned.json\" --from '$SIGNER2' --multisig '$MSIG_NAME' \
      --sign-mode amino-json --keyring-backend '$KEYRING' --chain-id '$CHAIN_ID' \
      --node '$TERRAD_NODE' --output json > \"\$workdir/sig2.json\"
    terrad tx multisign \"\$workdir/unsigned.json\" '$MSIG_NAME' \
      \"\$workdir/sig1.json\" \"\$workdir/sig2.json\" --keyring-backend '$KEYRING' \
      --chain-id '$CHAIN_ID' --node '$TERRAD_NODE' --output json > \"\$workdir/signed.json\"
    terrad tx broadcast \"\$workdir/signed.json\" --keyring-backend '$KEYRING' \
      --chain-id '$CHAIN_ID' --node '$TERRAD_NODE' --broadcast-mode sync --output json
    rm -rf \"\$workdir\"
  " | jq -r '.txhash // empty'
}

A0=""            # original admin to restore to (the address ADMIN_KEY controls)
MSIG_ADDR=""
# Restore the factory admin to A0 via the multisig. Retries; treats an unreadable admin as
# "needs checking" (never a silent no-op) so a transient query failure cannot strand the admin.
restore_admin() {
  [[ -z "$A0" || -z "$MSIG_ADDR" ]] && return 0
  local cur n=0 tx
  cur="$(contract_admin_robust)"
  [[ "$cur" == "$A0" ]] && return 0
  while [[ $n -lt 3 ]]; do
    echo "  [restore] factory admin='${cur:-<unreadable>}' != original '$A0' — restoring via multisig (attempt $((n + 1)))" >&2
    tx="$(set_admin_from_msig "$A0" 2>/dev/null || true)"
    [[ -n "$tx" ]] && { wait_tx "$tx" 2>/dev/null || true; }
    cur="$(contract_admin_robust)"
    [[ "$cur" == "$A0" ]] && { echo "  [restore] factory admin restored to '$A0'" >&2; return 0; }
    n=$((n + 1)); sleep 2
  done
  echo "  [restore] CRITICAL: could not restore factory admin to '$A0' (now '${cur:-<unreadable>}')." >&2
  echo "  [restore] recover with: make rehearse-governance-key-rotation   (auto-detects + rotates the multisig back)" >&2
}
cleanup() { local rc=$?; restore_admin; exit "$rc"; }

declare -a TX_LOG=()
record() { TX_LOG+=("$1|$2"); echo "    txhash=$2"; }

echo "════════════════════════════════════════════════════════════════"
echo "  Governance key rotation rehearsal (SEC-D10, #408)"
echo "  network=$CHAIN_ID factory=$FACTORY"
echo "════════════════════════════════════════════════════════════════"

ADMIN_ADDR="$(terrad_q keys show "$ADMIN_KEY" -a --keyring-backend "$KEYRING" 2>/dev/null || true)"
MSIG_ADDR="$(setup_multisig)"
CUR0="$(contract_admin_robust)"
echo "  current wasm admin       = ${CUR0:-<unreadable>}"
echo "  rotation key '$ADMIN_KEY' = $ADMIN_ADDR"
echo "  rehearsal multisig (M1)  = $MSIG_ADDR (${THRESHOLD}-of-3: $SIGNER1,$SIGNER2,$SIGNER3)"

if [[ -z "$CUR0" ]]; then
  echo "ERROR: cannot read factory admin (node/RPC?)." >&2; exit 1
fi

# Self-healing: a prior run was killed mid-rotation and left the admin on the rehearsal multisig.
if [[ "$CUR0" == "$MSIG_ADDR" ]]; then
  echo ""
  echo "  RECOVERY: factory admin is the rehearsal multisig (stranded prior run) — rotating back to '$ADMIN_KEY'."
  fund_addr "$MSIG_ADDR" 10000000000uluna
  RTX="$(set_admin_from_msig "$ADMIN_ADDR")"
  [[ -n "$RTX" ]] || { echo "ERROR: recovery broadcast failed" >&2; exit 1; }
  wait_tx "$RTX"
  CURR="$(contract_admin_robust)"
  [[ "$CURR" == "$ADMIN_ADDR" ]] || { echo "ERROR: recovery failed (admin='$CURR')" >&2; exit 1; }
  echo "PASS: recovered factory admin to $ADMIN_ADDR."
  exit 0
fi

if [[ "$CUR0" != "$ADMIN_ADDR" ]]; then
  echo "SKIP: factory admin ($CUR0) is not the local '$ADMIN_KEY' key ($ADMIN_ADDR)." >&2
  echo "      This rehearsal only runs when it controls the current admin, so it can safely round-trip." >&2
  echo "      Set ROTATION_ADMIN_KEY to the keyring name holding the admin, or run on a fresh make deploy-local." >&2
  exit 2
fi

A0="$CUR0"
fund_addr "$MSIG_ADDR" 10000000000uluna
# From here the admin may be rotated; guarantee restoration on any exit (success, error, or signal).
trap cleanup EXIT
trap 'exit 130' INT TERM

echo ""
echo "[1] rotate wasm admin: '$ADMIN_KEY' -> multisig (single-key set-contract-admin)"
TX1="$(set_admin_from_key "$MSIG_ADDR")"
[[ -n "$TX1" ]] || { echo "ERROR: forward set-contract-admin broadcast failed" >&2; exit 1; }
wait_tx "$TX1"; record "set-contract-admin: key -> multisig" "$TX1"
CUR="$(contract_admin_robust)"
[[ "$CUR" == "$MSIG_ADDR" ]] || { echo "ERROR: admin not rotated to multisig (got '$CUR')" >&2; exit 1; }
echo "  verified factory admin = $MSIG_ADDR"

echo ""
echo "[2] rotate wasm admin: multisig -> '$ADMIN_KEY' (threshold ${THRESHOLD}-of-3 multisign)"
TX2="$(set_admin_from_msig "$A0")"
[[ -n "$TX2" ]] || { echo "ERROR: multisig set-contract-admin broadcast failed" >&2; exit 1; }
wait_tx "$TX2"; record "set-contract-admin (multisig) -> original" "$TX2"
CUR="$(contract_admin_robust)"
[[ "$CUR" == "$A0" ]] || { echo "ERROR: admin not restored (got '$CUR', expected '$A0')" >&2; exit 1; }
echo "  verified factory admin restored = $A0"

TS_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
TRANSCRIPT="$(mktemp /tmp/governance-key-rotation-XXXXXX.md)"
cat >"$TRANSCRIPT" <<EOF
# Governance key rotation rehearsal evidence

| Field | Value |
|-------|-------|
| Checklist | SEC-D10 (GitLab [#408](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/408)) |
| Network | $CHAIN_ID |
| Contract | factory \`$FACTORY\` |
| Original admin (restored) | \`$A0\` |
| Rehearsal multisig | \`$MSIG_ADDR\` (${THRESHOLD}-of-3: \`$SIGNER1\`, \`$SIGNER2\`, \`$SIGNER3\`) |
| Rehearsal UTC | $TS_UTC |
| Mechanism | \`terrad tx wasm set-contract-admin\` (forward single-key, return via \`tx sign\`/\`multisign\`/\`broadcast\` threshold $THRESHOLD) |

## Operations (in order)

| Step | Action | Tx hash |
|------|--------|---------|
EOF
for entry in "${TX_LOG[@]}"; do
  printf '| %s | rotate | `%s` |\n' "${entry%%|*}" "${entry#*|}" >>"$TRANSCRIPT"
done
cat >>"$TRANSCRIPT" <<EOF

## On-chain verification

- After step 1: \`contract_info.admin\` == rehearsal multisig (\`$MSIG_ADDR\`).
- After step 2: \`contract_info.admin\` == original admin (\`$A0\`) — round-trip restored.
- Query: \`terrad query wasm contract <factory> --output json | jq -r .contract_info.admin\`.

## Scope and production sign-off

This rehearsal exercises the **wasm contract-admin** rotation (migrate authority) round-trip on
LocalTerra, including a **multisig-signed** \`set-contract-admin\` (the production rotation-away
scenario). The factory **\`governance\` pointer** rotation (\`UpdateConfig { governance }\`) uses the
identical multisig flow — see [\`docs/runbooks/governance-key-rotation.md\`](../../docs/runbooks/governance-key-rotation.md).

Before mainnet, repeat from the **planned production multisig** on **testnet/staging** and attach
this table (with the real network + multisig address + signer name + UTC) to the launch tracking
issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

**Automated re-run:** \`make rehearse-governance-key-rotation\` or \`./scripts/rehearse-governance-key-rotation.sh\`
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
echo "PASS: governance key rotation rehearsal complete (wasm admin round-trip restored)."
