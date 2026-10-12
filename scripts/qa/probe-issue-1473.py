#!/usr/bin/env python3
"""Bounded, read-only bidirectional quote census; never signing or economic acceptance."""
import argparse
import importlib.util
import json
import math
import re
import signal
import time
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from urllib.parse import urlencode, urlsplit

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("quote_probe", Path(__file__).with_name("probe-issue-1464.py"))
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
REASONS = {
    "protection": 422, "price_impact_too_high": 422, "amount_too_small": 422, "amount_too_large": 422, "insufficient_liquidity": 422,
    "unsupported_semantics": 422, "reference_policy_excluded": 422, "trading_blocked": 422, "no_route": 422,
    "missing_observations": 503, "stale_observations": 503,
    "accounting_unavailable": 503, "invalid_request": 400,
}


def corpus(profile="full", issue=1473):
    if issue not in (1473, 1479):
        raise ValueError("bounded matrix identity required")
    matrix = json.loads((ROOT / f"docs/qa/issue-{issue}/matrix.json").read_text())
    amounts = matrix["human_amounts"]
    pairs = matrix["pairs"]
    if profile == "reproduction":
        pairs, amounts = [pairs[0]], ["1", "10", "100", "1000"]
    elif profile == "census":
        amounts = ["0.000001", "0.01", "1", "100", "10000"]
    for pair in pairs:
        for offer, ask in (pair, pair[::-1]):
            asset = matrix["assets"][offer]
            raw_amounts = [(human, str(int(Decimal(human) * 10 ** asset["decimals"]))) for human in amounts]
            if profile == "full":
                raw_amounts.insert(0, ("one_raw_unit", "1"))
            # Keep minimum human and one raw scenarios distinct even at six decimals.
            for human, raw in raw_amounts:
                for spread in matrix["max_spreads"]:
                    yield {"pair": f"{offer}->{ask}", "human_amount": human, "amount": raw,
                           "spread": spread, "token_in": asset["identity"],
                           "token_out": matrix["assets"][ask]["identity"],
                           "pool_only": offer in ("USTC", "LUNC") or ask in ("USTC", "LUNC")}


def positive_u128(value):
    return isinstance(value, str) and re.fullmatch(r"[1-9][0-9]{0,38}", value) is not None and int(value) < 2 ** 128


def quoted_policy_valid(body, scenario, now_ms=None):
    """Independently check policy, exact selected net floors and bounded L9 ceiling."""
    try:
        meta = body["execution_metadata"]
        now_ms = int(time.time() * 1000) if now_ms is None else now_ms
        observed, expiry = meta.get("observed_at_ms"), meta.get("expires_at_ms")
        if type(observed) is not int or type(expiry) is not int or not 0 <= observed <= now_ms < expiry or not 0 < expiry - observed <= 20_000:
            return False
        if meta.get("protection_policy") != "quote_relative_v1":
            return False
        selected = Decimal(meta["max_spread"])
        if selected != Decimal(scenario["spread"]) or not 0 < selected <= Decimal("0.5"):
            return False
        scale = 10 ** 18
        tolerance = int(selected * scale)
        ceiling = tolerance
        for hop in meta["hops"]:
            numbers = []
            for field in ["pool_return_amount", "commission_amount", "book_return_net", "pool_input_actual", "book_offer_consumed", "spread_amount", "return_amount", "min_return"]:
                raw = hop[field]
                if not isinstance(raw, str) or re.fullmatch(r"0|[1-9][0-9]{0,38}", raw) is None or int(raw) >= 2 ** 128:
                    return False
                numbers.append(int(raw))
            pool, fee, book, pool_input, consumed, spread, net, floor = numbers
            if net <= 0 or floor < (net * (scale - tolerance) + scale - 1) // scale:
                return False
            gross = pool + fee
            denominator = gross + book
            shortfall = max(0, pool * consumed // pool_input - book) if pool_input and consumed else 0
            numerator = min(spread, gross) + shortfall
            if denominator <= 0 or denominator >= 2 ** 128 or numerator > denominator:
                return False
            n = (numerator * scale + denominator * tolerance) * scale
            d = denominator * (scale - tolerance)
            ceiling = max(ceiling, min(scale, (n + d - 1) // d))
        market = meta.get("execution_market_spread")
        if not isinstance(market, str) or re.fullmatch(r"0\.[0-9]{1,18}|1", market) is None or Decimal(market) * scale != ceiling:
            return False
        net = int(body["estimated_amount_out_net"])
        return int(meta["minimum_receive"]) >= (net * (scale - tolerance) + scale - 1) // scale
    except (KeyError, TypeError, ValueError, ArithmeticError):
        return False


def classify(status, body, scenario, now_ms):
    """Never echo unknown fields or accept an indicative body as an executable quote."""
    if not isinstance(body, dict):
        return "malformed", False
    if status == 200:
        valid = common.useful(body, scenario["amount"], now_ms, scenario["token_in"], scenario["token_out"]) and quoted_policy_valid(body, scenario, now_ms)
        return ("executable" if valid else "client_validation"), valid
    reason = body.get("error")
    if not isinstance(reason, str) or REASONS.get(reason) != status:
        return "unclassified", False
    if "indicative_amount_out" in body:
        observed, expiry = body.get("observed_at_ms"), body.get("expires_at_ms")
        valid = (reason == "protection" and body.get("execution_supported") is False
                 and positive_u128(body["indicative_amount_out"])
                 and isinstance(body.get("generation"), str)
                 and re.fullmatch(r"[a-zA-Z0-9:_-]{1,256}", body["generation"]) is not None
                 and type(observed) is int and type(expiry) is int
                 and 0 < observed <= now_ms < expiry <= now_ms + 20_000
                 and expiry - observed <= 20_000
                 and not any(field in body for field in ("router_operations", "execution_metadata", "hops", "minimum_receive")))
        return ("indicative_protection" if valid else "malformed_indicative"), valid
    return reason, True


def connection_for(base):
    target = urlsplit(base)
    if target.scheme not in ("http", "https") or not target.hostname or target.username or target.password or target.query or target.fragment or target.path not in ("", "/"):
        raise ValueError("credential-free HTTP(S) origin required")
    factory = common.http.client.HTTPSConnection if target.scheme == "https" else common.http.client.HTTPConnection
    return factory(target.hostname, target.port, timeout=10)


class HealthRevisionUnavailable(ValueError):
    pass


class RevisionMismatch(ValueError):
    pass


def health_revision(connection, on_stage=lambda _stage: None, on_status=lambda _status: None):
    on_stage("request")
    connection.request("GET", "/health", headers={"Accept": "application/json"})
    on_stage("response")
    response = connection.getresponse()
    on_status(response.status)
    on_stage("body")
    payload = response.read(4097)
    if response.status != 200 or len(payload) > 4096:
        raise HealthRevisionUnavailable()
    on_stage("decode")
    data = json.loads(payload)
    revision = data.get("git_sha") if isinstance(data, dict) else None
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise HealthRevisionUnavailable()
    return revision


def utc_now():
    return datetime.now(timezone.utc).isoformat()


def probe(args):
    signal.signal(signal.SIGALRM, common.deadline)
    started = utc_now()
    connection = None
    rows, revisions, prerequisites, failures = [], [], [], []
    stage, trial_name = "origin", "first"

    def provenance(phase):
        nonlocal stage
        # Health provenance never warms the measured quote connection. Each
        # check owns a fresh connection, including after the long cycle gap.
        health_connection = None
        record = {"trial": trial_name, "phase": phase, "utc_start": utc_now()}
        prerequisites.append(record)
        try:
            stage = phase + ".connection"
            health_connection = connection_for(args.base_url)

            def health_stage(value):
                nonlocal stage
                stage = phase + "." + value

            signal.alarm(10)
            revision = health_revision(health_connection, health_stage,
                                       lambda status: record.update(status=status))
            revisions.append(revision)
            record["revision"] = revision
            stage = phase + ".revision"
            if revision != args.revision:
                raise RevisionMismatch()
            record["outcome"] = "revision_match"
        except (ValueError, OSError, TimeoutError, common.http.client.HTTPException) as exc:
            record.update(outcome="FAIL", failure_stage=stage, failure_class=type(exc).__name__)
            raise
        finally:
            signal.alarm(0)
            if health_connection is not None:
                health_connection.close()
            record["utc_end"] = utc_now()

    try:
        stage = "corpus"
        scenarios = list(corpus(args.profile, getattr(args, "matrix", 1473)))
        if not 1 <= len(scenarios) <= 420:
            raise ValueError()
        for trial in range(args.cycles):
            trial_name = "first" if trial == 0 else "retest"
            if trial:
                stage = "cycle_delay"
                time.sleep(args.cycle_delay)
            provenance("provenance_before")
            stage = "quote_connection"
            connection = connection_for(args.base_url)
            first_request = True
            for endpoint in args.endpoints:
                for scenario in scenarios:
                    query = {"token_in": scenario["token_in"], "token_out": scenario["token_out"],
                             "amount_in": scenario["amount"], "max_spread": scenario["spread"],
                             "pool_only": str(scenario["pool_only"]).lower(), "max_maker_fills": 8,
                             "protection_policy": "quote_relative_v1"}
                    start = time.perf_counter()
                    status, transport, outcome, valid = 0, "unknown", "transport_unavailable", False
                    row_started, failure = utc_now(), {}
                    try:
                        signal.alarm(10)
                        stage = "quote.request"
                        transport = common.request_transport(connection, ("/api/v1/route/solve" if endpoint == "solve" else "/api/v1/route/solve/best") + "?" + urlencode(query), {"Accept": "application/json"})
                        stage = "quote.response"
                        response = connection.getresponse()
                        status = response.status
                        body_limit = common.MAX_BODY if status == 200 else 4096
                        stage = "quote.body"
                        payload = response.read(body_limit + 1)
                        if len(payload) > body_limit:
                            outcome = "oversized_body"
                            connection.close()
                        else:
                            stage = "quote.decode"
                            outcome, valid = classify(status, json.loads(payload), scenario, int(time.time() * 1000))
                    except (OSError, TimeoutError, ValueError, common.http.client.HTTPException) as exc:
                        failure = {"failure_stage": stage, "failure_class": type(exc).__name__}
                        connection.close()
                    finally:
                        signal.alarm(0)
                    elapsed = (time.perf_counter() - start) * 1000
                    rows.append({"trial": trial_name, "endpoint": endpoint,
                                 **{k: scenario[k] for k in ("pair", "human_amount", "amount", "spread")},
                                 "status": status, "outcome": outcome, "classified": valid,
                                 "transport": transport, "elapsed_ms": round(elapsed, 3),
                                 "transport_plan": "cold_cycle_start" if first_request else "pooled",
                                 "utc_start": row_started, "utc_end": utc_now(), **failure,
                                 "socket_budget_pass": outcome == "executable" and elapsed <= 500})
                    first_request = False
                    stage = "case_interval"
                    time.sleep(args.interval)
            # Explicitly end this measured pool before the idle gap. A cold
            # first request next cycle includes connection setup in its timing;
            # no failing request is retried or has its clock restarted.
            connection.close()
            connection = None
            provenance("provenance_after")
    except (ValueError, OSError, TimeoutError, common.http.client.HTTPException) as exc:
        failures.append({"trial": trial_name, "stage": stage, "failure_class": type(exc).__name__,
                         "utc": utc_now(), "completed_request_count": len(rows)})
    finally:
        signal.alarm(0)
        if connection is not None:
            connection.close()
    counts = {reason: sum(row["outcome"] == reason for row in rows) for reason in sorted({row["outcome"] for row in rows})}
    failed = bool(failures) or any(not row["classified"] for row in rows) or (args.enforce_socket_budget and any(row["outcome"] == "executable" and not row["socket_budget_pass"] for row in rows))
    return {"issue": getattr(args, "matrix", 1473), "revision": args.revision, "backend_revisions": revisions,
            "utc_start": started, "utc_end": utc_now(),
            "overall": "FAIL" if failed else "PASS WITH SKIPS", "request_count": len(rows),
            "counts": counts, "samples": rows, "prerequisites": prerequisites, "failures": failures,
            "transport_policy": "fresh provenance connection per check; planned cold measured connection per cycle, pooled within cycle; no quote retry",
            "useful_latency_by_transport": {mode: common.distribution([row["elapsed_ms"] for row in rows if row["outcome"] == "executable" and row["transport"] == mode]) for mode in ("new", "reused")},
            "scope": "public socket classification; safe blocked outcomes count as classified, never executable or economic acceptance",
            "skips": ["pinned economic parity", "frontend artifact provenance", "browser input-event latency", "cache hit/miss attestation", "receive/manual/hybrid/actor/abuse fixtures", "physical devices", "settlement"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="https://indexer.dex.cl8y.com")
    parser.add_argument("--revision", required=True)
    parser.add_argument("--profile", choices=["reproduction", "census", "full"], default="reproduction")
    parser.add_argument("--matrix", type=int, choices=[1473, 1479], default=1473)
    parser.add_argument("--endpoints", nargs="+", choices=["solve", "best"], default=["solve"])
    parser.add_argument("--cycles", type=int, default=1)
    parser.add_argument("--cycle-delay", type=float, default=181)
    parser.add_argument("--interval", type=float, default=0.5)
    parser.add_argument("--enforce-socket-budget", action="store_true")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("exact 40-character revision required")
    if not 1 <= args.cycles <= 2 or not math.isfinite(args.interval) or not 0.5 <= args.interval <= 10 or not math.isfinite(args.cycle_delay) or not 181 <= args.cycle_delay <= 600 or len(set(args.endpoints)) != len(args.endpoints):
        parser.error("finite read budget: 1..2 cycles, unique endpoints, 0.5..10s interval, 181..600s cycle delay")
    result = probe(args)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: result[k] for k in ("overall", "request_count", "counts", "failures", "useful_latency_by_transport")}))
    return int(result["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
