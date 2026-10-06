#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/route-common.sh"
route_ops
python3 - "$ROUTE_OPS_STATE/manifest.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
i = m['inventory']
print('Immutable plan:', sys.argv[1])
print('Manifest identity:', m['identity_hash'])
print('Affected pairs:', len(i['affected_pairs']))
for role, target in i['instances'].items():
    print(role, target['address'], 'live', target['code_id'], target['cw2']['version'], 'admin', target['admin'])
for target in i['tokens']:
    print('token', target['address'], 'live', target['code_id'], target['cw2']['version'], 'admin', target['admin'])
PY
