#!/usr/bin/env python3
"""Offline evidence integrity checks for the #1428 synthetic chain harness."""
import contextlib
import importlib.util
import io
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "reverse_fixture", Path(__file__).with_name("localterra-reverse-1389.py"))
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class EvidenceTests(unittest.TestCase):
    def test_issue1428_cli_rejects_larger_envelope_before_node_access(self):
        for expectation in ("--expect-populated-gas-failure", "--expect-four-hop-greedy-rejection"):
            with self.subTest(expectation=expectation):
                argv = ["fixture", expectation, "--query-gas-limit", "16000000",
                        "--evidence-file", "/unused/synthetic.json"]
                error_output = io.StringIO()
                with patch.object(sys, "argv", argv), patch.object(fixture, "EvidenceLocalTerra") as chain:
                    with contextlib.redirect_stderr(error_output), self.assertRaises(SystemExit) as failure:
                        fixture.main()
                self.assertEqual(failure.exception.code, 2)
                self.assertIn("unchanged historical 15,000,000", error_output.getvalue())
                chain.assert_not_called()

    def test_real_provider_failure_is_saved_before_failed_probe(self):
        class FailedProvider:
            def smart(self, address, message):
                raise RuntimeError("raw provider out of gas; gasWanted: 15000000, gasUsed: 15003218")

        snapshots = []
        row = {"contract": "synthetic-router", "query": {"reverse": {"amount": "100"}},
               "smart_status": "PENDING"}
        quote = fixture.observe_smart_query(FailedProvider(), row["contract"], row["query"],
                                           row, lambda: snapshots.append(dict(row)))
        # A subsequent measurement failure cannot remove or replace this saved
        # provider error with the transaction's different meter/error.
        row["probe"] = {"query_status": "measurement_error", "error": "probe unavailable"}
        self.assertIsNone(quote)
        self.assertEqual(snapshots[0]["smart_status"], "FAIL")
        self.assertTrue(fixture.provider_gas_failure(snapshots[0]["error"], 15_000_000))
        self.assertNotIn("probe", snapshots[0])
        self.assertEqual(row["error"], snapshots[0]["error"])

    def test_provider_failure_requires_actual_historical_meter(self):
        self.assertTrue(fixture.provider_gas_failure(
            "Out of gas; gasWanted: 15000000, gasUsed: 15003218", 15_000_000))
        for error in (
            "Out of gas; gasWanted: 12000000, gasUsed: 12000123",
            "Out of gas; gasWanted: 150000001, gasUsed: 150000123",
            "Out of gas; gasWanted: 15000000",
            "REVERSE_QUERY_BUDGET_EXCEEDED",
        ):
            self.assertFalse(fixture.provider_gas_failure(error, 15_000_000), error)

    def test_measurement_keeps_failed_transaction_receipt(self):
        chain = fixture.EvidenceLocalTerra.__new__(fixture.EvidenceLocalTerra)
        chain.capture_probe = True
        chain.receipts = []
        receipt = {"txhash": "synthetic-test-hash", "height": "123", "code": 11,
                   "gas_used": "12000777", "raw_log": "transaction out of gas"}
        chain.query = lambda *parts: receipt
        self.assertEqual(chain.wait({"txhash": receipt["txhash"]}), receipt)
        self.assertEqual(chain.receipts, [receipt])

    def test_bootstrap_rejection_is_never_a_measurement_receipt(self):
        chain = fixture.EvidenceLocalTerra.__new__(fixture.EvidenceLocalTerra)
        chain.capture_probe = True
        chain.receipts = []
        with self.assertRaisesRegex(RuntimeError, "broadcast rejected"):
            chain.wait({"code": 5, "raw_log": "rejected"})
        self.assertEqual(chain.receipts, [])

    def test_full_raw_query_error_is_preserved(self):
        chain = fixture.EvidenceLocalTerra.__new__(fixture.EvidenceLocalTerra)
        chain.container = "synthetic-node"
        error = b"provider raw error " + b"x" * 2000
        result = subprocess.CompletedProcess([], 1, b"", error)
        with patch.object(fixture.subprocess, "run", return_value=result):
            with self.assertRaises(RuntimeError) as failure:
                chain.command("terrad", "query")
        self.assertEqual(str(failure.exception), error.decode())

    def test_node_envelope_cannot_be_raised_to_make_fixture_pass(self):
        chain = fixture.EvidenceLocalTerra.__new__(fixture.EvidenceLocalTerra)
        chain.command = lambda *parts: "[wasm]\nquery_gas_limit = 16000000\n"
        with self.assertRaisesRegex(AssertionError, "Expected existing 15000000"):
            fixture.node_metadata(chain, "/synthetic", 15_000_000)

    def test_node_image_and_sdk_are_attested_separately_from_gas_receipts(self):
        root = Path(__file__).resolve().parents[2]
        pinned = re.search(r"image:\s*(\S*/localterra-cl8y@sha256:[0-9a-f]+)",
                           (root / "docker-compose.yml").read_text()).group(1)
        chain = fixture.EvidenceLocalTerra.__new__(fixture.EvidenceLocalTerra)
        chain.container = "synthetic-node"
        chain.command = lambda *parts: (
            "[wasm]\nquery_gas_limit = 15000000\n" if parts[0] == "cat" else
            "cosmos_sdk_version: v0.53.0\nversion: synthetic\n")
        inspected = [{"Config": {"Image": pinned}, "Image": "sha256:synthetic"}]
        with patch.object(fixture.subprocess, "check_output",
                          return_value=json.dumps(inspected).encode()):
            metadata = fixture.node_metadata(chain, "/synthetic", 15_000_000)
        self.assertEqual(metadata["wasm_query_gas_limit"], 15_000_000)
        self.assertEqual(metadata["image_id"], "sha256:synthetic")
        self.assertEqual(metadata["terrad_version"]["cosmos_sdk_version"], "v0.53.0")
        self.assertEqual(len(metadata["terrad_version_dump_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
