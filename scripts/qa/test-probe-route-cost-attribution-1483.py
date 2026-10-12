#!/usr/bin/env python3
"""Independent public-receipt boundary tests for #1483."""
import copy
import importlib.util
import json
import signal
import time
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("probe", Path(__file__).with_name("probe-route-cost-attribution-1483.py"))
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


class AttributionBoundary(unittest.TestCase):
    def quote(self):
        meta = {"generation": "a" * 64, "observed_at_ms": 1000, "expires_at_ms": 21000}
        return {"execution_metadata": meta, "spot_amount_out": "1000", "estimated_amount_out_net": "900",
                "economic_attribution": {"version": 1, "status": "available", "method": "spot_bridge_v1", **meta,
                "fair_amount_out": "1000", "net_amount_out": "900", "dominant_cause": "market_movement",
                "components": {"wrapper_loss": "20", "trading_cost": "30", "market_movement": "70", "reference_divergence": "-20"}}}

    def test_signed_common_units_reconcile_without_percent_addition(self):
        result = PROBE.attribution(self.quote())
        self.assertEqual(result["deviation_percent"], "10")
        self.assertEqual(result["components"]["reference_divergence"], "-20")
        self.assertNotIn("execution_metadata", result)

    def test_favorable_deviation_remains_symmetric(self):
        quote = self.quote()
        quote["spot_amount_out"] = "800"
        quote["economic_attribution"].update(fair_amount_out="800", dominant_cause="reference_divergence")
        quote["economic_attribution"]["components"]["reference_divergence"] = "-220"
        self.assertGreater(float(PROBE.attribution(quote)["deviation_percent"]), 11)

    def test_tamper_replay_injection_and_overflow_rejected(self):
        base = self.quote()
        mutations = [
            ("generation", "b" * 64), ("expires_at_ms", 22000), ("fair_amount_out", "999"),
            ("net_amount_out", "901"), ("dominant_cause", "wrapper_loss"), ("provider_body", "<script>"),
        ]
        for key, value in mutations:
            quote = copy.deepcopy(base)
            quote["economic_attribution"][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                PROBE.attribution(quote)
        for value in ("71", "01", "-0", str(1 << 128), "NaN", "<script>"):
            quote = copy.deepcopy(base)
            quote["economic_attribution"]["components"]["market_movement"] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                PROBE.attribution(quote)

    def test_unavailable_never_means_zero(self):
        self.assertEqual(PROBE.attribution({"economic_attribution": {"version": 1, "status": "unavailable", "method": "spot_bridge_v1"}}), {"status": "unavailable"})

    def test_health_revision_never_exports_unvalidated_provider_content(self):
        self.assertEqual(PROBE.health_revision(200, {"git_sha": "a" * 40}), "a" * 40)
        for body in ([], None, {"git_sha": "<script>provider prose"}, {"git_sha": 12}):
            with self.subTest(body=body), self.assertRaises(ValueError):
                PROBE.health_revision(200, body)

    def test_quote_outputs_reject_prose_and_uint128_overflow_before_export(self):
        self.assertEqual(PROBE.quoted_outputs(self.quote()), ("900", "1000"))
        for field in ("spot_amount_out", "estimated_amount_out_net"):
            for value in ("<script>provider prose", "0", str(1 << 128), "01", "-1"):
                quote = self.quote()
                quote[field] = value
                with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                    PROBE.quoted_outputs(quote)

    def test_slow_body_cannot_extend_hard_request_deadline(self):
        class SlowConnection:
            status = 200

            def __init__(self, *args, **kwargs):
                pass

            def request(self, *args, **kwargs):
                pass

            def getresponse(self):
                return self

            def read(self, *args):
                time.sleep(0.2)
                return b'{}'

            def close(self):
                pass

        previous = signal.getsignal(signal.SIGALRM)
        try:
            with patch.object(PROBE.http.client, "HTTPSConnection", SlowConnection), patch.object(PROBE, "REQUEST_TIMEOUT_SECONDS", 0.02):
                started = time.monotonic()
                result = PROBE.probe("a" * 40, False, 1)
                self.assertEqual(result["overall"], "FAIL")
                self.assertEqual(result["failures"][0]["failure_class"], "TimeoutError")
                self.assertLess(time.monotonic() - started, 0.15)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous)

    def test_useful_latency_includes_validation_not_only_transport(self):
        quote = self.quote()
        quote["execution_metadata"]["hops"] = [{}]

        class FastConnection:
            status = 200

            def __init__(self, *args, **kwargs):
                self.path = ""

            def request(self, method, path, **kwargs):
                self.path = path

            def getresponse(self):
                return self

            def read(self, *args):
                return json.dumps({"git_sha": "a" * 40} if self.path == "/health" else quote).encode()

            def close(self):
                pass

        def delayed_validation(*args):
            time.sleep(0.51)
            return "executable", True

        previous = signal.getsignal(signal.SIGALRM)
        scenario = [({"LUNC": {"identity": "uluna"}, "USTR": {"identity": "token"}}, "LUNC", "USTR", "1000000", "1", "test")]
        try:
            with patch.object(PROBE.http.client, "HTTPSConnection", FastConnection), patch.object(PROBE, "scenarios", return_value=iter(scenario)), patch.object(PROBE.OUTCOMES, "classify", side_effect=delayed_validation):
                result = PROBE.probe("a" * 40, False, 1)
            self.assertEqual(result["overall"], "FAIL")
            self.assertTrue(all(row["elapsed_ms"] > 500 and not row["socket_budget_pass"] for row in result["samples"]))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous)


if __name__ == "__main__":
    unittest.main()
