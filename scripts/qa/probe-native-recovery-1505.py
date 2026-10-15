#!/usr/bin/env python3
"""Bounded anonymous native quote recovery; no signing, retries or raw response export."""
import argparse
import importlib.util
import json
import math
import os
import re
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode

SPEC = importlib.util.spec_from_file_location("outcomes", Path(__file__).with_name("probe-issue-1473.py"))
OUTCOMES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OUTCOMES)
REQUEST_ID = re.compile(r"[0-9a-f]{16}")
MAX_READS = 256


def cases():
    assets = json.loads((OUTCOMES.ROOT / "docs/qa/issue-1473/matrix.json").read_text())["assets"]
    for offer, ask in (("USTC", "CL8Y"), ("CL8Y", "USTC")):
        yield {"pair": offer + "->" + ask, "token_in": assets[offer]["identity"],
               "token_out": assets[ask]["identity"], "amount": str(132 * 10 ** assets[offer]["decimals"]),
               "spread": "0.05"}


def evaluate(status, body, scenario, now_ms, request_id, seen_ids, require_correlation):
    """Only fixed codes, timing scalars and the server's opaque counter leave memory."""
    outcome, classified = OUTCOMES.classify(status, body, scenario, now_ms)
    valid_id = isinstance(request_id, str) and REQUEST_ID.fullmatch(request_id) is not None
    repeated = valid_id and request_id in seen_ids
    if valid_id:
        seen_ids.add(request_id)
    row = {"status": status, "outcome": outcome, "classified": classified,
           "useful": outcome == "executable" and classified,
           "request_id_present": request_id is not None,
           "request_id_valid": valid_id, "request_id_repeated": repeated}
    if valid_id:
        row["request_id"] = request_id
    row["correlation_pass"] = (valid_id and not repeated) if require_correlation else (request_id is None or (valid_id and not repeated))
    if row["useful"]:
        meta = body["execution_metadata"]
        row["observation"] = {k: meta[k] for k in ("observed_at_ms", "expires_at_ms")}
        row["observation"]["age_at_validation_ms"] = now_ms - meta["observed_at_ms"]
    elif status == 200:
        row["validation_diagnostic"] = OUTCOMES.validation_diagnostic(body, scenario, now_ms)
    return row


def summarize(rows, required_pairs):
    failures = []
    for pair in required_pairs:
        samples = [row for row in rows if row["pair"] == pair]
        useful = [row for row in samples if row.get("useful")]
        if not useful:
            failures.append({"pair": pair, "failure_class": "no_useful_output"})
        for row in samples:
            for field, failure in (("classified", "unclassified_outcome"), ("correlation_pass", "request_correlation")):
                if not row.get(field):
                    failures.append({"pair": pair, "sample": row["sample"], "failure_class": failure})
            if not row.get("useful"):
                failures.append({"pair": pair, "sample": row["sample"], "failure_class": "native_unavailable", "outcome": row["outcome"]})
            if row.get("useful") and row["transport"] == "reused" and not row["socket_budget_pass"]:
                failures.append({"pair": pair, "sample": row["sample"], "failure_class": "initialized_socket_over500"})
        if not any(row.get("useful") and row["transport"] == "reused" for row in samples):
            failures.append({"pair": pair, "failure_class": "no_initialized_useful_sample"})
        # A real original producer replacement must be observed within the run.
        if len({row["observation"]["observed_at_ms"] for row in useful}) < 2:
            failures.append({"pair": pair, "failure_class": "no_observation_replacement"})
    return failures


def probe(args):
    started = datetime.now(timezone.utc).isoformat()
    clock = time.monotonic()
    scenarios = list(cases())
    rows, failures, revisions, seen_ids, recovery = [], [], [], set(), []
    pending = {}
    conn = OUTCOMES.connection_for(args.base_url)
    reads = 0

    def health():
        nonlocal reads
        connection = OUTCOMES.connection_for(args.base_url)
        try:
            reads += 1
            revision = OUTCOMES.health_revision(connection, lambda _stage: None, lambda _status: None)
            revisions.append(revision)
            if revision != args.revision:
                raise ValueError("revision_mismatch")
        finally:
            connection.close()

    try:
        health()
        while time.monotonic() - clock < args.duration:
            for scenario in scenarios:
                if reads >= MAX_READS - 1:
                    raise ValueError("read_budget")
                row = {"pair": scenario["pair"], "sample": len(rows), "utc_start": datetime.now(timezone.utc).isoformat()}
                query = {"token_in": scenario["token_in"], "token_out": scenario["token_out"], "amount_in": scenario["amount"],
                         "pool_only": "true", "max_spread": scenario["spread"], "protection_policy": "quote_relative_v1"}
                begin = time.perf_counter()
                try:
                    reads += 1
                    transport = OUTCOMES.common.request_transport(conn, "/api/v1/route/solve?" + urlencode(query),
                        {"Accept": "application/json", "User-Agent": "CL8Y-issue1505-read-only-probe/1"})
                    response = conn.getresponse()
                    limit = OUTCOMES.common.MAX_BODY if response.status == 200 else 4096
                    payload = response.read(limit + 1)
                    if len(payload) > limit:
                        raise ValueError("body_bound")
                    body = json.loads(payload)
                    row.update(evaluate(response.status, body, scenario, int(time.time() * 1000),
                        response.getheader("x-quote-request-id"), seen_ids, args.require_correlation))
                    row["transport"] = transport
                except (ValueError, OSError, OUTCOMES.common.http.client.HTTPException) as error:
                    conn.close()
                    row.update(status=None, outcome="transport_or_shape_failure", classified=False, useful=False,
                        correlation_pass=False, transport="unobserved", failure_class=type(error).__name__)
                elapsed = (time.perf_counter() - begin) * 1000
                row.update(elapsed_ms=round(elapsed, 3), elapsed_from_run_start_ms=round((time.monotonic() - clock) * 1000, 3),
                    socket_budget_pass=row["useful"] and math.isfinite(elapsed) and 0 <= elapsed <= 500)
                if not row["useful"]:
                    pending.setdefault(row["pair"], row["elapsed_from_run_start_ms"])
                elif row["pair"] in pending:
                    recovery.append({"pair": row["pair"], "sample": row["sample"],
                        "unavailable_to_next_sampled_useful_ms": round(row["elapsed_from_run_start_ms"] - pending.pop(row["pair"]), 3)})
                rows.append(row)
            time.sleep(min(args.interval, max(0, args.duration - (time.monotonic() - clock))))
        health()
    except (ValueError, OSError, OUTCOMES.common.http.client.HTTPException) as error:
        failures.append({"failure_class": type(error).__name__, "phase": "provenance_or_budget"})
    finally:
        conn.close()
    failures.extend(summarize(rows, [case["pair"] for case in scenarios]))
    return {"issue": 1505, "revision": args.revision, "backend_revisions": revisions,
        "utc_start": started, "utc_end": datetime.now(timezone.utc).isoformat(), "read_only": True, "signing": False,
        "duration_seconds": args.duration, "interval_seconds": args.interval, "requests": reads,
        "socket_budget_ms": 500, "require_correlation": args.require_correlation,
        "measurement_clock": "original_request_start_through_body_parse_and_useful_validation; failed attempts retained; no retries",
        "initialized_useful_timing": OUTCOMES.common.distribution([r["elapsed_ms"] for r in rows if r.get("useful") and r["transport"] == "reused"]),
        "initialized_useful_timing_by_direction": {case["pair"]: OUTCOMES.common.distribution([r["elapsed_ms"] for r in rows if r.get("useful") and r["transport"] == "reused" and r["pair"] == case["pair"]]) for case in scenarios},
        "counts": {code: sum(r["outcome"] == code for r in rows) for code in sorted({r["outcome"] for r in rows})},
        "sampled_recoveries": recovery, "unrecovered_directions": sorted(pending), "samples": rows,
        "failures": failures, "overall": "FAIL" if failures else "PASS",
        "limits": ["two affected anonymous native directions at fixed original exact input; no broad matrix or signing",
            "sampling recovery intervals are polling observations, not original browser-input latency",
            "response validation rejects expired original observations; display expiry and browser budget require the independent browser observer",
            "request IDs correlate server logs only; provider cause requires separate bounded scheduled diagnostics"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="https://indexer.dex.cl8y.com")
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--duration", type=float, default=190)
    parser.add_argument("--interval", type=float, default=2)
    parser.add_argument("--require-correlation", action="store_true")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision) or not math.isfinite(args.duration) or not 21 <= args.duration <= 240 or not math.isfinite(args.interval) or not 1 <= args.interval <= 10:
        parser.error("exact revision, finite 21..240s duration and 1..10s interval required")
    if math.ceil(args.duration / args.interval) * 2 + 2 > MAX_READS:
        parser.error("read budget exceeded")
    receipt = probe(args)
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as output:
        os.chmod(args.output, 0o600)
        output.write(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps({k: receipt[k] for k in ("overall", "requests", "counts", "initialized_useful_timing")}))
    return int(receipt["overall"] != "PASS")


if __name__ == "__main__":
    raise SystemExit(main())
