#!/usr/bin/env python3
"""Validate #1453 guidance links/receipt shape; never certify live receipt claims."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = (
    'skills/AGENTS_LIQUIDITY_FEED.md',
    'docs/qa/issue-1453/README.md',
)


def main():
    for name in DOCS:
        path = ROOT / name
        text = path.read_text()
        for target in re.findall(r'\[[^\]]+\]\(([^)]+)\)', text):
            if '://' in target or target.startswith('#'):
                continue
            local = target.split('#', 1)[0]
            assert (path.parent / local).exists(), f'{name}: missing {target}'
    skill = (ROOT / DOCS[0]).read_text()
    for number in range(1, 11):
        assert f'LF1453-{number} ' in skill, f'missing invariant {number}'
    for filename in ('operational-receipt.json', 'historical-reconciliation.json'):
        receipt = json.loads((ROOT / 'docs/qa/issue-1453' / filename).read_text())
        assert receipt['status'] in ('PASS', 'PARTIAL', 'SKIP', 'FAIL')
        assert receipt.get('evidence_scope'), f'{filename}: explicit scope required'
        assert 'source_revision' in receipt and 'observed_at_utc' in receipt
        if receipt['status'] in ('SKIP', 'PARTIAL', 'FAIL'):
            assert any(receipt.get(key) for key in ('reason', 'live_failover_acceptance', 'repair_decision')), f'{filename}: missing limitation/finding reason'
    assert 'AGENTS_LIQUIDITY_FEED.md' in (ROOT / 'AGENTS.md').read_text()
    assert 'Cached liquidity feed (#1453)' in (ROOT / 'docs/indexer-invariants.md').read_text()
    assert 'verify-issue-1453:' in (ROOT / 'Makefile').read_text()
    tests = (ROOT / 'indexer/tests/book_snapshot_loop.rs').read_text()
    for name in (
        'issue_1453_pool_success_survives_component_failures',
        'issue_1453_monotonic_pool_and_hybrid_generations',
        'issue_1453_background_lanes_refresh_pool_while_book_stalls',
    ):
        assert f'fn {name}(' in tests, f'required issue regression missing: {name}'
    print('PASS: guidance links, numbered invariants, required regression presence and receipt structure.')
    print('Receipt structure does not establish live acceptance or historical reconstruction.')


if __name__ == '__main__':
    main()
