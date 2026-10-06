#!/usr/bin/env python3
"""Finite native-wrap/funded-tax smoke; callable by the custody release harness."""
import argparse
import base64
from decimal import Decimal, ROUND_CEILING
import hashlib
import json
from pathlib import Path
import sys
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT/'scripts/lib'))
from route_tax_rollout import Rollout, require, write_json
from route_tax_rollout_cli import TerradChain, parser as transport_parser

CLUNC = 'terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg'
UST1 = 'terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72'
ALPHA = 'terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz'
PAIR1 = 'terra1su5363453fj326u4t0kqar30f35cm3n0dc9yksg379u6875z350s4mm7h4'
PAIR2 = 'terra1rmdtckz5gd0usja36ydwat6prnmew639ry37yq72xh9ek4s3m83sehn5u6'
NATIVE_INPUT = 1_000_000
GAS_RESERVE = 150_000_000
MAX_TOTAL_FEE = 160_000_000
FUND_GAS = 400_000


def raw_transaction(chain, messages, gas, folder, name, key):
    """Checkpoint exact signed bytes before broadcast; uncertain outcomes never resend."""
    folder.mkdir(parents=True, exist_ok=True, mode=0o700)
    pending = folder/(name+'-pending.json')
    if pending.exists():
        saved = json.loads(pending.read_text())
        require(saved['messages'] == messages, 'Smoke action changed after signing')
        validate_pending(chain, saved, messages, gas)
        receipt = chain.receipt(saved['tx_hash'])
        require(receipt is not None, 'Smoke inclusion unknown; reconcile saved hash instead of resending')
        Rollout._receipt_ok(receipt)
        return receipt, saved['fee_uluna']
    chain.unlock()
    require(chain.network() == 'columbus-5', 'Smoke refuses another chain')
    account, sequence = chain._account(chain.args.store_address)
    fee = int((Decimal(chain.args.gas_prices.removesuffix('uluna'))*gas).to_integral_value(rounding=ROUND_CEILING))
    require(0 < fee <= MAX_TOTAL_FEE, 'Smoke fee exceeds its finite bound')
    unsigned = {'body': {'messages': messages, 'memo': '', 'timeout_height': '0',
                         'extension_options': [], 'non_critical_extension_options': []},
                'auth_info': {'signer_infos': [], 'fee': {'amount': [{'denom': 'uluna', 'amount': str(fee)}],
                              'gas_limit': str(gas), 'payer': '', 'granter': ''}, 'tip': None}, 'signatures': []}
    unsigned_path, signed_path = folder/(name+'-unsigned.json'), folder/(name+'-signed.json')
    write_json(unsigned_path, unsigned)
    signed = json.loads(chain.command(['tx', 'sign', str(unsigned_path), '--from', key,
        '--offline', '--account-number', account, '--sequence', sequence, *chain.flags(), '--output', 'json']))
    chain._signed_proof(unsigned, signed, key, sequence, False)
    write_json(signed_path, signed)
    tx_hash = chain._encoded_hash(signed_path)
    write_json(pending, {'messages': messages, 'signed_path': str(signed_path), 'tx_hash': tx_hash, 'fee_uluna': fee})
    receipt = chain.rebroadcast(signed_path, tx_hash)
    write_json(folder/(name+'-receipt.json'), receipt)
    return receipt, fee


def validate_pending(chain, saved, messages, gas):
    signed_path = Path(saved['signed_path'])
    signed = json.loads(signed_path.read_text())
    require(signed['body']['messages'] == messages, 'Smoke signed messages differ from checkpointed action')
    require(chain._encoded_hash(signed_path) == saved['tx_hash'], 'Smoke signed bytes changed')
    fee = signed['auth_info']['fee']
    require(fee['amount'] == [{'denom': 'uluna', 'amount': str(saved['fee_uluna'])}]
            and int(fee['gas_limit']) == gas and not fee.get('payer') and not fee.get('granter'),
            'Smoke saved fee/gas/payer changed')
    require(0 < saved['fee_uluna'] <= MAX_TOTAL_FEE, 'Smoke saved fee exceeds bound')


def finish_smoke(donor, config, evidence, receipt, folder):
    """Reconcile the included original transaction without quoting or signing again."""
    wrapped = evidence['wrapped_offer_amount']
    before, trader, router = evidence['before'], evidence['synthetic_trader'], config['router']
    minimum = evidence['signed_minimum_receive']
    fund = json.loads((folder/'fund-pending.json').read_text())
    swap = json.loads((folder/'swap-pending.json').read_text())
    funding = [{'@type': '/cosmos.bank.v1beta1.MsgSend', 'from_address': donor.args.store_address, 'to_address': trader,
                'amount': [{'denom': 'uluna', 'amount': str(GAS_RESERVE+NATIVE_INPUT)}]}]
    validate_pending(donor, fund, funding, FUND_GAS)
    validate_pending(donor, swap, evidence['swap_messages'], 5_000_000)
    require(receipt['txhash'] == swap['tx_hash'], 'Smoke receipt differs from original signed hash')
    Rollout._receipt_ok(receipt)
    fund_receipt = donor.receipt(fund['tx_hash'])
    require(fund_receipt is not None, 'Donor funding receipt unavailable')
    Rollout._receipt_ok(fund_receipt)
    require(fund['fee_uluna']+swap['fee_uluna'] <= MAX_TOTAL_FEE, 'Smoke cumulative fee cap exceeded')
    raw = donor.get('/cosmos/tx/v1beta1/txs/'+receipt['txhash'])['tx_response']
    events = raw.get('events') or [e for log in raw.get('logs', []) for e in log['events']]
    wasm = [dict((a['key'], a['value']) for a in e['attributes']) for e in events if e['type']=='wasm']
    swaps = [next(e for e in wasm if e.get('_contract_address')==p and e.get('action')=='swap') for p in (PAIR1, PAIR2)]
    require(int(swaps[0]['offer_amount']) == wrapped, 'First pair credit differs from authorized wrapped budget')
    require(swaps[1]['offer_amount'] == swaps[0]['net_return_amount'], 'Intermediate pair credit differs from preceding net output')
    taxes = [e for e in wasm if e.get('_contract_address')==ALPHA and e.get('tax_kind')=='buy' and e.get('from')==PAIR2]
    require(len(taxes)==1, 'Expected exactly one final buy-tax boundary')
    tax = taxes[0]
    require(tax.get('hop_trader') == trader, 'Final tax boundary lost the authenticated original trader')
    # Pair net_return_amount is after pair hooks, before CW20 route Buy tax.
    declared = int(swaps[1]['net_return_amount'])
    bps = donor.smart(ALPHA, {'get_config': {}})['buy_bps']
    require(int(tax['declared'])==declared and int(tax['tax'])==declared*bps//10000, 'Actual final buy tax mismatch')
    require(int(tax['amount'])+int(tax['tax'])==declared, 'Final gross/net/sink parity failed')
    require(tax.get('sink') == config['cmm'] and int(tax.get('sink_amount', -1)) == int(tax['tax']),
            'Actual configured treasury tax sink differs from its receipt')
    after = snapshot(donor, trader, router, config['cmm'])
    require(after[ALPHA]['trader']==int(tax['amount']) and after[ALPHA]['trader']>=minimum, 'Final recipient credit differs from taxed net floor')
    require(after[CLUNC]['trader']==0 and after[UST1]['trader']==0, 'Unexpected route residue; reconcile receipt')
    require(all(before[t]['router']==after[t]['router'] for t in (CLUNC, UST1, ALPHA)), 'Router dust changed; reconcile concurrent state/receipt')
    evidence.update(status='PASS', after=after, pair_swap_events=swaps, buy_tax_event=tax,
                    receipts=[fund_receipt, receipt], gas_used=receipt['gas_used'],
                    total_fee_uluna=fund['fee_uluna']+swap['fee_uluna'],
                    note='Native input wrapped and routed in one transaction; all intermediate trader assets started at zero.')
    evidence.pop('error', None)
    evidence.pop('containment_required', None)
    write_json(folder/'evidence.json', evidence)
    return evidence


def run_smoke(donor, state_dir):
    """The caller can reuse its already-unlocked cl8ydeploy in-memory adapter."""
    state_dir = Path(state_dir)
    journal = json.loads((state_dir/'journal.json').read_text())
    require(journal['status'] == 'complete', 'Restore the reviewed rollout pauses before live smoke')
    config = journal['identity']['config']
    require(donor.args.store_address == 'terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv', 'Smoke donor key mismatch')
    require(donor.network() == config['chain_id'] == 'columbus-5', 'Smoke chain mismatch')
    folder = state_dir/'live-smoke'
    folder.mkdir(mode=0o700, exist_ok=True)
    public = folder/'evidence.json'
    if public.exists():
        prior = json.loads(public.read_text())
        require(prior['manifest_identity'] == journal['identity_hash'], 'Smoke evidence belongs to another release manifest')
        if prior.get('status') == 'PASS':
            saved = json.loads((folder/'swap-pending.json').read_text())
            receipt = donor.receipt(saved['tx_hash'])
            require(receipt is not None, 'Previously included smoke receipt is unavailable')
            return finish_smoke(donor, config, prior, receipt, folder)
    if (folder/'swap-pending.json').exists():
        saved = json.loads((folder/'swap-pending.json').read_text())
        receipt = donor.receipt(saved['tx_hash'])
        require(receipt is not None, f"Smoke inclusion unknown for {saved['tx_hash']}; preserve original evidence and do not re-sign")
        require(public.exists(), 'Original smoke evidence is missing')
        return finish_smoke(donor, config, json.loads(public.read_text()), receipt, folder)
    evidence = {'status': 'RUNNING', 'manifest_identity': journal['identity_hash'],
                'native_gross_uluna': NATIVE_INPUT, 'gas_reserve_uluna': GAS_RESERVE,
                'max_total_fee_uluna': MAX_TOTAL_FEE, 'donor_address_sha256': hashlib.sha256(donor.args.store_address.encode()).hexdigest(),
                'receipts': [], 'limitations': ['One two-hop pool path with native wrap; no populated greedy or sell-tax route.']}
    write_json(public, evidence)
    try:
        router = config['router']
        mapper = journal['inventory']['router_config']['wrap_mapper']
        wrapper_config = donor.smart(mapper, {'config': {}})
        mapping = donor.smart(mapper, {'denom_mapping': {'denom': 'uluna'}})
        require(mapping['cw20_addr'] == CLUNC and not wrapper_config['paused'], 'Live native wrapper changed/unavailable')
        wrap_bps = wrapper_config.get('fee_wrap_bps', wrapper_config.get('fee_bps'))
        require(isinstance(wrap_bps, int) and 0 <= wrap_bps < 10000, 'Invalid actual wrap fee')
        wrapped = NATIVE_INPUT - NATIVE_INPUT*wrap_bps//10000
        require(donor.smart(CLUNC, {'token_info': {}})['decimals'] == 6, 'Native wrapper amount scale changed')
        for pair, a, b in [(PAIR1, CLUNC, UST1), (PAIR2, UST1, ALPHA)]:
            actual = donor.smart(config['factory'], {'pair': {'asset_infos': [asset(a), asset(b)]}})['pair']['contract_addr']
            require(actual == pair and not donor.smart(pair, {'is_paused': {}})['paused'], 'Smoke pair identity/pause mismatch')
        synthetic_home = folder/'synthetic-home'
        synthetic_home.mkdir(mode=0o700, exist_ok=True)
        key_file = folder/'synthetic-public-key.json'
        key = 'release-20261006-smoke'
        args = transport_parser().parse_args(['--state-dir', str(folder), '--home', str(synthetic_home),
            '--keyring-backend', 'test', '--node', donor.args.node, '--lcd', donor.args.lcd,
            '--store-key', key, '--gas-prices', donor.args.gas_prices])
        synthetic = TerradChain(args)
        if key_file.exists():
            record = json.loads(key_file.read_text())
        else:
            record = json.loads(synthetic.command(['keys', 'add', key, '--no-backup', '--home', str(synthetic_home),
                '--keyring-backend', 'test', '--output', 'json']))
            require('mnemonic' not in record, 'Synthetic key output includes signing material')
            write_json(key_file, record)
        trader = record['address']
        synthetic.args.store_address = trader
        synthetic.public_keys[key] = json.loads(record['pubkey']) if isinstance(record['pubkey'], str) else record['pubkey']
        synthetic.roles = {'store': key}
        synthetic.unlocked = True
        evidence['synthetic_trader'] = trader
        evidence['wrapper_config'] = wrapper_config
        evidence['wrapped_offer_amount'] = wrapped
        operations, amounts, gross = [], [], wrapped
        for offer, ask, pair in [(CLUNC, UST1, PAIR1), (UST1, ALPHA, PAIR2)]:
            sim = donor.smart(pair, {'hybrid_simulation': {'offer_asset': {'info': asset(offer), 'amount': str(gross)},
                'hybrid': {'book_input': '0', 'pool_input': str(gross), 'max_maker_fills': 1, 'book_start_hint': None},
                'trader': trader, 'sender': trader}})
            pair_net = int(sim['return_amount'])
            require(pair_net > 0, 'Smoke quote is dust; no weaker floor permitted')
            floor = max(1, pair_net*99//100)
            operations.append({'terra_swap': {'offer_asset_info': asset(offer), 'ask_asset_info': asset(ask), 'min_return': str(floor)}})
            amounts.append({'pair': pair, 'offer_amount': str(gross), 'net_return_amount': str(pair_net), 'min_return': str(floor)})
            if ask == ALPHA:
                preview = donor.smart(ALPHA, {'route_tax_preview': {'kind': 'buy', 'amount': str(pair_net), 'trader': trader, 'pair': pair}})
                gross = int(preview['net'])
                evidence['buy_preview'] = preview
            else:
                gross = pair_net
        forward = donor.smart(router, {'simulate_swap_operations': {'offer_amount': str(wrapped), 'operations': operations,
            'trader': trader, 'sender': trader}})
        require(int(forward['amount']) == gross and gross > 0, 'Independent funded quote parity failed')
        reverse = donor.smart(router, {'reverse_simulate_swap_operations': {'ask_amount': str(gross), 'operations': operations, 'trader': trader}})
        fresh = donor.smart(router, {'simulate_swap_operations': {'offer_amount': reverse['amount'], 'operations': operations, 'trader': trader}})
        require(int(fresh['amount']) >= gross, 'Fresh reverse forward sufficiency failed')
        evidence.update(operations=operations, hop_quotes=amounts, forward=forward, reverse=reverse, fresh_forward=fresh)
        before = snapshot(donor, trader, router, config['cmm'])
        require(all(before[t]['trader'] == 0 for t in (CLUNC, UST1, ALPHA)), 'Synthetic trader has existing route assets; refuse replay')
        evidence['before'] = before
        write_json(public, evidence)
        donor.unlock()
        funding = [{'@type': '/cosmos.bank.v1beta1.MsgSend', 'from_address': donor.args.store_address, 'to_address': trader,
                    'amount': [{'denom': 'uluna', 'amount': str(GAS_RESERVE+NATIVE_INPUT)}]}]
        receipt, fee1 = raw_transaction(donor, funding, FUND_GAS, folder, 'fund', donor.args.store_key)
        evidence['receipts'].append(receipt)
        minimum = max(1, gross*99//100)
        hook = {'execute_route_swap_operations': {'funding_version': 1, 'operations': operations, 'max_spread': '0.01',
                'minimum_receive': str(minimum), 'to': trader, 'deadline': int(time.time())+180, 'unwrap_output': False}}
        messages = [{'@type': '/cosmwasm.wasm.v1.MsgExecuteContract', 'sender': trader,
                     'contract': wrapper_config['treasury'], 'msg': {'wrap_deposit': {}},
                     'funds': [{'denom': 'uluna', 'amount': str(NATIVE_INPUT)}]},
                    {'@type': '/cosmwasm.wasm.v1.MsgExecuteContract', 'sender': trader, 'contract': CLUNC,
                     'msg': {'send': {'contract': router, 'amount': str(wrapped),
                        'msg': base64.b64encode(json.dumps(hook).encode()).decode()}}, 'funds': []}]
        evidence['signed_minimum_receive'] = minimum
        evidence['swap_messages'] = messages
        write_json(public, evidence)
        proposed_swap_fee = int((Decimal(synthetic.args.gas_prices.removesuffix('uluna'))*5_000_000)
                                .to_integral_value(rounding=ROUND_CEILING))
        require(fee1+proposed_swap_fee <= MAX_TOTAL_FEE, 'Smoke cumulative fee bound would be exceeded before signing')
        receipt, fee2 = raw_transaction(synthetic, messages, 5_000_000, folder, 'swap', key)
        require(fee1+fee2 <= MAX_TOTAL_FEE, 'Smoke cumulative fee cap exceeded')
        evidence['receipts'].append(receipt)
        return finish_smoke(donor, config, evidence, receipt, folder)
    except Exception as error:
        evidence.update(status='FAIL', error=str(error), containment_required=True)
        write_json(public, evidence)
        raise


def asset(token):
    return {'token': {'contract_addr': token}}


def snapshot(chain, trader, router, sink):
    return {t: {'trader': int(chain.smart(t, {'balance': {'address': trader}})['balance']),
                'router': int(chain.smart(t, {'balance': {'address': router}})['balance']),
                'sink': int(chain.smart(t, {'balance': {'address': sink}})['balance']),
                'supply': chain.smart(t, {'token_info': {}})['total_supply']}
            for t in (CLUNC, UST1, ALPHA)}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--state-dir', type=Path, default=Path.home()/'.local/state/cl8y-dex/release-20261006-route')
    args = p.parse_args()
    journal = json.loads((args.state_dir/'journal.json').read_text())
    tr = journal['identity']['config']['transport']
    adapter = TerradChain(transport_parser().parse_args(['--state-dir', str(args.state_dir), '--home', tr['home'],
        '--keyring-backend', tr['keyring_backend'], '--node', tr['node'], '--lcd', tr['lcd']]))
    result = run_smoke(adapter, args.state_dir)
    print(json.dumps({'status': result['status'], 'evidence': str(args.state_dir/'live-smoke/evidence.json')}, indent=2))


if __name__=='__main__':
    main()
