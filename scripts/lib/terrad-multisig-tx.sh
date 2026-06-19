#!/usr/bin/env bash
# CosmWasm execute via terrad multisig (threshold signing). Used by governance emergency rehearsal (SEC-B09).
# shellcheck shell=bash

# Args: container node chain_id multisig_name keyring fees gas_adj contract msg_json signer1 signer2
# Prints broadcast JSON to stdout. All tx files are written inside the container.
terrad_multisig_wasm_execute() {
  local container="$1"
  local node="$2"
  local chain_id="$3"
  local multisig_name="$4"
  local keyring="$5"
  local fees="$6"
  local gas_adj="$7"
  local contract="$8"
  local msg_json="$9"
  local signer1="${10}"
  local signer2="${11}"

  docker exec "$container" sh -c "
    set -e
    workdir=/tmp/msig-\$\$
    mkdir -p \"\$workdir\"
    terrad tx wasm execute '$contract' '$msg_json' \
      --from '$multisig_name' \
      --keyring-backend '$keyring' \
      --chain-id '$chain_id' \
      --gas auto \
      --gas-adjustment '$gas_adj' \
      --fees '$fees' \
      --node '$node' \
      --generate-only \
      --output json \
      > \"\$workdir/unsigned.json\"
    terrad tx sign \"\$workdir/unsigned.json\" \
      --from '$signer1' \
      --multisig '$multisig_name' \
      --sign-mode amino-json \
      --keyring-backend '$keyring' \
      --chain-id '$chain_id' \
      --node '$node' \
      --output json \
      > \"\$workdir/sig1.json\"
    terrad tx sign \"\$workdir/unsigned.json\" \
      --from '$signer2' \
      --multisig '$multisig_name' \
      --sign-mode amino-json \
      --keyring-backend '$keyring' \
      --chain-id '$chain_id' \
      --node '$node' \
      --output json \
      > \"\$workdir/sig2.json\"
    terrad tx multisign \"\$workdir/unsigned.json\" '$multisig_name' \
      \"\$workdir/sig1.json\" \"\$workdir/sig2.json\" \
      --keyring-backend '$keyring' \
      --chain-id '$chain_id' \
      --node '$node' \
      --output json \
      > \"\$workdir/signed.json\"
    terrad tx broadcast \"\$workdir/signed.json\" \
      --keyring-backend '$keyring' \
      --chain-id '$chain_id' \
      --node '$node' \
      --broadcast-mode sync \
      --output json
    rm -rf \"\$workdir\"
  "
}
