#!/usr/bin/env bash
# Disposable optimizer for locked external LocalTerra fixture workspaces.
set -euo pipefail
PROJECT="$(cd "${1:?Cargo project directory required}" && pwd)"
IMAGE="cosmwasm/workspace-optimizer:0.16.1@sha256:b9c92b2900b7ebaab3499203615c1b8589592bc557355ed3432e48851ffde69e"
test -f "$PROJECT/Cargo.lock"
RUN_ID="cl8y-fixture-opt-$$-${RANDOM}-${RANDOM}"
cleanup() {
  docker rm -f "$RUN_ID" >/dev/null 2>&1 || true
  docker volume rm "${RUN_ID}-target" "${RUN_ID}-registry" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$PROJECT/target" "$PROJECT/artifacts"
docker volume create "${RUN_ID}-target" >/dev/null
docker volume create "${RUN_ID}-registry" >/dev/null
timeout --signal=TERM --kill-after=30s "${OPTIMIZER_TIMEOUT_SECONDS:-2700}s" \
  docker run --rm --name "$RUN_ID" --entrypoint /bin/sh \
  -e "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}" \
  --mount "type=bind,source=$PROJECT,target=/code,readonly" \
  --mount "type=bind,source=$PROJECT/artifacts,target=/code/artifacts" \
  --mount "type=volume,source=${RUN_ID}-target,target=/code/target" \
  --mount "type=volume,source=${RUN_ID}-target,target=/target" \
  --mount "type=volume,source=${RUN_ID}-registry,target=/usr/local/cargo/registry" \
  "$IMAGE" -c "set -eu; trap 'chown -R $(id -u):$(id -g) /code/artifacts' EXIT; \
    { rustc --version; cargo --version; wasm-opt --version; } > artifacts/toolchain.txt; \
    /usr/local/bin/optimize.sh ."
