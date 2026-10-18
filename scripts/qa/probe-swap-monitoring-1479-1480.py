#!/usr/bin/env python3
"""Read bounded Grafana/Loki aggregates without publishing credentials or log lines."""
import argparse
import base64
import importlib.util
import json
import math
import os
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote, urlencode
from urllib.request import Request

SPEC = importlib.util.spec_from_file_location("deployment_attester", Path(__file__).with_name("attest-trading-quotes-1473.py"))
ATTESTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ATTESTER)
SERVICES = ("cl8y-dex-terraclassic-indexer", "cl8y-dex-frontend")
GRAFANA_ORIGIN = "https://logs.cl8y.com"
# Native correlation plus datasource health/scoped metrics uses 19 reads.
# Existing monitoring profiles retain their original default allowance.
MAX_MONITOR_READS = 19
PROMETHEUS_UID = "cl8y-dex-prometheus"
PROMETHEUS_URL = "http://dex-prometheus:9090"
DEX_INSTANCE = "dex-exporter:9800"
DEX_SERVICE = "cl8y-dex-terraclassic-indexer"
DEX_JOBS = ("cl8y-dex-api", "cl8y-dex-indexer")
MAX_SAMPLE_AGE_SECONDS = 90
# Fixed low-cardinality identities. Readiness is observed separately from serving.
EXPECTED_METRICS = {
    (job, metric, "") for job in DEX_JOBS
    for metric in ("up", "dex_observation_timestamp_seconds")
} | {
    ("cl8y-dex-api", "dex_endpoint_up", purpose)
    for purpose in ("liveness", "serving", "quote_readiness")
} | {("cl8y-dex-indexer", "dex_endpoint_up", "indexer_readiness")}
PROM_SELECTOR = ('{__name__=~"up|dex_observation_timestamp_seconds|dex_endpoint_up",'
                 'job=~"cl8y-dex-api|cl8y-dex-indexer",service="' + DEX_SERVICE + '"}')
# timestamp() drops __name__; preserve the metric identity before applying it.
PROM_NAMED_SELECTOR = 'label_replace(' + PROM_SELECTOR + ',"monitor_metric","$1","__name__","(.+)")'
PROM_QUERY = ('label_replace(' + PROM_NAMED_SELECTOR + ',"monitor_check","value","","") or '
              'label_replace(timestamp(' + PROM_NAMED_SELECTOR + '),"monitor_check","timestamp","","")')


def aggregate(payload):
    """Only a successful finite scalar/vector sum enters the public receipt."""
    if not isinstance(payload, dict) or payload.get("status") != "success":
        raise ValueError("aggregate_unavailable")
    data = payload.get("data", {})
    if not isinstance(data, dict) or data.get("resultType") != "vector":
        raise ValueError("aggregate_shape")
    rows = data.get("result")
    if not isinstance(rows, list) or len(rows) > 32:
        raise ValueError("aggregate_bound")
    total = 0.0
    for row in rows:
        sample = row.get("value") if isinstance(row, dict) else None
        if not isinstance(sample, list) or len(sample) != 2:
            raise ValueError("aggregate_shape")
        value = float(sample[1])
        if not math.isfinite(value) or value < 0:
            raise ValueError("aggregate_value")
        total += value
    if not math.isfinite(total):
        raise ValueError("aggregate_value")
    return total


def prometheus_frames(payload):
    """Normalize Grafana's bounded instant frames without exposing metadata."""
    if not isinstance(payload, dict) or not isinstance(payload.get("results"), dict) or set(payload["results"]) != {"A"}:
        raise ValueError("prometheus_query")
    result = payload["results"]["A"]
    if not isinstance(result, dict) or result.get("error") or result.get("status", 200) != 200:
        raise ValueError("prometheus_query")
    frames = result.get("frames")
    if not isinstance(frames, list) or len(frames) != 2 * len(EXPECTED_METRICS):
        raise ValueError("prometheus_coverage")
    rows = []
    for frame in frames:
        if not isinstance(frame, dict) or not isinstance(frame.get("schema"), dict) or not isinstance(frame.get("data"), dict):
            raise ValueError("prometheus_shape")
        fields, values = frame["schema"].get("fields"), frame["data"].get("values")
        if not isinstance(fields, list) or len(fields) != 2 or not isinstance(values, list) or len(values) != 2:
            raise ValueError("prometheus_shape")
        if any(not isinstance(field, dict) for field in fields) or [field.get("type") for field in fields] != ["time", "number"]:
            raise ValueError("prometheus_shape")
        if any(not isinstance(value, list) or len(value) != 1 for value in values):
            raise ValueError("prometheus_shape")
        rows.append({"metric": fields[1].get("labels"), "value": [float(values[0][0]) / 1000, values[1][0]]})
    return {"status": "success", "data": {"resultType": "vector", "result": rows}}


def collect_prometheus(instance, sources, *, now=None):
    """Require this deployment's datasource and every intended fresh DEX series.

    Receipts contain fixed identities and finite values only. Datasource payloads,
    target instances and any unexpected labels are never copied into evidence.
    Exactly two reads preserve the explicit native monitoring profile's hard cap.
    """
    prometheus = [row for row in sources if isinstance(row, dict) and row.get("type") == "prometheus"]
    if (len(prometheus) != 1 or prometheus[0].get("uid") != PROMETHEUS_UID
            or prometheus[0].get("url") != PROMETHEUS_URL):
        raise ValueError("prometheus_identity")
    uid = quote(PROMETHEUS_UID, safe="")
    health = instance.read("/api/datasources/uid/" + uid + "/health")
    if not isinstance(health, dict) or health.get("status") != "OK":
        raise ValueError("prometheus_health")
    payload = instance.query_prometheus(PROM_QUERY)
    if not isinstance(payload, dict) or payload.get("status") != "success":
        raise ValueError("prometheus_query")
    data = payload.get("data")
    if not isinstance(data, dict) or data.get("resultType") != "vector":
        raise ValueError("prometheus_shape")
    rows = data.get("result")
    if not isinstance(rows, list) or len(rows) != 2 * len(EXPECTED_METRICS):
        raise ValueError("prometheus_coverage")
    now = time.time() if now is None else now
    values = {}
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("metric"), dict):
            raise ValueError("prometheus_shape")
        labels, sample = row["metric"], row.get("value")
        identity = (labels.get("job"), labels.get("monitor_metric"), labels.get("purpose", ""))
        check = labels.get("monitor_check")
        if (identity not in EXPECTED_METRICS or check not in ("value", "timestamp")
                or labels.get("service") != DEX_SERVICE or labels.get("instance") != DEX_INSTANCE):
            raise ValueError("prometheus_target_identity")
        if set(labels) - {"__name__", "instance", "job", "service", "purpose", "monitor_metric", "monitor_check"}:
            raise ValueError("prometheus_labels")
        if (identity, check) in values:
            raise ValueError("prometheus_duplicate")
        if not isinstance(sample, list) or len(sample) != 2:
            raise ValueError("prometheus_shape")
        value, evaluated = float(sample[1]), float(sample[0])
        if not math.isfinite(value) or not math.isfinite(evaluated):
            raise ValueError("prometheus_value")
        if not -5 <= now - evaluated <= MAX_SAMPLE_AGE_SECONDS:
            raise ValueError("prometheus_evaluation_stale")
        if check == "timestamp" or identity[1] == "dex_observation_timestamp_seconds":
            if not -5 <= now - value <= MAX_SAMPLE_AGE_SECONDS:
                raise ValueError("prometheus_sample_stale")
        elif value not in (0, 1):
            raise ValueError("prometheus_value")
        values[identity, check] = value
    if set(values) != {(identity, check) for identity in EXPECTED_METRICS for check in ("value", "timestamp")}:
        raise ValueError("prometheus_coverage")
    targets = {}
    for job in DEX_JOBS:
        if values[(job, "up", ""), "value"] != 1:
            raise ValueError("prometheus_scrape_down")
        purposes = {identity[2]: values[identity, "value"] for identity in EXPECTED_METRICS
                    if identity[0] == job and identity[1] == "dex_endpoint_up"}
        if job == "cl8y-dex-api" and any(purposes[purpose] != 1 for purpose in ("liveness", "serving")):
            raise ValueError("prometheus_serving_down")
        targets[job] = {"scrape_up": 1, "observation_age_seconds": round(max(0, now - values[(job, "dex_observation_timestamp_seconds", ""), "value"]), 3),
                        "endpoint_up": purposes}
    return {"status": "PASS", "datasource_uid": PROMETHEUS_UID, "datasource_health": "OK",
            "max_sample_age_seconds": MAX_SAMPLE_AGE_SECONDS, "queries": [PROM_QUERY], "targets": targets,
            "scope": "Scrape and serving availability; readiness values remain independent and do not certify quote acceptance."}


class Monitor:
    read_budget = ATTESTER.MAX_READS

    def __init__(self, *, read_budget=ATTESTER.MAX_READS):
        if type(read_budget) is not int or read_budget not in (ATTESTER.MAX_READS, MAX_MONITOR_READS):
            raise ValueError("read_budget")
        self.read_budget = read_budget
        self.reader = ATTESTER.Reader()
        apps = ATTESTER.rows(self.reader.api("applications"))
        if len(apps) > 512:
            raise ValueError("inventory_bound")
        selected = [row.get("uuid") for row in apps if isinstance(row, dict) and row.get("name") == "cl8y-logs"]
        if len(selected) != 1 or not isinstance(selected[0], str) or not ATTESTER.APP_ID.fullmatch(selected[0]):
            raise ValueError("monitor_identity")
        envs = self.reader.api("applications/" + quote(selected[0], safe="") + "/envs")
        if not isinstance(envs, list) or len(envs) > 512:
            raise ValueError("environment_bound")
        values = {row["key"]: row.get("value") for row in envs if isinstance(row, dict) and not row.get("is_preview") and row.get("key") in {"GRAFANA_ADMIN_USER", "GRAFANA_ADMIN_PASSWORD"}}
        username, password = values.get("GRAFANA_ADMIN_USER"), values.get("GRAFANA_ADMIN_PASSWORD")
        if not isinstance(username, str) or not isinstance(password, str) or not username or not password:
            raise ValueError("monitor_credentials")
        self.authorization = "Basic " + base64.b64encode((username + ":" + password).encode()).decode()

    def read(self, path, *, payload=None):
        self.reader.reads += 1
        if self.reader.reads > self.read_budget:
            raise ValueError("read_budget")
        body = None if payload is None else json.dumps(payload).encode()
        request = Request(GRAFANA_ORIGIN + path, data=body, headers={"Authorization": self.authorization, "Accept": "application/json", "Cache-Control": "no-cache", "Content-Type": "application/json"})
        with self.reader.opener.open(request, timeout=15) as response:
            data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("body_bound")
        return json.loads(data)

    def query_prometheus(self, expression):
        # Existing logs.cl8y.com proxy permits only Loki datasource proxy paths.
        # Grafana's authenticated query API preserves that boundary unchanged.
        now_ms = int(time.time() * 1000)
        payload = {"from": str(now_ms - 60000), "to": str(now_ms), "queries": [{
            "refId": "A", "datasource": {"type": "prometheus", "uid": PROMETHEUS_UID},
            "expr": expression, "instant": True, "range": False, "format": "time_series",
            "intervalMs": 15000, "maxDataPoints": 1}]}
        return prometheus_frames(self.read("/api/ds/query", payload=payload))

    def collect(self):
        sources = self.read("/api/datasources")
        if not isinstance(sources, list) or len(sources) > 32:
            raise ValueError("datasource_bound")
        loki = [row for row in sources if isinstance(row, dict) and row.get("type") == "loki"]
        if len(loki) != 1 or not isinstance(loki[0].get("uid"), str) or not ATTESTER.APP_ID.fullmatch(loki[0]["uid"]):
            raise ValueError("loki_identity")
        prefix = "/api/datasources/proxy/uid/" + quote(loki[0]["uid"], safe="") + "/loki/api/v1/query?"
        counts = {}
        for service in SERVICES:
            row = {}
            for label, predicate in (("lines", ""), ("error_pattern_lines", ' |~ "(?i)(error|panic|fatal)"'), ("provider_capacity_pattern_lines", ' |~ "(?i)(capacity|429|cooldown)"')):
                query = 'sum(count_over_time({service="' + service + '"}' + predicate + '[15m]))'
                row[label] = aggregate(self.read(prefix + urlencode({"query": query})))
            counts[service] = row
        prom = collect_prometheus(self, sources)
        return {"loki": {"status": "PASS", "window_seconds": 900, "counts": counts}, "prometheus": prom, "read_count": self.reader.reads}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    receipt = {"issues": [1479, 1480], "utc": datetime.now(timezone.utc).isoformat(), "read_only": True, "scope": "15-minute text-pattern aggregates; no log lines, actor identifiers, raw bodies or endpoints; counts alone do not prove quote or deployment readiness"}
    try:
        receipt.update(Monitor().collect())
        receipt["overall"] = "PASS"
    except Exception as error:
        receipt.update(overall="FAIL", failure_class=type(error).__name__)
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as output:
        os.chmod(args.output, 0o600)
        output.write(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({key: receipt[key] for key in ("overall", "utc")}))
    return int(receipt["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
