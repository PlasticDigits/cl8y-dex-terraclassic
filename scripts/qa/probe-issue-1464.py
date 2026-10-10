#!/usr/bin/env python3
"""Bounded public quote corpus; error completion never counts as useful latency."""
import argparse
import http.client
import json
import math
import re
import signal
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode, urlsplit

ASSETS = {
    "LUNC": "uluna",
    "cUSTC": "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch",
    "CL8Y": "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3",
    "USTR": "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv",
    "ALPHA": "terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz",
}
CORPUS = [
    ("LUNC", "cUSTC", "10000000000", "100000000"),
    ("CL8Y", "USTR", "1000000000000000000", "10000000000000000"),
    ("USTR", "cUSTC", "100000000000000000000", "1000000000000000000"),
    ("ALPHA", "CL8Y", "1000000", "100000"),
]
MAX_BODY = 2 * 1024 * 1024


def deadline(_signum, _frame):
    raise TimeoutError("read deadline")


def positive_raw(value):
    return isinstance(value, str) and len(value) <= 78 and re.fullmatch(r"[1-9][0-9]*", value) is not None


def useful(body, amount, now_ms, token_in, token_out):
    """Validate executable shape without publishing raw operations or actor fields."""
    try:
        meta = body["execution_metadata"]
        operations = body["router_operations"]
        hops = meta["hops"]
        return (
            body["token_in"] == token_in
            and body["token_out"] == token_out
            and body.get("execution_blocker") is None
            and body.get("execution_supported", True) is True
            and meta.get("execution_supported", True) is True
            and positive_raw(body["estimated_amount_out_net"])
            and meta["input_amount"] == amount
            and isinstance(meta["generation"], str)
            and re.fullmatch(r"[0-9a-f]{64}", meta["generation"]) is not None
            and type(meta["observed_at_ms"]) is int
            and type(meta["expires_at_ms"]) is int
            and 0 <= meta["observed_at_ms"] <= now_ms < meta["expires_at_ms"]
            and positive_raw(meta["minimum_receive"])
            and int(meta["minimum_receive"]) <= int(body["estimated_amount_out_net"])
            and bool(operations)
            and len(operations) == len(hops)
            and all(positive_raw(op["terra_swap"]["min_return"]) for op in operations)
            and all(positive_raw(hop["min_return"]) for hop in hops)
            and all(op["terra_swap"]["min_return"] == hop["min_return"] for op, hop in zip(operations, hops))
        )
    except (KeyError, TypeError, ValueError, OverflowError):
        return False


def distribution(values):
    if not values:
        return {"samples": 0, "p50_ms": None, "p95_ms": None, "p99_ms": None, "max_ms": None}
    ordered = sorted(values)
    return {"samples": len(values), **{
        f"p{percent}_ms": round(ordered[max(0, math.ceil(len(ordered) * percent / 100) - 1)], 3)
        for percent in (50, 95, 99)
    }, "max_ms": round(ordered[-1], 3)}


def request_transport(connection, path, headers):
    """Observe socket reuse, including automatic reconnect on a pooled object."""
    previous = connection.sock
    connection.request("GET", path, headers=headers)
    return "reused" if previous is not None and connection.sock is previous else "new"


def probe(args):
    signal.signal(signal.SIGALRM, deadline)
    target = urlsplit(args.base_url)
    if target.scheme not in ("http", "https") or not target.hostname or target.username or target.password or target.query or target.fragment:
        raise ValueError("base URL must be credential-free HTTP(S), without query or fragment")
    factory = http.client.HTTPSConnection if target.scheme == "https" else http.client.HTTPConnection
    rows = []
    started = datetime.now(timezone.utc).isoformat()
    for transport in ("new", "reused"):
        connection = None
        for cycle in range(args.cycles):
            for offer, ask, ordinary, small in CORPUS:
                for size, amount, spread in (("ordinary", ordinary, "0.05"), ("small", small, "0.01"), ("small", small, "0.30")):
                    if connection is None:
                        connection = factory(target.hostname, target.port, timeout=10)
                    query = {"token_in": ASSETS[offer], "token_out": ASSETS[ask], "amount_in": amount, "max_spread": spread}
                    if offer == "LUNC":
                        query["pool_only"] = "true"
                    row = {"cycle": cycle, "direction": f"{offer}->{ask}", "size": size, "spread": spread,
                           "connection_group": transport, "transport": "unobserved"}
                    start = time.monotonic()
                    try:
                        signal.setitimer(signal.ITIMER_REAL, 10)
                        row["transport"] = request_transport(
                            connection, target.path.rstrip("/") + "/api/v1/route/solve?" + urlencode(query),
                            {"User-Agent": "CL8Y-issue1464-read-only-probe/1"},
                        )
                        response = connection.getresponse()
                        raw = response.read(MAX_BODY + 1)
                        row["status"] = response.status
                        try:
                            body = json.loads(raw) if len(raw) <= MAX_BODY else None
                        except (ValueError, UnicodeError):
                            body = None
                        row["useful"] = response.status == 200 and useful(body, amount, int(time.time() * 1000), ASSETS[offer], ASSETS[ask])
                        row["outcome"] = "useful" if row["useful"] else "unavailable_or_nonexecutable"
                        if row["useful"]:
                            meta = body["execution_metadata"]
                            row["observation"] = {key: meta[key] for key in ("observed_at_ms", "expires_at_ms")}
                            row["observation"]["generation_shape_valid"] = True
                    except (OSError, http.client.HTTPException):
                        row.update(status=None, useful=False, outcome="transport_error")
                        connection.close()
                        connection = None
                    finally:
                        signal.setitimer(signal.ITIMER_REAL, 0)
                    row["elapsed_ms"] = round((time.monotonic() - start) * 1000, 3)
                    rows.append(row)
                    if transport == "new" and connection is not None:
                        connection.close()
                        connection = None
                    time.sleep(args.interval)
        if connection is not None:
            connection.close()
    failed = any(not row["useful"] or row["elapsed_ms"] >= 250 for row in rows)
    return {
        "issue": 1464, "revision": args.revision, "utc_start": started,
        "utc_end": datetime.now(timezone.utc).isoformat(), "overall": "FAIL" if failed else "PASS WITH SKIPS",
        "useful_count": sum(row["useful"] for row in rows), "request_count": len(rows),
        "useful_latency": distribution([row["elapsed_ms"] for row in rows if row["useful"]]),
        "useful_latency_by_transport": {mode: distribution([row["elapsed_ms"] for row in rows if row["useful"] and row["transport"] == mode]) for mode in ("new", "reused")},
        "samples": rows,
        "unverified": ["deployed revision requires independent health receipt", "cold initialization versus initialized cache hit/miss", "regional RTT", "desktop/phone browser", "request upstream counters", "settlement parity", "bounded deployed outage trial"],
        "scope": "public socket reads only; no signing, deployment, actor contexts, raw bodies or endpoint URLs retained",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-url", default="https://indexer.dex.cl8y.com")
    parser.add_argument("--cycles", type=int, default=1, help="1..10 cycles per connection mode; 24..240 requests")
    parser.add_argument("--interval", type=float, default=0.5, help="0.5..10 seconds between requests")
    parser.add_argument("--revision", required=True, help="independently observed deployed revision")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not 1 <= args.cycles <= 10 or not 0.5 <= args.interval <= 10 or not math.isfinite(args.interval):
        parser.error("cycles/interval exceed finite read budget")
    if len(args.revision) != 40 or any(c not in "0123456789abcdef" for c in args.revision):
        parser.error("revision must be a lowercase 40-character commit SHA")
    try:
        result = probe(args)
    except ValueError:
        parser.error("base URL must be valid credential-free HTTP(S)")
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("overall", "request_count", "useful_count", "useful_latency")}))
    return 1 if result["overall"] == "FAIL" else 0


if __name__ == "__main__":
    raise SystemExit(main())
