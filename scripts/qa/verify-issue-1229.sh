#!/usr/bin/env bash
# Focused source acceptance; on-chain migration receipts remain separate.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
timeout 59s cargo test --manifest-path smartcontracts/Cargo.toml \
  -p cl8y-community-tax-token -p cl8y-community-token-launcher --offline -- \
  --test-threads=1
timeout 59s python3 scripts/tests/test_launcher_origin_1229_ops.py
for file in skills/AGENTS_LAUNCHER_ORIGIN.md docs/qa/issue-1229/README.md \
  skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md; do
  test -s "$file"
done
rg -q 'LO1229-1' skills/AGENTS_LAUNCHER_ORIGIN.md
rg -q 'CREATED_TOKENS' smartcontracts/contracts/community-token-launcher/src/contract.rs
printf '%s\n' 'PASS source origin hardening; LocalTerra and columbus-5 receipts remain separate.'
