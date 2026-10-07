#!/usr/bin/env python3
"""Apply verified rollout code identities to the two existing Coolify apps."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
import urllib.request
import urllib.error
import hashlib
import re
import importlib.util
from urllib.parse import urljoin, urlparse

ROOT = Path(__file__).resolve().parents[3]
INDEXER = 's14i3w45r5g6ulyu6nsxzm0n'
FRONTEND = 'afs50d2ect995yzc9jkp7u2u'
source_spec = importlib.util.spec_from_file_location('release_application_source',
    Path(__file__).with_name('application-source.py'))
application_source = importlib.util.module_from_spec(source_spec)
source_spec.loader.exec_module(application_source)
REVISION = application_source.revision(ROOT)
TOKEN_HASH = ('sha256', '64f94a96b294c0396c7bf630ddba74e8891d7f76bd3059cc21e7c69ceac2cd15')[1]


def api(path, method='GET', data=None):
    base = os.environ['COOLIFY_URL'].rstrip('/')
    if not base.endswith('/api/v1'):
        base += '/api/v1'
    request = urllib.request.Request(base + path, method=method,
        data=None if data is None else json.dumps(data).encode(),
        headers={'Authorization': 'Bearer ' + os.environ['COOLIFY_WIRE_TOKEN'],
                 'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(request, timeout=45) as response:
            return json.load(response)
    except urllib.error.HTTPError as exc:
        raise RuntimeError('Coolify HTTP ' + str(exc.code) + ' on ' + path) from None


def public_readback(values):
    """Read actual running application configuration after restart/cache invalidation."""
    def read(url, data=None):
        request = urllib.request.Request(url, data=None if data is None else json.dumps(data).encode(),
            headers={'User-Agent': 'cl8y-release-pin-readback', 'Content-Type': 'application/json'})
        with urllib.request.urlopen(request, timeout=20) as response:
            body = response.read(8_000_001)
            assert len(body) <= 8_000_000, 'Public application readback exceeds its byte budget'
            return body
    health = json.loads(read('https://indexer.dex.cl8y.com/health'))
    assert health.get('git_sha') == REVISION, 'Live indexer revision readback failed'
    catalog = json.loads(read('https://indexer.dex.cl8y.com/api/v1/community-tokens?limit=1'))
    assert catalog.get('configured') is True and str(catalog.get('code_id')) == values['COMMUNITY_TAX_CODE_ID'], \
        'Live Create/Manage catalog pin readback failed'
    html = read('https://dex.cl8y.com/').decode()
    scripts = re.findall(r'<script[^>]+src="([^"]+)"', html)
    entrypoints = [s for s in scripts if s.startswith('/assets/') and s.endswith('.js')]
    assert entrypoints, 'Live frontend entrypoint unavailable'
    queue = ['https://dex.cl8y.com'+p for p in entrypoints]
    bundles, observed_urls, total_bytes = [], [], 0
    while queue:
        url = queue.pop(0)
        if url in observed_urls:
            continue
        assert len(observed_urls) < 64, 'Frontend chunk readback exceeds its finite budget'
        content = read(url).decode()
        total_bytes += len(content.encode())
        assert total_bytes <= 32_000_000, 'Frontend bundle readback exceeds its finite byte budget'
        observed_urls.append(url)
        bundles.append(content)
        for ref in re.findall(r'''["']((?:\.{1,2}/|/assets/|assets/)?[A-Za-z0-9_-]+\.js)["']''', content):
            target = urljoin(url, '/'+ref if ref.startswith('assets/') else ref)
            parsed = urlparse(target)
            if parsed.netloc == 'dex.cl8y.com' and parsed.path.startswith('/assets/') and target not in observed_urls:
                queue.append(target)
    merged = '\n'.join(bundles)
    assert values['VITE_ROUTER_ADDRESS'] in merged, 'Live frontend router pin missing'
    assert values['VITE_COMMUNITY_TAX_CODE_ID'] in merged, 'Live frontend token pin missing'
    return {'indexer_health': health, 'catalog_configured': catalog['configured'],
            'catalog_code_id': catalog['code_id'],
            'frontend_entrypoints': entrypoints,
            'frontend_chunks': observed_urls,
            'frontend_bundle_sha256': [hashlib.sha256(b.encode()).hexdigest() for b in bundles]}


def live_trading_readback():
    """Quote the actual running trusted v1 policy after pause restoration."""
    trader = 'terra19zfqyjneztmfzx25c9zzxrkv9hll882n8f5tn7'
    payload = {'token_in': 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz',
               'token_out': 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72',
               'amount_in': '1000000', 'trader': trader, 'sender': trader}
    request = urllib.request.Request('https://indexer.dex.cl8y.com/api/v1/route/solve',
        data=json.dumps(payload).encode(), headers={'User-Agent': 'cl8y-release-trading-readback', 'Content-Type': 'application/json'})
    with urllib.request.urlopen(request, timeout=30) as response:
        trading = json.load(response)
    assert trading.get('tax_kind') == 'route_local_v1' and trading.get('router_hops_tax') is True, \
        'Running indexer trading trust/capability readback failed'
    assert int(trading.get('estimated_amount_out', 0)) > 0 and \
        trading.get('estimated_amount_out_net') == trading.get('estimated_amount_out'), \
        'Running indexer funded-net quote parity failed'
    return {'request': payload, 'response': trading, 'quote_only': True}


def apply_public_pins(app, changes):
    existing = {e['key'] for e in api('/applications/'+app+'/envs') if not e.get('is_preview')}
    for key, value in changes.items():
        api('/applications/'+app+'/envs', 'PATCH' if key in existing else 'POST', {
            'key': key, 'value': value, 'is_preview': False, 'is_literal': True,
            'is_buildtime': app == FRONTEND, 'is_runtime': True})
    observed = {e['key']: e.get('real_value') or e.get('value')
                for e in api('/applications/'+app+'/envs') if not e.get('is_preview')}
    assert all(observed.get(k) == v for k, v in changes.items()), 'Public pin readback failed'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--state-dir', type=Path,
        default=Path.home()/'.local/state/cl8y-dex/release-20261006-route')
    args = parser.parse_args()
    env = os.environ.copy()
    env['ROUTE_OPS_STATE'] = str(args.state_dir)
    subprocess.run(['bash', str(ROOT/'scripts/ops/2026-10-06/route-03-verify.sh')],
                   cwd=ROOT, env=env, check=True)
    journal = json.loads((args.state_dir/'journal.json').read_text())
    assert journal['status'] in ('contracts_ready_paused', 'complete')
    values = dict(line.split('=', 1) for line in
                  (args.state_dir/'deployment.env').read_text().splitlines())
    assert values['TRADING_ROUTE_TAX_DATA_HASHES'] == TOKEN_HASH
    token_code = str(journal['codes']['token'])
    assert values['COMMUNITY_TAX_CODE_ID'] == token_code
    for app, expected in [(INDEXER, 'CL8Y Dex TerraClassic Indexer'),
                          (FRONTEND, 'CL8Y Dex Frontend')]:
        current = api('/applications/' + app)
        assert current['name'] == expected
        assert current['git_repository'].rstrip('/').removesuffix('.git') == \
            'https://git.cl8y.com/code/cl8y-dex-terraclassic'
    names = {e['key'] for e in api('/applications/'+INDEXER+'/envs')
             if not e.get('is_preview')}
    assert 'INDEXER_RUNTIME_ROLE' in names, 'Complete the separate DB role transition first'
    selected = {
        INDEXER: {k: values[k] for k in (
            'TRADING_ROUTE_TAX_CODE_IDS', 'TRADING_ROUTE_TAX_DATA_HASHES',
            'TRADING_TAX_FREE_CODE_IDS', 'COMMUNITY_TAX_CODE_ID',
            'COMMUNITY_TAX_OPTION2_CODE_IDS', 'ROUTER_ADDRESS')},
        FRONTEND: {k: values[k] for k in ('VITE_COMMUNITY_TAX_CODE_ID', 'VITE_ROUTER_ADDRESS')},
    }
    selected[INDEXER]['EXPECTED_CHAIN_ID'] = 'columbus-5'
    # Read-back precedes restart. A partial write leaves contracts contained.
    for app, changes in selected.items():
        apply_public_pins(app, changes)
        api('/applications/'+app, 'PATCH', {'git_branch': 'main', 'git_commit_sha': REVISION,
                                          'is_auto_deploy_enabled': False})
    deployments = []
    for app in selected:
        deployments.extend(api('/deploy?uuid='+app+'&force=false', 'POST')['deployments'])
    receipt = {'revision': REVISION, 'public_pins': selected, 'deployments': deployments}
    output = args.state_dir/'application-rollout.json'
    output.write_text(json.dumps(receipt, indent=2)+'\n')
    deadline = time.monotonic()+2400
    while time.monotonic() < deadline:
        states = [api('/deployments/'+d['deployment_uuid']) for d in deployments]
        status = [d.get('status') for d in states]
        if any(s in ('failed', 'cancelled') for s in status):
            raise RuntimeError('Application deployment failed; markets remain contained')
        if all(s == 'finished' for s in status):
            assert all(d.get('commit') == REVISION for d in states)
            # A successful deployment status alone does not prove the active app.
            readiness_deadline = time.monotonic()+300
            while True:
                try:
                    receipt['live_readback'] = public_readback(values)
                    break
                except (AssertionError, urllib.error.URLError, TimeoutError, ValueError):
                    if time.monotonic() >= readiness_deadline:
                        raise RuntimeError('Live revision/catalog/frontend pin readback failed; preserve rollout pauses') from None
                    time.sleep(10)
            receipt['status'] = 'finished'
            output.write_text(json.dumps(receipt, indent=2)+'\n')
            print('Integrated indexer/frontend deployed with verified live code pins.')
            return
        time.sleep(15)
    raise RuntimeError('Application deployment deadline expired; preserve rollout pauses')


if __name__ == '__main__':
    main()
