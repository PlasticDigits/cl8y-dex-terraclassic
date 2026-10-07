#!/usr/bin/env python3
"""One terminal handoff for the reviewed DEX and Bridge-Debt release."""
import argparse
import getpass
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import urllib.request
import importlib.util
import fcntl

ROOT = Path(__file__).resolve().parents[3]
OPS = ROOT/'scripts/ops/2026-10-06'
STATE = Path.home()/'.local/state/cl8y-dex/release-20261006-route'
sys.path.insert(0, str(ROOT/'scripts/lib'))
from route_tax_rollout_cli import (TerradChain, parser as transport_parser, child_environment,
                                  KEYRING_PASSWORD_ENV, KEYRING_FD_ENV, keyring_password_pipe)


def run(command, env=None, credentials=()):
    allowed = set(credentials) - {KEYRING_PASSWORD_ENV, KEYRING_FD_ENV}
    child = child_environment(env, allowed)
    if KEYRING_PASSWORD_ENV in credentials:
        source = os.environ if env is None else env
        with keyring_password_pipe(source[KEYRING_PASSWORD_ENV]) as fd:
            child[KEYRING_FD_ENV] = str(fd)
            subprocess.run(command, cwd=ROOT, env=child, pass_fds=(fd,), close_fds=True, check=True)
    else:
        subprocess.run(command, cwd=ROOT, env=child, close_fds=True, check=True)


def note(issue, body, env):
    with tempfile.NamedTemporaryFile('w', prefix='dex-custody-note-', suffix='.md') as file:
        file.write(body)
        file.flush()
        run(['fj', 'issue', 'comment', str(issue), '--body-file', file.name], env,
            ('FORGEJO_PLASTICDIGTS_PAT',))


def load_module(name, filename):
    spec = importlib.util.spec_from_file_location(name, OPS/filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def adapter(env):
    tr = json.loads((STATE/'journal.json').read_text())['identity']['config']['transport']
    chain = TerradChain(transport_parser().parse_args(['--state-dir', str(STATE), '--home', tr['home'],
        '--keyring-backend', tr['keyring_backend'], '--node', tr['node'], '--lcd', tr['lcd']]))
    # The in-process adapter receives custody directly; unrelated children and
    # terrad environments never need the passphrase.
    chain.password = env[KEYRING_PASSWORD_ENV]
    chain.unlock()
    return chain


def provider_readback(env):
    result = subprocess.run([sys.executable, str(OPS/'route-provider-readback.py')], cwd=ROOT,
                            env=child_environment(env), close_fds=True, text=True, capture_output=True)
    (STATE/'provider-readback.json').write_text(result.stdout)
    (STATE/'provider-readback.stderr').write_text(result.stderr)
    if result.returncode:
        raise RuntimeError('Provider readback failed; raw JSON/errors saved and rollout pauses preserved')
    evidence = json.loads(result.stdout)
    if evidence.get('status') != 'PASS':
        raise RuntimeError('Provider qualification did not pass')


def complete_route(env):
    containment = load_module('route_containment', 'route-containment.py')
    smoke = load_module('route_live_smoke', 'route-live-smoke.py')
    chain = adapter(env)
    containment_path = STATE/'containment.json'
    if containment_path.exists():
        saved = json.loads(containment_path.read_text())
        if saved['status'] != 'restored':
            if saved['status'] in ('new', 'containing'):
                containment.contain(chain, STATE, saved.get('reason', 'Resume interrupted containment'))
            # An included original swap can reconcile while markets are paused.
            # A failed/unknown swap cannot authorize another unpause or new smoke.
            smoke.run_smoke(chain, STATE)
            containment.restore_after_success(chain, STATE)
    journal = json.loads((STATE/'journal.json').read_text())
    if journal['status'] in ('contracts_ready_paused', 'complete'):
        run(['bash', str(OPS/'route-03-verify.sh')], env)
    else:
        run(['bash', str(OPS/'route-02-execute.sh')], env, (KEYRING_PASSWORD_ENV,))
    run([sys.executable, str(OPS/'deploy-route-pins.py')], env, ('COOLIFY_WIRE_TOKEN',))
    provider_readback(env)
    journal = json.loads((STATE/'journal.json').read_text())
    if journal['status'] != 'complete':
        run(['bash', str(OPS/'route-04-finalize.sh')], env, (KEYRING_PASSWORD_ENV,))
    try:
        smoke.run_smoke(chain, STATE)
        deploy = load_module('route_live_deployment', 'deploy-route-pins.py')
        trading_path = STATE/'application-trading-readback.json'
        try:
            trading = deploy.live_trading_readback()
        except Exception as error:
            trading_path.write_text(json.dumps({'status': 'FAIL', 'error': str(error)})+'\n')
            raise
        trading_path.write_text(json.dumps({'status': 'PASS', **trading}, indent=2)+'\n')
    except Exception as error:
        if (STATE/'live-smoke/swap-pending.json').exists():
            try:
                containment.contain(chain, STATE, str(error))
            except Exception as containment_error:
                raise RuntimeError('Smoke failed and containment is incomplete; preserve both journals: '+str(containment_error)) from error
        raise
    run(['bash', str(OPS/'route-03-verify.sh')], env)
    evidence = json.loads((STATE/'live-smoke/evidence.json').read_text())
    application = json.loads((STATE/'application-rollout.json').read_text())
    trading = json.loads((STATE/'application-trading-readback.json').read_text())
    if evidence.get('status') != 'PASS' or application.get('status') != 'finished' or not application.get('live_readback') or trading.get('status') != 'PASS':
        raise RuntimeError('Actual custody/application/smoke completion receipts are incomplete')
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', action='store_true', help='Perform the prepared release with local keyring custody')
    parser.add_argument('--check', action='store_true', help='Read-only plan and custody prerequisites')
    args = parser.parse_args()
    if not (args.run or args.check) or (args.run and args.check):
        parser.error('Choose --run or --check')
    for name in ('COOLIFY_URL', 'COOLIFY_WIRE_TOKEN', 'FORGEJO_PLASTICDIGTS_PAT'):
        if not os.environ.get(name):
            raise RuntimeError('Run this script through the documented bao-exec command')
    # Planning and artifact checks happen before custody is requested.
    run(['bash', str(OPS/'route-01-plan.sh')])
    run([sys.executable, str(OPS/'bridge-debt.py'), 'plan'])
    request = urllib.request.Request('https://indexer.dex.cl8y.com/health',
        headers={'User-Agent': 'cl8y-dex-release-20261006'})
    with urllib.request.urlopen(request, timeout=15) as response:
        health = json.load(response)
    deploy = load_module('release_deployment_preflight', 'deploy-route-pins.py')
    if health.get('git_sha') != deploy.REVISION:
        raise RuntimeError('Integrated indexer deployment must be completed before contract custody')
    if args.check:
        print('Read-only release plans and deployed integrated indexer checks passed.')
        return
    if not sys.stdin.isatty():
        raise RuntimeError('Run in your interactive terminal for encrypted terrad keyring custody')
    lock = os.fdopen(os.open(STATE/'custody.lock', os.O_WRONLY | os.O_CREAT, 0o600), 'w')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        lock.close()
        raise RuntimeError('Another release process owns this custody workflow') from None
    env = os.environ.copy()
    env['TERRAD_HOST_KEYRING_PASS'] = getpass.getpass('terrad keyring passphrase (this process only): ')
    if not env['TERRAD_HOST_KEYRING_PASS']:
        raise RuntimeError('Empty keyring passphrase')
    try:
        evidence = complete_route(env)
        note(1384, 'Local-custody release completed: reviewed existing-address migrations, '
             'AutoLP creation template rotation, actual code/hash/config readbacks, '
             'integrated application pins and restart, pause restoration, two one-hop '
             'funded quote checks and independently funded native-wrap/two-hop/buy-tax '
             'execution passed. Durable receipts are in '
             '`~/.local/state/cl8y-dex/release-20261006-route/`. '
             'Provider configured gas limits and populated greedy qualification remain '
             'separate; no universal 3M envelope is claimed. Native gross input '
             'was 1,000,000 uluna with zero initial intermediate token balances. '
             'Actual smoke transaction: '+evidence['receipts'][-1]['txhash']+'; '
             'gas used: '+str(evidence['gas_used'])+'.\n', env)
        run(['fj', 'issue', 'close', '1384'], env, ('FORGEJO_PLASTICDIGTS_PAT',))
        run([sys.executable, str(OPS/'bridge-debt.py'), 'execute'], env, (KEYRING_PASSWORD_ENV,))
        run([sys.executable, str(OPS/'bridge-debt.py'), 'verify'], env)
        run([sys.executable, str(OPS/'bridge-debt.py'), 'probes'], env, (KEYRING_PASSWORD_ENV,))
        note(1421, 'Local-custody Bridge-Debt 1.1 release completed with exact reviewed '
             'legacy market pins, finite limits, preserved accounting and Issue paused. '
             'The journal confirms one-raw-unit repayment and plain-transfer/recovery '
             'receipts, with accounting readback. DEX contract migration, app pins, '
             'pause restoration and independently funded native-wrap/two-hop execution '
             'also passed. Receipts: `~/.local/state/cl8y-dex/release-20261006-bridge-debt/` '
             'and `~/.local/state/cl8y-dex/release-20261006-route/`. '
             'These bounded receipts do not establish arbitrary-hook or every-provider gas compatibility.\n', env)
        print('DEX rollout and Bridge-Debt custody operations passed; fj notes posted.')
    finally:
        env.pop('TERRAD_HOST_KEYRING_PASS', None)
        lock.close()


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
