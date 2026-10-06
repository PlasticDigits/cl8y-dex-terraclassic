#!/usr/bin/env bash
# Fresh release outputs; no previously compiled Cargo target can satisfy this build.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_TARGET="$(mktemp -d "${TMPDIR:-/tmp}/cl8y-indexer-release.XXXXXXXX")"
trap 'rm -rf "$BUILD_TARGET"' EXIT
cd "$ROOT/indexer"
PROVENANCE_IMAGE="${CI_JOB_IMAGE:-local-host}"
TOOLCHAIN="$(rustc --version); $(cargo --version)"
python3 "$ROOT/scripts/ci/build-provenance.py" \
  --root "$ROOT" --source indexer --lock indexer/Cargo.lock \
  --image "$PROVENANCE_IMAGE" --toolchain "$TOOLCHAIN" \
  --capture-inputs --output "$BUILD_TARGET/inputs.json"
timeout --signal=TERM --kill-after=30s "${INDEXER_RELEASE_TIMEOUT_SECONDS:-2700}s" \
  cargo build --locked --release --target-dir "$BUILD_TARGET"
# Source/lock/recipe mutations during compilation cannot receive a valid receipt.
python3 "$ROOT/scripts/ci/build-provenance.py" --root "$ROOT" \
  --verify "$BUILD_TARGET/inputs.json" --inputs-only --expected-image "$PROVENANCE_IMAGE"
mkdir -p target/release
install -m 755 "$BUILD_TARGET/release/cl8y-dex-indexer" target/release/cl8y-dex-indexer
python3 "$ROOT/scripts/ci/build-provenance.py" \
  --root "$ROOT" --source indexer --lock indexer/Cargo.lock \
  --image "$PROVENANCE_IMAGE" --toolchain "$TOOLCHAIN" \
  --output target/release/build-provenance.json --artifact target/release/cl8y-dex-indexer
