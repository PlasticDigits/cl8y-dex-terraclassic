#!/usr/bin/env python3
"""LocalTerra harness must never resend an accepted or uncertain transaction."""
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "funding", Path(__file__).with_name("localterra-route-tax-1381.py"))
funding = importlib.util.module_from_spec(spec)
spec.loader.exec_module(funding)


class RetryTests(unittest.TestCase):
    def chain(self, responses):
        chain = funding.LocalTerra.__new__(funding.LocalTerra)
        chain.default_key = "synthetic-owner"
        chain.receipts = []
        chain.calls = []
        pending = iter(responses)

        def command(*parts, stdin=None):
            chain.calls.append(parts)
            return json.dumps(next(pending))

        chain.command = command
        return chain

    @patch.object(funding.time, "sleep")
    def test_explicit_checktx_sequence_rejection_retries_using_isolated_owner(self, sleep):
        chain = self.chain([
            {"code": 32, "raw_log": "account sequence mismatch, expected 42, got 41"},
            {"code": 0, "txhash": "synthetic"}])
        chain.query = lambda *parts: {"code": 0, "txhash": "synthetic", "height": "1", "gas_used": "1"}
        self.assertEqual(chain.tx("wasm", "execute")["txhash"], "synthetic")
        self.assertEqual(len(chain.calls), 2)
        self.assertIn("synthetic-owner", chain.calls[0])
        sleep.assert_called_once()

    def test_guard_owner_alias_selects_the_funded_signer(self):
        chain = self.chain([{"code": 0, "txhash": "synthetic"}])
        chain.owner_key = "guard-fixture-owner"
        chain.query = lambda *parts: {"code": 0, "txhash": "synthetic", "height": "1", "gas_used": "1"}
        chain.tx("wasm", "execute")
        self.assertEqual(chain.default_key, "guard-fixture-owner")
        self.assertIn("guard-fixture-owner", chain.calls[0])
        chain.tx_key = "reverse-fixture-owner"
        chain.command = lambda *parts, stdin=None: chain.calls.append(parts) or json.dumps({"code": 0, "txhash": "synthetic"})
        chain.tx("wasm", "execute")
        self.assertEqual(chain.owner_key, "reverse-fixture-owner")
        self.assertIn("reverse-fixture-owner", chain.calls[1])

    @patch.object(funding.time, "sleep")
    def test_non_sequence_code_with_sequence_text_is_not_retried(self, sleep):
        chain = self.chain([{"code": 5, "raw_log": "account sequence mismatch"}])
        with self.assertRaisesRegex(RuntimeError, "account sequence mismatch"):
            chain.tx("wasm", "execute")
        self.assertEqual(len(chain.calls), 1)
        sleep.assert_not_called()

    @patch.object(funding.time, "sleep")
    def test_accepted_transaction_with_unknown_inclusion_is_never_resent(self, sleep):
        chain = self.chain([{"code": 0, "txhash": "synthetic"}])

        def unavailable(*parts):
            raise RuntimeError("synthetic unavailable inclusion")

        chain.query = unavailable
        with self.assertRaises(TimeoutError):
            chain.tx("wasm", "execute")
        self.assertEqual(len(chain.calls), 1)

    @patch.object(funding.time, "sleep")
    def test_other_checktx_failure_is_not_retried(self, sleep):
        chain = self.chain([{"code": 5, "raw_log": "insufficient funds"}])
        with self.assertRaisesRegex(RuntimeError, "insufficient funds"):
            chain.tx("wasm", "execute")
        self.assertEqual(len(chain.calls), 1)
        sleep.assert_not_called()

    @patch.object(funding.time, "sleep")
    def test_sequence_rejection_retries_are_bounded(self, sleep):
        chain = self.chain([{"code": 32, "raw_log": "account sequence mismatch"}] * 8)
        with self.assertRaisesRegex(RuntimeError, "account sequence mismatch"):
            chain.tx("wasm", "execute")
        self.assertEqual(len(chain.calls), 8)
        self.assertEqual(sleep.call_count, 7)


if __name__ == "__main__":
    unittest.main()
