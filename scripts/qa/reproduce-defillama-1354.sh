#!/usr/bin/env bash
# Desired accounting assertions against the existing isolated integration harness.
# Both original acceptance probes must now pass; never run against production.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
probe="$(pwd)/indexer/tests/qa1354_generated.rs"
if [[ -e "$probe" ]]; then echo "Refusing to replace an existing probe file" >&2; exit 2; fi
if [[ ! -f indexer/.env.qa && -z "${TEST_DATABASE_ADMIN_URL:-}" ]]; then
  echo "Provision a disposable DB1396 QA cluster and scoped indexer/.env.qa first" >&2
  exit 2
fi
trap 'rm -f "$probe"' EXIT
cat indexer/tests/indexer_defillama.rs docs/qa/issue-1354/acceptance-probes.rs > "$probe"
cd indexer
cargo test --locked --test qa1354_generated qa1354_ -- --test-threads=1
