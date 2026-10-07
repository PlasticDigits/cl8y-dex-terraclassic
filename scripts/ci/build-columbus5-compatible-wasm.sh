#!/usr/bin/env bash
# Compile only token/pair with the pinned compiler and size optimization. The
# source and lock are read-only; outputs must be outside the repository.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="cosmwasm/workspace-optimizer:0.16.1@sha256:b9c92b2900b7ebaab3499203615c1b8589592bc557355ed3432e48851ffde69e"
if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 /absolute/new-output-directory [z|s]" >&2
  exit 2
fi
[[ "$1" = /* ]] || { echo 'Output directory must be absolute' >&2; exit 2; }
OUTPUT="$(realpath -m "$1")"
case "$OUTPUT/" in "$ROOT/"*) echo 'Outputs must be outside this checkout' >&2; exit 2;; esac
LEVEL="${2:-z}"
[[ "$LEVEL" == z || "$LEVEL" == s ]] || { echo 'Optimization level must be z or s' >&2; exit 2; }
[[ ! -e "$OUTPUT" ]] || { echo 'Use a new output directory to preserve previous evidence' >&2; exit 2; }
mkdir -p "$OUTPUT"
RUN_ID="cl8y-compat-$$-${RANDOM}-${RANDOM}"
TARGET_VOLUME="${RUN_ID}-target"
CARGO_VOLUME="${RUN_ID}-cargo"
cleanup() {
  docker rm -f "$RUN_ID" >/dev/null 2>&1 || true
  docker volume rm "$TARGET_VOLUME" "$CARGO_VOLUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
python3 "$ROOT/scripts/ci/build-provenance.py" --root "$ROOT" \
  --source smartcontracts --lock smartcontracts/Cargo.lock --image "$IMAGE" \
  --toolchain "CARGO_PROFILE_RELEASE_OPT_LEVEL=$LEVEL; cargo build --locked; wasm-opt -Os --signext-lowering" \
  --capture-inputs --output "$OUTPUT/build-inputs.json"
docker volume create "$TARGET_VOLUME" >/dev/null
docker volume create "$CARGO_VOLUME" >/dev/null
docker run --rm --name "$RUN_ID" --entrypoint /bin/sh \
  --mount "type=bind,source=$ROOT/smartcontracts,target=/code,readonly" \
  --mount "type=bind,source=$OUTPUT,target=/out" \
  --mount "type=volume,source=$TARGET_VOLUME,target=/diagnostic-target" \
  --mount "type=volume,source=$CARGO_VOLUME,target=/diagnostic-cargo" \
  -e CARGO_TARGET_DIR=/diagnostic-target -e CARGO_HOME=/diagnostic-cargo \
  -e "CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}" -e "CARGO_PROFILE_RELEASE_OPT_LEVEL=$LEVEL" \
  -w /code "$IMAGE" -c 'set -eu
    { rustc --version; cargo --version; wasm-opt --version; } > /out/toolchain.txt
    # Separate invocations preserve each contract feature graph. Building both
    # packages together would add token CosmWasm capability markers to pair.
    for package in cl8y-community-tax-token cl8y-dex-pair; do
      cargo build --locked --release --target wasm32-unknown-unknown -p "$package" --lib
      module=$(echo "$package" | tr - _)
      wasm-opt -Os --signext-lowering \
        /diagnostic-target/wasm32-unknown-unknown/release/$module.wasm -o /out/$module.wasm
    done'
python3 "$ROOT/scripts/ci/build-provenance.py" --root "$ROOT" \
  --verify "$OUTPUT/build-inputs.json" --inputs-only --expected-image "$IMAGE"
python3 "$ROOT/scripts/ci/check-wasm-local-limit.py" --json "$OUTPUT"/*.wasm > "$OUTPUT/local-limit.json"
python3 - "$ROOT" "$OUTPUT" "$IMAGE" "$LEVEL" <<'PY'
import hashlib, json, sys
from pathlib import Path
root, output = Path(sys.argv[1]), Path(sys.argv[2])
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
receipt = {
    'schema': 'cl8y-columbus5-compatibility-build-v1',
    'source': json.loads((output / 'build-inputs.json').read_text()),
    'builder_image': sys.argv[3], 'toolchain': (output / 'toolchain.txt').read_text(),
    'compiler_environment': {'CARGO_PROFILE_RELEASE_OPT_LEVEL': sys.argv[4]},
    'optimizer_flags': ['-Os', '--signext-lowering'],
    'packages': ['cl8y-community-' + 'tax-token', 'cl8y-dex-' + 'pair'],
    'package_build_policy': 'separate cargo invocations; no workspace feature unification',
    'cache_policy': 'fresh owned Cargo and executable-target volumes, deleted after each run',
    'recipe_sha256': digest(root / 'scripts/ci/build-columbus5-compatible-wasm.sh'),
    'local_checker_sha256': digest(root / 'scripts/ci/check-wasm-local-limit.py'),
    'artifacts': {path.name: digest(path) for path in sorted(output.glob('*.wasm'))},
    'local_limit': json.loads((output / 'local-limit.json').read_text()),
    'scope': 'Compilation and declared-local bound only; not VM simulation, upload or migration.',
}
(output / 'compatibility-build.json').write_text(json.dumps(receipt, indent=2) + '\n')
PY
echo "Candidate artifacts and compatibility-build.json: $OUTPUT"
