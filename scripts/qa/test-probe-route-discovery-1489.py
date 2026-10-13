#!/usr/bin/env python3
"""Original-economics discriminator and public receipt isolation regressions."""
import copy
import importlib.util
import json
import signal
from pathlib import Path
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("probe1489", Path(__file__).with_name("probe-route-discovery-1489.py"))
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


def quote(net):
    return {"token_in": "uluna", "token_out": "out", "estimated_amount_out_net": str(net), "spot_amount_out": "100",
            "execution_metadata": {"generation": "a" * 64, "observed_at_ms": 10, "expires_at_ms": 20,
                                   "input_amount": "10000", "max_spread": "0.05", "protection_policy": "quote_relative_v1",
                                   "wrapper": {"input_amount": "10000", "route_input_amount": "9800", "wrap_input": True, "unwrap_output": False}}}


class CohortTests(unittest.TestCase):
    def test_unavailable_attribution_cannot_be_counted_as_eligible_or_compared(self):
        forged = quote(85)
        forged["execution_metadata"]["policy"] = "pool_only"
        forged["router_operations"] = [{"terra_swap": {"hybrid": None}}]
        forged["economic_attribution"] = {"version": 1, "status": "unavailable", "method": "spot_bridge_v1"}

        class Connection:
            status = 200

            def __init__(self, *args, **kwargs):
                self.sock = object()
                self.paths = []

            def request(self, _method, path, **_kwargs):
                self.paths.append(path)

            def getresponse(self):
                return self

            def read(self, _limit):
                return json.dumps({"git_sha": "a" * 40} if self.paths[-1] == "/health" else forged).encode()

        connection = Connection()
        previous = signal.getsignal(signal.SIGALRM)
        try:
            # Even if an executable-shape validator accepts the remaining body,
            # the actual attribution parser must block eligibility before any
            # automatic/declared ranking comparison or additional quote read.
            with patch.object(PROBE.http.client, "HTTPSConnection", return_value=connection), patch.object(PROBE.COMMON.OUTCOMES, "classify", return_value=("executable", True)), patch.object(PROBE, "compare") as compared:
                with self.assertRaisesRegex(ValueError, "eligible_attribution_unavailable"):
                    PROBE.run("a" * 40, 1)
                compared.assert_not_called()
            self.assertEqual(len(connection.paths), 2)
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous)

    def test_automatic_and_declared_null_pool_operations_share_semantics(self):
        automatic, declared = quote(90), quote(84)
        automatic["execution_metadata"]["policy"] = "pool_only"
        declared["execution_metadata"]["policy"] = "declared_hybrid_manual"
        automatic["router_operations"] = [{"terra_swap": {"hybrid": None}}] * 2
        declared["router_operations"] = [{"terra_swap": {"hybrid": None}}] * 3
        self.assertTrue(PROBE.compare(automatic, declared)["same_original_cohort"])

    def test_positive_hybrid_unknown_modes_and_missing_operations_remain_distinct(self):
        automatic = quote(90)
        automatic["execution_metadata"]["policy"] = "pool_only"
        automatic["router_operations"] = [{"terra_swap": {"hybrid": None}}]
        declared = copy.deepcopy(automatic)
        declared["execution_metadata"]["policy"] = "declared_hybrid_manual"
        for operations in ([], [{"terra_swap": {}}], [{"terra_swap": {"hybrid": {"book_offer_amount": "1"}}}]):
            declared["router_operations"] = operations
            self.assertFalse(PROBE.compare(automatic, declared)["same_original_cohort"])
        declared["router_operations"] = [{"terra_swap": {"hybrid": None}}]
        declared["execution_metadata"]["policy"] = "unknown_manual_mode"
        self.assertFalse(PROBE.compare(automatic, declared)["same_original_cohort"])

    def test_original_failure_cannot_pass_from_http_or_generation_equality(self):
        result = PROBE.compare(quote(63), quote(84))
        self.assertTrue(result["same_original_cohort"])
        self.assertFalse(result["automatic_not_worse"])

    def test_original_expiry_credit_fair_protection_and_identity_must_match(self):
        first = quote(90)
        for target, field, value in (("meta", "expires_at_ms", 21), ("meta", "observed_at_ms", 11),
                                     ("meta", "max_spread", "0.3"), ("meta", "input_amount", "1"),
                                     ("meta", "protection_policy", "other"), ("wrapper", "route_input_amount", "10000"),
                                     ("wrapper", "wrap_input", False), ("body", "spot_amount_out", "101"),
                                     ("body", "token_in", "wrapped")):
            other = copy.deepcopy(first)
            obj = other if target == "body" else other["execution_metadata"] if target == "meta" else other["execution_metadata"]["wrapper"]
            obj[field] = value
            with self.subTest(field=field):
                self.assertFalse(PROBE.compare(first, other)["same_original_cohort"])

    def test_higher_delivered_net_wins_and_actor_prose_cannot_affect_cohort(self):
        automatic, declared = quote(85), quote(84)
        automatic["execution_metadata"]["trader"] = "private-actor"
        automatic["optimality_scope"] = "arbitrary injected HTML"
        self.assertTrue(PROBE.compare(automatic, declared)["automatic_not_worse"])

    def test_receipt_rejects_injected_token_prose_and_omits_actors(self):
        body = quote(85)
        meta = body["execution_metadata"]
        body["paths_considered"] = "private arbitrary injected prose"
        meta.update(minimum_receive="80", trader="private-actor", sender="private-sender")
        meta["hops"] = [{"offer_token": "uluna", "ask_token": PROBE.QUERY["token_out"], "min_return": "80", "return_amount": "85", "trader": "private-hop"}]
        with patch.object(PROBE.COMMON, "attribution", return_value={"status": "available"}):
            receipt = PROBE.summarize(body)
            self.assertNotIn("private", str(receipt))
            meta["hops"][0]["ask_token"] = "<script>private</script>"
            with self.assertRaises(ValueError):
                PROBE.summarize(body)


if __name__ == "__main__":
    unittest.main()
