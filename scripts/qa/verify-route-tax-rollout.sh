#!/usr/bin/env bash
# Source verification only; SDK and production evidence are recorded separately.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
bash -n scripts/upgrade-1381-route-tax.sh
python3 -m py_compile scripts/lib/route_tax_rollout.py scripts/lib/route_tax_rollout_cli.py \
  scripts/qa/localterra-route-tax-rollout.py scripts/qa/test-route-tax-rollout.py
bash scripts/upgrade-1381-route-tax.sh --help >/dev/null
for pass in first retest; do
  printf '\n[%s behavioral rollout checks]\n' "$pass"
  python3 scripts/qa/test-route-tax-rollout.py
done
python3 scripts/qa/check-issue-1381-docs.py
git diff --check
echo 'PASS: rollout source verification and retest; production execution remains unverified.'
