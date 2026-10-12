#!/usr/bin/env python3
"""Offline receipt boundary regressions with synthetic included transaction data."""
import importlib.util
import json
import unittest
from unittest.mock import patch
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('gas_fixture', Path(__file__).with_name('localterra-offline-gas-1480.py'))
DRIVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRIVER)


class IncludedReceipt(DRIVER.BudgetChain):
    def __init__(self):
        self.gas_budget = 3200000
        self.default_key = 'synthetic'
        self.message = {'send': {'amount': '10000', 'contract': 'fixture-router', 'msg': 'fixture-hook'}}
        self.receipt = {'code': 0, 'gas_wanted': '3200000', 'gas_used': '2639000',
            'tx': {'auth_info': {'fee': {'gas_limit': '3200000', 'amount': [{'denom': 'uluna', 'amount': '90640000'}]}},
                   'body': {'messages': [{'msg': self.message}]}}}

    def command(self, *parts, **kwargs):
        self.request = parts
        return json.dumps({'code': 0, 'txhash': 'synthetic-inclusion'})

    def wait(self, response, expect_failure=False):
        return self.receipt

    def submit(self):
        return self.tx('wasm', 'execute', 'fixture-token', json.dumps(self.message))


class SignedFeeTests(unittest.TestCase):
    def test_actual_fee_is_decoded_and_preserves_requested_execution(self):
        chain = IncludedReceipt()
        self.assertIs(chain.submit(), chain.receipt)
        request = chain.request
        self.assertEqual(request[request.index('--gas') + 1], '3200000')
        self.assertEqual(request[request.index('--fees') + 1], '90640000uluna')

    def test_signed_gas_or_fee_rewrites_and_altered_message_fail(self):
        mutations = [
            lambda receipt: receipt['tx']['auth_info']['fee'].update(gas_limit='3199999'),
            lambda receipt: receipt.update(gas_wanted='3199999'),
            lambda receipt: receipt['tx']['auth_info']['fee'].update(amount=[{'denom': 'uusd', 'amount': '90640000'}]),
            lambda receipt: receipt['tx']['auth_info']['fee'].update(amount=[{'denom': 'uluna', 'amount': '90639999'}]),
            lambda receipt: receipt['tx']['body'].update(messages=[{'msg': {'swap': {}}}]),
        ]
        for mutation in mutations:
            with self.subTest(mutation=mutations.index(mutation)):
                chain = IncludedReceipt()
                mutation(chain.receipt)
                with self.assertRaises(AssertionError): chain.submit()



class IncludedReadTests(unittest.TestCase):
    def test_delayed_original_receipt_is_reconciled_without_resend(self):
        chain = object.__new__(DRIVER.BudgetChain)
        chain.receipts = []
        txhash = 'a' * 64
        included = {'txhash': txhash, 'height': '1', 'code': 0, 'gas_used': '20'}
        with patch.object(chain, 'command', side_effect=[json.dumps({'code': 5}), json.dumps({'tx': {}, 'tx_response': included})]) as command, patch.object(DRIVER.time, 'sleep'):
            self.assertEqual(chain.wait({'code': 0, 'txhash': txhash})['tx'], {})
            self.assertEqual(command.call_count, 2)
            self.assertTrue(all(call.args[0] == 'curl' for call in command.call_args_list))

    def test_unknown_inclusion_is_terminal_and_never_resends(self):
        chain = object.__new__(DRIVER.BudgetChain)
        chain.receipts = []
        with patch.object(chain, 'command', return_value='{}') as command, patch.object(DRIVER.time, 'monotonic', side_effect=[0, 1, 121]), patch.object(DRIVER.time, 'sleep'):
            with self.assertRaises(TimeoutError): chain.wait({'code': 0, 'txhash': 'a' * 64})
            self.assertEqual(command.call_count, 1)
            self.assertEqual(command.call_args.args[0], 'curl')


class ContractManifestTests(unittest.TestCase):
    def test_complete_contract_package_lock_inputs_match(self):
        current = {'smartcontracts/Cargo.lock': 'lock', 'smartcontracts/contracts/pair/src/lib.rs': 'source',
                   'scripts/ci/build-provenance.py': 'current-recipe'}
        original = dict(current, **{'scripts/ci/build-provenance.py': 'original-recipe'})
        self.assertEqual(DRIVER.validate_contract_inputs(original, current),
            {name: value for name, value in current.items() if name.startswith('smartcontracts/')})

    def test_empty_partial_changed_and_unexpected_contract_manifests_fail(self):
        current = {'smartcontracts/Cargo.lock': 'lock', 'smartcontracts/contracts/pair/src/lib.rs': 'source'}
        invalid = [{}, {'smartcontracts/Cargo.lock': 'lock'}, dict(current, **{'smartcontracts/Cargo.lock': 'wrong'}),
                   dict(current, **{'smartcontracts/packages/unexpected.rs': 'extra'})]
        for declared in invalid:
            with self.subTest(declared=declared):
                with self.assertRaises(AssertionError): DRIVER.validate_contract_inputs(declared, current)
        with self.assertRaises(AssertionError): DRIVER.validate_contract_inputs({}, {})

if __name__ == '__main__':
    unittest.main()
