#!/usr/bin/env bash
# Cross-links for the integrated release close. No chain, database, or secret access.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
fail=0
check() {
  local label="$1"
  shift
  if "$@"; then
    printf 'PASS: %s\n' "$label"
  else
    printf 'FAIL: %s\n' "$label"
    fail=$((fail + 1))
  fi
}
check 'release skill' rg -q 'REL1421-1' skills/AGENTS_RELEASE_OPS_1421.md
check 'route rollout cross-link' rg -q 'AGENTS_RELEASE_OPS_1421.md' skills/AGENTS_ROUTE_TAX_ROLLOUT.md
check 'database skill cross-link' rg -q 'issue-1385/production-maintenance.json' skills/AGENTS_INDEXER_DATABASE_ISOLATION.md
check 'bridge-debt skill cross-link' rg -q 'AGENTS_RELEASE_OPS_1421.md' skills/AGENTS_BRIDGE_DEBT_MARKET_PINS.md
check 'agents index' rg -q 'AGENTS_RELEASE_OPS_1421.md' AGENTS.md
check 'testing index' rg -q 'verify-issue-1421' docs/testing.md
check '1384 checklist' rg -q 'production_execution' docs/qa/issue-1384/README.md
check '1396 reconfirmation' rg -q '2026-10-07 close reconfirmation' docs/qa/issue-1396/README.md
check '1421 close record' rg -q 'REL1421-10' docs/qa/issue-1421/README.md
check 'runbook route pointer' rg -q 'docs/qa/issue-1384/README.md' docs/runbooks/route-local-tax-1381.md
check 'runbook database pointer' rg -q 'production-maintenance.json' docs/runbooks/indexer-database-isolation.md
python3 - <<'PY'
import json
from pathlib import Path
receipt = json.loads(Path('docs/qa/issue-1385/production-maintenance.json').read_text())
verify = receipt['result']['verify']
assert verify['runtime_acl'] == 'PASS'
assert verify['migration_history_preserved'] is True
assert verify['schema_version'] == '20261006133000'
assert verify['migrations'] == 61
assert verify['actual_permission_denials'] == 4
assert receipt['result']['transition']['objects_transferred'] == 61
PY
printf 'PASS: production maintenance receipt fields\n'
if [[ "$fail" -ne 0 ]]; then
  printf 'Results: %s FAIL\n' "$fail"
  exit 1
fi
printf 'Results: cross-links PASS\n'
