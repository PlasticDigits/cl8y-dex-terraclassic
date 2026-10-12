#!/usr/bin/env python3
"""Read bounded Grafana/Loki aggregates without publishing credentials or log lines."""
import argparse
import base64
import importlib.util
import json
import math
import os
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote, urlencode
from urllib.request import Request

SPEC = importlib.util.spec_from_file_location("deployment_attester", Path(__file__).with_name("attest-trading-quotes-1473.py"))
ATTESTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ATTESTER)
SERVICES = ("cl8y-dex-terraclassic-indexer", "cl8y-dex-frontend")
GRAFANA_ORIGIN = "https://logs.cl8y.com"


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


class Monitor:
    def __init__(self):
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

    def read(self, path):
        self.reader.reads += 1
        if self.reader.reads > ATTESTER.MAX_READS:
            raise ValueError("read_budget")
        request = Request(GRAFANA_ORIGIN + path, headers={"Authorization": self.authorization, "Accept": "application/json", "Cache-Control": "no-cache"})
        with self.reader.opener.open(request, timeout=15) as response:
            data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("body_bound")
        return json.loads(data)

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
        prometheus = [row for row in sources if isinstance(row, dict) and row.get("type") == "prometheus"]
        prom = {"status": "SKIP", "reason": "no_prometheus_datasource_provisioned"}
        if len(prometheus) == 1 and isinstance(prometheus[0].get("uid"), str) and ATTESTER.APP_ID.fullmatch(prometheus[0]["uid"]):
            prefix = "/api/datasources/proxy/uid/" + quote(prometheus[0]["uid"], safe="") + "/api/v1/query?"
            prom = {"status": "PASS", "up_sum": aggregate(self.read(prefix + urlencode({"query": "sum(up)"})))}
        return {"loki": {"status": "PASS", "window_seconds": 900, "counts": counts}, "prometheus": prom, "read_count": self.reader.reads}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    receipt = {"issues": [1479, 1480], "utc": datetime.now(timezone.utc).isoformat(), "read_only": True, "scope": "15-minute text-pattern aggregates; no log lines, actor identifiers, raw bodies or endpoints; counts alone do not prove quote or deployment readiness"}
    try:
        receipt.update(Monitor().collect())
        receipt["overall"] = "PASS WITH SKIPS" if receipt["prometheus"]["status"] == "SKIP" else "PASS"
    except Exception as error:
        receipt.update(overall="FAIL", failure_class=type(error).__name__)
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as output:
        os.chmod(args.output, 0o600)
        output.write(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({key: receipt[key] for key in ("overall", "utc")}))
    return int(receipt["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
