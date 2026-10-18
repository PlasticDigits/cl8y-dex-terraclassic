#!/usr/bin/env python3
"""Bounded anonymous deployed digest/cohort smoke; no actor or response-body export."""
import argparse
from datetime import datetime, timedelta, timezone
from decimal import Decimal
import json
import os
from pathlib import Path
import re
import signal
import urllib.error
import urllib.request

ORIGIN = 'https://indexer.dex.cl8y.com'
SOURCES = ('swap_amm', 'book_take', 'limit_place', 'wrap', 'unwrap', 'ust1_mint', 'ust1_redeem', 'pair_creation', 'sku_unlock', 'settings_fee')
FORBIDDEN = {'actor', 'actor_hash', 'sender', 'receiver', 'maker', 'owner', 'provider', 'trader', 'tx_hash', 'txhash', 'swap_tx_hash', 'swap_event_id', 'unique_traders', 'distinct_senders', 'events', 'pairs'}
NUMBER = re.compile(r'\d+(?:\.\d+)?')

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def validate_anonymous(value, path=()):
    if isinstance(value, dict):
        for key, item in value.items():
            if key in FORBIDDEN:
                raise ValueError('digest_identity_key')
            validate_anonymous(item, (*path, key))
    elif isinstance(value, list):
        for item in value:
            validate_anonymous(item, (*path, 'item'))
    elif isinstance(value, str) and 'terra1' in value:
        if path != ('fees', 'by_token', 'item', 'contract_or_denom'):
            raise ValueError('digest_identity_value')


def validate_fee_row(row):
    if not {'event_count', 'amount_usd', 'share_pct'} <= row.keys():
        raise ValueError('fee_row_shape')
    if type(row['event_count']) is not int or row['event_count'] < 0:
        raise ValueError('fee_count')
    for key in ('amount_usd', 'share_pct'):
        value = row[key]
        if value is not None and (not isinstance(value, str) or len(value) > 128 or not NUMBER.fullmatch(value)):
            raise ValueError('fee_plain_decimal')
        if key == 'share_pct' and value is not None and Decimal(value) > 100:
            raise ValueError('fee_share')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.revision):
        parser.error('exact deployed revision required')
    receipt = {'schema': 'dex_public_evidence_live_v1', 'expected_revision': args.revision,
        'utc': datetime.now(timezone.utc).isoformat(), 'read_only': True, 'requests': 0, 'checks': {}}
    opener = urllib.request.build_opener(NoRedirect())

    def request(path, expected=200, method='GET'):
        receipt['requests'] += 1
        if receipt['requests'] > 14:
            raise ValueError('request_budget')
        req = urllib.request.Request(ORIGIN + path, method=method,
            headers={'Accept': 'application/json', 'Cache-Control': 'no-cache', 'User-Agent': 'CL8Y-anonymous-evidence-smoke/1'})
        try:
            response = opener.open(req, timeout=4)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            if response.code != expected:
                raise ValueError('unexpected_http_status')
            body = response.read(4_000_001)
            if len(body) > 4_000_000:
                raise ValueError('body_bound')
        return json.loads(body) if expected == 200 else None

    def check(name, passed):
        receipt['checks'][name] = bool(passed)
        if not passed:
            raise ValueError(name)

    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
    signal.alarm(50)
    try:
        check('serving_revision', request('/health').get('git_sha') == args.revision)
        today = datetime.now(timezone.utc).date()
        day = str(today - timedelta(days=1))
        digest = request('/api/v1/evidence/digest?day=' + day)
        validate_anonymous(digest)
        check('anonymous_digest', True)
        check('utc_complete_day', digest.get('day') == day and digest.get('timezone') == 'UTC' and digest.get('complete') is True)
        fees = digest.get('fees', {})
        check('fee_schema', isinstance(fees, dict) and {'total_usd', 'by_source', 'by_token'} <= fees.keys())
        total = fees['total_usd']
        check('fee_headline_plain', total is None or isinstance(total, str) and len(total) <= 128 and NUMBER.fullmatch(total) is not None)
        check('canonical_sources', [r.get('source') for r in fees['by_source']] == list(SOURCES))
        for row in fees['by_source']:
            validate_fee_row(row)
        for row in fees['by_token']:
            validate_fee_row(row)
            check('token_shape', {'contract_or_denom', 'is_other'} <= row.keys() and type(row['is_other']) is bool)
        check('token_count_reconciliation', sum(r['event_count'] for r in fees['by_source']) == sum(r['event_count'] for r in fees['by_token']))
        check('bounded_token_mix', len(digest['fees']['by_token']) <= 9)
        surfaces = {'swap': {'swap'}, 'lp': {'add', 'remove'}, 'limit': {'place', 'cancel', 'fill'}, 'wrap': {'wrap', 'unwrap'}, 'window': {'ust1_mint', 'ust1_redeem'}}
        check('count_surfaces', set(digest.get('counts', {})) == set(surfaces))
        for surface, kinds in surfaces.items():
            values = digest['counts'][surface]
            check('counts_' + surface, set(values) == kinds and all(type(v) is int and v >= 0 for v in values.values()))
        current = request('/api/v1/evidence/digest?day=' + str(today))
        check('today_incomplete', current.get('complete') is False and current.get('day') == str(today) and current.get('timezone') == 'UTC')
        for suffix in ('', '?day=invalid', '?day=' + day + '&sender=synthetic', '?day=2099-01-01', '?day=' + day + '&format=csv'):
            request('/api/v1/evidence/digest' + suffix, 400)
        check('invalid_queries', True)
        request('/api/v1/evidence/digest?day=' + day, 405, 'POST')
        check('get_only', True)
        check('openapi_digest', '/api/v1/evidence/digest' in request('/api-docs/openapi.json').get('paths', {}))
        # Three allowed windows; GET reads materialized snapshots only.
        for window in ('24h', '7d', '30d'):
            rows = request('/api/v1/protocol/fees?window=' + window)['by_cohort']
            check('cohorts_' + window, [r.get('cohort') for r in rows] == ['retail', 'mm', 'unknown'])
            for row in rows:
                if set(row) != {'cohort', 'amount_usd', 'share_pct', 'event_count'}:
                    raise ValueError('cohort_shape')
                validate_fee_row(row)
        check('serving_revision_after', request('/health').get('git_sha') == args.revision)
        receipt['overall'] = 'PASS'
    except Exception as error:
        receipt.update(overall='FAIL', failure_class=type(error).__name__)
        if isinstance(error, ValueError):
            receipt['failed_check'] = str(error)
    finally:
        signal.alarm(0)
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600), 'w') as stream:
        stream.write(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps({'overall': receipt['overall'], 'requests': receipt['requests'], 'checks': receipt['checks']}))
    return int(receipt['overall'] != 'PASS')

if __name__ == '__main__':
    raise SystemExit(main())
