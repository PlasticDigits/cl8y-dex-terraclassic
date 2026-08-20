#!/bin/bash
set -e

# Named volume overlays /code/target so the optimizer (uid 0) does not fill host
# smartcontracts/target. Docker may still create an empty host `target/` as root —
# see AGENTS.md § Rust / Docker gotchas. Do not copy this `docker run -v $(pwd)`
# pattern onto indexer/: that crate has no volume overlay and cargo lock files
# become root-owned (scripts/lib/docker-indexer-bind-mount.sh).
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.16.1

echo "Optimized contracts built successfully!"
echo "Output is in artifacts/ directory"
