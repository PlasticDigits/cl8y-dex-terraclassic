#!/usr/bin/env python3
"""Owned mocked transport tests for the read-only wallet preparation helper."""
import contextlib
import importlib.util
import io
import json
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

SPEC = importlib.util.spec_from_file_location("wallet_preparation", Path(__file__).with_name("prepare-wallet-attribution-1483.py"))
PREP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PREP)
SHA = "a" * 40


def quote(native):
    tokens = PREP.assets()
    offer = tokens["LUNC" if native else "cLUNC"]["identity"]
    meta = {"input_amount": "1000000" if native else "980000", "generation": "b" * 64,
            "observed_at_ms": 1000, "expires_at_ms": 21000,
            "protection_policy": "quote_relative_v1", "max_spread": "0.05",
            "minimum_receive": "8550", "execution_market_spread": "0.157894736842105264",
            "hops": [{"offer_token": tokens["cLUNC"]["identity"], "offer_amount": "980000",
                      "pool_return_amount": "9000", "commission_amount": "0", "book_return_net": "0",
                      "pool_input_actual": "980000", "book_offer_consumed": "0", "spread_amount": "900",
                      "return_amount": "9000", "min_return": "8550"}]}
    if native:
        meta["wrapper"] = {"input_amount": "1000000", "route_input_amount": "980000",
                           "wrap_input": True, "unwrap_output": False}
    fair = "10000" if native else "9800"
    return {"token_in": offer, "token_out": tokens["USTR"]["identity"],
            "estimated_amount_out": "9000", "estimated_amount_out_net": "9000", "spot_amount_out": fair,
            "router_operations": [{"terra_swap": {"min_return": "8550", "hybrid": None}}],
            "execution_metadata": meta,
            "untrusted_server_prose": "private-response-sentinel",
            "economic_attribution": {"version": 1, "status": "available", "method": "spot_bridge_v1",
                "generation": meta["generation"], "observed_at_ms": 1000, "expires_at_ms": 21000,
                "fair_amount_out": fair, "net_amount_out": "9000", "dominant_cause": "market_movement",
                "components": {"wrapper_loss": "200" if native else "0", "trading_cost": "0",
                               "market_movement": "800", "reference_divergence": "0"}}}


class Reader:
    def __init__(self):
        self.reads = 0
        self.requests = []
        self.revision = SHA
        self.quotes = [quote(True), quote(False)]

    def api(self, path):
        self.reads += 1
        if path == "applications":
            return [{"uuid": label, "fqdn": origin} for label, origin in
                    (("frontend", PREP.ATTEST.FRONTEND), ("backend", PREP.ATTEST.BACKEND))]
        if path.startswith("deployments/applications/"):
            return [{"status": "finished", "commit": self.revision, "created_at": "2026-10-12T01:00:00Z",
                     "finished_at": "2026-10-12T01:01:00Z"}]
        origin = PREP.ATTEST.FRONTEND if path == "applications/frontend" else PREP.ATTEST.BACKEND
        return {"fqdn": origin}

    def read(self, url, limit):
        self.reads += 1
        self.requests.append(url)
        if url == PREP.ATTEST.FRONTEND + "/":
            return (b'<script src="/bootstrap/theme.js"></script>'
                    b'<script src="/bootstrap/trade-shell.js"></script>'
                    b'<script src="/assets/root.js"></script>')
        if url.endswith(("/assets/root.js", "/bootstrap/theme.js", "/bootstrap/trade-shell.js")):
            return b"owned root fixture"
        if url.endswith("/health"):
            return json.dumps({"git_sha": self.revision}).encode()
        self.assert_public_request(url)
        return json.dumps(self.quotes.pop(0)).encode()

    @staticmethod
    def assert_public_request(url):
        query = parse_qs(urlsplit(url).query)
        assert set(query) == {"token_in", "token_out", "amount_in", "max_spread", "pool_only", "protection_policy"}
        assert query["max_spread"] == ["0.05"]
        assert query["pool_only"] == ["true"]
        assert query["protection_policy"] == ["quote_relative_v1"]


class WalletPreparation(unittest.TestCase):
    def setUp(self):
        self.clock = patch.object(PREP.time, "time", return_value=2)
        self.clock.start()
        self.addCleanup(self.clock.stop)

    def test_prepare_attests_artifacts_and_uses_actual_wrapper_credit(self):
        reader = Reader()
        result = PREP.prepare(SHA, reader)
        self.assertEqual(result["status"], "PREPARED")
        self.assertFalse(result["signing"])
        self.assertEqual(result["samples"]["matched_credit_clunc"]["raw_input"], "980000")
        self.assertIn("amount=1", result["links"]["native_1_lunc"])
        self.assertIn("amount=0.98", result["links"]["matched_credit_clunc"])
        self.assertEqual(len(result["frontend"]["artifact"]["root_scripts"]), 3)
        self.assertLessEqual(reader.reads, PREP.ATTEST.MAX_READS)
        self.assertNotIn("private-response-sentinel", json.dumps(result))
        self.assertNotIn("execution_metadata", json.dumps(result))

    def test_revision_mismatch_never_proceeds_to_quotes(self):
        reader = Reader()
        reader.revision = "c" * 40
        with self.assertRaises(ValueError):
            PREP.prepare(SHA, reader)
        self.assertEqual(reader.requests, [])

    def test_issue1489_preparation_requires_actual_three_hop_native_control(self):
        reader = Reader()
        with self.assertRaisesRegex(ValueError, "required_native_route_unavailable"):
            PREP.prepare(SHA, reader, required_pool_hops=3)
        self.assertEqual(sum("/api/v1/route/solve?" in url for url in reader.requests), 1)

    def test_issue1489_preparation_envelope_is_closed_before_contact(self):
        for gross, hops in [("0", 3), ("100000000000", 3), ("10000000000", 4)]:
            reader = Reader()
            with self.subTest(gross=gross, hops=hops), self.assertRaisesRegex(ValueError, "unsupported_preparation_scope"):
                PREP.prepare(SHA, reader, gross=gross, required_pool_hops=hops)
            self.assertEqual(reader.reads, 0)

    def test_missing_attribution_or_unsafe_floor_never_prepares(self):
        for field, mutation in [
            ("economic_attribution", {"version": 1, "status": "unavailable", "method": "spot_bridge_v1"}),
            ("router_operations", [{"terra_swap": {"min_return": "0"}}]),
        ]:
            reader = Reader()
            reader.quotes[0][field] = mutation
            with self.subTest(field=field), self.assertRaises(ValueError):
                PREP.prepare(SHA, reader)

    def test_actor_expiry_and_credit_mismatch_never_prepares(self):
        for mutate in [
            lambda meta: meta.update(trader="private-actor"),
            lambda meta: meta.update(expires_at_ms=2000),
            lambda meta: meta["wrapper"].update(route_input_amount="980001"),
        ]:
            reader = Reader()
            mutate(reader.quotes[0]["execution_metadata"])
            with self.assertRaises(ValueError):
                PREP.prepare(SHA, reader)

    def test_terminal_and_receipt_do_not_disclose_secret_errors_or_open_browser(self):
        secret = "token-private-url-private-response"
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "receipt.json"
            terminal = io.StringIO()
            with patch.object(PREP.ATTEST, "Reader", side_effect=ValueError(secret)), \
                 patch.object(PREP.webbrowser, "open") as browser, contextlib.redirect_stdout(terminal):
                self.assertEqual(PREP.main(["--revision", SHA, "--output", str(output), "--open"]), 1)
            browser.assert_not_called()
            self.assertNotIn(secret, terminal.getvalue() + output.read_text())
            self.assertEqual(json.loads(output.read_text())["failure_class"], "ValueError")
            self.assertEqual(stat.S_IMODE(output.stat().st_mode), 0o600)

    def test_success_opens_only_verified_public_deep_links(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "receipt.json"
            with patch.object(PREP.ATTEST, "Reader", return_value=Reader()), \
                 patch.object(PREP.webbrowser, "open") as browser, contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(PREP.main(["--revision", SHA, "--output", str(output), "--open"]), 0)
            self.assertEqual(browser.call_count, 2)
            for call in browser.call_args_list:
                self.assertTrue(call.args[0].startswith("https://dex.cl8y.com/?"))
                self.assertNotIn("trader", call.args[0])

    def test_browser_children_cannot_inherit_scoped_credentials(self):
        snapshots = []
        def browser(url, new):
            snapshots.append(dict(os.environ))
            return True
        with patch.dict(os.environ, {"COOLIFY_WIRE_TOKEN": "credential", "COOLIFY_URL": "private-origin",
                                    "OTHER_SECRET": "private-other"}), patch.object(PREP.webbrowser, "open", side_effect=browser):
            self.assertTrue(PREP.open_links({"first": "https://dex.cl8y.com/?amount=1"}))
            self.assertEqual(os.environ["COOLIFY_WIRE_TOKEN"], "credential")
        self.assertTrue(snapshots)
        self.assertNotIn("COOLIFY_WIRE_TOKEN", snapshots[0])
        self.assertNotIn("COOLIFY_URL", snapshots[0])
        self.assertNotIn("OTHER_SECRET", snapshots[0])


if __name__ == "__main__":
    unittest.main()
