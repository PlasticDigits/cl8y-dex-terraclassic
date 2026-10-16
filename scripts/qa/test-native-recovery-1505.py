#!/usr/bin/env python3
"""Focused acceptance controls for native recovery timing/correlation and private log boundaries."""
import copy
import io
import importlib.util
import json
import re
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from contextlib import contextmanager, redirect_stdout


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


PROBE = module("native_probe", "probe-native-recovery-1505.py")
MONITOR = module("native_monitor", "probe-native-monitoring-1505.py")
FIXTURES = module("quote_fixtures", "test-probe-issue-1473.py")


@contextmanager
def monitoring_transport(prometheus=False):
    """Exercise the real Reader/Monitor caps with only in-memory HTTP responses."""
    class Response:
        def __init__(self, payload):
            self.data = json.dumps(payload).encode()

        def __enter__(self):
            return self

        def __exit__(self, *_args):
            return False

        def read(self, limit):
            return self.data[:limit]

    class Opener:
        def __init__(self):
            self.requests = []

        def open(self, request, timeout):
            self.requests.append(request.full_url)
            path = MONITOR.MONITORING.ATTESTER.urllib.parse.urlsplit(request.full_url).path
            if path == "/api/v1/applications":
                payload = [{"name": "cl8y-logs", "uuid": "logs-test"}]
            elif path == "/api/v1/applications/logs-test/envs":
                payload = [{"key": "GRAFANA_ADMIN_USER", "value": "synthetic-private-user"},
                           {"key": "GRAFANA_ADMIN_PASSWORD", "value": "synthetic-private-password"}]
            elif path == "/api/datasources":
                payload = [{"type": "loki", "uid": "Loki", "url": "synthetic-private-endpoint"}]
                if prometheus:
                    payload.append({"type": "prometheus", "uid": "Prometheus"})
            elif path.endswith("/query_range"):
                payload = {"status": "success", "data": {"resultType": "streams", "result": []}}
            elif path.endswith("/query"):
                payload = {"status": "success", "data": {"resultType": "vector", "result": [
                    {"value": [0, "2"], "metric": {"actor": "synthetic-private-actor"}}]}}
            else:
                raise AssertionError("unexpected_monitoring_test_path")
            return Response(payload)

    opener = Opener()
    with patch.dict(os.environ, {"COOLIFY_URL": "https://coolify.invalid", "COOLIFY_WIRE_TOKEN": "synthetic-private-token"}), \
            patch.object(MONITOR.MONITORING.ATTESTER.urllib.request, "build_opener", return_value=opener):
        yield opener


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
            for group, total in (("rust", 11), ("sockets", 2), ("qa", 3)):
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

    def test_read_witnesses_distinguish_dispatch_from_logical_lcd_polling(self):
        queued = ('Native wrapper read unavailable poll_id=96 slot="uusd_cap" phase="tax_cap" '
                  'reason="deadline" stage="global_dispatch" poll_elapsed_ms=7901 '
                  'denom_wait_ms=1 global_wait_ms=1200 active_ms=0 lcd_polled=false '
                  'actor=do-not-export endpoint=https://private.invalid/body')
        row = MONITOR.read_witness(queued, "1792089309106000000")
        self.assertEqual(row["poll_id"], 96)
        self.assertEqual(row["sample_timestamp_ms"], 1792089309106)
        self.assertFalse(row["lcd_polled"])
        self.assertEqual((row["denom_wait_ms"], row["global_wait_ms"], row["active_ms"]), (1, 1200, 0))
        active = queued.replace('stage="global_dispatch"', 'stage="lcd_read"').replace(
            'active_ms=0 lcd_polled=false', 'active_ms=60000 lcd_polled=true')
        polled = MONITOR.read_witness(active)
        self.assertTrue(polled["lcd_polled"])
        self.assertEqual(polled["active_ms"], 60000)
        self.assertNotIn("wire", json.dumps(polled))
        waiting = queued.replace('stage="global_dispatch"', 'stage="denom_dispatch"').replace(
            'global_wait_ms=1200', 'global_wait_ms=0').replace('reason="deadline"', 'reason="cancelled"')
        self.assertEqual(MONITOR.read_witness(waiting)["stage"], "denom_dispatch")
        self.assertNotIn("do-not-export", json.dumps([row, polled, MONITOR.read_witness(waiting)]))
        self.assertNotIn("private.invalid", json.dumps(row))
        self.assertIsNone(MONITOR.read_witness('Native wrapper read unavailable phase="uusd_tax_cap" reason="deadline"'))

    def test_read_witnesses_refuse_unknown_or_inconsistent_evidence(self):
        line = ('Native wrapper read unavailable poll_id=96 slot="uusd_cap" phase="tax_cap" '
                'reason="deadline" stage="global_dispatch" poll_elapsed_ms=7901 '
                'denom_wait_ms=1 global_wait_ms=1200 active_ms=0 lcd_polled=false')
        invalid = (
            line.replace('slot="uusd_cap"', 'slot="private_slot"'),
            line.replace('phase="tax_cap"', 'phase="rate_limit"'),
            line.replace('stage="global_dispatch"', 'stage="wire_attempt"'),
            line.replace('reason="deadline"', 'reason="private_reason"'),
            line.replace('lcd_polled=false', 'lcd_polled=true'),
            line.replace('active_ms=0', 'active_ms=1'),
            line.replace('stage="global_dispatch"', 'stage="denom_dispatch"'),
            line.replace('slot="uusd_cap" phase="tax_cap"', 'slot="router_config" phase="router_config"'),
            line.replace('active_ms=0 ', ''),
            line.replace('global_wait_ms=1200', 'global_wait_ms=60001'),
            line.replace('global_wait_ms=1200', 'global_wait_ms=-1'),
            line + ' stage="global_dispatch"',
        )
        for bad in invalid:
            with self.subTest(bad=bad):
                self.assertIsNone(MONITOR.read_witness(bad))
        unknown = MONITOR.read_witness(line.replace('poll_id=96', 'poll_id=private_actor'), "private_timestamp")
        self.assertIsNone(unknown["poll_id"])
        self.assertIsNone(unknown["sample_timestamp_ms"])
        self.assertNotIn("private", json.dumps(unknown))

    def test_read_witness_samples_keep_the_existing_resource_and_privacy_bounds(self):
        line = ('Native wrapper read unavailable poll_id=96 slot="uluna_mapping" phase="denom_mapping" '
                'reason="deadline" stage="denom_dispatch" poll_elapsed_ms=7901 '
                'denom_wait_ms=7900 global_wait_ms=0 active_ms=0 lcd_polled=false actor=do-not-export')
        payload = {"status": "success", "data": {"resultType": "streams", "result": [
            {"stream": {"actor": "do-not-export"}, "values": [["1792089309106000000", line]] * 300}]}}
        result = MONITOR.sample_summary(payload)
        self.assertEqual(result["bounded_sample_lines"], 300)
        self.assertEqual(len(result["producer_read_witnesses"]), 300)
        self.assertEqual(result["producer_read_witnesses"][0]["slot"], "uluna_mapping")
        self.assertNotIn("do-not-export", json.dumps(result))
        payload["data"]["result"][0]["values"].append(["1", line])
        with self.assertRaises(ValueError):
            MONITOR.sample_summary(payload)

    def test_scheduled_alternative_witnesses_keep_only_bounded_fixed_fields(self):
        line = ('LCD scheduled alternatives deferred phase="data" reason="capacity" '
                'attempt_count=1 unattempted_capacity_count=2 original_remaining_ms=30000 '
                'actor=do-not-export endpoint=https://private.invalid/body')
        row = MONITOR.scheduled_alternative_witness(line, "1792096605480000000")
        self.assertEqual(row, {"phase": "data", "reason": "capacity", "attempt_count": 1,
                              "unattempted_capacity_count": 2, "original_remaining_ms": 30000,
                              "sample_timestamp_ms": 1792096605480})
        self.assertEqual(MONITOR.scheduled_alternative_witness(line.replace(
            'original_remaining_ms=30000', 'original_remaining_ms=0'))['original_remaining_ms'], 0)
        self.assertNotIn("do-not-export", json.dumps(row))
        self.assertNotIn("private.invalid", json.dumps(row))
        for bad in (line.replace('phase="data"', 'phase="private"'),
                    line.replace('reason="capacity"', 'reason="provider_unavailable"'),
                    line.replace('attempt_count=1', 'attempt_count=0'),
                    line.replace('attempt_count=1', 'attempt_count=9'),
                    line.replace('unattempted_capacity_count=2', 'unattempted_capacity_count=0'),
                    line.replace('unattempted_capacity_count=2', 'unattempted_capacity_count=32'),
                    line.replace('attempt_count=1 unattempted_capacity_count=2',
                                 'attempt_count=8 unattempted_capacity_count=25'),
                    line.replace('original_remaining_ms=30000', 'original_remaining_ms=30001'),
                    line.replace('original_remaining_ms=30000', 'original_remaining_ms=-1'),
                    line.replace('original_remaining_ms=30000 ', ''),
                    line + ' attempt_count=1'):
            with self.subTest(bad=bad):
                self.assertIsNone(MONITOR.scheduled_alternative_witness(bad))

    def test_scheduled_alternative_samples_keep_existing_three_hundred_line_bound(self):
        line = ('LCD scheduled alternatives deferred phase="data" reason="capacity" '
                'attempt_count=8 unattempted_capacity_count=24 original_remaining_ms=42 '
                'actor=do-not-export')
        payload = {"status": "success", "data": {"resultType": "streams", "result": [
            {"stream": {"actor": "do-not-export"}, "values": [["1792096605480000000", line]] * 300}]}}
        result = MONITOR.sample_summary(payload)
        self.assertEqual(result['bounded_sample_lines'], 300)
        self.assertEqual(len(result['scheduled_alternative_witnesses']), 300)
        self.assertEqual(result['producer_read_witnesses'], [])
        self.assertNotIn('do-not-export', json.dumps(result))
        payload['data']['result'][0]['values'].append(['1', line])
        with self.assertRaises(ValueError):
            MONITOR.sample_summary(payload)

    def test_wrapper_dispatch_witness_keeps_only_bounded_fixed_fields(self):
        line = ('native_wrapper_poll{poll_id=119}: Native wrapper observations completed '
                'native_borrowed_reads=2 actor=do-not-export endpoint=https://private.invalid/body')
        self.assertEqual(MONITOR.wrapper_dispatch_witness(line, "1792109168515000000"), {
            "native_borrowed_reads": 2, "poll_id": 119, "sample_timestamp_ms": 1792109168515})
        for count in (0, 6):
            self.assertEqual(MONITOR.wrapper_dispatch_witness(line.replace(
                'native_borrowed_reads=2', 'native_borrowed_reads=' + str(count)))["native_borrowed_reads"], count)
        for value in ('7', '-1', '2.0', '02', 'private', '"2"'):
            with self.subTest(value=value):
                self.assertIsNone(MONITOR.wrapper_dispatch_witness(line.replace(
                    'native_borrowed_reads=2', 'native_borrowed_reads=' + value)))
        for extra in (' native_borrowed_reads=2', ' native_borrowed_reads=private'):
            self.assertIsNone(MONITOR.wrapper_dispatch_witness(line + extra))
        self.assertIsNone(MONITOR.wrapper_dispatch_witness(line.replace('native_borrowed_reads=2', '')))
        self.assertIsNone(MONITOR.wrapper_dispatch_witness(line.replace('observations completed', 'observations unavailable')))
        result = json.dumps(MONITOR.wrapper_dispatch_witness(line))
        self.assertNotIn('do-not-export', result)
        self.assertNotIn('private.invalid', result)

    def test_wrapper_dispatch_sample_preserves_existing_line_and_read_bounds(self):
        line = ('native_wrapper_poll{poll_id=119}: Native wrapper observations completed '
                'native_borrowed_reads=6 actor=do-not-export')
        payload = {"status": "success", "data": {"resultType": "streams", "result": [
            {"stream": {"actor": "do-not-export"}, "values": [["1792109168515000000", line]] * 300}]}}
        result = MONITOR.sample_summary(payload)
        self.assertEqual(result['bounded_sample_lines'], 300)
        self.assertEqual(len(result['wrapper_dispatch_witnesses']), 300)
        self.assertNotIn('do-not-export', json.dumps(result))
        self.assertEqual(MONITOR.NATIVE_READ_BUDGET, 18)
        self.assertEqual(len(MONITOR.PATTERNS), 12)
        payload['data']['result'][0]['values'].append(['1', line])
        with self.assertRaises(ValueError):
            MONITOR.sample_summary(payload)

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

    def test_native_monitoring_optional_branches_use_the_real_reader_budget(self):
        for prometheus in (False, True):
            for correlation in (False, True):
                with self.subTest(prometheus=prometheus, correlation=correlation), monitoring_transport(prometheus) as opener:
                    instance = MONITOR.MONITORING.Monitor(read_budget=MONITOR.NATIVE_READ_BUDGET)
                    ids = ["0000000000000001"] if correlation else []
                    result = MONITOR.collect(instance, ids)
                    expected = 16 + int(prometheus) + int(correlation)
                    self.assertEqual(result["read_count"], expected)
                    self.assertEqual(len(opener.requests), expected)
                    self.assertEqual(instance.read_budget, 18)
                    self.assertEqual(result["prometheus"]["status"], "PASS" if prometheus else "SKIP")
                    self.assertNotIn("synthetic-private", json.dumps(result))
                    while instance.reader.reads < instance.read_budget:
                        instance.read("/api/datasources")
                    self.assertEqual(len(opener.requests), 18)
                    with self.assertRaisesRegex(ValueError, "read_budget"):
                        instance.read("/api/datasources")
                    self.assertEqual(len(opener.requests), 18)

    def test_native_monitoring_cli_retains_old_cap_failure_and_uses_explicit_profile(self):
        for budget, expected_status in ((16, 1), (18, 0)):
            with self.subTest(budget=budget), monitoring_transport(True) as opener, tempfile.TemporaryDirectory() as temporary:
                output = Path(temporary) / "receipt.json"
                with patch.object(MONITOR, "NATIVE_READ_BUDGET", budget), \
                        patch.object(sys, "argv", ["native-monitor", "--output", str(output), "--request-id", "0000000000000001"]), \
                        redirect_stdout(io.StringIO()):
                    status = MONITOR.main()
                receipt = json.loads(output.read_text())
                self.assertEqual(status, expected_status)
                self.assertEqual(receipt["overall"], "FAIL" if budget == 16 else "PASS")
                self.assertEqual(len(opener.requests), budget)
                if budget == 16:
                    self.assertEqual(receipt["failure_class"], "ValueError")
                else:
                    self.assertEqual(receipt["read_count"], 18)
                    self.assertEqual(receipt["read_budget"], 18)
                self.assertEqual(output.stat().st_mode & 0o777, 0o600)
                self.assertNotIn("synthetic-private", json.dumps(receipt))

    def test_monitoring_budget_profile_is_closed_before_credentials_or_transport(self):
        self.assertEqual(MONITOR.MONITORING.Monitor.read_budget, 16)
        for bad in (0, 1, 17, 19, True, "18", 18.0):
            with self.subTest(bad=bad), patch.object(MONITOR.MONITORING.ATTESTER, "Reader") as reader:
                with self.assertRaisesRegex(ValueError, "read_budget"):
                    MONITOR.MONITORING.Monitor(read_budget=bad)
                reader.assert_not_called()


if __name__ == "__main__":
    unittest.main()
