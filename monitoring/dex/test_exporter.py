"""No external network or chain calls; real local HTTP tests for scrape isolation."""
import threading
import tempfile
import unittest
from http.server import ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import urlopen

import exporter


class ExporterTests(unittest.TestCase):
    def setUp(self):
        self.now = 1000.0
        self.calls = []
        self.payloads = {
            "/health": (200, {"status": "ok", "private": "actor-value"}),
            "/health/serving": (200, {"admitted": True}),
            "/ready": (503, {"ready": False, "status": "running", "checkpoint_height": 40, "latest_height": 44}),
            "/ready-api": (503, {"ready": False, "scenario_count": 2}),
        }
        self.observer = exporter.Observer(self.read, lambda: self.now)

    def read(self, path):
        self.calls.append(path)
        value = self.payloads[path]
        if isinstance(value, Exception):
            raise value
        return value

    def test_readiness_503_is_observed_and_distinct_from_serving(self):
        self.observer.poll()
        text = self.observer.metrics().decode()
        self.assertEqual(self.calls, list(exporter.ENDPOINTS.values()))
        self.assertIn('dex_endpoint_up{purpose="liveness"} 1', text)
        self.assertIn('dex_endpoint_up{purpose="serving"} 1', text)
        self.assertIn('dex_endpoint_up{purpose="indexer_readiness"} 0', text)
        self.assertIn('dex_endpoint_up{purpose="quote_readiness"} 0', text)
        self.assertIn('dex_endpoint_http_status{purpose="indexer_readiness"} 503', text)
        self.assertIn('dex_indexer_lag_blocks 4', text)
        self.assertIn('dex_observation_timestamp_seconds 1000.0', text)
        self.assertNotIn("actor", text)
        self.assertNotIn("https", text)

    def test_progress_clock_updates_only_when_checkpoint_changes(self):
        self.observer.poll()
        self.now = 1100.0
        self.observer.poll()
        self.assertIn(b"dex_indexer_progress_timestamp_seconds 1000.0", self.observer.metrics())
        self.payloads["/ready"][1]["checkpoint_height"] = 41
        self.observer.poll()
        self.assertIn(b"dex_indexer_progress_timestamp_seconds 1100.0", self.observer.metrics())

    def test_transport_and_malformed_failure_omit_freshness_and_heights(self):
        self.observer.poll()
        self.payloads["/ready"] = TimeoutError("credential-private")
        self.payloads["/ready-api"] = (200, {"ready": False, "scenario_count": 2})
        self.observer.poll()
        text = self.observer.metrics().decode()
        self.assertNotIn("dex_indexer_", text)
        self.assertNotIn('dex_observation_timestamp_seconds', text)
        self.assertNotIn("credential", text)
        self.assertIn('dex_endpoint_up{purpose="indexer_readiness"} 0', text)

    def test_unknown_head_is_not_zero_lag(self):
        self.payloads["/ready"][1]["latest_height"] = None
        self.observer.poll()
        self.assertNotIn(b"dex_indexer_lag_blocks", self.observer.metrics())
        self.assertNotIn(b"dex_indexer_observed_head_height", self.observer.metrics())

    def test_height_bool_and_mismatched_status_are_rejected(self):
        self.payloads["/ready"][1]["checkpoint_height"] = True
        self.payloads["/health/serving"] = (503, {"admitted": True})
        self.observer.poll()
        text = self.observer.metrics().decode()
        self.assertNotIn("dex_indexer_", text)
        self.assertIn('dex_endpoint_up{purpose="serving"} 0', text)

    def test_scraping_cached_http_never_polls_upstream(self):
        self.observer.poll()
        self.calls.clear()
        server = ThreadingHTTPServer(("127.0.0.1", 0), exporter.handler(self.observer))
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        origin = f"http://127.0.0.1:{server.server_port}"
        try:
            for _ in range(3):
                with urlopen(origin + "/metrics", timeout=1) as response:
                    self.assertEqual(response.status, 200)
                    self.assertIn(b"dex_endpoint_up", response.read())
            with self.assertRaises(HTTPError) as error:
                urlopen(origin + "/metrics?private=value", timeout=1)
            self.assertEqual(error.exception.code, 404)
            self.assertEqual(self.calls, [])
        finally:
            server.shutdown()
            server.server_close()
            worker.join(timeout=1)

    def test_fetch_bounds_body_and_never_follows_redirect(self):
        class FakeResponse:
            code = 200
            def __enter__(self):
                return self
            def __exit__(self, *_args):
                return None
            def read(self, limit):
                self.limit = limit
                return b"x" * limit
        class FakeOpener:
            def open(self, request, timeout):
                self.request, self.timeout = request, timeout
                return FakeResponse()
        original = exporter.OPENER
        fake = FakeOpener()
        exporter.OPENER = fake
        try:
            with self.assertRaisesRegex(ValueError, "body_bound"):
                exporter.fetch("/ready")
            self.assertEqual(fake.timeout, 3)
            self.assertEqual(fake.request.full_url, exporter.ORIGIN + "/ready")
            self.assertIsNone(exporter.NoRedirect().redirect_request(None, None, 302, "", {}, "https://private/"))
        finally:
            exporter.OPENER = original

    def test_drill_expires_without_background_or_upstream_access(self):
        with tempfile.TemporaryDirectory() as directory:
            state = exporter.AlertState(directory + "/alerts.json", lambda: self.now)
            self.assertIn(b"dex_monitoring_drill 0", state.metrics())
            self.assertEqual(state.set_drill(True), {"enabled": True})
            self.assertIn(b"dex_monitoring_drill 1", state.metrics())
            self.now += exporter.DRILL_SECONDS
            self.assertIn(b"dex_monitoring_drill 0", state.metrics())
            self.assertEqual(state.set_drill(False), {"enabled": False})
            self.assertEqual(self.calls, [])

    def test_alert_receipt_is_bounded_persistent_sanitized_and_recovers(self):
        with tempfile.TemporaryDirectory() as directory:
            path = directory + "/alerts.json"
            state = exporter.AlertState(path, lambda: self.now)
            def payload(status):
                return {"alerts": [{"status": status, "labels": {"alertname": "DexMonitoringDrill",
                        "severity": "info", "service": "cl8y-dex-terraclassic-indexer",
                        "job": "cl8y-dex-monitor", "purpose": "monitoring", "actor": "private-actor"},
                        "annotations": {"private": "secret"}, "generatorURL": "https://private/"}]}
            state.receive(payload("firing"))
            self.now += 1
            state.receive(payload("resolved"))
            restored = exporter.AlertState(path, lambda: self.now)
            self.assertEqual(restored.recent(), state.recent())
            self.assertIn(b'dex_alert_notifications_total{status="firing"} 1', restored.metrics())
            self.assertIn(b'dex_alert_notifications_total{status="resolved"} 1', restored.metrics())
            serialized = str(restored.recent())
            self.assertNotIn("private", serialized)
            self.assertNotIn("secret", serialized)
            self.assertNotIn("actor", serialized)
            for _ in range(9):
                state.receive({"alerts": payload("firing")["alerts"] * 32})
            self.assertEqual(len(state.entries), 256)
            self.assertEqual(len(state.recent()["alerts"]), 32)
            self.assertEqual(len(exporter.AlertState(path).entries), 256)
            bad = payload("firing")
            bad["alerts"][0]["labels"]["service"] = "arbitrary-actor"
            with self.assertRaisesRegex(ValueError, "alert_label_value"):
                state.receive(bad)


if __name__ == "__main__":
    unittest.main()
