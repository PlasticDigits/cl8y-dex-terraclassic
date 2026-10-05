#!/usr/bin/env python3
"""Check the #1392 agent contract and its relative documentation links."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
DOCS = [
    'skills/AGENTS_CHAIN_ATTESTATION.md',
    'docs/runbooks/chain-attestation.md',
    'docs/qa/issue-1392/README.md',
    'docs/qa/issue-1392/verification.md',
]
for name in DOCS:
    source = ROOT / name
    content = source.read_text()
    for target in re.findall(r'\]\(([^)]+)\)', content):
        if '://' in target or target.startswith('#'):
            continue
        path = target.split('#', 1)[0]
        assert (source.parent / path).exists(), f'{name}: missing {target}'
    assert '#1392' in content or '1392' in content, name
skill = (ROOT / DOCS[0]).read_text()
for number in range(1, 11):
    assert f'NET1392-{number} —' in skill
checklist = (ROOT / DOCS[2]).read_text()
assert 'overall PASS with explicit skips' in checklist
assert 'An executed failed' in checklist
for number in range(1, 5):
    assert f'**AC{number}:**' in checklist
assert 'EXPECTED_CHAIN_ID=$CHAIN_ID' in (ROOT / 'scripts/deploy-dex-local.sh').read_text()
assert 'EXPECTED_CHAIN_ID=$TERRAD_HOST_CHAIN_ID' in (ROOT / 'scripts/deploy-dex-mainnet-soft-launch.sh').read_text()
assert 'AGENTS_CHAIN_ATTESTATION.md' in (ROOT / 'AGENTS.md').read_text()
print('PASS: #1392 invariant, checklist, generated environment and documentation links')
