#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$(cd .. && pwd)"
IMAGE="cosmwasm/workspace-optimizer:0.16.1@sha256:b9c92b2900b7ebaab3499203615c1b8589592bc557355ed3432e48851ffde69e"
RUN_ID="cl8y-opt-${CI_JOB_ID:-local}-$$-${RANDOM}-${RANDOM}"
TARGET_VOLUME="${RUN_ID}-target"
REGISTRY_VOLUME="${RUN_ID}-registry"
INPUT_RECEIPT="$(mktemp "${TMPDIR:-/tmp}/cl8y-opt-inputs.XXXXXXXX.json")"
cleanup() {
  docker rm -f "$RUN_ID" >/dev/null 2>&1 || true
  docker volume rm "$TARGET_VOLUME" "$REGISTRY_VOLUME" >/dev/null 2>&1 || true
  rm -f "$INPUT_RECEIPT"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Source is read-only. Both the historical /code/target overlay and the actual
# v0.16.1 bob output directory /target use a fresh volume, removed on every exit.
# bob already invokes cargo build --locked; source lockfiles cannot be rewritten.
# The registry is fresh too: no code downloaded by another trust level executes.
mkdir -p target artifacts
rm -f artifacts/*.wasm artifacts/build-provenance.json artifacts/toolchain.txt
python3 "$ROOT/scripts/ci/build-provenance.py" \
  --root "$ROOT" --source smartcontracts --lock smartcontracts/Cargo.lock \
  --image "$IMAGE" --toolchain 'image-defined; exact toolchain captured by build' \
  --capture-inputs --output "$INPUT_RECEIPT"
docker volume create "$TARGET_VOLUME" >/dev/null
docker volume create "$REGISTRY_VOLUME" >/dev/null
timeout --signal=TERM --kill-after=30s "${OPTIMIZER_TIMEOUT_SECONDS:-2700}s" \
  docker run --rm --name "$RUN_ID" --entrypoint /bin/sh \
  -e "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}" \
  --mount "type=bind,source=$(pwd),target=/code,readonly" \
  --mount "type=bind,source=$(pwd)/artifacts,target=/code/artifacts" \
  --mount "type=volume,source=$TARGET_VOLUME,target=/code/target" \
  --mount "type=volume,source=$TARGET_VOLUME,target=/target" \
  --mount "type=volume,source=$REGISTRY_VOLUME,target=/usr/local/cargo/registry" \
  "$IMAGE" -c "set -eu; trap 'chown -R $(id -u):$(id -g) /code/artifacts' EXIT; \
    { rustc --version; cargo --version; wasm-opt --version; } > artifacts/toolchain.txt; \
    /usr/local/bin/optimize.sh ."
python3 "$ROOT/scripts/ci/build-provenance.py" --root "$ROOT" \
  --verify "$INPUT_RECEIPT" --inputs-only --expected-image "$IMAGE"

shopt -s nullglob
WASM=(artifacts/*.wasm)
if [ "${#WASM[@]}" -eq 0 ]; then
  echo 'ERROR: optimizer produced no wasm artifacts' >&2
  exit 1
fi
(cd artifacts && sha256sum -- *.wasm) > wasm-checksums.txt
ARTIFACT_ARGS=()
for file in "${WASM[@]}"; do ARTIFACT_ARGS+=(--artifact "$file"); done
python3 "$ROOT/scripts/ci/build-provenance.py" \
  --root "$ROOT" --source smartcontracts --lock smartcontracts/Cargo.lock \
  --image "$IMAGE" --toolchain "$(cat artifacts/toolchain.txt)" \
  --output artifacts/build-provenance.json "${ARTIFACT_ARGS[@]}"
echo 'Optimized contracts and build-provenance.json are in artifacts/'
