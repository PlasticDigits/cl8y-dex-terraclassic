#!/usr/bin/env python3
"""Executable-shape and evidence controls for the public latency corpus."""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("probe-issue-1464.py"))
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class CorpusTests(unittest.TestCase):
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
