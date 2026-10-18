#!/usr/bin/env python3
"""Strict target/freshness and bounded public-receipt controls for monitoring."""
import copy
import importlib.util
import json
import unittest
from pathlib import Path
from unittest.mock import patch
from urllib.parse import parse_qs, urlsplit
from monitoring_fixtures_1544 import prometheus_payload, grafana_frames

SPEC = importlib.util.spec_from_file_location("monitor", Path(__file__).with_name("probe-swap-monitoring-1479-1480.py"))
MONITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITOR)
NOW = 1800000000
SOURCES = [{"type": "loki", "uid": "Loki"}, {"type": "prometheus", "uid": MONITOR.PROMETHEUS_UID, "url": MONITOR.PROMETHEUS_URL}]


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

    def test_collect_preserves_loki_aggregates_and_requires_scoped_fresh_prometheus(self):
        instance = MONITOR.Monitor.__new__(MONITOR.Monitor)
        instance.reader = type("Reader", (), {"reads": 2})()
        paths = []
        def read(path, *, payload=None):
            paths.append(path)
            instance.reader.reads += 1
            if path == "/api/datasources":
                return SOURCES
            if path.endswith("/health"):
                return {"status": "OK", "message": "synthetic-private-value"}
            if path == "/api/ds/query":
                self.assertEqual(payload["queries"][0]["expr"], MONITOR.PROM_QUERY)
                return grafana_frames(prometheus_payload(MONITOR))
            return {"status": "success", "data": {"resultType": "vector", "result": [{"metric": {"actor": "synthetic-private-value"}, "value": [123, "3"]}]}}
        with patch.object(instance, "read", side_effect=read):
            receipt = instance.collect()
        self.assertEqual(len(paths), 9)
        self.assertEqual(receipt["read_count"], 11)
        self.assertEqual(receipt["prometheus"]["status"], "PASS")
        self.assertNotIn("synthetic-private", json.dumps(receipt))
        self.assertTrue(all("query_range" not in path for path in paths))
        queries = [parse_qs(urlsplit(path).query)["query"][0] for path in paths if "?" in path]
        self.assertEqual(paths[-1], "/api/ds/query")
        self.assertNotIn("sum(up)", queries)
        self.assertTrue(all(query.startswith("sum(count_over_time(") for query in queries))

    def collect_prometheus(self, payload=None, *, health=None, sources=None):
        instance = type("Monitor", (), {})()
        instance.read = lambda path: ({"status": "OK"} if health is None else health) if path.endswith("/health") else payload
        instance.query_prometheus = lambda query: payload
        return MONITOR.collect_prometheus(instance, SOURCES if sources is None else sources, now=NOW)

    def test_missing_wrong_duplicate_datasource_and_bad_health_fail_closed(self):
        for sources in ([], SOURCES[:1], SOURCES + SOURCES[1:], [{"type": "prometheus", "uid": "unrelated"}]):
            with self.subTest(sources=sources), self.assertRaisesRegex(ValueError, "prometheus_identity"):
                self.collect_prometheus(sources=sources)
        with self.assertRaisesRegex(ValueError, "prometheus_identity"):
            self.collect_prometheus(sources=[{"type": "prometheus", "uid": MONITOR.PROMETHEUS_UID, "url": "http://unrelated:9090"}])
        for health in ({}, {"status": "ERROR"}, {"status": "OKish"}):
            with self.subTest(health=health), self.assertRaisesRegex(ValueError, "prometheus_health"):
                self.collect_prometheus(health=health)

    def test_missing_stale_wrong_service_duplicate_and_sensitive_labels_are_rejected(self):
        baseline = prometheus_payload(MONITOR, now=NOW)
        cases = []
        missing = copy.deepcopy(baseline)
        missing["data"]["result"].pop()
        cases.append(missing)
        for label, value in (("job", "unrelated-healthy"), ("service", "unrelated"), ("instance", "wrong-target:9800"), ("actor", "private-actor")):
            payload = copy.deepcopy(baseline)
            payload["data"]["result"][0]["metric"][label] = value
            cases.append(payload)
        for timestamp in (NOW - 91, NOW + 6, "NaN", "Infinity"):
            payload = copy.deepcopy(baseline)
            row = next(row for row in payload["data"]["result"] if row["metric"]["monitor_check"] == "timestamp")
            row["value"][1] = str(timestamp)
            cases.append(payload)
        stalled_exporter = copy.deepcopy(baseline)
        row = next(row for row in stalled_exporter["data"]["result"] if row["metric"]["monitor_metric"] == "dex_observation_timestamp_seconds" and row["metric"]["monitor_check"] == "value")
        row["value"][1] = str(NOW - 91)
        cases.append(stalled_exporter)
        duplicate = copy.deepcopy(baseline)
        duplicate["data"]["result"][-1] = copy.deepcopy(duplicate["data"]["result"][0])
        cases.append(duplicate)
        stale_evaluation = copy.deepcopy(baseline)
        stale_evaluation["data"]["result"][0]["value"][0] = NOW - 91
        cases.append(stale_evaluation)
        for payload in cases:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                self.collect_prometheus(payload)

    def test_down_scrape_or_serving_fails_but_separate_readiness_is_reported(self):
        payload = prometheus_payload(MONITOR, now=NOW, readiness=0)
        receipt = self.collect_prometheus(payload)
        self.assertEqual(receipt["targets"]["cl8y-dex-api"]["endpoint_up"]["quote_readiness"], 0)
        self.assertEqual(receipt["targets"]["cl8y-dex-indexer"]["endpoint_up"]["indexer_readiness"], 0)
        for metric, purpose in (("up", ""), ("dex_endpoint_up", "liveness"), ("dex_endpoint_up", "serving")):
            failed = copy.deepcopy(payload)
            row = next(row for row in failed["data"]["result"] if row["metric"]["monitor_metric"] == metric and row["metric"].get("purpose", "") == purpose and row["metric"]["monitor_check"] == "value")
            row["value"][1] = "0"
            with self.subTest(metric=metric, purpose=purpose), self.assertRaisesRegex(ValueError, "prometheus_(scrape|serving)_down"):
                self.collect_prometheus(failed)
        self.assertNotIn("synthetic-private", json.dumps(receipt))

    def test_grafana_frames_reject_errors_and_unbounded_or_malformed_samples(self):
        fixture = grafana_frames(prometheus_payload(MONITOR, now=NOW))
        self.assertEqual(len(MONITOR.prometheus_frames(fixture)["data"]["result"]), 16)
        bad = copy.deepcopy(fixture)
        bad["results"]["A"]["error"] = "synthetic-private-error"
        malformed = copy.deepcopy(fixture)
        malformed["results"]["A"]["frames"][0]["data"]["values"][0].append(NOW * 1000)
        for payload in ({}, bad, malformed):
            with self.assertRaises(ValueError):
                MONITOR.prometheus_frames(payload)

    def test_transport_hard_cap_precedes_any_request(self):
        instance = MONITOR.Monitor.__new__(MONITOR.Monitor)
        instance.reader = type("Reader", (), {"reads": MONITOR.ATTESTER.MAX_READS})()
        with self.assertRaisesRegex(ValueError, "read_budget"):
            instance.read("/api/datasources")


if __name__ == "__main__":
    unittest.main()
