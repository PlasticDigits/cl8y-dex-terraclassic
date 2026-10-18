#!/usr/bin/env python3
"""Read bounded native producer/rejection evidence from Grafana; never export log lines."""
import argparse
import importlib.util
import json
import os
import re
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote, urlencode

SPEC = importlib.util.spec_from_file_location("monitoring", Path(__file__).with_name("probe-swap-monitoring-1479-1480.py"))
MONITORING = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITORING)
NATIVE_READ_BUDGET = 18
PATTERNS = {
    "wrapper_completed": "Native wrapper observations completed",
    "wrapper_unavailable": "Native wrapper observations unavailable",
    "wrapper_read_unavailable": "Native wrapper read unavailable",
    "stale_prerequisite": "Local quote prerequisite stale",
    "reference_committed": "Hub mark cohort committed",
    "reference_woken": "Reference refresh scheduled",
    "provider_pressure": r'(?i)(LCD provider admission deferred|all provider circuits open|reason="?(capacity|circuit_open))',
    "historical_height": "(?i)(historical|height.*(unavailable|not available|not found|pruned)|requested height|lowest height)",
    "http_429": "429",
    "deadline": "(?i)(timeout|deadline|exhausted|timed out)",
    "admission_budget": "Local quote admission exhausted pass budget",
    "errors": "(?i)(error|panic|fatal)",
}
PHASES = frozenset(("uluna_tax_cap", "uusd_tax_cap", "uluna_mapping", "uusd_mapping", "mapper_floor", "treasury_floor", "identity", "data", "chain", "mapper", "treasury", "mappings", "output", "floor_capabilities", "mapper_config", "treasury_config", "denom_bindings", "denom_mapping", "rate_limit", "tax_cap", "tax_rate", "router_config", "floor_capability", "hub_marks", "reference_marks", "reference_schedule", "reference_accounting"))
REASONS = frozenset(("mapper_paused", "treasury_wrapping_paused", "capacity", "circuit_open", "provider_unavailable", "chain_identity", "historical_height", "lagging", "contract_rejected", "query_gas_limit", "http_rejected", "malformed", "deadline", "cancelled", "missing_or_invalid", "missing_observations", "stale_observations", "accounting_unavailable", "protection", "unsupported_semantics", "no_route"))
REQUEST_ID = re.compile(r"[0-9a-f]{16}")
COMPONENTS = frozenset(("pair_config", "pool_mirror", "hybrid_mirror", "reference", "tax_policy", "tax_actor", "wrapper", "simulation_mirror"))
READ_SLOTS = {
    "chain": "chain", "mapper_config": "mapper_config", "router_config": "router_config",
    "treasury_config": "treasury_config", "denom_bindings": "denom_bindings",
    "uluna_mapping": "denom_mapping", "uusd_mapping": "denom_mapping",
    "uluna_rate": "rate_limit", "uusd_rate": "rate_limit",
    "uluna_cap": "tax_cap", "uusd_cap": "tax_cap", "tax_rate": "tax_rate",
    "mapper_floor": "floor_capability", "treasury_floor": "floor_capability",
}
MISSING_STAGES = {
    "asset_identity": "pair_config", "pair_admission": "pair_config",
    "mirror_admission": "simulation_mirror", "simulation_mirror": "simulation_mirror",
    "simulation_fee": "simulation_mirror", "tax_pair": "tax_policy",
    "tax_actor": "tax_actor", "tax_balance": "tax_actor", "reference_price": "reference",
    "native_mapping": "wrapper", "native_router": "wrapper",
    "native_tax_rate": "wrapper", "native_tax_cap": "wrapper",
}
READ_STAGES = frozenset(("denom_dispatch", "global_dispatch", "lcd_read"))
READ_REASONS = frozenset(("capacity", "circuit_open", "provider_unavailable", "chain_identity", "historical_height", "lagging", "contract_rejected", "query_gas_limit", "http_rejected", "malformed", "deadline", "cancelled"))


def read_witness(line, timestamp=None):
    """Fixed failure evidence. LCD-future polling does not prove a wire attempt."""
    if "Native wrapper read unavailable" not in line:
        return None

    def field(name, pattern):
        values = re.findall(r'\b' + name + r'="?(' + pattern + r')"?(?=\s|$|[,}])', line)
        return values[0] if len(values) == 1 else None

    slot = field("slot", r"[a-z_]{1,24}")
    phase = field("phase", r"[a-z_]{1,24}")
    stage = field("stage", r"[a-z_]{1,24}")
    reason = field("reason", r"[a-z_]{1,24}")
    polled = field("lcd_polled", "true|false")
    if slot not in READ_SLOTS or phase != READ_SLOTS[slot] or stage not in READ_STAGES or reason not in READ_REASONS:
        return None
    if polled not in ("true", "false") or (polled == "true") != (stage == "lcd_read"):
        return None
    row = {"slot": slot, "phase": phase, "reason": reason, "stage": stage, "lcd_polled": polled == "true"}
    for name in ("poll_elapsed_ms", "denom_wait_ms", "global_wait_ms", "active_ms"):
        value = field(name, r"\d{1,5}")
        if value is None or int(value) > 60_000:
            return None
        row[name] = int(value)
    native = slot.startswith(("uluna_", "uusd_"))
    if (not native and (stage == "denom_dispatch" or row["denom_wait_ms"] != 0)
            or stage != "lcd_read" and row["active_ms"] != 0
            or stage == "denom_dispatch" and row["global_wait_ms"] != 0):
        return None
    poll_id = field("poll_id", r"\d{1,20}")
    row["poll_id"] = int(poll_id) if poll_id is not None and int(poll_id) <= 2**64 - 1 else None
    row["sample_timestamp_ms"] = int(timestamp) // 1_000_000 if isinstance(timestamp, str) and re.fullmatch(r"\d{1,20}", timestamp) else None
    return row



def scheduled_alternative_witness(line, timestamp=None):
    """Actual prior attempts and pending local capacity; no provider identity."""
    if "LCD scheduled alternatives deferred" not in line:
        return None

    def field(name, pattern):
        values = re.findall(r'\b' + name + r'="?(' + pattern + r')"?(?=\s|$|[,}])', line)
        return values[0] if len(values) == 1 else None

    if field("phase", r"[a-z_]{1,24}") != "data" or field("reason", r"[a-z_]{1,24}") != "capacity":
        return None
    row = {"phase": "data", "reason": "capacity"}
    for name, maximum, minimum in (("attempt_count", 8, 1),
                                   ("unattempted_capacity_count", 31, 1),
                                   ("original_remaining_ms", 30_000, 0)):
        value = field(name, r"\d{1,5}")
        if value is None or not minimum <= int(value) <= maximum:
            return None
        row[name] = int(value)
    if row["attempt_count"] + row["unattempted_capacity_count"] > 32:
        return None
    row["sample_timestamp_ms"] = int(timestamp) // 1_000_000 if isinstance(timestamp, str) and re.fullmatch(r"\d{1,20}", timestamp) else None
    return row


def wrapper_dispatch_witness(line, timestamp=None):
    """Bounded poll-local borrowing count, without endpoints or raw labels."""
    if "Native wrapper observations completed" not in line:
        return None
    if len(re.findall(r'\bnative_borrowed_reads=', line)) != 1:
        return None
    value = re.search(r'\bnative_borrowed_reads=(\d)(?=\s|$|[,}])', line)
    if value is None or int(value[1]) > 6:
        return None
    poll_ids = re.findall(r'\bpoll_id=(\d{1,20})(?=\s|$|[,}])', line)
    poll_id = int(poll_ids[0]) if len(poll_ids) == 1 and int(poll_ids[0]) <= 2**64 - 1 else None
    return {"native_borrowed_reads": int(value[1]), "poll_id": poll_id,
        "sample_timestamp_ms": int(timestamp) // 1_000_000 if isinstance(timestamp, str) and re.fullmatch(r"\d{1,20}", timestamp) else None}


def missing_witness(line, requested_ids):
    """A fixed missing stage with explicitly unknown original clocks."""
    if "Local quote prerequisite missing" not in line:
        return None

    def field(name, pattern):
        values = re.findall(r'\b' + name + r'="?(' + pattern + r')"?(?=\s|$|[,}])', line)
        return values[0] if len(values) == 1 else None

    stage = field("stage", r"[a-z_]{1,24}")
    component = field("component", r"[a-z_]{1,24}")
    if stage not in MISSING_STAGES or component != MISSING_STAGES[stage]:
        return None
    if any(field(name, "true|false") != "false" for name in ("age_known", "expiry_known")):
        return None
    if any(field(name, r"-?\d{1,9}") != "0" for name in ("age_ms", "expires_in_ms")):
        return None
    row = {"component": component, "stage": stage, "age_ms": None, "expires_in_ms": None}
    for rid in requested_ids:
        if re.search(r'\brequest_id[=:]"?' + rid + r'\b', line):
            row["request_id"] = rid
            break
    return row


def stale_witness(line, requested_ids):
    """A bounded candidate witness is evidence, not a sole-cause assertion."""
    if "Local quote prerequisite stale" not in line:
        return None
    match = re.search(r'\bcomponent="?([a-z_]{1,24})"?(?=\s|$|[,}])', line)
    if not match or match[1] not in COMPONENTS:
        return None
    row = {"component": match[1]}
    for flag, field in (("age_known", "age_ms"), ("expiry_known", "expires_in_ms")):
        known = re.search(r'\b' + flag + r'=(true|false)(?=\s|$|[,}])', line)
        value = re.search(r'\b' + field + r'=(-?\d{1,9})(?=\s|$|[,}])', line)
        if not known or not value or abs(int(value[1])) > 86_400_000:
            return None
        row[field] = int(value[1]) if known[1] == "true" else None
    for rid in requested_ids:
        if re.search(r'\brequest_id[=:]"?' + rid + r'\b', line):
            row["request_id"] = rid
            break
    return row


def sample_summary(payload, requested_ids=()):
    if not isinstance(payload, dict) or payload.get("status") != "success":
        raise ValueError("sample_unavailable")
    data = payload.get("data", {})
    streams = data.get("result", [])
    if data.get("resultType") != "streams" or not isinstance(streams, list) or len(streams) > 300:
        raise ValueError("sample_shape")
    lines, records = [], []
    for stream in streams:
        values = stream.get("values", []) if isinstance(stream, dict) else None
        if not isinstance(values, list) or len(values) > 300:
            raise ValueError("sample_bound")
        for value in values:
            if not isinstance(value, list) or len(value) != 2 or not isinstance(value[1], str) or len(value[1]) > 32768:
                raise ValueError("sample_shape")
            lines.append(value[1])
            records.append(value)
            if len(lines) > 300:
                raise ValueError("sample_bound")
    fields = {"phase": PHASES, "reason": REASONS, "direction": frozenset(("native_input", "native_output", "native_both", "cw20"))}
    counts = {field: {} for field in fields}
    for line in lines:
        # Tracing's ANSI escapes are ignored; only exact enum values are retained.
        line = re.sub(r"\x1b\[[0-9;]*m", "", line)
        for field, allowlist in fields.items():
            for value in re.findall(r'\b' + field + r'=(?:"([a-z0-9_]{1,40})"|([a-z0-9_]{1,40}))(?=\s|$|[,}])', line):
                selected = value[0] or value[1]
                if selected in allowlist:
                    counts[field][selected] = counts[field].get(selected, 0) + 1
    witnesses = [w for line in lines if (w := stale_witness(re.sub(r"\x1b\[[0-9;]*m", "", line), requested_ids)) is not None]
    missing = [w for line in lines if (w := missing_witness(re.sub(r"\x1b\[[0-9;]*m", "", line), requested_ids)) is not None]
    reads = [w for timestamp, line in records if (w := read_witness(re.sub(r"\x1b\[[0-9;]*m", "", line), timestamp)) is not None]
    alternatives = [w for timestamp, line in records if (w := scheduled_alternative_witness(re.sub(r"\x1b\[[0-9;]*m", "", line), timestamp)) is not None]
    dispatches = [w for timestamp, line in records if (w := wrapper_dispatch_witness(re.sub(r"\x1b\[[0-9;]*m", "", line), timestamp)) is not None]
    return {"bounded_sample_lines": len(lines), "typed_fields": counts, "candidate_stale_witnesses": witnesses,
        "candidate_missing_witnesses": missing, "producer_read_witnesses": reads, "scheduled_alternative_witnesses": alternatives,
        "wrapper_dispatch_witnesses": dispatches,
        "request_correlation": {rid: sum(bool(re.search(r'\brequest_id[=:]"?' + rid + r'\b', re.sub(r"\x1b\[[0-9;]*m", "", line))) for line in lines) for rid in requested_ids}}


def collect(instance, request_ids=()):
    if len(request_ids) > 8 or any(not REQUEST_ID.fullmatch(rid) for rid in request_ids):
        raise ValueError("correlation_bound")
    sources = instance.read("/api/datasources")
    if not isinstance(sources, list) or len(sources) > 32:
        raise ValueError("datasource_bound")
    loki = [row for row in sources if isinstance(row, dict) and row.get("type") == "loki"]
    if len(loki) != 1 or not MONITORING.ATTESTER.APP_ID.fullmatch(str(loki[0].get("uid", ""))):
        raise ValueError("loki_identity")
    prefix = "/api/datasources/proxy/uid/" + quote(loki[0]["uid"], safe="") + "/loki/api/v1/"
    selector = '{service="cl8y-dex-terraclassic-indexer"}'
    counts = {}
    for label, pattern in PATTERNS.items():
        query = "sum(count_over_time(" + selector + " |~ " + json.dumps(pattern) + "[15m]))"
        counts[label] = MONITORING.aggregate(instance.read(prefix + "query?" + urlencode({"query": query})))
    params = {"query": selector + ' |~ "(Native wrapper|LCD scheduled admission deferred|LCD scheduled alternatives deferred|Local quote rejected|Local quote prerequisite stale|Local quote prerequisite missing|Hub mark cohort committed|Reference refresh scheduled)"',
        "start": str(int((time.time() - 900) * 1e9)), "end": str(int(time.time() * 1e9)), "limit": "300", "direction": "backward"}
    sample = sample_summary(instance.read(prefix + "query_range?" + urlencode(params)))
    if request_ids:
        params["query"] = selector + " |~ " + json.dumps("|".join(request_ids))
        requested = sample_summary(instance.read(prefix + "query_range?" + urlencode(params)), request_ids)
        sample["request_correlation"] = requested["request_correlation"]
        sample["candidate_stale_witnesses"] = requested["candidate_stale_witnesses"]
        sample["candidate_missing_witnesses"] = requested["candidate_missing_witnesses"]
    prometheus = [row for row in sources if isinstance(row, dict) and row.get("type") == "prometheus"]
    prom = {"status": "SKIP", "reason": "no_prometheus_datasource_provisioned"}
    if len(prometheus) == 1 and MONITORING.ATTESTER.APP_ID.fullmatch(str(prometheus[0].get("uid", ""))):
        prom = {"status": "PASS", "up_sum": MONITORING.aggregate(instance.read("/api/datasources/proxy/uid/" + quote(prometheus[0]["uid"], safe="") + "/api/v1/query?" + urlencode({"query": "sum(up)"})))}
    return {"loki": {"status": "PASS", "window_seconds": 900, "pattern_counts": counts, **sample}, "prometheus": prom, "read_count": instance.reader.reads}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--request-id", action="append", default=[])
    args = parser.parse_args()
    if len(args.request_id) > 8 or any(not REQUEST_ID.fullmatch(rid) for rid in args.request_id):
        parser.error("at most eight fixed 16-character lower-hex request IDs")
    receipt = {"issue": 1505, "utc": datetime.now(timezone.utc).isoformat(), "read_only": True,
        "read_budget": NATIVE_READ_BUDGET,
        "scope": "15-minute fixed pattern counts and at most 300 sampled lines summarized to fixed enums and bounded producer-read/alternative-admission diagnostics; no log bodies, actors or endpoints exported; LCD-future polling does not prove a wire attempt and pattern counts do not prove causality"}
    try:
        receipt.update(collect(MONITORING.Monitor(read_budget=NATIVE_READ_BUDGET), args.request_id))
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
