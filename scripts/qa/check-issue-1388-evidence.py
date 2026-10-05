#!/usr/bin/env python3
"""Validate recorded synthetic acceptance receipts against the reviewed manifest."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[2]
manifest = dict(line.split()[::-1] for line in
                (root / 'smartcontracts/artifacts/checksums.txt').read_text().splitlines())
evidence = root / 'docs/qa/issue-1388/evidence'
for name, minimum in [('pre-v1-funding-final.json', 6), ('pre-v1-flows-final.json', 5),
                      ('pre-v1-trust-final.json', 12), ('v12-funding-final.json', 6),
                      ('v12-trust-final.json', 12)]:
    data = json.loads((evidence / name).read_text())
    assert data['status'] == 'PASS' and data['chain_id'] == 'localterra'
    assert data['synthetic_only'] and len(data['checks']) >= minimum
    assert all(check['status'] == 'PASS' for check in data['checks'])
    if 'codes' in data:
        for artifact in ['cl8y_dex_router', 'cl8y_community_tax_token']:
            assert data['codes'][artifact]['sha256'] == manifest[artifact + '.wasm']
print('PASS recorded LocalTerra receipts and reviewed router/token hashes; production unverified')
