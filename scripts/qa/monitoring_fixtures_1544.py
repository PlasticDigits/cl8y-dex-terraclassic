"""Synthetic fixtures shared by the bounded monitoring probe tests."""
import time


def prometheus_payload(module, *, now=None, readiness=1):
    now = time.time() if now is None else now
    rows = []
    for job, metric, purpose in sorted(module.EXPECTED_METRICS):
        value = now - 10 if metric == "dex_observation_timestamp_seconds" else 1
        if purpose in ("quote_readiness", "indexer_readiness"):
            value = readiness
        for check, sample in (("value", value), ("timestamp", now - 5)):
            labels = {"job": job, "service": module.DEX_SERVICE, "monitor_metric": metric,
                      "monitor_check": check, "instance": module.DEX_INSTANCE}
            if purpose:
                labels["purpose"] = purpose
            rows.append({"metric": labels, "value": [now, str(sample)]})
    return {"status": "success", "data": {"resultType": "vector", "result": rows}}


def grafana_frames(payload):
    return {"results": {"A": {"status": 200, "frames": [
        {"schema": {"fields": [{"name": "Time", "type": "time"},
                               {"name": "Value", "type": "number", "labels": row["metric"]}]},
         "data": {"values": [[row["value"][0] * 1000], [float(row["value"][1])]]}}
        for row in payload["data"]["result"]]}}}
