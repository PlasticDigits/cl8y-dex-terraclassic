#!/usr/bin/env python3
"""Fixed bounded Grafana/Loki stage and HTTP outcome counts; no raw log lines."""
import argparse
import importlib.util
import json
import os
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote, urlencode

SPEC = importlib.util.spec_from_file_location("monitor1489", Path(__file__).with_name("probe-swap-monitoring-1479-1480.py"))
COMMON = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMMON)


def run():
    monitor = COMMON.Monitor()
    sources = monitor.read("/api/datasources")
    if not isinstance(sources, list) or len(sources) > 32:
        raise ValueError("datasource_bound")
    loki = [source for source in sources if isinstance(source, dict) and source.get("type") == "loki"]
    if len(loki) != 1 or not COMMON.ATTESTER.APP_ID.fullmatch(str(loki[0].get("uid", ""))):
        raise ValueError("loki_identity")
    prefix = "/api/datasources/proxy/uid/" + quote(loki[0]["uid"], safe="") + "/loki/api/v1/query?"
    counts = {}
    # Fixed classifications avoid publishing arbitrary messages, actor labels or
    # endpoints; access entries containing 'error' and HTTP200 are not failures.
    predicates = {
        "indexer_error_lines": ('cl8y-dex-terraclassic-indexer', ' |~ "(?i)(error|panic|fatal)"'),
        "indexer_capacity_lines": ('cl8y-dex-terraclassic-indexer', ' |~ "(?i)(capacity|429|cooldown)"'),
        "frontend_server_failure_lines": ('cl8y-dex-frontend', ' |~ "(?i)(\\\\[error\\\\]|\\\\[emerg\\\\]|\\\\[crit\\\\]|\\\" (404|5[0-9][0-9]) )"'),
        "frontend_http200_lines": ('cl8y-dex-frontend', ' |= "\\\" 200 "'),
    }
    for stage in ("queued", "evaluated", "promoted", "discarded"):
        predicates["route_discovery_" + stage] = ('cl8y-dex-terraclassic-indexer', ' |= "route_discovery" |~ "stage[=\\\": ]+' + stage + '"')
    for label, (service, predicate) in predicates.items():
        query = 'sum(count_over_time({service="' + service + '"}' + predicate + '[15m]))'
        counts[label] = COMMON.aggregate(monitor.read(prefix + urlencode({"query": query})))
    prom = COMMON.collect_prometheus(monitor, sources)
    return {"issue": 1489, "utc": datetime.now(timezone.utc).isoformat(), "read_only": True,
            "window_seconds": 900, "counts": counts, "prometheus": prom,
            "overall": "FAIL" if counts["indexer_error_lines"] or counts["frontend_server_failure_lines"] else "PASS",
            "scope": "Fixed text-pattern aggregate counts only; capacity/stage counts do not prove cause, readiness, stage coverage or settlement."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    try:
        result = run()
    except Exception as error:
        result = {"issue": 1489, "overall": "FAIL", "failure_class": type(error).__name__}
    with os.fdopen(os.open(args.output, os.O_CREAT | os.O_TRUNC | os.O_WRONLY, 0o600), "w") as handle:
        handle.write(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result.get(key) for key in ("overall", "counts", "failure_class")}))
    return int(result["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
