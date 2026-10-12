#!/usr/bin/env python3
"""Synthetic public-receipt controls for the read-only monitoring probe."""
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("monitor", Path(__file__).with_name("probe-swap-monitoring-1479-1480.py"))
MONITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITOR)


class Tests(unittest.TestCase):
    def test_public_aggregate_discards_labels_and_requires_finite_nonnegative_numbers(self):
        payload = {"status": "success", "data": {"resultType": "vector", "result": [{"metric": {"actor": "synthetic-private-value"}, "value": [123, "7"]}]}}
        self.assertEqual(MONITOR.aggregate(payload), 7)
        for value in ("NaN", "Infinity", "-1", "bad"):
            payload["data"]["result"][0]["value"][1] = value
            with self.assertRaises(ValueError):
                MONITOR.aggregate(payload)
        payload["data"]["resultType"] = "streams"
        with self.assertRaises(ValueError):
            MONITOR.aggregate(payload)

    def test_collect_uses_only_fixed_aggregate_queries_and_declares_missing_prometheus(self):
        instance = MONITOR.Monitor.__new__(MONITOR.Monitor)
        instance.reader = type("Reader", (), {"reads": 2})()
        paths = []
        def read(path):
            paths.append(path)
            instance.reader.reads += 1
            if path == "/api/datasources":
                return [{"type": "loki", "uid": "Loki", "url": "synthetic-private-value"}]
            return {"status": "success", "data": {"resultType": "vector", "result": [{"metric": {"actor": "synthetic-private-value"}, "value": [123, "3"]}]}}
        with patch.object(instance, "read", side_effect=read):
            receipt = instance.collect()
        self.assertEqual(len(paths), 7)
        self.assertEqual(receipt["read_count"], 9)
        self.assertEqual(receipt["prometheus"]["status"], "SKIP")
        self.assertNotIn("synthetic-private-value", json.dumps(receipt))
        self.assertTrue(all("query_range" not in path for path in paths))
        self.assertTrue(all("sum%28count_over_time%28" in path for path in paths[1:]))

    def test_transport_hard_cap_precedes_any_request(self):
        instance = MONITOR.Monitor.__new__(MONITOR.Monitor)
        instance.reader = type("Reader", (), {"reads": MONITOR.ATTESTER.MAX_READS})()
        with self.assertRaisesRegex(ValueError, "read_budget"):
            instance.read("/api/datasources")


if __name__ == "__main__":
    unittest.main()
