#!/usr/bin/env python3
"""No-broadcast accounting and durable-resume checks for the custody smoke."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

spec = importlib.util.spec_from_file_location('smoke', Path(__file__).with_name('route-live-smoke.py'))
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class Chain:
    def __init__(self):
        self.args = SimpleNamespace(store_address='terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv')
        self.receipts = {}
        self.trader, self.router, self.sink = 'synthetic', 'router', 'treasury'
        self.events = []
        self.final_amount = 764
        self.tax_declared = 800

    def _encoded_hash(self, path):
        return hashlib.sha256(Path(path).read_bytes()).hexdigest().upper()

    def receipt(self, tx_hash):
        return self.receipts.get(tx_hash)

    def network(self):
        return 'columbus-5'

    def unlock(self):
        raise AssertionError('Resume must not prompt, unlock, sign or broadcast')

    def command(self, *args):
        raise AssertionError('Fake transport never signs or broadcasts')

    def get(self, path):
        attrs = [
            {'_contract_address': smoke.PAIR1, 'action': 'swap', 'offer_amount': '980000', 'net_return_amount': '1000'},
            {'_contract_address': smoke.PAIR2, 'action': 'swap', 'offer_amount': '1000', 'return_amount': '1000',
             'hook_fee_amount': '200', 'net_return_amount': '800'},
            {'_contract_address': smoke.ALPHA, 'action': 'transfer', 'from': smoke.PAIR2, 'tax_kind': 'buy',
             'declared': str(self.tax_declared), 'amount': '764', 'tax': '36', 'hop_trader': self.trader,
             'sink': self.sink, 'sink_amount': '36'},
        ]
        return {'tx_response': {'events': [{'type': 'wasm', 'attributes': [{'key': k, 'value': v} for k, v in a.items()]} for a in attrs]}}

    def smart(self, token, query):
        if query == {'get_config': {}}:
            return {'buy_bps': 450}
        if query == {'token_info': {}}:
            return {'total_supply': '100000'}
        address = query['balance']['address']
        value = self.final_amount if token == smoke.ALPHA and address == self.trader else 36 if token == smoke.ALPHA and address == self.sink else 0
        return {'balance': str(value)}


class SmokeTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.state_dir = Path(self.temp.name)
        self.folder = self.state_dir/'live-smoke'
        self.folder.mkdir()
        self.chain = Chain()
        self.config = {'router': self.chain.router, 'cmm': self.chain.sink}
        self.messages = [{'test_original_atomic_wrap_and_send': True}]
        funding = [{'@type': '/cosmos.bank.v1beta1.MsgSend', 'from_address': self.chain.args.store_address,
                    'to_address': self.chain.trader, 'amount': [{'denom': 'uluna', 'amount': '151000000'}]}]
        for name, messages, gas, fee in [('fund', funding, 400000, 11330000), ('swap', self.messages, 5000000, 141625000)]:
            signed = self.folder/(name+'-signed.json')
            smoke.write_json(signed, {'body': {'messages': messages}, 'auth_info': {'fee': {
                'amount': [{'denom': 'uluna', 'amount': str(fee)}], 'gas_limit': str(gas), 'payer': '', 'granter': ''}}})
            hash_value = self.chain._encoded_hash(signed)
            smoke.write_json(self.folder/(name+'-pending.json'), {'messages': messages,
                'signed_path': str(signed), 'tx_hash': hash_value, 'fee_uluna': fee})
            self.chain.receipts[hash_value] = {'txhash': hash_value, 'code': 0, 'gas_used': '1234567'}
        self.receipt = self.chain.receipts[json.loads((self.folder/'swap-pending.json').read_text())['tx_hash']]
        self.evidence = {'synthetic_trader': self.chain.trader, 'wrapped_offer_amount': 980000,
            'signed_minimum_receive': 756, 'swap_messages': self.messages,
            'before': {token: {'trader': 0, 'router': 0, 'sink': 0, 'supply': '100000'}
                       for token in [smoke.CLUNC, smoke.UST1, smoke.ALPHA]}, 'status': 'RUNNING', 'manifest_identity': 'proof'}
        smoke.write_json(self.state_dir/'journal.json', {'status': 'complete', 'identity_hash': 'proof',
            'identity': {'config': {**self.config, 'chain_id': 'columbus-5'}}})

    def test_after_hook_amount_is_buy_tax_gross_and_resume_never_signs(self):
        result = smoke.finish_smoke(self.chain, self.config, self.evidence, self.receipt, self.folder)
        self.assertEqual('PASS', result['status'])
        self.assertEqual(764, result['after'][smoke.ALPHA]['trader'])
        self.assertEqual(152955000, result['total_fee_uluna'])

    def test_pre_hook_tax_declaration_fails(self):
        self.chain.tax_declared = 1000
        with self.assertRaisesRegex(RuntimeError, 'buy tax mismatch'):
            smoke.finish_smoke(self.chain, self.config, self.evidence, self.receipt, self.folder)

    def test_changed_signed_message_rejected_before_receipt_reuse(self):
        saved = json.loads((self.folder/'swap-pending.json').read_text())
        signed_path = Path(saved['signed_path'])
        signed = json.loads(signed_path.read_text())
        signed['body']['messages'] = [{'changed_spend': True}]
        smoke.write_json(signed_path, signed)
        with self.assertRaisesRegex(RuntimeError, 'signed messages differ'):
            smoke.validate_pending(self.chain, saved, self.messages, 5000000)

    def test_pending_donor_funding_reconciles_without_unlock_or_repeat(self):
        saved = json.loads((self.folder/'fund-pending.json').read_text())
        receipt, fee = smoke.raw_transaction(self.chain, saved['messages'], 400000, self.folder, 'fund', 'cl8ydeploy')
        self.assertEqual(saved['tx_hash'], receipt['txhash'])
        self.assertEqual(11330000, fee)

    def test_unknown_donor_inclusion_never_resends(self):
        saved = json.loads((self.folder/'fund-pending.json').read_text())
        self.chain.receipts.pop(saved['tx_hash'])
        with self.assertRaisesRegex(RuntimeError, 'inclusion unknown'):
            smoke.raw_transaction(self.chain, saved['messages'], 400000, self.folder, 'fund', 'cl8ydeploy')

    def test_included_swap_resume_reconciles_original_evidence_without_requote(self):
        smoke.write_json(self.folder/'evidence.json', self.evidence)
        result = smoke.run_smoke(self.chain, self.state_dir)
        self.assertEqual('PASS', result['status'])
        self.assertEqual(self.receipt['txhash'], result['receipts'][1]['txhash'])

    def test_unknown_swap_inclusion_never_requotes_or_resigns(self):
        smoke.write_json(self.folder/'evidence.json', self.evidence)
        self.chain.receipts.pop(self.receipt['txhash'])
        with self.assertRaisesRegex(RuntimeError, 'inclusion unknown'):
            smoke.run_smoke(self.chain, self.state_dir)


if __name__ == '__main__':
    unittest.main(verbosity=2)
