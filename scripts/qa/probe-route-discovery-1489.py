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


def progress_eligibility(body):
    value = body.get("quote_eligibility")
    allowed = {"executable_quote", "waiting_for_observations", "policy_excluded", "unverified"}
    return value if isinstance(value, str) and value in allowed else "other"


class PromotionWindow:
    """One immutable comparator and discovery clock; newer cohorts never renew it."""
    ENDPOINTS = ("solve", "best", "automatic_post")

    def __init__(self, declared, observations):
        self.key = cohort(declared["body"])
        self.expires_ms = declared["body"]["execution_metadata"]["expires_at_ms"]
        matching = [row for row in observations if row["eligible"] and cohort(row["body"]) == self.key]
        self.started = min(row["request_started"] for row in matching)
        expiry_monotonic = declared["validated_at"] + (self.expires_ms - declared["wall_ms"]) / 1000
        self.deadline = min(self.started + 3, expiry_monotonic)
        self.maximum_net, self.known_at = 0, self.started
        self.latest, self.cursor = {}, 0
        for row in sorted(observations, key=lambda row: row["validated_at"]):
            self.observe(row)

    def compatible(self, row):
        return row is not None and row["eligible"] and cohort(row["body"]) == self.key and row["wall_ms"] < self.expires_ms

    def observe(self, row):
        if row["endpoint"] in self.ENDPOINTS:
            self.latest[row["endpoint"]] = row
        if self.compatible(row):
            value = int(row["body"]["estimated_amount_out_net"])
            if value > self.maximum_net:
                self.maximum_net, self.known_at = value, row["validated_at"]

    def ready(self, endpoint):
        row = self.latest.get(endpoint)
        return (self.compatible(row) and row["request_started"] >= self.known_at
                and row["validated_at"] <= self.deadline
                and row["body"].get("search_complete") is True
                and int(row["body"]["estimated_amount_out_net"]) >= self.maximum_net)

    def next_pending_endpoint(self):
        for _ in self.ENDPOINTS:
            endpoint = self.ENDPOINTS[self.cursor % len(self.ENDPOINTS)]
            self.cursor += 1
            if not self.ready(endpoint):
                return endpoint
        return None

    def verdict(self, now, now_ms):
        if now_ms >= self.expires_ms:
            return {"status": "UNVERIFIED", "reason": "original_cohort_expired"}
        if all(self.ready(endpoint) for endpoint in self.ENDPOINTS):
            return {"status": "PASS", "reason": "compatible_post_benchmark_convergence"}
        # Only an actually captured inferior response at/after the fixed deadline
        # can demonstrate a miss. Earlier incomplete reads are never failed
        # retrospectively when a later POST reveals a higher candidate.
        if self.known_at <= self.deadline and any(self.compatible(row) and row["request_started"] >= self.known_at and row["validated_at"] >= self.deadline
               and int(row["body"]["estimated_amount_out_net"]) < self.maximum_net
               for endpoint, row in self.latest.items() if endpoint in ("solve", "best")):
            return {"status": "FAIL", "reason": "same_cohort_inferior_after_deadline"}
        if now < self.deadline:
            return {"status": "PENDING", "reason": "compatible_comparison_pending"}
        if self.known_at > self.deadline:
            return {"status": "UNVERIFIED", "reason": "benchmark_not_known_within_original_window"}
        if not any(self.compatible(self.latest.get(endpoint)) for endpoint in ("solve", "best")):
            return {"status": "UNVERIFIED", "reason": "original_compatible_cohort_unavailable"}
        return {"status": "UNVERIFIED", "reason": "promotion_deadline_not_verified"}


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
    requests, rows, comparisons, failures, health, initial_comparisons, windows = 0, [], [], [], [], [], []
    cycle_started = started
    scenario = {"token_in": QUERY["token_in"], "token_out": QUERY["token_out"], "amount": QUERY["amount_in"], "spread": QUERY["max_spread"]}

    def fetch(label, method="GET", payload=None, cycle=0, timeout=10):
        nonlocal requests
        requests += 1
        if requests > 100 or time.monotonic() - started > 240:
            raise ValueError("request_budget")
        suffix = {"best": "/best", "progress": "/progress"}.get(label, "")
        path = "/health" if label == "health" else "/api/v1/route/solve" + suffix
        if label != "health" and method == "GET":
            path += "?" + urlencode(QUERY)
        begin = time.monotonic()
        signal.setitimer(signal.ITIMER_REAL, timeout)
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
            row["quote_eligibility"] = progress_eligibility(body)
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
        finished, wall_ms = time.monotonic(), int(time.time() * 1000)
        elapsed = (finished - begin) * 1000
        row["elapsed_ms"] = round(elapsed, 3)
        row["request_started_offset_ms"] = round((begin - cycle_started) * 1000, 3)
        row["validated_offset_ms"] = round((finished - cycle_started) * 1000, 3)
        if label != "progress":
            useful = valid and outcome == "executable"
            row["initialized_socket_budget_pass"] = useful and (row["transport"] != "reused" or elapsed <= 500)
            if useful and not row["initialized_socket_budget_pass"]:
                failures.append({"cycle": cycle, "endpoint": label, "failure_class": "initialized_socket_latency"})
        rows.append(row)
        signal.setitimer(signal.ITIMER_REAL, 0)
        if label == "progress":
            return body
        return {"body": body, "endpoint": label, "eligible": valid and outcome == "executable",
                "request_started": begin, "validated_at": finished, "wall_ms": wall_ms}

    fetch("health")
    for cycle, offset in enumerate((0, 25, 185)[:cycles]):
        while time.monotonic() - started < offset:
            time.sleep(min(1, offset - (time.monotonic() - started)))
        cycle_started = time.monotonic()
        automatic = fetch("solve", cycle=cycle)
        payload = {key: value for key, value in QUERY.items() if key != "pool_only"}
        declared = fetch("declared_three_pool_hops", "POST", {**payload, "hybrid_by_hop": [None, None, None]}, cycle)
        automatic_post = fetch("automatic_post", "POST", payload, cycle)
        best = fetch("best", cycle=cycle)
        for label, quote in (("solve", automatic), ("automatic_post", automatic_post), ("best", best)):
            if quote["eligible"] and declared["eligible"]:
                initial_comparisons.append({"cycle": cycle, "endpoint": label, **compare(quote["body"], declared["body"])})
        if declared["eligible"]:
            window = PromotionWindow(declared, [automatic, declared, automatic_post, best])
            for _ in range(12):
                if time.monotonic() >= window.deadline or int(time.time() * 1000) >= window.expires_ms:
                    break
                endpoint = window.next_pending_endpoint()
                if endpoint is None:
                    break
                # Cohort mismatch remains pending; it never triggers early success.
                observation = fetch(endpoint, "POST" if endpoint == "automatic_post" else "GET",
                                    payload if endpoint == "automatic_post" else None, cycle)
                window.observe(observation)
            # One finite boundary observation per GET may demonstrate a real
            # post-deadline miss. It cannot turn late success into <=3s acceptance.
            if time.monotonic() >= window.deadline and int(time.time() * 1000) < window.expires_ms:
                for endpoint in ("solve", "best"):
                    if window.ready(endpoint):
                        continue
                    remaining = (window.expires_ms - int(time.time() * 1000)) / 1000
                    if remaining <= 0:
                        break
                    try:
                        window.observe(fetch(endpoint, cycle=cycle, timeout=min(.5, remaining)))
                    except (OSError, http.client.HTTPException):
                        signal.setitimer(signal.ITIMER_REAL, 0)
                        conn.close()
                        conn = http.client.HTTPSConnection(COMMON.HOST, timeout=10)
            now, now_ms = time.monotonic(), int(time.time() * 1000)
            verdict = window.verdict(now, now_ms)
            if verdict["status"] == "PENDING":
                verdict = {"status": "UNVERIFIED", "reason": "bounded_poll_budget_exhausted"}
            windows.append({"cycle": cycle, **verdict, "original_expires_at_ms": window.expires_ms,
                            "discovery_started_offset_ms": round((window.started - cycle_started) * 1000, 3),
                            "deadline_offset_ms": round((window.deadline - cycle_started) * 1000, 3),
                            "benchmark_known_offset_ms": round((window.known_at - cycle_started) * 1000, 3),
                            "maximum_eligible_net": str(window.maximum_net)})
            if verdict["status"] == "FAIL":
                failures.append({"cycle": cycle, "failure_class": verdict["reason"]})
            for endpoint in window.ENDPOINTS:
                observation = window.latest.get(endpoint)
                same = window.compatible(observation)
                comparisons.append({"cycle": cycle, "endpoint": endpoint,
                                    "same_declared_discriminator_cohort": same,
                                    "automatic_net": observation["body"].get("estimated_amount_out_net") if observation and observation["eligible"] else None,
                                    "benchmark_maximum_net": str(window.maximum_net),
                                    "automatic_not_worse": same and int(observation["body"]["estimated_amount_out_net"]) >= window.maximum_net,
                                    "post_benchmark_within_original_window": window.ready(endpoint),
                                    "original_cohort_unexpired_at_comparison": now_ms < window.expires_ms})
        else:
            windows.append({"cycle": cycle, "status": "UNVERIFIED", "reason": "eligible_declared_comparator_unavailable"})
        fetch("progress", cycle=cycle)
        fetch("health", cycle=cycle)
    conn.close()
    same = sum(c["same_declared_discriminator_cohort"] for c in comparisons)
    convergence_verified = all(any(c["cycle"] == cycle and c["endpoint"] == endpoint and c["post_benchmark_within_original_window"] and c["original_cohort_unexpired_at_comparison"] for c in comparisons)
                               for cycle in range(cycles) for endpoint in ("solve", "best"))
    return {"issue": 1489, "revision": revision, "utc_start": utc_start, "utc_end": datetime.now(timezone.utc).isoformat(),
            "driver_sha256": driver_sha256, "requests": requests,
            "read_only_quote_planning": True, "signing": False, "cycle_offsets_seconds": (0, 25, 185)[:cycles],
            "elapsed_seconds": round(time.monotonic() - started, 3), "backend_revisions": health,
            "cohort_comparisons": comparisons, "same_original_cohort_comparison_count": same,
            "all_cycles_solve_best_convergence_verified": convergence_verified,
            "initial_cohort_comparisons": initial_comparisons, "promotion_deadline_seconds": 3,
            "promotion_windows": windows, "all_cycles_automatic_post_agreement_verified": all(w["status"] == "PASS" for w in windows),
            "samples": rows, "failures": failures,
            "initialized_useful_socket_timing": COMMON.OUTCOMES.common.distribution([r["elapsed_ms"] for r in rows if r.get("outcome") == "executable" and r["transport"] == "reused"]),
            "overall": "FAIL" if failures else "PASS WITH SKIPS" if convergence_verified and all(w["status"] == "PASS" for w in windows) else "UNVERIFIED",
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
    return 0 if result["overall"] == "PASS WITH SKIPS" else 2 if result["overall"] == "UNVERIFIED" else 1


if __name__ == "__main__":
    raise SystemExit(main())
