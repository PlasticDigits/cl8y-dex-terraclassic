#!/usr/bin/env python3
"""Read the immutable application revision prepared by release coordination."""
import json
from pathlib import Path
import re
import subprocess

BASELINE = 'fab88dbbccb514657d786e3034e12c99423026eb'
RECORD = Path.home()/'.local/state/cl8y-dex/release-20261006/application-source.json'
RUNTIME_INPUTS = ('indexer/src', 'indexer/Cargo.toml', 'indexer/Cargo.lock',
    'indexer/build.rs', 'indexer/migrations', 'indexer/.cargo',
    'frontend-dapp/src', 'frontend-dapp/public', 'frontend-dapp/package.json',
    'frontend-dapp/package-lock.json', 'docker/frontend/Dockerfile', 'tokenlist')


def revision(root):
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
    if not RECORD.exists():
        if head != BASELINE:
            raise RuntimeError('Prepared immutable application source record is missing')
        return BASELINE
    data = json.loads(RECORD.read_text())
    target = data['revision']
    if data.get('baseline') != BASELINE or not re.fullmatch(r'[0-9a-f]{40}', target) or head != target:
        raise RuntimeError('Checkout differs from the prepared application release')
    subprocess.run(['git', 'merge-base', '--is-ancestor', BASELINE, target], cwd=root, check=True)
    expected = data.get('runtime_inputs')
    if not isinstance(expected, dict) or set(expected) != set(RUNTIME_INPUTS):
        raise RuntimeError('Prepared application input identities are incomplete')
    for path, identity in expected.items():
        result = subprocess.run(['git', 'rev-parse', '--verify', target+':'+path],
                                cwd=root, capture_output=True, text=True)
        actual = result.stdout.strip() if result.returncode == 0 else None
        if actual != identity:
            raise RuntimeError('Prepared application input identity differs: '+path)
    subprocess.run(['git', 'diff', '--quiet', BASELINE, target, '--', 'smartcontracts'],
                   cwd=root, check=True)
    subprocess.run(['git', 'diff', '--quiet', target, '--', *RUNTIME_INPUTS,
                    'docker/indexer/Dockerfile'], cwd=root, check=True)
    return target
