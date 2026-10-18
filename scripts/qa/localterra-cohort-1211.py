#!/usr/bin/env python3
"""Owned direct-swap/register/paid-wrap fixture; no shared contract mutations.

Prepare may compile/provision independently. Run emits three fresh actual LCD
receipts for the separately bounded production-ingestion test.
"""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import time
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[2]
STATE = Path('/tmp/dex-cohort-1211-localterra.json')
RECEIPTS = Path('/tmp/dex-cohort-1211-receipts.json')
spec = importlib.util.spec_from_file_location('origin_fixture', ROOT / 'scripts/qa/localterra-origin-1229.py')
chain = importlib.util.module_from_spec(spec)
spec.loader.exec_module(chain)


def save(state):
    STATE.write_text(json.dumps(state, indent=2) + '\n')
    STATE.chmod(0o600)


def execute(state, contract, message, funds=None):
    args = ['wasm', 'execute', contract, json.dumps(message)]
    if funds:
        args += ['--amount', funds]
    return chain.tx(state['key'], *args)


def code(state, role, artifact):
    data = artifact.read_bytes()
    checksum = hashlib.sha256(data).hexdigest()
    manifest = dict((name, digest) for digest, name in
                    (line.split() for line in artifact.with_name('checksums.txt').read_text().splitlines()))
    assert manifest[artifact.name] == checksum, 'unreviewed artifact'
    if role + '_code' not in state:
        key = None
        for _ in range(4):
            args = ['wasm', 'list-code', '--limit', '100', '--reverse']
            if key:
                args += ['--page-key', key]
            page = chain.query(*args)
            matched = [row for row in page['code_infos'] if row['data_hash'].lower() == checksum]
            if matched:
                state[role + '_code'] = int(matched[0]['code_id'])
                break
            key = page.get('pagination', {}).get('next_key')
            if not key:
                break
        else:
            raise RuntimeError('bounded code discovery exhausted')
        if role + '_code' not in state:
            target = '/tmp/' + state['run'] + '-' + artifact.name
            subprocess.run(['docker', 'cp', str(artifact), chain.CONTAINER + ':' + target],
                           check=True, capture_output=True)
            result = chain.tx(state['key'], 'wasm', 'store', target)
            state[role + '_code'] = int(chain.event_value(result, 'code_id'))
    actual = chain.query('wasm', 'code-info', str(state[role + '_code']))['checksum'].lower()
    assert actual == checksum
    state['artifacts'] = [row for row in state.get('artifacts', []) if row['role'] != role]
    state['artifacts'].append({'role': role, 'sha256': checksum, 'code_id': state[role + '_code']})
    save(state)
    return state[role + '_code']


def instantiate(state, role, code_id, message):
    if role not in state:
        state[role] = chain.instantiate(state, code_id, message, role)
        save(state)
    return state[role]


def once(state, label, contract, message, funds=None):
    if label not in state.setdefault('prepared', []):
        execute(state, contract, message, funds)
        state['prepared'].append(label)
        save(state)


def prepare():
    chain.INCLUSION_SECONDS = 60
    chain.attest()
    if STATE.exists():
        state = json.loads(STATE.read_text())
    else:
        origin = json.loads(Path('/tmp/dex-origin-1229-localterra-op-ref.json').read_text())
        state = {key: origin[key] for key in ['key', 'manager', 'cmm', 'ust1']}
        state.update(run='cohort1211-' + uuid.uuid4().hex[:12], chain_id='localterra')
        save(state)
    primary = Path(subprocess.check_output(['git', 'rev-parse', '--path-format=absolute', '--git-common-dir'],
                                          cwd=ROOT, text=True).strip()).parent
    dex = primary / 'smartcontracts/artifacts'
    cmm = primary.parent / 'ustr-cmm/contracts/artifacts'
    codes = {role: code(state, role, artifact) for role, artifact in {
        'factory': dex / 'cl8y_dex_factory.wasm', 'pair': dex / 'cl8y_dex_pair.wasm',
        'registry': dex / 'cl8y_dex_fee_discount.wasm', 'treasury': cmm / 'treasury.wasm',
        'mapper': cmm / 'wrap_mapper.wasm', 'cw20': dex / 'cw20_mintable.wasm',
    }.items()}
    instantiate(state, 'stable', codes['cw20'], {
        'name': 'Owned Stable Fixture', 'symbol': 'USDTEST', 'decimals': 6,
        'initial_balances': [{'address': state['manager'], 'amount': '500000000'}],
        'mint': None, 'marketing': None,
    })
    instantiate(state, 'registry', codes['registry'], {
        'governance': state['manager'], 'cl8y_token': state['ust1'],
    })
    once(state, 'tier0', state['registry'], {'add_tier': {
        'tier_id': 0, 'min_cl8y_balance': '0', 'discount_bps': 10000,
        'limit_discount_bps': 10000, 'governance_only': True,
    }})
    instantiate(state, 'factory', codes['factory'], {
        'governance': state['manager'], 'treasury': state['cmm'], 'default_fee_bps': 30,
        'pair_code_id': codes['pair'], 'lp_token_code_id': codes['cw20'],
        'whitelisted_code_ids': [codes['cw20']], 'default_limit_batch_max_rungs': 10,
        'pair_creation_fee_uluna': '0',
    })
    legacy_code = int(chain.query('wasm', 'contract', state['ust1'])['contract_info']['code_id'])
    if not chain.smart(state['factory'], {'is_code_id_whitelisted': {'code_id': legacy_code}})['whitelisted']:
        once(state, 'legacy_fixture_cw20_pin', state['factory'], {'add_whitelisted_code_id': {'code_id': legacy_code}})
    once(state, 'registry_pointer', state['factory'], {'update_config': {'discount_registry': state['registry']}})
    infos = [{'token': {'contract_addr': token}} for token in [state['ust1'], state['stable']]]
    once(state, 'create_pair', state['factory'], {'create_pair': {'asset_infos': infos}})
    pair_info = chain.smart(state['factory'], {'pair': {'asset_infos': infos}})['pair']
    state['pair'] = pair_info['contract_addr']
    state['pair_info'] = pair_info
    for index, token in enumerate([state['ust1'], state['stable']]):
        once(state, 'allowance' + str(index), token, {'increase_allowance': {'spender': state['pair'], 'amount': '100000000'}})
    once(state, 'provide', state['pair'], {'provide_liquidity': {
        'assets': [{'info': info, 'amount': '100000000'} for info in infos],
        'slippage_tolerance': None, 'receiver': None, 'deadline': None,
    }})
    instantiate(state, 'treasury', codes['treasury'], {'governance': state['manager']})
    instantiate(state, 'mapper', codes['mapper'], {
        'governance': state['manager'], 'treasury': state['treasury'],
        'fee_wrap_bps': 50, 'fee_unwrap_bps': 50,
    })
    instantiate(state, 'wrapped', codes['cw20'], {
        'name': 'Owned Wrapped Fixture', 'symbol': 'WRAPPED', 'decimals': 6,
        'initial_balances': [], 'mint': {'minter': state['mapper'], 'cap': None}, 'marketing': None,
    })
    once(state, 'denom_mapper', state['mapper'], {'set_denom_mapping': {'denom': 'uusd', 'cw20_addr': state['wrapped']}})
    once(state, 'treasury_mapper', state['treasury'], {'set_denom_wrapper': {'denom': 'uusd', 'wrapper': state['mapper']}})
    if 'native_fund' not in state.setdefault('prepared', []):
        chain.tx('test1', 'bank', 'send', 'test1', state['manager'], '100000000uusd')
        state['prepared'].append('native_fund')
    save(state)
    print(json.dumps({'status': 'prepared', 'chain_id': 'localterra', 'state': str(STATE)}))


def lcd_receipt(txhash):
    with urllib.request.urlopen('http://127.0.0.1:1317/cosmos/tx/v1beta1/txs/' + txhash, timeout=4) as response:
        data = response.read(2_000_001)
    assert len(data) <= 2_000_000
    receipt = json.loads(data)['tx_response']
    assert int(receipt['code']) == 0
    return receipt


def run():
    began = time.monotonic()
    chain.attest()
    state = json.loads(STATE.read_text())
    membership = chain.smart(state['registry'], {'get_registration': {'trader': state['manager']}})
    assert not membership['registered'], 'prepare a new owned registry before rerunning the chain path'
    hook = base64.b64encode(json.dumps({'swap': {
        'belief_price': None, 'max_spread': '0.05', 'min_return': '1', 'to': None,
        'deadline': None, 'trader': None, 'hybrid': None, 'greedy': None,
    }}).encode()).decode()
    swap = execute(state, state['ust1'], {'send': {'contract': state['pair'], 'amount': '1000000', 'msg': hook}})
    register = execute(state, state['registry'], {'register_wallet': {'wallet': state['manager'], 'tier_id': 0}})
    wrapped = execute(state, state['treasury'], {'wrap_deposit': {}}, '1000000uusd')
    rows = [lcd_receipt(result['txhash']) for result in [swap, register, wrapped]]
    output = {**state, 'txs': rows, 'elapsed_seconds': time.monotonic() - began,
              'utc': datetime.now(timezone.utc).isoformat(), 'pricing_scope': 'explicit owned QA stable/oracle fixtures only'}
    RECEIPTS.write_text(json.dumps(output, indent=2) + '\n')
    RECEIPTS.chmod(0o600)
    print(json.dumps({'status': 'chain_path_pass', 'elapsed_seconds': output['elapsed_seconds'], 'receipts': str(RECEIPTS)}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('prepare', 'run'))
    args = parser.parse_args()
    prepare() if args.mode == 'prepare' else run()
