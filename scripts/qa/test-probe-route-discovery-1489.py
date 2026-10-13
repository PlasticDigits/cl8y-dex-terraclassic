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


def observed(net, endpoint, start, finish, *, generation="a", expires=21000):
    body = quote(net)
    body["search_complete"] = True
    body["execution_metadata"].update(generation=generation * 64, observed_at_ms=1000, expires_at_ms=expires)
    return {"body": body, "endpoint": endpoint, "request_started": start, "validated_at": finish,
            "wall_ms": int(1000 + finish * 1000), "eligible": True}


class PromotionWindowTests(unittest.TestCase):
    def window(self, expires=21000):
        first = observed(63, "solve", 0, .2, expires=expires)
        declared = observed(84, "declared_three_pool_hops", .3, .6, expires=expires)
        return PROBE.PromotionWindow(declared, [first, declared])

    def fill(self, window, net=84, start=1):
        for index, endpoint in enumerate(("solve", "best", "automatic_post")):
            t = start + index * .2
            window.observe(observed(net, endpoint, t, t + .1))

    def test_incompatible_reads_cannot_early_break_or_claim_convergence(self):
        window = self.window()
        window.observe(observed(63, "best", .7, .8, generation="b"))
        self.assertIsNotNone(window.next_pending_endpoint())
        self.assertEqual(window.verdict(.9, 1900)["status"], "PENDING")
        self.fill(window)
        self.assertEqual(window.verdict(1.6, 2600)["status"], "PASS")

    def test_late_higher_post_requires_fresh_gets_without_resetting_clock(self):
        window = self.window()
        self.fill(window)
        window.observe(observed(90, "automatic_post", 2.7, 2.9))
        self.assertEqual(window.started, 0)
        self.assertEqual(window.deadline, 3)
        self.assertEqual(window.maximum_net, 90)
        self.assertFalse(window.ready("solve"))
        window.observe(observed(84, "solve", 2.95, 3.1))
        self.assertEqual(window.verdict(3.1, 4100)["status"], "FAIL")

    def test_predeadline_short_read_is_not_retrospective_deadline_failure(self):
        window = self.window()
        window.observe(observed(63, "best", 1, 1.1))
        result = window.verdict(3.1, 4100)
        self.assertEqual(result["status"], "UNVERIFIED")
        self.assertNotEqual(result["reason"], "same_cohort_inferior_after_deadline")

    def test_late_benchmark_cannot_retrospectively_fail_an_earlier_get(self):
        window = self.window()
        window.observe(observed(84, "solve", 3, 3.1))
        window.observe(observed(90, "automatic_post", 3.2, 3.4))
        self.assertEqual(window.deadline, 3)
        self.assertEqual(window.verdict(3.5, 4500)["reason"], "benchmark_not_known_within_original_window")

    def test_request_started_before_benchmark_cannot_prove_promotion_miss(self):
        window = self.window()
        window.observe(observed(63, "solve", .4, 3.1))
        self.assertEqual(window.verdict(3.2, 4200)["status"], "UNVERIFIED")

    def test_expiry_wins_and_later_cohort_cannot_relabel_original(self):
        window = self.window(expires=2500)
        key, deadline = window.key, window.deadline
        self.assertAlmostEqual(deadline, 1.5)
        window.observe(observed(100, "best", 1, 1.1, generation="b", expires=30000))
        self.assertEqual(window.key, key)
        self.assertEqual(window.deadline, deadline)
        self.assertEqual(window.verdict(1.6, 2600)["reason"], "original_cohort_expired")

    def test_only_after_benchmark_gets_can_certify_and_late_success_is_unverified(self):
        window = self.window()
        self.fill(window)
        window.observe(observed(90, "automatic_post", 2.7, 2.9))
        self.fill(window, net=90, start=3.1)
        self.assertEqual(window.verdict(3.8, 4800)["status"], "UNVERIFIED")

    def test_unstable_original_cohort_never_yields_pass(self):
        window = self.window()
        for endpoint in ("solve", "best", "automatic_post"):
            window.observe(observed(100, endpoint, 2, 2.1, generation="b"))
        result = window.verdict(3.1, 4100)
        self.assertEqual(result["status"], "UNVERIFIED")
        self.assertEqual(result["reason"], "original_compatible_cohort_unavailable")


class CohortTests(unittest.TestCase):
    def test_progress_eligibility_only_emits_fixed_safe_enums(self):
        for value in ("executable_quote", "waiting_for_observations", "policy_excluded", "unverified"):
            self.assertEqual(PROBE.progress_eligibility({"quote_eligibility": value}), value)
        for value in (None, False, {"actor": "private"}, "private-actor", "<script>provider prose</script>"):
            self.assertEqual(PROBE.progress_eligibility({"quote_eligibility": value}), "other")

    def test_fast_classified_503_never_counts_as_useful_socket_acceptance(self):
        class Connection:
            def __init__(self, *args, **kwargs):
                self.sock, self.path = object(), ""

            def request(self, _method, path, **_kwargs):
                self.path = path

            def getresponse(self):
                self.status = 200 if self.path == "/health" or "/progress?" in self.path else 503
                return self

            def read(self, _limit):
                return json.dumps({"git_sha": "a" * 40} if self.path == "/health" else {"error": "missing_observations"}).encode()

            def close(self):
                pass

        previous = signal.getsignal(signal.SIGALRM)
        try:
            with patch.object(PROBE.http.client, "HTTPSConnection", Connection):
                receipt = PROBE.run("a" * 40, 1)
            self.assertEqual(receipt["overall"], "FAIL")
            self.assertEqual(receipt["initialized_useful_socket_timing"]["samples"], 0)
            quotes = [row for row in receipt["samples"] if row["endpoint"] != "progress"]
            self.assertTrue(all(not row["initialized_socket_budget_pass"] for row in quotes))
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
            signal.signal(signal.SIGALRM, previous)

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
