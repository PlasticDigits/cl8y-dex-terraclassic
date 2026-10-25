#!/usr/bin/env python3
"""One bounded, restart-resumable monitoring-only alert drill transition."""
import argparse
import json
import math
import os
import signal
import time
from datetime import datetime, timezone
from urllib.request import HTTPRedirectHandler, Request, build_opener

STATE_PATH = "/state/drill-state.json"
RECEIPT_PATH = "/state/drill-receipt.json"
DEADLINE_SECONDS = 240
MAX_STEPS = 64
MAX_BODY_BYTES = 65_536
TIMEOUT_SECONDS = 3
PHASES = frozenset(("pending", "firing", "delivery", "recovery", "PASS", "FAIL"))
MARKERS = frozenset(("pending", "firing", "delivered", "disabled", "recovered"))
IDENTITY = {"alertname": "DexMonitoringDrill", "job": "cl8y-dex-monitor",
            "purpose": "monitoring", "service": "cl8y-dex-terraclassic-indexer"}


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


OPENER = build_opener(NoRedirect())


def request(endpoint, payload=None):
    # No caller-supplied origin/path, environment credentials or DEX endpoints.
    origins = {"active": "http://dex-prometheus:9090/api/v1/alerts",
               "receipts": "http://dex-exporter:9801/alerts",
               "toggle": "http://dex-exporter:9801/drill"}
    data = None if payload is None else json.dumps(payload).encode("ascii")
    req = Request(origins[endpoint], data=data, headers={"Accept": "application/json", "Content-Type": "application/json"})
    with OPENER.open(req, timeout=TIMEOUT_SECONDS) as response:
        body = response.read(MAX_BODY_BYTES + 1)
    if len(body) > MAX_BODY_BYTES:
        raise ValueError("body_bound")
    return json.loads(body)


def utc(value):
    return datetime.fromtimestamp(value, timezone.utc).isoformat()


def persist(path, payload):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    temporary = path + ".tmp"
    with os.fdopen(os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w", encoding="ascii") as output:
        json.dump(payload, output, separators=(",", ":"), sort_keys=True)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, path)


def load(path):
    try:
        with open(path, encoding="ascii") as source:
            body = source.read(4097)
    except FileNotFoundError:
        return None
    if len(body) > 4096:
        raise ValueError("state_bound")
    state = json.loads(body)
    if not isinstance(state, dict) or set(state) != {"started", "phase", "steps", "errors", "markers", "reason"}:
        raise ValueError("state_shape")
    if state["phase"] not in PHASES or state["reason"] not in (None, "deadline", "step_bound", "pending_not_observed"):
        raise ValueError("state_phase")
    if not finite_time(state["started"]) or type(state["steps"]) is not int or not 1 <= state["steps"] <= MAX_STEPS:
        raise ValueError("state_count")
    if type(state["errors"]) is not int or not 0 <= state["errors"] <= MAX_STEPS:
        raise ValueError("state_count")
    if not isinstance(state["markers"], dict) or set(state["markers"]) - MARKERS or any(not finite_time(v) for v in state["markers"].values()):
        raise ValueError("state_markers")
    return state


def finite_time(value):
    return type(value) in (int, float) and math.isfinite(value) and value > 0


def active_state(payload):
    if not isinstance(payload, dict) or payload.get("status") != "success" or not isinstance(payload.get("data"), dict):
        raise ValueError("alerts_shape")
    rows = payload["data"].get("alerts")
    if not isinstance(rows, list) or len(rows) > 64:
        raise ValueError("alerts_bound")
    selected = []
    for row in rows:
        if not isinstance(row, dict) or not isinstance(row.get("labels"), dict):
            raise ValueError("alerts_shape")
        if all(row["labels"].get(key) == value for key, value in IDENTITY.items()):
            if row.get("state") not in ("pending", "firing"):
                raise ValueError("alerts_state")
            selected.append(row["state"])
    if len(selected) > 1:
        raise ValueError("alerts_duplicate")
    return selected[0] if selected else None


def delivered(payload, status, since):
    if not isinstance(payload, dict) or not isinstance(payload.get("alerts"), list) or len(payload["alerts"]) > 32:
        raise ValueError("receipts_bound")
    for row in payload["alerts"]:
        if not isinstance(row, dict) or not isinstance(row.get("labels"), dict):
            raise ValueError("receipts_shape")
        if (row.get("status") == status and finite_time(row.get("timestamp")) and row["timestamp"] >= since
                and all(row["labels"].get(key) == value for key, value in IDENTITY.items())):
            return True
    return False


def receipt(state, now):
    return {"issue": 1544, "overall": state["phase"] if state["phase"] in ("PASS", "FAIL") else "IN_PROGRESS",
            "phase": state["phase"], "started_utc": utc(state["started"]), "observed_utc": utc(now),
            "deadline_seconds": DEADLINE_SECONDS, "steps": state["steps"], "transport_errors": state["errors"],
            "markers_utc": {key: utc(value) for key, value in state["markers"].items()}, "reason": state["reason"],
            "alert": IDENTITY, "scope": "Monitoring-only synthetic alert; no DEX calls or availability/lag fault injection."}


def step(state_path=STATE_PATH, receipt_path=RECEIPT_PATH, *, read=request, clock=time.time):
    now = clock()
    state = load(state_path)
    if state is not None and state["phase"] in ("PASS", "FAIL"):
        # Persisted completion is final across every container replacement.
        return receipt(state, now)
    initial = state is None
    if initial:
        state = {"started": now, "phase": "pending", "steps": 1, "errors": 0, "markers": {}, "reason": None}
        # Persist before sending enable: an interrupted/ambiguous POST is never
        # retried and cannot renew the exporter's original 180-second drill.
        persist(state_path, state)
    else:
        state["steps"] = min(MAX_STEPS, state["steps"] + 1)
    try:
        if now - state["started"] >= DEADLINE_SECONDS or state["steps"] >= MAX_STEPS:
            state["phase"], state["reason"] = "FAIL", "deadline" if now - state["started"] >= DEADLINE_SECONDS else "step_bound"
            read("toggle", {"enabled": False})
        elif initial:
            if read("toggle", {"enabled": True}) != {"enabled": True}:
                raise ValueError("enable_unconfirmed")
        elif state["phase"] in ("pending", "firing"):
            observed = active_state(read("active"))
            if observed == "pending":
                state["markers"].setdefault("pending", now)
                state["phase"] = "firing"
            elif observed == "firing":
                state["markers"]["firing"] = now
                state["phase"] = "delivery"
        elif state["phase"] == "delivery":
            if delivered(read("receipts"), "firing", state["started"]):
                state["markers"]["delivered"] = now
                if read("toggle", {"enabled": False}) != {"enabled": False}:
                    raise ValueError("disable_unconfirmed")
                state["markers"]["disabled"] = now
                state["phase"] = "recovery"
        elif state["phase"] == "recovery":
            active = active_state(read("active"))
            recovered = delivered(read("receipts"), "resolved", state["markers"]["disabled"])
            if active is None and recovered:
                state["markers"]["recovered"] = now
                if "pending" in state["markers"]:
                    state["phase"] = "PASS"
                else:
                    state["phase"], state["reason"] = "FAIL", "pending_not_observed"
    except Exception:
        # Bodies, exception text, URLs and arbitrary remote labels never persist.
        state["errors"] = min(MAX_STEPS, state["errors"] + 1)
    persist(state_path, state)
    result = receipt(state, clock())
    persist(receipt_path, result)
    return result


def run_once():
    """Run the existing finite transition without starting another process."""
    try:
        result = step()
    except Exception:
        result = {"issue": 1544, "overall": "FAIL", "reason": "invalid_persistent_state"}
        try:
            persist(RECEIPT_PATH, result)
        except OSError:
            pass
    print(json.dumps(result, separators=(",", ":"), sort_keys=True), flush=True)
    return int(result["overall"] == "FAIL")


def run_loop():
    # PID 1 owns termination. An in-flight finite transition completes before
    # exiting. The handler only assigns a flag: Event.set() could deadlock when
    # a signal interrupts its own non-reentrant condition lock. Idle shutdown
    # can take up to five seconds; it does not claim an immediate wakeup.
    stopped = False
    def stop(_signal, _frame):
        nonlocal stopped
        stopped = True
    previous = {number: signal.signal(number, stop)
                for number in (signal.SIGTERM, signal.SIGINT)}
    try:
        while not stopped:
            run_once()
            if not stopped:
                time.sleep(5)
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--step", action="store_true")
    mode.add_argument("--loop", action="store_true")
    args = parser.parse_args()
    return run_loop() if args.loop else run_once()


if __name__ == "__main__":
    raise SystemExit(main())
