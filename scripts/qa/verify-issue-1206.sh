#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
# Compilation is separate; each actual test invocation is bounded below one minute.
cargo test --locked --manifest-path indexer/Cargo.toml --test api_evidence_digest --lib --no-run
timeout 59s cargo test --locked --manifest-path indexer/Cargo.toml --test api_evidence_digest -- --test-threads=1
timeout 59s cargo test --locked --manifest-path indexer/Cargo.toml --lib evidence_digest -- --test-threads=1
git diff --check
