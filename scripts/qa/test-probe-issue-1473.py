#!/usr/bin/env python3
"""Quote corpus and actual socket safety tests with synthetic public payloads."""
import importlib.util
import json
import threading
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit

spec = importlib.util.spec_from_file_location("matrix_probe", Path(__file__).with_name("probe-issue-1473.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class Tests(unittest.TestCase):
    def test_all_pairs_directions_amounts_and_protection_settings(self):
        rows = list(probe.corpus())
        self.assertEqual(len(rows), 420)
        self.assertEqual(len({r["pair"] for r in rows}), 14)
        for direction in {r["pair"] for r in rows}:
            cases = [r for r in rows if r["pair"] == direction]
            self.assertEqual(len(cases), 30)
            self.assertEqual({r["spread"] for r in cases}, {"0.01", "0.05", "0.30"})
            self.assertTrue(all(probe.positive_u128(r["amount"]) for r in cases))
        cl8y = next(r for r in rows if r["pair"] == "CL8Y->UST1" and r["human_amount"] == "1000")
        ust1 = next(r for r in rows if r["pair"] == "UST1->CL8Y" and r["human_amount"] == "1000")
        self.assertEqual(cl8y["amount"], "1000000000000000000000")
        self.assertEqual(ust1["amount"], "1000000000")

    def test_indicative_is_separate_and_expiry_and_signing_metadata_rejected(self):
        case = next(probe.corpus("reproduction"))
        body = {"error": "protection", "indicative_amount_out": "123", "generation": "a" * 64,
                "execution_supported": False, "observed_at_ms": 1000, "expires_at_ms": 21000}
        self.assertEqual(probe.classify(422, body, case, 2000), ("indicative_protection", True))
        self.assertEqual(probe.classify(200, body, case, 2000), ("client_validation", False))
        for field, value in [("execution_supported", True), ("observed_at_ms", 3000),
                             ("expires_at_ms", 2000), ("expires_at_ms", 22001),
                             ("router_operations", []), ("indicative_amount_out", str(2 ** 128)),
                             ("generation", "<script>"), ("minimum_receive", "1")]:
            altered = {**body, field: value}
            self.assertEqual(probe.classify(422, altered, case, 2000), ("malformed_indicative", False))
        for value in ["0", "-1", "01", "1e18", "1" * 100000]:
            self.assertFalse(probe.positive_u128(value))

    def test_unknown_and_wrong_status_errors_do_not_pass(self):
        case = next(probe.corpus())
        for status, body in [(422, {"error": "No eligible route within selected protection"}),
                             (422, {"error": "stale_observations"}),
                             (503, {"error": "protection"}), (422, {"error": []}),
                             (422, ["protection"]), (200, {})]:
            self.assertFalse(probe.classify(status, body, case, 2000)[1])
        for reason, status in probe.REASONS.items():
            self.assertEqual(probe.classify(status, {"error": reason}, case, 2000), (reason, True))

    def test_quote_policy_recomputes_ceiling_and_small_raw_net_floors(self):
        scenario = {"spread": "0.05"}
        hop = {"pool_return_amount": "9000", "commission_amount": "0", "book_return_net": "0",
               "pool_input_actual": "1000", "book_offer_consumed": "0", "spread_amount": "900",
               "return_amount": "9000", "min_return": "8550"}
        body = {"estimated_amount_out_net": "9000", "execution_metadata": {
            "protection_policy": "quote_relative_v1", "max_spread": "0.05", "hops": [hop],
            "minimum_receive": "8550", "execution_market_spread": "0.157894736842105264",
            "observed_at_ms": 1000, "expires_at_ms": 21000}}
        self.assertTrue(probe.quoted_policy_valid(body, scenario, 2000))
        for observed, expiry in [(0, 30000), (2001, 21000), (1000, 21001), (1000, 2000)]:
            altered = json.loads(json.dumps(body))
            altered["execution_metadata"].update(observed_at_ms=observed, expires_at_ms=expiry)
            self.assertFalse(probe.quoted_policy_valid(altered, scenario, 2000))
        for value in ["0.05", "0.157894736842105263", "1", "0.157894736842105265"]:
            altered = json.loads(json.dumps(body))
            altered["execution_metadata"]["execution_market_spread"] = value
            self.assertFalse(probe.quoted_policy_valid(altered, scenario, 2000))
        for output, floor in [("2", "1"), ("19", "18")]:
            altered = json.loads(json.dumps(body))
            altered["execution_metadata"]["hops"][0].update(return_amount=output, min_return=floor)
            self.assertFalse(probe.quoted_policy_valid(altered, scenario, 2000))
        for field, value in [("return_amount", "09000"), ("commission_amount", "00"), ("pool_return_amount", str(2 ** 128))]:
            altered = json.loads(json.dumps(body))
            altered["execution_metadata"]["hops"][0][field] = value
            self.assertFalse(probe.quoted_policy_valid(altered, scenario, 2000))

    def test_real_socket_matrix_retains_classified_outcomes_and_never_raw_bodies(self):
        requests = []
        paths = []
        revision = "a" * 40
        sentinel = "private-actor-field-never-export"

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                target = urlsplit(self.path)
                if target.path == "/health":
                    status, body = 200, {"git_sha": revision}
                else:
                    requests.append(parse_qs(target.query))
                    paths.append(target.path)
                    # Raw private fields must not enter the public receipt.
                    status, body = 422, {"error": "protection", "private_actor": sentinel}
                payload = json.dumps(body).encode()
                self.send_response(status)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            args = SimpleNamespace(base_url=f"http://127.0.0.1:{server.server_port}", revision=revision,
                                   cycles=1, cycle_delay=181, endpoints=["solve", "best"],
                                   profile="reproduction", interval=0.5, enforce_socket_budget=False)
            with patch.object(probe.time, "sleep"):
                result = probe.probe(args)
            self.assertEqual(result["request_count"], 48)
            self.assertEqual(set(paths), {"/api/v1/route/solve", "/api/v1/route/solve/best"})
            self.assertEqual(result["counts"], {"protection": 48})
            self.assertEqual(result["overall"], "PASS WITH SKIPS")
            self.assertEqual(result["backend_revisions"], [revision, revision])
            self.assertNotIn(sentinel, json.dumps(result))
            self.assertTrue(all("trader" not in q and "sender" not in q for q in requests))
            self.assertEqual(result["useful_latency_by_transport"]["new"]["samples"], 0)
        finally:
            server.shutdown()
            server.server_close()
            worker.join()

    def test_origin_rejects_credentials_and_path_confusion(self):
        for origin in ["https://user:secret@example.invalid", "https://example.invalid/path", "file:///tmp/x", "https://example.invalid/?actor=x"]:
            with self.assertRaises(ValueError):
                probe.connection_for(origin)


if __name__ == "__main__":
    unittest.main()
