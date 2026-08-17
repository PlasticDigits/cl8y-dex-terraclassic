#!/usr/bin/env bash
# Generate-only → 2-of-3 sign → multisign → broadcast on columbus-5.
#
# Default keys (this machine's keyring):
#   MSIG=multisig_2of3   SIGNER1=multisig1   SIGNER2=multisig2
#
# Always uses TERRAD_HOST_NODE (public columbus-5 RPC), never localhost:26657.
#
# Prompts once for the file-keyring passphrase (or uses TERRAD_HOST_KEYRING_PASS).
#
# Usage:
#   ./scripts/multisig-2of3-host-tx.sh wasm migrate "$FACTORY" 11578 '{}'
#   ./scripts/multisig-2of3-host-tx.sh wasm execute "$FACTORY" '{"update_config":{"treasury":"terra16j5u6…"}}'
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/terrad-host.sh
source "$SCRIPT_DIR/lib/terrad-host.sh"

MSIG="${MSIG:-multisig_2of3}"
SIGNER1="${SIGNER1:-multisig1}"
SIGNER2="${SIGNER2:-multisig2}"
WORKDIR="${MSIG_TX_DIR:-$(pwd)/.msig-tx}"

[[ $# -ge 1 ]] || {
  echo "Usage: $0 <terrad tx args…>   e.g. wasm migrate FACTORY 11578 '{}'" >&2
  exit 1
}

if [[ "${1:-}" == "wasm" && ( "${2:-}" == "migrate" || "${2:-}" == "execute" ) ]]; then
  addr="${3:-}"
  if [[ -z "$addr" || "$addr" != terra1* ]]; then
    echo "ERROR: wasm $2 needs a terra1… contract address as the first argument." >&2
    echo "  got: $*   (empty \$PAIR / \$FACTORY produces this)" >&2
    exit 1
  fi
fi

terrad_host_resolve_keyring_backend
mkdir -p "$WORKDIR"
UNSIGNED="$WORKDIR/unsigned.json"
SIG1="$WORKDIR/sig1.json"
SIG2="$WORKDIR/sig2.json"
SIGNED="$WORKDIR/signed.json"
GENERR="$WORKDIR/generate.err"
rm -f "$UNSIGNED" "$SIG1" "$SIG2" "$SIGNED" "$GENERR"

echo "node:    $TERRAD_HOST_NODE"
echo "chain:   $TERRAD_HOST_CHAIN_ID"
echo "keyring: $TERRAD_HOST_KEYRING_BACKEND"
echo "from:    $MSIG  (signers $SIGNER1 + $SIGNER2)"
echo "tx:      $*"
echo ""

terrad_host_ensure_keyring_pass

# shellcheck disable=SC2046
if ! terrad_host_exec tx "$@" \
  --from "$MSIG" \
  --generate-only \
  $(terrad_host_common_flags) \
  $(terrad_host_gas_flags) \
  $(terrad_host_fee_flags) \
  --output json >"$UNSIGNED" 2>"$GENERR"; then
  echo "ERROR: generate-only failed. Did generate-only reach RPC?" >&2
  cat "$GENERR" >&2 || true
  exit 1
fi

jq -e '.body.messages | length > 0' "$UNSIGNED" >/dev/null \
  || {
    echo "ERROR: unsigned tx is not valid JSON (check $UNSIGNED)." >&2
    cat "$GENERR" >&2 || true
    head -c 400 "$UNSIGNED" >&2 || true
    echo >&2
    exit 1
  }
echo "  unsigned ok ($(wc -c <"$UNSIGNED") bytes)"

# shellcheck disable=SC2046
terrad_host_exec tx sign "$UNSIGNED" \
  --from "$SIGNER1" \
  --multisig "$MSIG" \
  --sign-mode amino-json \
  $(terrad_host_common_flags) \
  --output json >"$SIG1"
jq -e . "$SIG1" >/dev/null || { echo "ERROR: sig1 is not JSON" >&2; exit 1; }
echo "  signed $SIGNER1"

# shellcheck disable=SC2046
terrad_host_exec tx sign "$UNSIGNED" \
  --from "$SIGNER2" \
  --multisig "$MSIG" \
  --sign-mode amino-json \
  $(terrad_host_common_flags) \
  --output json >"$SIG2"
jq -e . "$SIG2" >/dev/null || { echo "ERROR: sig2 is not JSON" >&2; exit 1; }
echo "  signed $SIGNER2"

# shellcheck disable=SC2046
terrad_host_exec tx multisign "$UNSIGNED" "$MSIG" "$SIG1" "$SIG2" \
  $(terrad_host_common_flags) \
  --output json >"$SIGNED"
jq -e . "$SIGNED" >/dev/null || { echo "ERROR: signed tx is not JSON" >&2; exit 1; }
echo "  combined"

# shellcheck disable=SC2046
out="$(terrad_host_exec tx broadcast "$SIGNED" \
  $(terrad_host_common_flags) \
  --broadcast-mode "${TERRAD_HOST_BROADCAST_MODE:-sync}" \
  -y --output json)"
txhash="$(printf '%s' "$out" | jq -r '.txhash // empty')"
code="$(printf '%s' "$out" | jq -r '.code // 0')"
echo "$out" | jq '{txhash, code, raw_log}'
[[ -n "$txhash" && "$code" == "0" ]] || { echo "ERROR: broadcast failed" >&2; exit 1; }
terrad_host_wait_tx_inclusion "$txhash"
echo "OK $txhash"
