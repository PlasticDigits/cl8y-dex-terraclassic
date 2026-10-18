#!/usr/bin/env bash
# #1211: separate compilation from bounded test execution.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
for control in cohort_preflight_1211 cohort_migration_ops cohort_online_worker_1211 cohort_online_ops; do
  timeout 59s python3 "scripts/tests/test_${control}.py"
done
artifacts="$(mktemp -d)"
trap 'rm -rf "$artifacts"' EXIT
(cd indexer && cargo test --locked --no-run --message-format=json --lib \
  --test indexer_fee_cohorts --test indexer_protocol_fees --test indexer_tier_sync \
  --test indexer_economic_fee_usd --test indexer_defillama > "$artifacts/build.json")
python3 - "$artifacts/build.json" "$artifacts" <<'PY'
import json, pathlib, sys
out=pathlib.Path(sys.argv[2])
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    try: artifact=json.loads(line)
    except ValueError: continue
    if artifact.get('reason')=='compiler-artifact' and artifact.get('executable') and artifact.get('profile',{}).get('test'):
        (out/artifact['target']['name']).write_text(artifact['executable'])
PY
timeout 59s "$(cat "$artifacts/cl8y_dex_indexer")" fee_cohorts::tests --test-threads=1
for suite in indexer_fee_cohorts indexer_protocol_fees indexer_tier_sync indexer_economic_fee_usd indexer_defillama; do
  timeout 59s "$(cat "$artifacts/$suite")" --test-threads=1
 done
printf '%s\n' 'PASS: local cohort/provider/source/privacy fixtures. Live migration, deployment and LocalTerra evidence remain separately required.'
