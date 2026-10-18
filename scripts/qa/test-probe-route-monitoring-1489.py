#!/usr/bin/env python3
"""Monitoring classifications cannot hide executed errors or publish log payloads."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit
from monitoring_fixtures_1544 import prometheus_payload

SPEC = importlib.util.spec_from_file_location("monitor1489", Path(__file__).with_name("probe-route-monitoring-1489.py"))
MONITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITOR)


class Reader:
    def __init__(self, failure=False):
        self.failure = failure
        self.queries = []

    def read(self, path):
        if path == "/api/datasources":
            return [{"type": "loki", "uid": "loki"}, {"type": "prometheus", "uid": MONITOR.COMMON.PROMETHEUS_UID, "url": MONITOR.COMMON.PROMETHEUS_URL}]
        if path.endswith("/health"):
            return {"status": "OK"}
        query = parse_qs(urlsplit(path).query)["query"][0]
        self.queries.append(query)
        value = 1 if "capacity|429" in query or (self.failure and "error|panic|fatal" in query) else 0
        return {"status": "success", "data": {"resultType": "vector", "result": [{"value": [0, str(value)], "metric": {"actor": "private-actor"}}]}}


    def query_prometheus(self, query):
        self.queries.append(query)
        return prometheus_payload(MONITOR.COMMON)


class MonitorTests(unittest.TestCase):
    def test_capacity_diagnostic_preserves_scoped_metrics_checks(self):
        reader = Reader()
        with patch.object(MONITOR.COMMON, "Monitor", return_value=reader):
            receipt = MONITOR.run()
        self.assertEqual(receipt["overall"], "PASS")
        self.assertEqual(receipt["prometheus"]["status"], "PASS")
        self.assertEqual(receipt["counts"]["indexer_capacity_lines"], 1)
        self.assertNotIn("private-actor", str(receipt))
        self.assertEqual(len(reader.queries), 9)
        self.assertTrue(all("sum(count_over_time(" in query and "[15m]" in query for query in reader.queries[:-1]))
        self.assertEqual(reader.queries[-1], MONITOR.COMMON.PROM_QUERY)

    def test_executed_error_count_remains_failure(self):
        with patch.object(MONITOR.COMMON, "Monitor", return_value=Reader(True)):
            receipt = MONITOR.run()
        self.assertEqual(receipt["overall"], "FAIL")


if __name__ == "__main__":
    unittest.main()
