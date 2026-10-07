#!/usr/bin/env bash
# Fresh release outputs; no previously compiled Cargo target can satisfy this build.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BUILD_TARGET="$ROOT/indexer/target/release-cold-build"
mkdir -p "$ROOT/indexer/target"
# A stable logical path also keeps Cargo's crate metadata stable. Atomic mkdir
# rejects both concurrent builds and leftover targets; never reuse their bytes.
if ! mkdir "$BUILD_TARGET"; then
  echo 'Fresh release target already exists; reconcile the other or interrupted build first.' >&2
  exit 1
fi
trap 'rm -rf "$BUILD_TARGET"' EXIT
cd "$ROOT/indexer"
PROVENANCE_IMAGE="${CI_JOB_IMAGE:-local-host}"
# rust-embed otherwise puts extraction-time metadata from vendored Swagger
# assets into the binary. This reviewed epoch denotes normalized asset metadata,
# not a release date, and must not inherit a caller's wall clock or environment.
export SOURCE_DATE_EPOCH=0
TOOLCHAIN="$(rustc --version); $(cargo --version); SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
# Generated sources (notably Swagger's embed.rs) contain file!() paths. Keep
# fresh targets while normalizing their directory in the release bytes.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$BUILD_TARGET=/cl8y-build/indexer-target"
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
