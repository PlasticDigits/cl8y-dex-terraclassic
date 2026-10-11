#!/usr/bin/env python3
"""Quote corpus and actual socket safety tests with synthetic public payloads."""
import importlib.util
import json
import subprocess
import sys
import tempfile
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

    def test_idle_keepalive_closes_but_each_cycle_has_fresh_provenance_and_cold_quote(self):
        revision = "b" * 40
        idle_closed = threading.Event()
        connections = {"health": [], "quote": []}

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def setup(self):
                super().setup()
                self.connection.settimeout(0.04)

            def log_message(self, *_args):
                pass

            def finish(self):
                super().finish()
                idle_closed.set()

            def do_GET(self):
                is_health = self.path == "/health"
                connections["health" if is_health else "quote"].append(self.client_address[1])
                payload = json.dumps({"git_sha": revision} if is_health else {"error": "protection"}).encode()
                self.send_response(200 if is_health else 422)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        origin = f"http://127.0.0.1:{server.server_port}"
        stale = probe.connection_for(origin)
        try:
            self.assertEqual(probe.health_revision(stale), revision)
            self.assertTrue(idle_closed.wait(1), "owned server must actually close the idle keepalive")
            self.assertIsNotNone(stale.sock, "client still retains the stale pooled socket")
            with self.assertRaises((OSError, probe.common.http.client.HTTPException)):
                probe.health_revision(stale)
            connections["health"].clear()
            # Short idle gap is internal to this local regression. CLI minimums
            # remain 181s cycle delay and 0.5s request spacing for public probes.
            args = SimpleNamespace(base_url=origin, revision=revision, cycles=2, cycle_delay=0.08,
                                   endpoints=["solve"], profile="reproduction", interval=0,
                                   enforce_socket_budget=False)
            cases = list(probe.corpus("reproduction"))[:2]
            with patch.object(probe, "corpus", return_value=iter(cases)):
                result = probe.probe(args)
            self.assertEqual(result["overall"], "PASS WITH SKIPS")
            self.assertEqual(result["request_count"], 4)
            self.assertEqual(result["backend_revisions"], [revision] * 4)
            self.assertEqual([row["transport"] for row in result["samples"]], ["new", "reused"] * 2)
            self.assertEqual([row["transport_plan"] for row in result["samples"]], ["cold_cycle_start", "pooled"] * 2)
            self.assertEqual(len(set(connections["health"])), 4)
            self.assertEqual(len(set(connections["quote"])), 2)
            self.assertTrue(set(connections["health"]).isdisjoint(connections["quote"]))
        finally:
            stale.close()
            server.shutdown()
            server.server_close()
            worker.join()

    def test_failed_provenance_preserves_completed_cases_without_raw_health_data(self):
        revision = "c" * 40
        sentinel = "private-actor-and-url-never-exported"
        health_count = 0

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                nonlocal health_count
                if self.path == "/health":
                    health_count += 1
                    status, body = 200, {"git_sha": revision if health_count < 3 else sentinel}
                else:
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
                                   cycles=2, cycle_delay=0, endpoints=["solve"], profile="reproduction",
                                   interval=0, enforce_socket_budget=False)
            cases = list(probe.corpus("reproduction"))[:2]
            with patch.object(probe, "corpus", return_value=iter(cases)):
                result = probe.probe(args)
            self.assertEqual(result["overall"], "FAIL")
            self.assertEqual(result["request_count"], 2)
            self.assertEqual(result["counts"], {"protection": 2})
            self.assertEqual(result["backend_revisions"], [revision, revision])
            self.assertEqual(result["failures"][0]["stage"], "provenance_before.decode")
            self.assertEqual(result["failures"][0]["failure_class"], "HealthRevisionUnavailable")
            self.assertEqual(result["failures"][0]["completed_request_count"], 2)
            self.assertEqual(result["failures"][0]["trial"], "retest")
            self.assertTrue(all(row["utc_start"] <= row["utc_end"] for row in result["samples"]))
            encoded = json.dumps(result)
            self.assertNotIn(sentinel, encoded)
            self.assertNotIn(args.base_url, encoded)
        finally:
            server.shutdown()
            server.server_close()
            worker.join()

    def test_failed_quote_is_recorded_once_and_never_retried_or_counted_as_latency(self):
        revision = "d" * 40
        quote_count = 0

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                nonlocal quote_count
                if self.path == "/health":
                    status, body = 200, {"git_sha": revision}
                else:
                    quote_count += 1
                    if quote_count == 1:
                        self.close_connection = True
                        return  # Actual connection close before response headers.
                    status, body = 422, {"error": "protection"}
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
                                   cycles=1, cycle_delay=0, endpoints=["solve"], profile="reproduction",
                                   interval=0, enforce_socket_budget=False)
            cases = list(probe.corpus("reproduction"))[:2]
            with patch.object(probe, "corpus", return_value=iter(cases)):
                result = probe.probe(args)
            self.assertEqual(quote_count, 2)
            self.assertEqual(result["overall"], "FAIL")
            self.assertEqual(result["request_count"], 2)
            failed = result["samples"][0]
            self.assertEqual(failed["failure_stage"], "quote.response")
            self.assertEqual(failed["failure_class"], "RemoteDisconnected")
            self.assertFalse(failed["classified"])
            self.assertFalse(failed["socket_budget_pass"])
            self.assertEqual(result["samples"][1]["outcome"], "protection")
            self.assertEqual(result["useful_latency_by_transport"]["new"]["samples"], 0)
            self.assertEqual(result["useful_latency_by_transport"]["reused"]["samples"], 0)
        finally:
            server.shutdown()
            server.server_close()
            worker.join()

    def test_cli_writes_failure_receipt_when_initial_provenance_is_malformed(self):
        sentinel = "private-health-field-never-print"

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                payload = json.dumps({"git_sha": sentinel}).encode()
                self.send_response(200)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            with tempfile.TemporaryDirectory() as temporary:
                output = Path(temporary) / "partial.json"
                origin = f"http://127.0.0.1:{server.server_port}"
                completed = subprocess.run([sys.executable, str(Path(probe.__file__)),
                    "--base-url", origin, "--revision", "e" * 40, "--output", str(output)],
                    capture_output=True, text=True, timeout=10)
                self.assertEqual(completed.returncode, 1)
                self.assertTrue(output.is_file(), "prerequisite failure must preserve a receipt")
                result = json.loads(output.read_text())
                self.assertEqual(result["overall"], "FAIL")
                self.assertEqual(result["request_count"], 0)
                self.assertEqual(result["failures"][0]["stage"], "provenance_before.decode")
                self.assertEqual(result["failures"][0]["failure_class"], "HealthRevisionUnavailable")
                self.assertNotIn(sentinel, output.read_text() + completed.stdout + completed.stderr)
                self.assertNotIn(origin, output.read_text() + completed.stdout + completed.stderr)
        finally:
            server.shutdown()
            server.server_close()
            worker.join()


if __name__ == "__main__":
    unittest.main()
