#!/usr/bin/env bash
# Build CosmWasm cw20-base optimized wasm into smartcontracts/artifacts/cw20_base.wasm.
# Used by mainnet soft-launch when MAINNET_CW20_BASE_CODE_ID is explicitly emptied.
#
# Pins cosmwasm/cw-plus @ v1.1.2 (matches smartcontracts/Cargo.toml cw20-base version).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$REPO_ROOT/smartcontracts/artifacts}"
CW_PLUS_REF="${CW_PLUS_REF:-v1.1.2}"
OUT_WASM="$ARTIFACTS_DIR/cw20_base.wasm"

mkdir -p "$ARTIFACTS_DIR"

if [[ -f "$OUT_WASM" && "${FORCE_REBUILD_CW20_BASE:-0}" != "1" ]]; then
  echo "OK: $OUT_WASM already present (set FORCE_REBUILD_CW20_BASE=1 to rebuild)"
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "ERROR: docker required to build cw20_base.wasm" >&2
  exit 1
fi

TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR"; }
trap cleanup EXIT

echo "Cloning cosmwasm/cw-plus @ ${CW_PLUS_REF}..."
git clone --depth 1 --branch "$CW_PLUS_REF" https://github.com/CosmWasm/cw-plus.git "$TMP_DIR/cw-plus"

# Single-contract optimizer layout: copy cw20-base as the project root.
mkdir -p "$TMP_DIR/build"
cp -a "$TMP_DIR/cw-plus/contracts/cw20-base/." "$TMP_DIR/build/"
# Ensure Cargo.lock exists for optimizer (workspace member may lack one).
if [[ ! -f "$TMP_DIR/build/Cargo.lock" ]]; then
  (cd "$TMP_DIR/build" && cargo generate-lockfile)
fi

echo "Running cosmwasm/rust-optimizer..."
docker run --rm \
  -v "$TMP_DIR/build":/code \
  --mount type=volume,source=cw20_base_soft_launch_cache,target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.16.1

if [[ -f "$TMP_DIR/build/artifacts/cw20_base.wasm" ]]; then
  cp "$TMP_DIR/build/artifacts/cw20_base.wasm" "$OUT_WASM"
elif [[ -f "$TMP_DIR/build/artifacts/cw20-base.wasm" ]]; then
  cp "$TMP_DIR/build/artifacts/cw20-base.wasm" "$OUT_WASM"
else
  echo "ERROR: optimizer did not produce cw20_base.wasm under artifacts/" >&2
  ls -la "$TMP_DIR/build/artifacts" 2>/dev/null || true
  exit 1
fi

echo "Wrote $OUT_WASM"
sha256sum "$OUT_WASM" || true
