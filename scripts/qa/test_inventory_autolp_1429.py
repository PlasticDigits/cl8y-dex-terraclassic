#!/usr/bin/env python3
"""Offline regression checks for the bounded #1429 LCD inventory observer."""

import base64
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import urllib.parse


SPEC = importlib.util.spec_from_file_location(
    "inventory_autolp_1429", Path(__file__).with_name("inventory-autolp-1429.py")
)
inventory = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(inventory)


def client(*, page_size=2, max_pages=3, max_contracts=6):
    return inventory.Client("https://offline.invalid", 1, page_size, max_pages, max_contracts)


def page(addresses, next_key=None):
    return {"contracts": addresses, "pagination": {"next_key": next_key}}


class PaginationTests(unittest.TestCase):
    def test_all_pages_and_encoded_cursor_are_consumed(self):
        observer = client()
        cursor = "a+b/="
        with patch.object(observer, "get", side_effect=[
            page(["terra1first"], cursor), page(["terra1second"], "")
        ]) as get:
            result = observer.contracts(11633)
        self.assertEqual(result, {
            "addresses": ["terra1first", "terra1second"], "pages": 2, "next_key": None
        })
        params = urllib.parse.parse_qs(urllib.parse.urlsplit(get.call_args.args[0]).query)
        self.assertEqual(params["pagination.key"], [cursor])
        self.assertEqual(params["pagination.limit"], ["2"])

    def test_empty_completed_inventory_is_distinct_from_failure(self):
        observer = client()
        with patch.object(observer, "get", return_value=page([])):
            self.assertEqual(observer.contracts(11633)["addresses"], [])

    def test_requested_page_size_is_enforced(self):
        observer = client(page_size=1)
        with patch.object(observer, "get", return_value=page(["terra1one", "terra1two"])):
            with self.assertRaisesRegex(inventory.ObservationError, "page size"):
                observer.contracts(11633)

    def test_total_contract_bound_is_enforced_across_pages(self):
        observer = client(max_contracts=2)
        with patch.object(observer, "get", side_effect=[
            page(["terra1one", "terra1two"], "next"), page(["terra1three"])
        ]):
            with self.assertRaisesRegex(inventory.ObservationError, "count bound"):
                observer.contracts(11633)

    def test_page_bound_prevents_partial_inventory_success(self):
        observer = client(max_pages=1)
        with patch.object(observer, "get", return_value=page(["terra1one"], "next")) as get:
            with self.assertRaisesRegex(inventory.ObservationError, "page bound"):
                observer.contracts(11633)
        self.assertEqual(get.call_count, 1)

    def test_cursor_cycles_fail_before_another_request(self):
        observer = client()
        with patch.object(observer, "get", side_effect=[
            page(["terra1one"], "cycle"), page(["terra1two"], "cycle")
        ]) as get:
            with self.assertRaisesRegex(inventory.ObservationError, "repeated pagination key"):
                observer.contracts(11633)
        self.assertEqual(get.call_count, 2)

    def test_duplicate_addresses_are_rejected_within_or_between_pages(self):
        for pages in [
            [page(["terra1one", "terra1one"])],
            [page(["terra1one"], "next"), page(["terra1one"])],
        ]:
            with self.subTest(pages=pages):
                observer = client()
                with patch.object(observer, "get", side_effect=pages):
                    with self.assertRaisesRegex(inventory.ObservationError, "duplicate address"):
                        observer.contracts(11633)

    def test_malformed_pages_are_rejected(self):
        for response in [
            {}, {"contracts": {}, "pagination": {}},
            {"contracts": [], "pagination": []}, page([None]),
            page(["cosmos1wrong"]), page([], ["cursor"]),
            page([], 0), page([], False),
        ]:
            with self.subTest(response=response):
                observer = client()
                with patch.object(observer, "get", return_value=response):
                    with self.assertRaises(inventory.ObservationError):
                        observer.contracts(11633)


class PolicyObservationTests(unittest.TestCase):
    def test_null_policy_and_present_policy_have_distinct_results(self):
        for policy, status in [(None, "missing"), ({"pair": "terra1pair"}, "present")]:
            with self.subTest(status=status):
                observer = client()
                with patch.object(observer, "smart", return_value=policy) as smart:
                    self.assertEqual(inventory.policy_observation(observer, "terra1sister"), {
                        "status": status, "value": policy
                    })
                smart.assert_called_once_with("terra1sister", {"get_lp_policy": {}})

    def test_unsupported_queries_and_unavailable_provider_are_distinct(self):
        for detail, status in [
            ("HTTP 400: unknown variant `get_lp_policy`", "unsupported"),
            ("HTTP 503: query unavailable", "unavailable"),
            ("unknown variant `other_query`", "unavailable"),
            ("get_lp_policy query timeout", "unavailable"),
        ]:
            with self.subTest(detail=detail):
                observer = client()
                with patch.object(observer, "smart", side_effect=inventory.ObservationError(detail)):
                    result = inventory.policy_observation(observer, "terra1sister")
                self.assertEqual(result, {"status": status, "error": detail})
                self.assertNotIn("value", result)

    def test_nonobject_nonnull_policies_are_unavailable(self):
        for policy in [[], 0, False, "null"]:
            with self.subTest(policy=policy):
                observer = client()
                with patch.object(observer, "smart", return_value=policy):
                    result = inventory.policy_observation(observer, "terra1sister")
                self.assertEqual(result["status"], "unavailable")
                self.assertNotIn("value", result)

    def test_encoded_and_direct_null_smart_responses_are_supported(self):
        for body in [{"data": None}, {"data": base64.b64encode(b"null").decode()}]:
            with self.subTest(body=body):
                observer = client()
                with patch.object(observer, "get", return_value=body):
                    self.assertEqual(inventory.policy_observation(observer, "terra1sister"), {
                        "status": "missing", "value": None
                    })

    def test_encoded_object_smart_response_is_decoded(self):
        policy = {"pair": "terra1pair", "total_share": "123"}
        body = {"data": base64.b64encode(json.dumps(policy).encode()).decode()}
        observer = client()
        with patch.object(observer, "get", return_value=body):
            self.assertEqual(observer.smart("terra1sister", {"get_lp_policy": {}}), policy)

    def test_malformed_smart_responses_are_unavailable_not_missing(self):
        for body in [
            {}, {"data": "%%%"}, {"data": base64.b64encode(b"invalid JSON").decode()},
        ]:
            with self.subTest(body=body):
                observer = client()
                with patch.object(observer, "get", return_value=body):
                    result = inventory.policy_observation(observer, "terra1sister")
                self.assertEqual(result["status"], "unavailable")
                self.assertNotIn("value", result)


if __name__ == "__main__":
    unittest.main()
