#!/usr/bin/env python3
"""Measure exact offline route fees on an explicitly owned LocalTerra fixture.

No production signing, no user provider calls, no shared chain reset. Receipts
omit generated actors, transaction hashes and raw logs. Current artifact input
hashes are checked against the build provenance before deployment.
"""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('fixture', Path(__file__).with_name('localterra-route-tax-1381.py'))
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class BudgetChain(fixture.LocalTerra):
    gas_budget = None

    def query(self, *parts):
        # SDK53's owned loopback read can transiently close before a response
        # during heavy fixture activity. Retry only this transport failure;
        # contract/query failures and all transaction submission remain strict.
        for attempt in range(3):
            try:
                return super().query(*parts)
            except RuntimeError as error:
                if 'post failed: Post "http://127.0.0.1:26657": EOF' not in str(error) or attempt == 2:
                    raise
                time.sleep(0.25 * (attempt + 1))

    def wait(self, response, expect_failure=False):
        if int(response.get('code', 0)) != 0:
            raise RuntimeError('Owned LocalTerra CheckTx rejected')
        txhash = response.get('txhash', '')
        assert re.fullmatch(r'[A-Fa-f0-9]{64}', txhash), 'Malformed owned inclusion identity'
        pending = getattr(self, 'pending_path', None)
        if pending:
            with os.fdopen(os.open(pending, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'w') as handle:
                handle.write(json.dumps({'txhash': txhash, 'expect_failure': expect_failure}))
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            # REST returns the decoded signed Tx and included response together;
            # polling never rebuilds or resends the accepted transaction.
            try:
                raw = self.command('curl', '--max-time', '3', '-s', 'http://127.0.0.1:1317/cosmos/tx/v1beta1/txs/' + txhash)
            except RuntimeError:
                time.sleep(0.5)
                continue
            try:
                value = json.loads(raw)
                included = value.get('tx_response')
                if not isinstance(included, dict) or included.get('txhash', '').upper() != txhash.upper():
                    time.sleep(0.5)
                    continue
            except (ValueError, AttributeError):
                time.sleep(0.5)
                continue
            receipt = dict(included, tx=value['tx'])
            assert (int(receipt.get('code', 0)) != 0) == expect_failure, 'Unexpected included settlement outcome'
            self.receipts.append({key: receipt[key] for key in ['height', 'code', 'gas_used']})
            if pending: pending.unlink(missing_ok=True)
            return receipt
        raise TimeoutError('Owned LocalTerra inclusion deadline; private pending identity retained, no resend')

    def tx(self, *parts, key=None, expect_failure=False):
        if self.gas_budget is None:
            return super().tx(*parts, key=key, expect_failure=expect_failure)
        response = json.loads(self.command('terrad', 'tx', *parts, '--from', key or self.default_key,
            '--keyring-backend', 'test', '--chain-id', 'localterra', '--gas', str(self.gas_budget),
            '--fees', str((self.gas_budget * 1133 + 39) // 40) + 'uluna', '--node', 'http://127.0.0.1:26657',
            '--broadcast-mode', 'sync', '-y', '--output', 'json'))
        receipt = self.wait(response, expect_failure)
        fee = receipt['tx']['auth_info']['fee']
        assert int(fee['gas_limit']) == self.gas_budget == int(receipt['gas_wanted'])
        assert fee['amount'] == [{'denom': 'uluna', 'amount': str((self.gas_budget * 1133 + 39) // 40)}]
        assert len(receipt['tx']['body']['messages']) == 1
        assert receipt['tx']['body']['messages'][0]['msg'] == json.loads(parts[3])
        return receipt


def validate_contract_inputs(recorded, current):
    declared = {name: digest for name, digest in recorded.items() if name.startswith('smartcontracts/')}
    expected = {name: digest for name, digest in current.items() if name.startswith('smartcontracts/')}
    assert expected and declared == expected, 'Complete contract/package/lock input manifest mismatch'
    return expected


def provenance(artifacts):
    value = json.loads((artifacts / 'build-provenance.json').read_text())
    root = Path(__file__).resolve().parents[2]
    assert value.get('source_directory') == 'smartcontracts', 'Wrong contract source directory'
    assert value.get('lockfile', {}).get('path') == 'smartcontracts/Cargo.lock', 'Wrong contract lockfile'
    helper_spec = importlib.util.spec_from_file_location('build_provenance', root / 'scripts/ci/build-provenance.py')
    helper = importlib.util.module_from_spec(helper_spec)
    helper_spec.loader.exec_module(helper)
    actual = helper.receipt(root, 'smartcontracts', 'smartcontracts/Cargo.lock',
        value['builder_image'], value['toolchain'], [])
    inputs = validate_contract_inputs(value.get('source_inputs', {}), actual['source_inputs'])
    assert value['lockfile'] == actual['lockfile'], 'Contract lockfile content mismatch'
    hashes = {}
    for name in ['cl8y_dex_factory', 'cl8y_dex_router', 'cl8y_dex_pair']:
        path = 'smartcontracts/artifacts/' + name + '.wasm'
        digest = hashlib.sha256((artifacts / (name + '.wasm')).read_bytes()).hexdigest()
        checksums = dict((line.split()[1], line.split()[0]) for line in (Path(__file__).resolve().parents[2] / 'smartcontracts/artifacts/checksums.txt').read_text().splitlines())
        assert digest == checksums[name + '.wasm'], 'Tracked artifact digest mismatch'
        hashes[name] = digest
    hashes['cw20_mintable'] = hashlib.sha256((artifacts / 'cw20_mintable.wasm').read_bytes()).hexdigest()
    return {'source_commit': value['source_commit'], 'contract_source_inputs_match_current': True, 'contract_input_count': len(inputs), 'contract_inputs_sha256': hashlib.sha256(json.dumps(inputs, sort_keys=True).encode()).hexdigest(), 'artifact_attestation': 'Current checked-in artifact checksums and complete current contract/package/lock input equivalence; original build recipe not re-attested; pair rebuilt after original receipt', 'sha256': hashes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--container', required=True)
    parser.add_argument('--artifacts', type=Path, required=True)
    parser.add_argument('--code-cache', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--measure', action='store_true', help='Use 12M for measurement; does not certify the offline budget')
    args = parser.parse_args()
    utc_start = datetime.now(timezone.utc).isoformat()
    owned = subprocess.run(['docker', 'inspect', '--format', '{{index .Config.Labels "cl8y.qa.owner"}}', args.container], capture_output=True, text=True, check=True).stdout.strip()
    assert owned == 'dex-1479-1480', 'Owned isolated fixture label required'
    artifacts = provenance(args.artifacts)
    budget_source = Path(__file__).resolve().parents[2] / 'frontend-dapp/src/services/terraclassic/offlineSwapBudget.ts'
    budget_body = budget_source.read_text()
    budget_values = {}
    for name in ['OFFLINE_ROUTER_BASE_GAS', 'OFFLINE_LEGACY_ROUTER_PER_HOP_GAS', 'OFFLINE_V1_ROUTER_PER_HOP_GAS']:
        match = re.search(r'export const ' + name + r' = ([0-9_]+)', budget_body)
        assert match, 'Offline budget source unavailable'
        budget_values[name] = int(match.group(1).replace('_',''))
    budget_source_sha256 = hashlib.sha256(budget_source.read_bytes()).hexdigest()
    for _ in range(60):
        ready = subprocess.run(['docker', 'exec', args.container, 'curl', '--max-time', '3', '-sf', 'http://127.0.0.1:26657/status'], capture_output=True, timeout=8)
        if ready.returncode == 0:
            break
        time.sleep(1)
    else:
        raise RuntimeError('Owned LocalTerra readiness deadline exceeded')
    chain = BudgetChain(args.container, args.code_cache)
    chain.pending_path = args.output.with_suffix('.pending.private.json')
    key, owner = chain.fresh_trader('1000000000000uluna')
    chain.default_key = key
    _, treasury = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + '.wasm')) for name in artifacts['sha256']}
    factory = chain.instantiate(codes['cl8y_dex_factory']['code_id'], {
        'governance': owner, 'treasury': treasury, 'default_fee_bps': 30,
        'pair_code_id': codes['cl8y_dex_pair']['code_id'], 'lp_token_code_id': codes['cw20_mintable']['code_id'],
        'whitelisted_code_ids': [codes['cw20_mintable']['code_id']], 'pair_creation_fee_uluna': '0'}, 'gas1480-factory')
    router = chain.instantiate(codes['cl8y_dex_router']['code_id'], {'factory': factory}, 'gas1480-router')
    tokens = []
    for i in range(5):
        tokens.append(chain.instantiate(codes['cw20_mintable']['code_id'], {
            'name': 'Offline Gas Fixture ' + str(i), 'symbol': 'GAS' + chr(65+i), 'decimals': 6,
            'initial_balances': [{'address': owner, 'amount': '1000000000000000'}], 'mint': {'minter': owner}}, 'gas1480-token' + str(i)))
    pairs = []
    for a, b in zip(tokens, tokens[1:]):
        infos = [fixture.asset(a), fixture.asset(b)]
        chain.execute(factory, {'create_pair': {'asset_infos': infos}})
        pair = chain.smart(factory, {'pair': {'asset_infos': infos}})['pair']['contract_addr']
        pairs.append(pair)
        for token in (a,b):
            chain.execute(token, {'increase_allowance': {'spender': pair, 'amount': '100000000000000'}})
        chain.execute(pair, {'provide_liquidity': {'assets': [{'info': fixture.asset(token), 'amount': '100000000000000'} for token in (a,b)], 'slippage_tolerance': None}})
    rows = []
    for mode in ['legacy', 'route_local_v1']:
        for hops in range(1, 5):
            for raw in [10000, 1000000, 1000000000]:
                path = tokens[:hops+1]
                expected = raw
                expected_hops = []
                for index, (a,b) in enumerate(zip(path,path[1:])):
                    pool = chain.smart(pairs[index], {'pool': {}})
                    reserve = {entry['info']['token']['contract_addr']: int(entry['amount']) for entry in pool['assets']}
                    gross = expected * reserve[b] // (reserve[a] + expected)
                    expected = gross - gross * 30 // 10000
                    expected_hops.append(expected)
                floor = max(1, (expected * 95 + 99) // 100)
                operations = fixture.operations(path)
                hop_floors = [max(1, (net * 95 + 99) // 100) for net in expected_hops]
                for op, hop_floor in zip(operations, hop_floors): op['terra_swap']['min_return'] = str(hop_floor)
                method = 'execute_swap_operations' if mode == 'legacy' else 'execute_route_swap_operations'
                body = {'operations': operations, 'max_spread': '0.05', 'minimum_receive': str(floor), 'to': owner, 'deadline': None}
                if mode == 'route_local_v1': body.update({'funding_version': 1, 'unwrap_output': False})
                hook = {method: body}
                message = {'send': {'contract': router, 'amount': str(raw), 'msg': base64.b64encode(json.dumps(hook).encode()).decode()}}
                budget = budget_values['OFFLINE_ROUTER_BASE_GAS'] + hops * budget_values['OFFLINE_V1_ROUTER_PER_HOP_GAS' if mode == 'route_local_v1' else 'OFFLINE_LEGACY_ROUTER_PER_HOP_GAS']
                chain.gas_budget = 12000000 if args.measure else budget
                def economics():
                    return {'pools': [chain.smart(pair, {'pool': {}}) for pair in pairs],
                        'wallet_tokens': [chain.balance(token, owner) for token in tokens]}
                original = economics()
                rejected_codes = []
                for poison in ['final', 'hop']:
                    bad_hook = json.loads(json.dumps(hook))
                    if poison == 'final': bad_hook[method]['minimum_receive'] = str(expected + 1)
                    else: bad_hook[method]['operations'][0]['terra_swap']['min_return'] = str(expected_hops[0] + 1)
                    bad_message = {'send': {'contract': router, 'amount': str(raw), 'msg': base64.b64encode(json.dumps(bad_hook).encode()).decode()}}
                    failed = chain.execute(path[0], bad_message, expect_failure=True)
                    assert int(failed.get('code',0)) != 0
                    assert economics() == original, 'Rejected floor changed wallet principal, reserves or LP shares'
                    rejected_codes.append(int(failed['code']))
                before = chain.balance(path[-1], owner)
                receipt = chain.execute(path[0], message)
                chain.gas_budget = None
                delivered = chain.balance(path[-1], owner) - before
                assert delivered == expected and delivered >= floor, {'raw':raw,'hops':hops,'mode':mode,'expected':expected,'delivered':delivered,'floor':floor}
                assert int(receipt['gas_used']) < int(receipt['gas_wanted'])
                if not args.measure:
                    assert budget >= (int(receipt['gas_used']) * 120 + 99) // 100, 'Measured 20% headroom not satisfied'
                rows.append({'family': mode, 'hops': hops, 'raw_offer': str(raw), 'expected_net': str(expected), 'delivered_net': str(delivered), 'floor': str(floor),
                    'offline_budget': budget, 'offline_budget_sufficient': int(receipt['gas_used']) < budget, 'requested_gas': chain.gas_budget or int(receipt['gas_wanted']), 'fee_uluna': (int(receipt['gas_wanted']) * 1133 + 39) // 40, 'signed_gas': int(receipt['tx']['auth_info']['fee']['gas_limit']), 'signed_fee': receipt['tx']['auth_info']['fee']['amount'], 'gas_used': int(receipt['gas_used']), 'included_code': int(receipt.get('code',0)),
                    'expected_hop_outputs': [str(value) for value in expected_hops], 'rejected_floor_codes': rejected_codes, 'failed_floors_wallet_reserves_lp_unchanged': True, 'execution_message': {'cw20_send_raw_amount': str(raw), 'method': method, 'funding_version': body.get('funding_version'), 'operation_count': len(operations), 'hybrid': None, 'greedy': None, 'per_hop_min_return': [str(value) for value in hop_floors], 'minimum_receive': str(floor), 'max_spread': '0.05', 'deadline': None, 'unwrap_output': body.get('unwrap_output')}, 'status': 'PASS'})
                print(json.dumps({key: rows[-1][key] for key in ['family','hops','raw_offer','gas_used','requested_gas','status']}), flush=True)
    budget_source_after_sha256 = hashlib.sha256(budget_source.read_bytes()).hexdigest()
    stable = budget_source_after_sha256 == budget_source_sha256
    result = {'utc_start': utc_start, 'offline_budget_source_sha256': budget_source_sha256, 'offline_budget_source_after_sha256': budget_source_after_sha256, 'stable_budget_source': stable, 'offline_budget_values': budget_values, 'issues': [1479,1480], 'utc': datetime.now(timezone.utc).isoformat(), 'scope': 'Owned LocalTerra CW20 hook-free empty-book pool routes; exact requested gas honored; independent net integer arithmetic',
        'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), 'artifacts': artifacts, 'overall': 'PASS' if stable else 'FAIL', 'measurement_only': args.measure, 'cases': rows,
        'skips': ['native wrap/unwrap', 'tax/hook/populated-book bounded worst case', 'physical extension/WalletConnect', 'original reporter signed transactions', 'production settlement']}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    assert stable, 'Budget source changed during funded run; executed cases retained in receipt'


if __name__ == '__main__':
    main()
