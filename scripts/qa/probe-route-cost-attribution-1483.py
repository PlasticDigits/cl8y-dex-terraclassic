#!/usr/bin/env python3
"""Bounded anonymous live economics checks; never signing or settlement evidence."""
import argparse
import http.client
import importlib.util
import json
import re
import signal
import time
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("outcomes", Path(__file__).with_name("probe-issue-1473.py"))
OUTCOMES = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OUTCOMES)
COMPONENTS = ("wrapper_loss", "trading_cost", "market_movement", "reference_divergence")
RAW_MAX = (1 << 128) - 1
HOST = "indexer.dex.cl8y.com"
REQUEST_TIMEOUT_SECONDS = 10


def deadline(_signal, _frame):
    raise TimeoutError("request_deadline")


def health_revision(status, body):
    if status != 200 or not isinstance(body, dict):
        raise ValueError("health_shape")
    revision = body.get("git_sha")
    if not isinstance(revision, str) or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise ValueError("health_revision_shape")
    return revision


def raw(value, signed=False):
    pattern = r"0|-?[1-9][0-9]{0,38}" if signed else r"0|[1-9][0-9]{0,38}"
    if not isinstance(value, str) or not re.fullmatch(pattern, value) or abs(int(value)) > RAW_MAX:
        raise ValueError("raw_shape")
    return int(value)


def quoted_outputs(body):
    net = raw(body["estimated_amount_out_net"])
    fair = raw(body["spot_amount_out"])
    if net <= 0 or fair <= 0:
        raise ValueError("quote_output_shape")
    return str(net), str(fair)


def attribution(body):
    """Check raw reconciliation independently; no received copy or actor fields survive."""
    a = body.get("economic_attribution")
    if not isinstance(a, dict):
        raise ValueError("attribution_missing")
    if type(a.get("version")) is int and a == {"version": 1, "status": "unavailable", "method": "spot_bridge_v1"}:
        return {"status": "unavailable"}
    expected = {"version", "status", "method", "fair_amount_out", "net_amount_out", "generation",
                "observed_at_ms", "expires_at_ms", "components", "dominant_cause"}
    if set(a) != expected or type(a["version"]) is not int or a["version"] != 1 or a["status"] != "available" or a["method"] != "spot_bridge_v1":
        raise ValueError("attribution_shape")
    meta = body["execution_metadata"]
    if any(a[k] != meta[k] for k in ("generation", "observed_at_ms", "expires_at_ms")):
        raise ValueError("attribution_cohort")
    fair, net = raw(a["fair_amount_out"]), raw(a["net_amount_out"])
    if fair <= 0 or net <= 0 or a["fair_amount_out"] != body["spot_amount_out"] or a["net_amount_out"] != body["estimated_amount_out_net"]:
        raise ValueError("attribution_economics")
    if not isinstance(a["components"], dict) or set(a["components"]) != set(COMPONENTS):
        raise ValueError("attribution_components")
    parts = {key: raw(a["components"][key], True) for key in COMPONENTS}
    if sum(parts.values()) != fair - net:
        raise ValueError("attribution_sum")
    maximum = max(abs(value) for value in parts.values())
    winners = [key for key, value in parts.items() if abs(value) == maximum]
    cause = winners[0] if len(winners) == 1 else "mixed"
    if cause != a["dominant_cause"]:
        raise ValueError("attribution_cause")
    deviation = Decimal(abs(fair - net)) * 100 / Decimal(max(fair, net))
    return {"status": "available", "dominant_cause": cause, "components": a["components"],
            "fair_amount_out": str(fair), "net_amount_out": str(net), "deviation_percent": str(deviation)}


def scenarios():
    assets = json.loads((ROOT / "docs/qa/issue-1473/matrix.json").read_text())["assets"]
    assets.update(json.loads((ROOT / "docs/qa/issue-1479/matrix.json").read_text())["assets"])
    amounts = ["0.01", "0.1", "1", "10", "100", "1000", "4000", "5000", "10000"]
    for amount in amounts:
        native = int(Decimal(amount) * 10**6)
        for asset, value, control in (("LUNC", native, "native_gross"), ("cLUNC", native * 98 // 100, "matched_credit_98pct"), ("cLUNC", native, "equal_gross")):
            yield assets, asset, "USTR", str(value), amount, control
    for pair in (("CL8Y", "USTR"), ("CL8Y", "USTC"), ("USTC", "USTR"), ("CL8Y", "UST1"), ("UST1", "USTR"), ("LUNC", "USTR"), ("cLUNC", "USTR"), ("KENA", "cLUNC")):
        for offer, ask in (pair, pair[::-1]):
            human = "0.1" if offer == "CL8Y" else "1"
            yield assets, offer, ask, str(int(Decimal(human) * 10**assets[offer]["decimals"])), human, "bidirectional_control"


def probe(revision, require_attribution, cycles):
    started = datetime.now(timezone.utc).isoformat()
    rows, failures, comparisons, revisions = [], [], [], []
    connection = http.client.HTTPSConnection(HOST, timeout=10)
    requests = 0

    def fetch(path):
        nonlocal requests
        requests += 1
        if requests > 320 or time.monotonic() - clock > 1200:
            raise ValueError("request_budget")
        begin = time.perf_counter()
        signal.setitimer(signal.ITIMER_REAL, min(REQUEST_TIMEOUT_SECONDS, 1200 - (time.monotonic() - clock)))
        try:
            connection.request("GET", path, headers={"Accept": "application/json"})
            response = connection.getresponse()
            payload = response.read(2 * 1024 * 1024 + 1)
            elapsed = (time.perf_counter() - begin) * 1000
        finally:
            signal.setitimer(signal.ITIMER_REAL, 0)
        if len(payload) > (2 * 1024 * 1024 if response.status == 200 else 4096):
            raise ValueError("body_budget")
        return response.status, json.loads(payload), elapsed

    clock = time.monotonic()
    signal.signal(signal.SIGALRM, deadline)
    for cycle in range(cycles):
        if cycle:
            # Independent producer replacement, including the configuration TTL.
            connection.close()
            if time.monotonic() - clock + 185 >= 1200:
                failures.append({"cycle": cycle, "phase": "replacement_gap", "failure_class": "elapsed_budget"})
                break
            time.sleep(185)
            connection = http.client.HTTPSConnection(HOST, timeout=10)
        aborted = False
        for phase in ("before", "after"):
            try:
                status, health, _ = fetch("/health")
                revisions.append(health_revision(status, health))
                if revisions[-1] != revision:
                    raise ValueError("revision_mismatch")
            except (ValueError, KeyError, TypeError, OSError, http.client.HTTPException) as error:
                failures.append({"cycle": cycle, "phase": "health_" + phase, "failure_class": type(error).__name__})
                aborted = True
                break
            if phase == "after":
                continue
            for assets, offer, ask, amount, human, control in scenarios():
                scenario = {"token_in": assets[offer]["identity"], "token_out": assets[ask]["identity"], "amount": amount, "spread": "0.05"}
                query = {"token_in": scenario["token_in"], "token_out": scenario["token_out"], "amount_in": amount,
                         "pool_only": "true", "max_spread": "0.05", "protection_policy": "quote_relative_v1"}
                observed = []
                for endpoint in ("solve", "best"):
                    suffix = "" if endpoint == "solve" else "/best"
                    row = {"cycle": cycle, "endpoint": endpoint, "pair": offer + "->" + ask, "amount": amount, "human_amount": human, "control": control}
                    begin = time.perf_counter()
                    try:
                        status, body, _transport_elapsed = fetch("/api/v1/route/solve" + suffix + "?" + urlencode(query))
                        outcome, valid = OUTCOMES.classify(status, body, scenario, int(time.time() * 1000))
                        row.update(status=status, outcome=outcome, valid=valid)
                        if outcome == "executable":
                            meta = body["execution_metadata"]
                            net_output, fair_output = quoted_outputs(body)
                            row.update(generation=meta["generation"], observed_at_ms=meta["observed_at_ms"], expires_at_ms=meta["expires_at_ms"],
                                       net_amount_out=net_output, fair_amount_out=fair_output, hop_count=len(meta["hops"]))
                            if require_attribution:
                                row["attribution"] = attribution(body)
                            observed.append(body)
                        elapsed = (time.perf_counter() - begin) * 1000
                        row["elapsed_ms"] = round(elapsed, 3)
                        if outcome == "executable":
                            row["socket_budget_pass"] = elapsed <= 500
                        if not valid:
                            failures.append({"cycle": cycle, "pair": row["pair"], "amount": amount, "endpoint": endpoint, "failure_class": outcome})
                        elif offer == "LUNC" and ask == "USTR" and control == "native_gross" and human in ("1", "1000", "4000") and outcome != "executable":
                            failures.append({"cycle": cycle, "pair": row["pair"], "amount": amount, "endpoint": endpoint, "failure_class": "required_executable_unavailable", "outcome": outcome})
                    except (ValueError, KeyError, TypeError, OSError, http.client.HTTPException) as error:
                        row.update(valid=False, failure_class=type(error).__name__, elapsed_ms=round((time.perf_counter() - begin) * 1000, 3))
                        failures.append({"cycle": cycle, "pair": row["pair"], "amount": amount, "endpoint": endpoint, "failure_class": type(error).__name__})
                        connection.close()
                    rows.append(row)
                    time.sleep(0.1)
                if len(observed) == 2:
                    first, best = observed
                    same = first["execution_metadata"]["generation"] == best["execution_metadata"]["generation"]
                    comparison = {"cycle": cycle, "pair": offer + "->" + ask, "amount": amount, "same_generation": same,
                                  "best_not_worse": int(best["estimated_amount_out_net"]) >= int(first["estimated_amount_out_net"])}
                    comparisons.append(comparison)
                    if same and not comparison["best_not_worse"]:
                        failures.append({**comparison, "failure_class": "best_worse"})
        if aborted:
            break
    connection.close()
    useful = [row["elapsed_ms"] for row in rows if row.get("outcome") == "executable" and row.get("valid")]
    return {"issue": 1483, "revision": revision, "utc_start": started, "utc_end": datetime.now(timezone.utc).isoformat(),
            "cycles": cycles, "replacement_gap_seconds": 185 if cycles > 1 else 0, "requests": requests, "backend_revisions": revisions,
            "read_only": True, "signing": False, "socket_budget_ms": 500, "measurement_clock": "request_start_through_parsing_and_useful_result_validation", "useful_socket_timing": OUTCOMES.common.distribution(useful),
            "failures": failures, "comparisons": comparisons, "samples": rows,
            "limits": ["anonymous public quotes; no reserve/SQL/contract attestation or settlement", "98% matching control is configured test input, not a discovery or historical reproduction", "different-generation responses never establish same-cohort equivalence"],
            "overall": "FAIL" if failures or not useful or any(row.get("socket_budget_pass") is False for row in rows) else "PASS WITH SKIPS"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--cycles", type=int, choices=(1, 2), default=2)
    parser.add_argument("--require-attribution", action="store_true")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("Exact revision required")
    try:
        result = probe(args.revision, args.require_attribution, args.cycles)
    except (ValueError, KeyError, TypeError, OSError, http.client.HTTPException) as error:
        result = {"issue": 1483, "revision": args.revision, "overall": "FAIL", "requests": None,
                  "useful_socket_timing": None, "failure_class": type(error).__name__,
                  "utc_end": datetime.now(timezone.utc).isoformat(), "read_only": True}
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("overall", "requests", "useful_socket_timing")}))
    return int(result["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
