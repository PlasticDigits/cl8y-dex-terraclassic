#!/usr/bin/env python3
"""Bounded anonymous automatic-versus-declared route promotion checks (#1489)."""
import argparse
import hashlib
import http.client
import importlib.util
import json
import os
import re
import signal
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode

SPEC = importlib.util.spec_from_file_location("attribution1489", Path(__file__).with_name("probe-route-cost-attribution-1483.py"))
COMMON = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COMMON)
QUERY = {"token_in": "uluna", "token_out": "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv",
         "amount_in": "10000000000", "max_spread": "0.05", "protection_policy": "quote_relative_v1", "pool_only": "true"}


def execution_policy(body):
    """Compare recognized pool semantics, separately from literal request intent.

    Both automatic pool paths and a declared all-null discriminator execute pool
    swaps. Only these two recognized policies may share this economic cohort;
    positive hybrid declarations and other modes remain distinct. Callers first
    validate executable context, original floors, funding and attribution.
    """
    policy = body["execution_metadata"].get("policy")
    operations = body.get("router_operations")
    if (policy in {"pool_only", "declared_hybrid_manual"}
            and isinstance(operations, list) and 1 <= len(operations) <= 4
            and all(isinstance(op, dict) and isinstance(op.get("terra_swap"), dict)
                    and "hybrid" in op["terra_swap"] and op["terra_swap"]["hybrid"] is None for op in operations)):
        return "pool_only"
    return policy


def cohort(body):
    """Generation equality alone cannot establish exact original-economics equality."""
    meta = body["execution_metadata"]
    wrapper = meta.get("wrapper") or {}
    return tuple(meta.get(k) for k in ("generation", "observed_at_ms", "expires_at_ms", "input_amount", "max_spread", "protection_policy", "tax_funding_version")) + (execution_policy(body),
        body.get("token_in"), body.get("token_out"), body.get("spot_amount_out"),
        wrapper.get("input_amount"), wrapper.get("route_input_amount"), wrapper.get("wrap_input"), wrapper.get("unwrap_output"),
        wrapper.get("mapper"), wrapper.get("treasury"))


def compare(automatic, declared):
    same = cohort(automatic) == cohort(declared)
    return {"same_original_cohort": same, "automatic_net": automatic["estimated_amount_out_net"],
            "declared_net": declared["estimated_amount_out_net"],
            "automatic_not_worse": same and int(automatic["estimated_amount_out_net"]) >= int(declared["estimated_amount_out_net"])}


def summarize(body):
    meta = body["execution_metadata"]
    wrapper = meta.get("wrapper") or {}
    if wrapper.get("route_input_amount") is not None and COMMON.raw(wrapper["route_input_amount"]) <= 0:
        raise ValueError("positive_wrapper_credit")
    paths_considered = body.get("paths_considered")
    if type(paths_considered) is not int or not 0 <= paths_considered <= 1024:
        paths_considered = None
    attribution = COMMON.attribution(body)
    if attribution.get("status") != "available":
        raise ValueError("eligible_attribution_unavailable")
    result = {key: meta[key] for key in ("generation", "observed_at_ms", "expires_at_ms", "input_amount", "minimum_receive")}
    result.update(net_amount_out=body["estimated_amount_out_net"], fair_amount_out=body["spot_amount_out"],
                  route_input_amount=wrapper.get("route_input_amount"), hop_count=len(meta["hops"]),
                  paths_considered=paths_considered, attribution=attribution)
    for key in ("search_complete", "search_truncated"):
        result[key] = body.get(key) if type(body.get(key)) is bool else None
    result["search_max_pool_hops"] = body.get("search_max_pool_hops") if type(body.get("search_max_pool_hops")) is int and 1 <= body["search_max_pool_hops"] <= 4 else None
    # Only public token identities and checked numeric fields survive; no actors,
    # arbitrary server prose, endpoint URLs, headers or received raw bodies.
    hops = []
    for hop in meta["hops"]:
        if any(not isinstance(hop.get(key), str) or not re.fullmatch(r"terra1[023456789acdefghjklmnpqrstuvwxyz]{38,64}|u[a-z]{2,12}", hop[key]) for key in ("offer_token", "ask_token")):
            raise ValueError("public_token_identity_shape")
        for key in ("min_return", "return_amount"):
            if COMMON.raw(hop[key]) <= 0:
                raise ValueError("positive_hop_economics")
        hops.append({key: hop[key] for key in ("offer_token", "ask_token", "min_return", "return_amount")})
    result["hops"] = hops
    return result


def run(revision, cycles):
    started = time.monotonic()
    driver_sha256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    signal.signal(signal.SIGALRM, COMMON.deadline)
    utc_start = datetime.now(timezone.utc).isoformat()
    conn = http.client.HTTPSConnection(COMMON.HOST, timeout=10)
    requests, rows, comparisons, failures, health, initial_comparisons = 0, [], [], [], [], []
    scenario = {"token_in": QUERY["token_in"], "token_out": QUERY["token_out"], "amount": QUERY["amount_in"], "spread": QUERY["max_spread"]}

    def fetch(label, method="GET", payload=None, cycle=0):
        nonlocal requests
        requests += 1
        if requests > 100 or time.monotonic() - started > 240:
            raise ValueError("request_budget")
        suffix = {"best": "/best", "progress": "/progress"}.get(label, "")
        path = "/health" if label == "health" else "/api/v1/route/solve" + suffix
        if label != "health" and method == "GET":
            path += "?" + urlencode(QUERY)
        begin = time.perf_counter()
        signal.setitimer(signal.ITIMER_REAL, 10)
        old_socket = conn.sock
        conn.request(method, path, body=None if payload is None else json.dumps(payload),
                     headers={"Accept": "application/json", "Content-Type": "application/json", "User-Agent": "CL8Y-issue1489-anonymous-quote-probe/1"})
        transport = "reused" if old_socket is not None and old_socket is conn.sock else "new"
        response = conn.getresponse()
        data = response.read(2 * 1024 * 1024 + 1)
        if len(data) > 2 * 1024 * 1024:
            raise ValueError("body_budget")
        body = json.loads(data)
        if label == "health":
            sha = COMMON.health_revision(response.status, body)
            health.append(sha)
            if sha != revision:
                raise ValueError("revision_mismatch")
            signal.setitimer(signal.ITIMER_REAL, 0)
            return body
        row = {"cycle": cycle, "endpoint": label, "status": response.status,
               "transport": transport}
        if label == "progress":
            row["done_units"] = body.get("done") if type(body.get("done")) is int and 0 <= body["done"] <= 1024 else None
            row["stage"] = body.get("stage") if body.get("stage") in {"idle", "cached", "complete", "evaluating", "queued", "error"} else "other"
            for key in ("search_complete", "search_truncated"):
                row[key] = body.get(key) if type(body.get(key)) is bool else None
            if COMMON.OUTCOMES.positive_u128(body.get("best_amount_out")):
                row["best_amount_out"] = body["best_amount_out"]
            if isinstance(body.get("quote_generation"), str) and re.fullmatch(r"[0-9a-f]{64}", body["quote_generation"]):
                row["quote_generation"] = body["quote_generation"]
            valid = response.status == 200
        else:
            outcome, valid = COMMON.OUTCOMES.classify(response.status, body, scenario, int(time.time() * 1000))
            if outcome == "executable" and valid and execution_policy(body) != "pool_only":
                outcome, valid = "unsupported_native_pool_policy", False
            row.update(outcome=outcome, valid=valid)
            if outcome == "executable" and valid:
                row.update(summarize(body))
            else:
                failures.append({"cycle": cycle, "endpoint": label, "failure_class": "required_executable_unavailable"})
        elapsed = (time.perf_counter() - begin) * 1000
        row["elapsed_ms"] = round(elapsed, 3)
        if label != "progress":
            row["initialized_socket_budget_pass"] = row["transport"] != "reused" or (valid and elapsed <= 500)
            if not row["initialized_socket_budget_pass"]:
                failures.append({"cycle": cycle, "endpoint": label, "failure_class": "initialized_socket_latency"})
        rows.append(row)
        signal.setitimer(signal.ITIMER_REAL, 0)
        return body

    fetch("health")
    for cycle, offset in enumerate((0, 25, 185)[:cycles]):
        while time.monotonic() - started < offset:
            time.sleep(min(1, offset - (time.monotonic() - started)))
        automatic = fetch("solve", cycle=cycle)
        payload = {key: value for key, value in QUERY.items() if key != "pool_only"}
        declared = fetch("declared_three_pool_hops", "POST", {**payload, "hybrid_by_hop": [None, None, None]}, cycle)
        automatic_post = fetch("automatic_post", "POST", payload, cycle)
        best = fetch("best", cycle=cycle)
        fetch("progress", cycle=cycle)
        for label, quote in (("solve", automatic), ("automatic_post", automatic_post), ("best", best)):
            try:
                initial_comparisons.append({"cycle": cycle, "endpoint": label, **compare(quote, declared)})
            except (KeyError, TypeError, ValueError):
                pass
        # Candidate discovery is asynchronous. Preserve the initial failed input,
        # but judge eventual promotion against every observed eligible candidate
        # on its exact original cohort, without extending its expiry.
        eligible = [q for q in (declared, automatic_post, best) if isinstance(q, dict) and "execution_metadata" in q]
        promotion_started = time.monotonic()
        for attempt in range(6):
            if attempt and time.monotonic() - promotion_started >= 3:
                break
            automatic = fetch("solve", cycle=cycle)
            best = fetch("best", cycle=cycle)
            pending = False
            for quote in (automatic, best):
                try:
                    candidates = [q for q in eligible if cohort(q) == cohort(quote)]
                    pending |= bool(candidates) and int(quote["estimated_amount_out_net"]) < max(int(q["estimated_amount_out_net"]) for q in candidates)
                except (KeyError, TypeError, ValueError):
                    pending = True
            if not pending or time.monotonic() - promotion_started >= 3:
                break
            time.sleep(0.2)
        automatic_post = fetch("automatic_post", "POST", payload, cycle)
        if isinstance(automatic_post, dict) and "execution_metadata" in automatic_post:
            eligible.append(automatic_post)
        for label, quote in (("solve", automatic), ("automatic_post", automatic_post), ("best", best)):
            try:
                candidates = [q for q in eligible if cohort(q) == cohort(quote)]
                candidate = max(candidates, key=lambda q: int(q["estimated_amount_out_net"]), default=declared)
                result = {"cycle": cycle, "endpoint": label, "same_declared_discriminator_cohort": cohort(quote) == cohort(declared),
                          "promotion_elapsed_seconds": round(time.monotonic() - promotion_started, 3), **compare(quote, candidate)}
                comparisons.append(result)
                if result["same_original_cohort"] and not result["automatic_not_worse"]:
                    failures.append({"cycle": cycle, "endpoint": label, "failure_class": "automatic_inferior_same_cohort"})
            except (KeyError, TypeError, ValueError):
                pass  # Already classified as unavailable; never invent a comparison.
        fetch("health", cycle=cycle)
    conn.close()
    same = sum(c["same_declared_discriminator_cohort"] for c in comparisons)
    convergence_verified = all(any(c["cycle"] == cycle and c["endpoint"] == endpoint and c["same_declared_discriminator_cohort"] and c["automatic_not_worse"] for c in comparisons)
                               for cycle in range(cycles) for endpoint in ("solve", "best"))
    return {"issue": 1489, "revision": revision, "utc_start": utc_start, "utc_end": datetime.now(timezone.utc).isoformat(),
            "driver_sha256": driver_sha256, "requests": requests,
            "read_only_quote_planning": True, "signing": False, "cycle_offsets_seconds": (0, 25, 185)[:cycles],
            "elapsed_seconds": round(time.monotonic() - started, 3), "backend_revisions": health,
            "cohort_comparisons": comparisons, "same_original_cohort_comparison_count": same,
            "all_cycles_solve_best_convergence_verified": convergence_verified,
            "initial_cohort_comparisons": initial_comparisons, "promotion_deadline_seconds": 3,
            "samples": rows, "failures": failures,
            "initialized_useful_socket_timing": COMMON.OUTCOMES.common.distribution([r["elapsed_ms"] for r in rows if r.get("outcome") == "executable" and r["transport"] == "reused"]),
            "overall": "FAIL" if failures else "PASS WITH SKIPS",
            "skips": (["automatic/declared compatible-cohort convergence unverified"] if not convergence_verified else []) + ["independent SQL/reserves", "production settlement", "physical wallets", "browser adoption"],
            "scope": "The declared three-null quote is an eligible candidate discriminator, never a signing workaround or proof of global optimality."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--cycles", type=int, choices=(1, 2, 3), default=3)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("exact serving revision required")
    try:
        result = run(args.revision, args.cycles)
    except Exception as error:
        result = {"issue": 1489, "overall": "FAIL", "failure_class": type(error).__name__}
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    with os.fdopen(os.open(args.output, os.O_CREAT | os.O_TRUNC | os.O_WRONLY, 0o600), "w") as handle:
        handle.write(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result.get(key) for key in ("overall", "requests", "same_original_cohort_comparison_count", "initialized_useful_socket_timing", "failure_class")}))
    return int(result["overall"] == "FAIL")


if __name__ == "__main__":
    raise SystemExit(main())
