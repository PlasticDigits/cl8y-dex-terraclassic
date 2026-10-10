#!/usr/bin/env python3
"""Executable-shape and evidence controls for the public latency corpus."""
import importlib.util
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import threading
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("probe-issue-1464.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class CorpusTests(unittest.TestCase):
    def test_socket_reuse_and_server_close_are_observed_on_same_connection_object(self):
        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Length", "2")
                if self.path == "/close":
                    self.send_header("Connection", "close")
                self.end_headers()
                self.wfile.write(b"ok")

            def log_message(self, *args):
                pass

        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        connection = http.client.HTTPConnection("127.0.0.1", server.server_port, timeout=2)
        try:
            for path, expected in [("/keep", "new"), ("/close", "reused"), ("/keep", "new")]:
                self.assertEqual(probe.request_transport(connection, path, {}), expected)
                self.assertEqual(connection.getresponse().read(), b"ok")
        finally:
            connection.close()
            server.shutdown()
            server.server_close()
            worker.join(timeout=2)

    def quote(self):
        return {"token_in": "input", "token_out": "output", "estimated_amount_out_net": "7", "router_operations": [{"terra_swap": {"min_return": "6"}}],
                "execution_metadata": {"input_amount": "10", "generation": "a" * 64, "observed_at_ms": 100,
                                       "expires_at_ms": 300, "minimum_receive": "6", "hops": [{"min_return": "6"}]}}

    def test_requires_positive_executable_shape_and_current_observation(self):
        self.assertTrue(probe.useful(self.quote(), "10", 200, "input", "output"))
        for key, value in (("estimated_amount_out_net", "0"), ("router_operations", []), ("execution_metadata", {})):
            body = self.quote()
            body[key] = value
            self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        for key, value in (("generation", ""), ("minimum_receive", "0"), ("expires_at_ms", 200), ("observed_at_ms", 201), ("input_amount", "11")):
            body = self.quote()
            body["execution_metadata"][key] = value
            self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        body = self.quote()
        body["execution_metadata"]["generation"] = "hostile unbounded label"
        self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        body = self.quote()
        body["router_operations"][0]["terra_swap"]["min_return"] = "0"
        self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        body = self.quote()
        body["execution_metadata"]["hops"].append({"min_return": "6"})
        self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        for body in (None, [], {}, "error"):
            self.assertFalse(probe.useful(body, "10", 200, "input", "output"))

    def test_no_sample_is_not_a_fast_success(self):
        self.assertEqual(probe.distribution([])["samples"], 0)
        self.assertIsNone(probe.distribution([])["max_ms"])
        stats = probe.distribution([1, 3, 2, 300])
        self.assertEqual(stats["max_ms"], 300)
        self.assertEqual(stats["p99_ms"], 300)

    def test_current_budget_keeps_fast_errors_and_unrounded_tail_failures_out(self):
        self.assertTrue(probe.measurement_pass(True, 500, 500))
        self.assertFalse(probe.measurement_pass(False, 1, 500))
        self.assertFalse(probe.measurement_pass(True, 500.00001, 500))
        self.assertFalse(probe.measurement_pass(True, -1, 500))
        self.assertFalse(probe.measurement_pass(True, float("nan"), 500))
        self.assertFalse(probe.measurement_pass(True, float("inf"), 500))

    def test_unsupported_wrong_direction_and_noninteger_evidence_never_count(self):
        for key, value in (("token_in", "other"), ("token_out", "other"), ("execution_supported", False),
                           ("execution_blocker", "unsupported"), ("estimated_amount_out_net", True),
                           ("estimated_amount_out_net", 7.0), ("estimated_amount_out_net", "7.0")):
            body = self.quote()
            body[key] = value
            self.assertFalse(probe.useful(body, "10", 200, "input", "output"))
        for key, value in (("execution_supported", False), ("minimum_receive", "8"), ("minimum_receive", True),
                           ("expires_at_ms", "300"), ("observed_at_ms", False)):
            body = self.quote()
            body["execution_metadata"][key] = value
            self.assertFalse(probe.useful(body, "10", 200, "input", "output"))

    def test_corpus_preserves_raw_6_and_18_decimal_units(self):
        self.assertEqual(len(probe.CORPUS), 4)
        self.assertEqual(probe.CORPUS[0][2], str(10_000 * 10**6))
        self.assertEqual(probe.CORPUS[1][3], str(10**16))


if __name__ == "__main__":
    unittest.main()
