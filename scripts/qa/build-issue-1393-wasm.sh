#!/usr/bin/env bash
# Narrow optimized release build plus CW20 fixture for standalone LocalTerra smoke.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
target="${CARGO_TARGET_DIR:-$PWD/smartcontracts/target}"
artifacts="${VERIFY_ISSUE_1393_ARTIFACTS:-$PWD/smartcontracts/artifacts}"
toolchain="${VERIFY_ISSUE_1393_RUST_TOOLCHAIN:-1.81.0}"
rust_compiler="$(rustup which --toolchain "$toolchain" rustc)"
# Older std is required for CosmWasm 1.x MVP compatibility: current prebuilt
# wasm std itself includes bulk instructions even when crate flags disable them.
rust_flags='-C target-feature=-reference-types,-multivalue,-bulk-memory -C link-arg=--allow-undefined'
command -v wasm-opt >/dev/null
env CARGO_TARGET_DIR="$target" \
  RUSTC="$rust_compiler" RUSTFLAGS="$rust_flags" \
  cargo build --locked --manifest-path smartcontracts/Cargo.toml --release \
    --target wasm32-unknown-unknown -p bridge-debt -p bridge-debt-token --lib -j2
mkdir -p "$artifacts"
for crate in bridge_debt bridge_debt_token; do
  wasm-opt -Oz --signext-lowering "$target/wasm32-unknown-unknown/release/$crate.wasm" \
    -o "$artifacts/$crate.wasm"
done
if [[ -n "${VERIFY_ISSUE_1393_CW20_ARTIFACT:-}" ]]; then
  # Explicitly reused fixture; smoke records its actual SHA256 on chain store.
  cp "$VERIFY_ISSUE_1393_CW20_ARTIFACT" "$artifacts/cw20_mintable.wasm"
  fixture_kind="reused artifact (compiler provenance not asserted)"
  fixture_source="$VERIFY_ISSUE_1393_CW20_ARTIFACT"
else
  # Git dependencies are not workspace members. Build the exact lockfile revision
  # from its standalone manifest and committed lock, without altering the checkout.
  cargo metadata --locked --manifest-path smartcontracts/Cargo.toml --format-version 1 \
    > "$artifacts/issue1393-cargo-metadata.json"
  fixture_manifest="$(python3 - "$artifacts/issue1393-cargo-metadata.json" <<'PY'
import json
import sys
packages = json.load(open(sys.argv[1]))['packages']
print(next(p['manifest_path'] for p in packages if p['name'] == 'cw20-mintable'))
PY
)"
  env CARGO_TARGET_DIR="$target" \
    RUSTC="$rust_compiler" RUSTFLAGS="$rust_flags" \
    cargo build --locked --manifest-path "$fixture_manifest" --release \
      --target wasm32-unknown-unknown --lib -j2
  wasm-opt -Oz --signext-lowering "$target/wasm32-unknown-unknown/release/cw20_mintable.wasm" \
    -o "$artifacts/cw20_mintable.wasm"
  fixture_kind="rebuilt pinned standalone manifest with committed lockfile"
  fixture_source="$fixture_manifest"
fi
python3 - "$artifacts/issue1393-build.json" "$rust_compiler" "$rust_flags" "$fixture_kind" "$fixture_source" <<'PY'
import json
import subprocess
import sys
from pathlib import Path
metadata = {"rustc": subprocess.check_output([sys.argv[2], "--version"], text=True).strip(),
            "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
            "wasm_opt": subprocess.check_output(["wasm-opt", "--version"], text=True).strip(),
            "rustflags": sys.argv[3], "optimizer_flags": "-Oz --signext-lowering",
            "fixture_kind": sys.argv[4], "fixture_source": sys.argv[5]}
Path(sys.argv[1]).write_text(json.dumps(metadata, indent=2) + "\n")
PY
sha256sum "$artifacts/bridge_debt.wasm" "$artifacts/bridge_debt_token.wasm" "$artifacts/cw20_mintable.wasm"
