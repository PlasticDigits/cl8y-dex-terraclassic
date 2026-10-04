#!/usr/bin/env python3
"""Read-only public quality evidence; bounded GETs, no credentials or chain writes."""
import argparse
import datetime
import json
import sys
from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import urlopen

MAX_BYTES = 1_048_576

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', default='https://indexer.dex.cl8y.com/api/v1')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--expect-quality', action='store_true', help='Require deployed quality metadata, not legacy payloads')
    args = parser.parse_args()
    parsed = urlsplit(args.base_url)
    if parsed.scheme not in ('https', 'http') or parsed.username or parsed.password or parsed.query or parsed.fragment:
        parser.error('base URL must be plain HTTP(S), without credentials/query/fragment')
    evidence = {'observed_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'base_url': args.base_url, 'reads': {}}
    for key, suffix in [('overview', '/overview'), ('hourly', '/protocol/liquidity/daily?grain=hourly&limit=72'),
                        ('daily', '/protocol/liquidity/daily?grain=daily&limit=35'),
                        ('monthly', '/protocol/liquidity/daily?grain=monthly&limit=24')]:
        try:
            with urlopen(args.base_url.rstrip('/') + suffix, timeout=15) as response:
                data = response.read(MAX_BYTES + 1)
                if len(data) > MAX_BYTES:
                    raise ValueError('response exceeds bounded evidence size')
                payload = json.loads(data)
            # Overview output contains protocol-wide public stats only. Never write
            # private account/wallet data even if an unexpected endpoint returns it.
            if key == 'overview':
                payload = {k: v for k, v in payload.items() if k.startswith(('liquidity_', 'total_liquidity_', 'priced_pair_', 'unpriced_pair_'))}
            elif not isinstance(payload, dict) or not isinstance(payload.get('series'), list):
                raise ValueError('unexpected public series shape')
            evidence['reads'][key] = {'status': 'ok', 'payload': payload}
        except Exception as exc:
            evidence['reads'][key] = {'status': 'unavailable', 'error_type': type(exc).__name__}
    if args.expect_quality:
        payload = evidence['reads']['overview'].get('payload', {})
        if payload.get('liquidity_quality') not in ('fresh', 'partial', 'stale', 'unavailable', 'verified_empty', 'unknown') or 'liquidity_as_of' not in payload:
            evidence['reads']['overview']['status'] = 'unavailable'
            evidence['reads']['overview']['error_type'] = 'MissingQualityContract'
    args.output.write_text(json.dumps(evidence, indent=2) + '\n')
    print(f'Public evidence written to {args.output}; reads={len(evidence["reads"])}')
    if any(read['status'] != 'ok' for read in evidence['reads'].values()):
        sys.exit(1)

if __name__ == '__main__':
    main()
