#!/usr/bin/env python3
"""Focused acceptance controls for native recovery timing/correlation and private log boundaries."""
import copy
import importlib.util
import json
import re
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


PROBE = module("native_probe", "probe-native-recovery-1505.py")
MONITOR = module("native_monitor", "probe-native-monitoring-1505.py")
FIXTURES = module("quote_fixtures", "test-probe-issue-1473.py")


class Tests(unittest.TestCase):
    def test_verifier_cannot_mask_an_earlier_nested_command_failure(self):
        root = Path(__file__).resolve().parents[2]
        prefix = (root / "scripts/verify-issue-1505.sh").read_text().split("\nstart\n", 1)[0]
        with tempfile.TemporaryDirectory(prefix="native-verifier-status-") as temporary:
            fixture = Path(temporary)
            (fixture / "scripts/qa").mkdir(parents=True)
            script = fixture / "scripts/verify-issue-1505.sh"
            script.write_text(prefix)
            shutil.copyfile(root / "scripts/qa/verify-followup-common.sh", fixture / "scripts/qa/verify-followup-common.sh")
            # Shell functions replace every executable: no Cargo, Node, Python
            # probe, database connection or private env file is used.
            control = r"""
source "$1"
seen=0
command_stub() { seen=$((seen+1)); [[ "$seen" -ne "$FAIL_AT" ]]; }
cargo() { command_stub; }
node() { command_stub; }
python3() { command_stub; }
run injected "$GROUP"
if [[ "$FAIL_AT" -eq 0 ]]; then
  [[ "$FAIL" -eq 0 && "$PASS" -eq 1 && "$seen" -eq "$TOTAL" ]]
else
  [[ "$FAIL" -eq 1 && "$PASS" -eq 0 && "$seen" -eq "$FAIL_AT" ]]
fi
"""
            for group, total in (("rust", 8), ("sockets", 2), ("qa", 2)):
                for failure in range(total + 1):
                    with self.subTest(group=group, failure=failure):
                        env = {"PATH": os.environ["PATH"], "GROUP": group,
                               "FAIL_AT": str(failure), "TOTAL": str(total),
                               "TEST_DATABASE_ADMIN_URL": "postgres://unused"}
                        result = subprocess.run(["bash", "-c", control, "status-control", str(script)],
                                                cwd=fixture, env=env, capture_output=True, text=True, timeout=5)
                        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_original_expiry_and_request_id_are_independent_requirements(self):
        case, body = FIXTURES.Tests.executable_fixture()
        seen = set()
        good = PROBE.evaluate(200, body, case, 2000, "0000000000000001", seen, True)
        self.assertTrue(good["useful"] and good["correlation_pass"])
        self.assertEqual(good["observation"]["age_at_validation_ms"], 1000)
        repeated = PROBE.evaluate(200, body, case, 2000, "0000000000000001", seen, True)
        self.assertTrue(repeated["useful"])
        self.assertFalse(repeated["correlation_pass"])
        expired = PROBE.evaluate(200, body, case, 21000, "0000000000000002", seen, True)
        self.assertFalse(expired["useful"])
        self.assertIn("observation_expired", expired["validation_diagnostic"]["reasons"])

    def test_fast_typed_error_is_classified_but_never_useful_acceptance(self):
        case, _ = FIXTURES.Tests.executable_fixture()
        row = PROBE.evaluate(503, {"error": "missing_observations", "private_actor": "do-not-export"}, case, 2000, "0000000000000001", set(), True)
        self.assertTrue(row["classified"] and row["correlation_pass"])
        self.assertFalse(row["useful"])
        row.update(pair="native", sample=0, transport="reused", elapsed_ms=1, socket_budget_pass=False)
        failures = PROBE.summarize([row], ["native"])
        self.assertIn("native_unavailable", {f["failure_class"] for f in failures})
        self.assertNotIn("do-not-export", json.dumps(row))

    def test_missing_invalid_and_client_supplied_ids_cannot_pass_required_correlation(self):
        case, body = FIXTURES.Tests.executable_fixture()
        for request_id in (None, "client-private-value", "A" * 16, "0" * 17):
            with self.subTest(request_id=request_id):
                row = PROBE.evaluate(200, body, case, 2000, request_id, set(), True)
                self.assertFalse(row["correlation_pass"])
                self.assertNotIn("request_id", row)
        self.assertTrue(PROBE.evaluate(200, body, case, 2000, None, set(), False)["correlation_pass"])

    def test_all_targeted_nonuseful_samples_and_slow_initialized_quotes_remain_failures(self):
        base = {"pair": "native", "status": 200, "classified": True, "correlation_pass": True,
            "sample": 0, "useful": True, "transport": "reused", "socket_budget_pass": True,
            "outcome": "executable", "observation": {"observed_at_ms": 1000}}
        replacement = {**base, "sample": 1, "observation": {"observed_at_ms": 2000}}
        self.assertEqual(PROBE.summarize([base, replacement], ["native"]), [])
        for changes, expected in (({"useful": False, "outcome": "client_validation"}, "native_unavailable"),
                                  ({"socket_budget_pass": False}, "initialized_socket_over500"),
                                  ({"correlation_pass": False}, "request_correlation")):
            failures = PROBE.summarize([base, replacement, {**base, **changes, "sample": 2}], ["native"])
            self.assertIn(expected, {f["failure_class"] for f in failures})
        self.assertIn("no_observation_replacement", {f["failure_class"] for f in PROBE.summarize([base], ["native"])})

    def test_native_probe_keeps_the_fixed_two_original_inputs(self):
        cases = list(PROBE.cases())
        self.assertEqual([c["pair"] for c in cases], ["USTC->CL8Y", "CL8Y->USTC"])
        self.assertEqual([c["amount"] for c in cases], ["132000000", "132000000000000000000"])
        self.assertTrue(all(c["spread"] == "0.05" for c in cases))

    def test_logs_export_only_enum_counts_and_requested_correlation(self):
        payload = {"status": "success", "data": {"resultType": "streams", "result": [{"stream": {"actor": "do-not-export"}, "values": [
            ["1", '\x1b[32mphase\x1b[0m="denom_mapping" reason="capacity" direction="native_input" request_id=0000000000000001 private=https://private.invalid/actor'],
            ["2", 'phase="private_phase" reason="private_reason" request_id=0000000000000002'],
            ["3", 'phase="output" reason="deadline"'],
            ["4", 'LCD scheduled admission deferred phase="identity" reason="circuit_open" attempts=0 private=https://private.invalid/actor'],
        ]}]}}
        result = MONITOR.sample_summary(payload, ["0000000000000001"])
        self.assertEqual(result["typed_fields"], {"phase": {"denom_mapping": 1, "output": 1, "identity": 1}, "reason": {"capacity": 1, "deadline": 1, "circuit_open": 1}, "direction": {"native_input": 1}})
        self.assertEqual(result["request_correlation"], {"0000000000000001": 1})
        self.assertNotIn("private", json.dumps(result))
        payload["data"]["result"][0]["values"] *= 101
        with self.assertRaises(ValueError):
            MONITOR.sample_summary(payload)

    def test_pressure_pattern_does_not_count_benign_capacity_census_fields(self):
        pattern = MONITOR.PATTERNS["provider_pressure"]
        self.assertIsNone(re.search(pattern, "Book snapshot cycle completed head_capacity_waits=0"))
        self.assertIsNone(re.search(pattern, 'Native wrapper read unavailable phase="rate_limit" reason="malformed"'))
        for line in ('LCD provider admission deferred', 'all provider circuits open', 'reason="capacity"', 'reason="circuit_open"'):
            self.assertIsNotNone(re.search(pattern, line))

    def test_stale_witnesses_keep_only_fixed_components_bounded_clocks_and_requested_ids(self):
        line = 'Local quote prerequisite stale component="reference" age_known=true age_ms=20589 expiry_known=true expires_in_ms=-589 request_id=0000000000000001 actor=do-not-export'
        self.assertEqual(MONITOR.stale_witness(line, ["0000000000000001"]), {
            "component": "reference", "age_ms": 20589, "expires_in_ms": -589,
            "request_id": "0000000000000001"})
        self.assertNotIn("request_id", MONITOR.stale_witness(line, []))
        unknown = line.replace('age_known=true', 'age_known=false').replace('expiry_known=true', 'expiry_known=false')
        self.assertIsNone(MONITOR.stale_witness(unknown, [])["age_ms"])
        self.assertIsNone(MONITOR.stale_witness(unknown, [])["expires_in_ms"])
        for bad in (line.replace('"reference"', '"private_component"'), line.replace('20589', '86400001'), line.replace('20589', 'private_age')):
            self.assertIsNone(MONITOR.stale_witness(bad, []))
        payload = {"status": "success", "data": {"resultType": "streams", "result": [{"values": [["1", line]]}]}}
        result = MONITOR.sample_summary(payload, ["0000000000000001"])
        self.assertEqual(len(result["candidate_stale_witnesses"]), 1)
        self.assertNotIn("do-not-export", json.dumps(result))

    def test_monitoring_uses_bounded_fixed_queries_without_raw_lines(self):
        instance = type("Monitor", (), {"reader": type("Reader", (), {"reads": 2})()})()
        paths = []
        def read(path):
            paths.append(path)
            instance.reader.reads += 1
            if path == "/api/datasources":
                return [{"type": "loki", "uid": "Loki", "url": "do-not-export"}]
            if "query_range" in path:
                return {"status": "success", "data": {"resultType": "streams", "result": []}}
            return {"status": "success", "data": {"resultType": "vector", "result": [{"value": [0, "2"], "metric": {"actor": "do-not-export"}}]}}
        instance.read = read
        result = MONITOR.collect(instance, ["0000000000000001"])
        self.assertEqual(result["read_count"], 17)
        self.assertEqual(result["prometheus"]["status"], "SKIP")
        self.assertNotIn("do-not-export", json.dumps(result))
        self.assertTrue(all("limit=300" in p for p in paths if "query_range" in p))
        with self.assertRaises(ValueError):
            MONITOR.collect(instance, ["actor/private"])


if __name__ == "__main__":
    unittest.main()
