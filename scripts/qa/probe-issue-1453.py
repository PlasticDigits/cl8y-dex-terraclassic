#!/usr/bin/env python3
"""Collect bounded, redacted, read-only liquidity-provider operational evidence."""
import argparse
import base64
import contextlib
import datetime
import json
import os
import re
import signal
import subprocess
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import quote, urlsplit

MAX_BYTES = 2_097_152
MAX_ENDPOINTS = 8
REQUEST_TIMEOUT_SECONDS = 5
DEFAULT_APP = 's14i3w45r5g6ulyu6nsxzm0n'
CONFIG_KEYS = {'LCD_URLS', 'LCD_URL', 'EXPECTED_CHAIN_ID'}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        # Authenticated redirects must not forward the injected Coolify token.
        return None


class ProbeDeadline(TimeoutError):
    pass


def plain_url(value):
    parsed = urlsplit(value)
    if (parsed.scheme not in ('https', 'http') or not parsed.hostname
            or parsed.username or parsed.password or parsed.query or parsed.fragment):
        raise ValueError('plain_http_url_required')
    return value.rstrip('/')


@contextlib.contextmanager
def request_deadline():
    # The CLI is single threaded. A hard deadline also bounds trickle responses,
    # unlike a socket timeout alone. Linux/Unix is required for the optional audit.
    def expired(_signum, _frame):
        raise ProbeDeadline('request_deadline')
    previous = signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, REQUEST_TIMEOUT_SECONDS)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


def read_json(url, token=None, height=None, include_height=False):
    headers = {'User-Agent': 'cl8y-1453-readonly-audit'}
    if token:
        headers['Authorization'] = 'Bearer ' + token
    if height is not None:
        headers['X-Cosmos-Block-Height'] = height
    request = urllib.request.Request(url, headers=headers, method='GET')
    with request_deadline(), urllib.request.build_opener(NoRedirect()).open(
            request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
        body = response.read(MAX_BYTES + 1)
        if len(body) > MAX_BYTES:
            raise ValueError('bounded_response')
        payload = json.loads(body)
        if include_height:
            return payload, response.headers.get('x-cosmos-block-height')
        return payload


def safe_error(exc):
    result = {'status': 'SKIP', 'error_type': type(exc).__name__}
    if isinstance(exc, urllib.error.HTTPError):
        result['http_status'] = exc.code
    return result


def safe_text(value, pattern, limit):
    return value if isinstance(value, str) and len(value) <= limit and re.fullmatch(pattern, value) else None


def chain_id(value):
    return safe_text(value, r'[a-zA-Z0-9_-]+', 80)


def provider_health(value):
    if not isinstance(value, dict):
        return {}
    counts = ('attempts', 'successes', 'failures', 'rate_deferrals',
              'concurrency_deferrals', 'circuit_deferrals', 'recovery_probes',
              'in_flight', 'consecutive_failures')
    sanitized = {key: value[key] for key in counts
                 if type(value.get(key)) is int and 0 <= value[key] < 2**64}
    for key in ('circuit_open', 'half_open'):
        if type(value.get(key)) is bool:
            sanitized[key] = value[key]
    return sanitized


def revision():
    try:
        return subprocess.check_output(['git', 'rev-parse', 'HEAD'], stderr=subprocess.DEVNULL,
                                       text=True, timeout=2).strip()
    except (OSError, subprocess.SubprocessError):
        return None


def health_evidence(receipt, health_url):
    try:
        health = read_json(health_url)
        identity = health.get('chain_identity', {})
        receipt['deployed_revision'] = safe_text(health.get('git_sha'), r'[0-9a-fA-F]+', 64)
        receipt['expected_chain_id'] = chain_id(identity.get('expected_chain_id'))
        entries = identity.get('endpoints', [])
        receipt['runtime_endpoint_count'] = len(entries)
        receipt['runtime_attestation'] = [
            {'endpoint_index': index, 'verified': entry.get('verified') is True,
             'observed_chain_id': chain_id(entry.get('observed_chain_id')),
             'provider': provider_health(entry.get('provider'))}
            for index, entry in enumerate(entries[:MAX_ENDPOINTS]) if isinstance(entry, dict)
        ]
        receipt['health_status'] = 'PASS'
    except Exception as exc:
        receipt['health'] = safe_error(exc)


def provider_probe(index, endpoint, pool_contract=None):
    item = {'endpoint_index': index}
    try:
        base = plain_url(endpoint)
    except ValueError as exc:
        item['endpoint_validation'] = safe_error(exc)
        return item
    for label, path in (
        ('identity', '/cosmos/base/tendermint/v1beta1/node_info'),
        ('latest', '/cosmos/base/tendermint/v1beta1/blocks/latest'),
    ):
        try:
            data = read_json(base + path)
            if label == 'identity':
                item['observed_chain_id'] = chain_id(data.get('default_node_info', {}).get('network'))
                if item['observed_chain_id'] is None:
                    raise ValueError('invalid_chain_header')
            else:
                header = data.get('block', {}).get('header', {})
                item['height'] = safe_text(header.get('height'), r'[0-9]+', 20)
                if item['height'] is None or int(item['height']) <= 0:
                    raise ValueError('invalid_height_header')
                timestamp = header.get('time')
                # Validate timestamps; never copy arbitrary upstream strings.
                parsed = datetime.datetime.fromisoformat(timestamp.replace('Z', '+00:00'))
                if parsed.tzinfo is None:
                    raise ValueError('timestamp_timezone_required')
                item['block_time'] = parsed.astimezone(datetime.timezone.utc).isoformat()
            item[label + '_status'] = 'PASS'
        except Exception as exc:
            item[label] = safe_error(exc)
    if pool_contract is not None:
        item['smart_pool'] = pinned_pool_probe(base, pool_contract, item.get('height'))
    return item


def pinned_pool_probe(base, contract, height):
    if height is None:
        return {'status': 'SKIP', 'error_type': 'LatestHeightUnavailable'}
    query = quote(base64.b64encode(b'{"pool":{}}').decode('ascii'), safe='')
    try:
        # Deliberately discard the contract response. Only request/response height
        # equality and successful bounded JSON retrieval belong in the receipt.
        _, observed_height = read_json(
            base + '/cosmwasm/wasm/v1/contract/' + contract + '/smart/' + query,
            height=height, include_height=True)
        if observed_height is None:
            return {'status': 'SKIP', 'error_type': 'MissingHeightHeader'}
        if observed_height != height:
            return {'status': 'SKIP', 'error_type': 'HeightMismatch'}
        return {'status': 'PASS'}
    except Exception as exc:
        return safe_error(exc)


def coolify_evidence(receipt, app, pool_contract=None):
    try:
        base = plain_url(os.environ['COOLIFY_URL'])
        if not base.endswith('/api/v1'):
            base += '/api/v1'
        token = os.environ['COOLIFY_WIRE_TOKEN']
        if not token:
            raise ValueError('missing_token')
        rows = read_json(base + '/applications/' + app + '/envs', token)
        if not isinstance(rows, list):
            raise ValueError('unexpected_configuration_shape')
        config = {row['key']: row.get('value', '') for row in rows if isinstance(row, dict)
                  and not row.get('is_preview') and row.get('key') in CONFIG_KEYS}
        urls = [url.strip() for url in config.get('LCD_URLS', config.get('LCD_URL', '')).split(',')
                if url.strip()]
        receipt['configured_endpoint_count'] = len(urls)
        receipt['configuration_status'] = 'PASS'
        receipt['configuration_scope'] = 'Non-preview LCD/expected-chain allowlist; raw values and URLs omitted.'
        receipt['multiple_endpoints_configured'] = len(urls) > 1
        receipt['configured_expected_chain_id'] = chain_id(config.get('EXPECTED_CHAIN_ID'))
        receipt['provider_probes'] = [provider_probe(index, url, pool_contract) for index, url in enumerate(urls[:MAX_ENDPOINTS])]
        receipt['unprobed_endpoint_count'] = max(0, len(urls) - MAX_ENDPOINTS)
    except Exception as exc:
        receipt['configuration'] = safe_error(exc)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--health-url', default='https://indexer.dex.cl8y.com/health')
    parser.add_argument('--audit-coolify', action='store_true', help='Read non-preview config using injected COOLIFY_URL/COOLIFY_WIRE_TOKEN')
    parser.add_argument('--coolify-app', default=DEFAULT_APP)
    parser.add_argument('--pool-contract', help='Optional lowercase Terra contract for height-pinned read-only pool availability checks')
    args = parser.parse_args()
    if args.pool_contract is not None:
        if not args.audit_coolify:
            parser.error('--pool-contract requires --audit-coolify to identify configured providers')
        if not re.fullmatch(r'terra1[023456789acdefghjklmnpqrstuvwxyz]{20,84}', args.pool_contract):
            parser.error('pool contract must be lowercase Terra address text, <=90 characters, without path/query characters')
    try:
        health_url = plain_url(args.health_url)
    except ValueError:
        parser.error('health URL must be plain HTTP(S), without credentials/query/fragment')
    if not re.fullmatch(r'[a-zA-Z0-9_-]{1,64}', args.coolify_app):
        parser.error('Coolify application ID must be 1–64 alphanumeric, underscore or dash characters')
    receipt = {
        'issue': 1453, 'status': 'PARTIAL',
        'observed_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'source_revision': revision(),
        'evidence_scope': 'Read-only public health; optional non-preview Coolify config and bounded node identity/latest probes. No signing, configuration writes or database mutation.',
        'bounds': {'max_provider_endpoints': MAX_ENDPOINTS, 'max_response_bytes': MAX_BYTES,
                   'request_timeout_seconds': REQUEST_TIMEOUT_SECONDS,
                   'max_http_reads': 26 if args.pool_contract else 18},
        'reason': 'Point-in-time configuration/headers do not establish independently operated providers, smart-query availability, history support, production freshness or verified failover/recovery.',
        'live_failover_acceptance': 'SKIP: new implementation deployment and bounded live recovery require separate evidence.',
    }
    health_evidence(receipt, health_url)
    if args.audit_coolify:
        coolify_evidence(receipt, args.coolify_app, args.pool_contract)
    else:
        receipt['configuration_status'] = 'SKIP: optional Coolify audit not requested.'
    args.output.write_text(json.dumps(receipt, indent=2) + '\n')
    print('Redacted read-only operational receipt saved; public health and optional configured-provider probes only.')


if __name__ == '__main__':
    main()
