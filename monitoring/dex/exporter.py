#!/usr/bin/env python3
"""Bounded, monitoring-only DEX health observations. /metrics never calls the DEX."""
import json
import math
import os
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import HTTPRedirectHandler, Request, build_opener

ORIGIN = "https://indexer.dex.cl8y.com"
ENDPOINTS = {
    "liveness": "/health",
    "serving": "/health/serving",
    "indexer_readiness": "/ready",
    "quote_readiness": "/ready-api",
}
INTERVAL_SECONDS = 30
TIMEOUT_SECONDS = 3
MAX_BODY_BYTES = 16_384
DRILL_SECONDS = 180
STATE_PATH = "/state/alerts.json"
STATUSES = frozenset(("starting", "running", "provider_unavailable", "halted", "stopped"))
ALERT_LABELS = {
    "alertname": frozenset(("DexScrapeMissing", "DexObservationStale", "DexServingUnavailable",
                            "DexIndexerLag", "DexIndexerProgressStalled", "DexQuoteUnavailable", "DexMonitoringDrill")),
    "severity": frozenset(("warning", "critical", "info")),
    "service": frozenset(("cl8y-dex-terraclassic-indexer",)),
    "job": frozenset(("cl8y-dex-api", "cl8y-dex-indexer", "cl8y-dex-monitor")),
    "purpose": frozenset((*ENDPOINTS, "monitoring")),
}


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


OPENER = build_opener(NoRedirect())


def fetch(path):
    """One fixed-origin GET, including bounded JSON bodies for readiness HTTP503."""
    request = Request(ORIGIN + path, headers={"Accept": "application/json", "Cache-Control": "no-cache"})
    try:
        response = OPENER.open(request, timeout=TIMEOUT_SECONDS)
    except HTTPError as error:
        response = error
    with response:
        status = response.code
        # Redirects are never followed, including HTTPS-to-HTTP and off-origin redirects.
        if status not in (200, 503):
            return status, None
        body = response.read(MAX_BODY_BYTES + 1)
        if len(body) > MAX_BODY_BYTES:
            raise ValueError("body_bound")
        return status, json.loads(body)


def height(value):
    if type(value) is not int or not 0 <= value <= 2**63 - 1:
        raise ValueError("height")
    return value


def validate(purpose, status, payload):
    if not isinstance(payload, dict):
        raise ValueError("shape")
    if purpose == "liveness":
        if status != 200 or payload.get("status") != "ok":
            raise ValueError("liveness")
    elif purpose == "serving":
        # Serving is a process-owned latch; it is independent from current quotes.
        if type(payload.get("admitted")) is not bool or (status == 200) != payload["admitted"]:
            raise ValueError("serving")
    else:
        if type(payload.get("ready")) is not bool or (status == 200) != payload["ready"]:
            raise ValueError("readiness")
        if purpose == "indexer_readiness":
            if payload.get("status") not in STATUSES:
                raise ValueError("indexer_status")
            height(payload.get("checkpoint_height"))
            if payload.get("latest_height") is not None:
                height(payload["latest_height"])
        elif type(payload.get("scenario_count")) is not int or not 0 <= payload["scenario_count"] <= 2:
            raise ValueError("scenario_count")


def sample(name, value, purpose=None):
    if not math.isfinite(value):
        raise ValueError("finite_metric")
    labels = '{purpose="' + purpose + '"}' if purpose else ""
    return f"{name}{labels} {value}"


class Observer:
    def __init__(self, read=fetch, clock=time.time, monotonic=time.monotonic):
        self.read = read
        self.clock = clock
        self.monotonic = monotonic
        self.lock = threading.Lock()
        self.cached = "# DEX observation has not completed yet.\n"
        self.last_checkpoint = None
        self.progress_at = None

    def poll(self):
        rows = []
        observations = []
        for purpose, path in ENDPOINTS.items():
            started = self.monotonic()
            status = None
            valid = False
            try:
                status, payload = self.read(path)
                validate(purpose, status, payload)
                valid = True
            except Exception:
                # Errors, exception bodies, arbitrary JSON fields and URLs never enter output.
                pass
            duration = max(0.0, self.monotonic() - started)
            rows.append(sample("dex_endpoint_duration_seconds", duration, purpose))
            rows.append(sample("dex_endpoint_up", int(valid and status == 200), purpose))
            if type(status) is int and 100 <= status <= 599:
                rows.append(sample("dex_endpoint_http_status", status, purpose))
            if not valid:
                # Omit unavailable observations instead of inventing heights or freshness.
                continue
            observed_at = self.clock()
            observations.append(observed_at)
            if purpose == "indexer_readiness":
                checkpoint = payload["checkpoint_height"]
                if self.last_checkpoint is None or self.last_checkpoint != checkpoint:
                    self.last_checkpoint, self.progress_at = checkpoint, observed_at
                rows.append(sample("dex_indexer_checkpoint_height", checkpoint))
                rows.append(sample("dex_indexer_progress_timestamp_seconds", self.progress_at))
                head = payload.get("latest_height")
                if head is not None:
                    rows.append(sample("dex_indexer_observed_head_height", head))
                    rows.append(sample("dex_indexer_lag_blocks", max(0, head - checkpoint)))
        if len(observations) == len(ENDPOINTS):
            rows.append(sample("dex_observation_timestamp_seconds", min(observations)))
        text = "\n".join(rows) + "\n"
        with self.lock:
            self.cached = text

    def metrics(self):
        with self.lock:
            return self.cached.encode("ascii")

    def run(self, stopped):
        # Waiting starts after the poll: at most four DEX reads in any 30-second period.
        while not stopped.is_set():
            self.poll()
            stopped.wait(INTERVAL_SECONDS)


class AlertState:
    """Private monitoring drill and delivery receipt; never retains arbitrary bodies."""
    def __init__(self, path=STATE_PATH, clock=time.time):
        self.path, self.clock = path, clock
        self.lock = threading.Lock()
        self.drill_until = 0
        self.entries = []
        try:
            with open(path, encoding="ascii") as source:
                body = source.read(131_073)
            if len(body) > 131_072:
                raise ValueError("state_bound")
            rows = json.loads(body)
            if not isinstance(rows, list) or len(rows) > 256:
                raise ValueError("state_shape")
            self.entries = [self.entry(row) for row in rows]
        except FileNotFoundError:
            pass

    @staticmethod
    def entry(row):
        if not isinstance(row, dict) or set(row) != {"status", "timestamp", "labels"}:
            raise ValueError("alert_shape")
        if row["status"] not in ("firing", "resolved"):
            raise ValueError("alert_status")
        timestamp = row["timestamp"]
        if type(timestamp) not in (float, int) or not math.isfinite(timestamp) or timestamp <= 0:
            raise ValueError("alert_timestamp")
        labels = row["labels"]
        if not isinstance(labels, dict) or set(labels) - {"alertname", "severity", "service", "job", "purpose"}:
            raise ValueError("alert_labels")
        for key, value in labels.items():
            if not isinstance(value, str) or value not in ALERT_LABELS[key]:
                raise ValueError("alert_label_value")
        if "alertname" not in labels:
            raise ValueError("alert_identity")
        return {"status": row["status"], "timestamp": timestamp, "labels": labels}

    def set_drill(self, enabled):
        if type(enabled) is not bool:
            raise ValueError("drill_shape")
        with self.lock:
            self.drill_until = self.clock() + DRILL_SECONDS if enabled else 0
            return {"enabled": self.drill_until > self.clock()}

    def receive(self, payload):
        if not isinstance(payload, dict) or not isinstance(payload.get("alerts"), list) or not 1 <= len(payload["alerts"]) <= 32:
            raise ValueError("alerts_bound")
        rows = []
        for alert in payload["alerts"]:
            if not isinstance(alert, dict) or not isinstance(alert.get("labels"), dict):
                raise ValueError("alert_shape")
            # Ignore annotations, URLs, fingerprints, arbitrary labels and request metadata.
            labels = {key: value for key, value in alert["labels"].items()
                      if key in {"alertname", "severity", "service", "job", "purpose"}}
            rows.append(self.entry({"status": alert.get("status"), "timestamp": self.clock(), "labels": labels}))
        with self.lock:
            entries = (self.entries + rows)[-256:]
            os.makedirs(os.path.dirname(os.path.abspath(self.path)), exist_ok=True)
            temporary = self.path + ".tmp"
            with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w", encoding="ascii") as output:
                json.dump(entries, output, separators=(",", ":"))
                output.flush()
                os.fsync(output.fileno())
            os.replace(temporary, self.path)
            self.entries = entries
        return {"accepted": len(rows)}

    def recent(self):
        with self.lock:
            return {"alerts": self.entries[-32:]}

    def metrics(self):
        with self.lock:
            rows = [sample("dex_monitoring_drill", int(self.drill_until > self.clock()))]
            for status in ("firing", "resolved"):
                selected = [row for row in self.entries if row["status"] == status]
                # Receipt-window counts are gauges, not lifetime counters.
                rows.append(f'dex_alert_notifications_total{{status="{status}"}} {len(selected)}')
            if self.entries:
                rows.append(sample("dex_alert_last_notification_timestamp_seconds", self.entries[-1]["timestamp"]))
            return ("\n".join(rows) + "\n").encode("ascii")


def handler(observer, alerts=None):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path != "/metrics":
                self.send_error(404)
                return
            body = observer.metrics() + (alerts.metrics() if alerts else b"")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args):
            pass
    return Handler


def admin_handler(alerts):
    class Handler(BaseHTTPRequestHandler):
        def reply(self, status, payload):
            body = json.dumps(payload, separators=(",", ":")).encode("ascii")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path == "/alerts":
                self.reply(200, alerts.recent())
            else:
                self.reply(404, {"error": "not_found"})

        def do_POST(self):
            self.connection.settimeout(3)
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if not 1 <= length <= MAX_BODY_BYTES or self.headers.get("Transfer-Encoding"):
                    raise ValueError("body_bound")
                payload = json.loads(self.rfile.read(length))
                if self.path == "/drill" and isinstance(payload, dict) and set(payload) == {"enabled"}:
                    result = alerts.set_drill(payload["enabled"])
                elif self.path == "/alerts":
                    result = alerts.receive(payload)
                else:
                    self.reply(404, {"error": "not_found"})
                    return
                self.reply(200, result)
            except (ValueError, TypeError, OSError):
                self.reply(400, {"error": "invalid_request"})

        def log_message(self, *_args):
            pass
    return Handler


def main():
    observer = Observer()
    alerts = AlertState()
    stopped = threading.Event()
    worker = threading.Thread(target=observer.run, args=(stopped,), daemon=True)
    worker.start()
    server = ThreadingHTTPServer(("0.0.0.0", 9800), handler(observer, alerts))
    admin = ThreadingHTTPServer(("0.0.0.0", 9801), admin_handler(alerts))
    threading.Thread(target=admin.serve_forever, daemon=True).start()
    try:
        server.serve_forever()
    finally:
        stopped.set()
        server.server_close()
        admin.shutdown()
        admin.server_close()


if __name__ == "__main__":
    main()
