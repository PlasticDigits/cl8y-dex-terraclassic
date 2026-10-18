#!/usr/bin/env python3
"""Read-only rollback diagnostic tests with synthetic credentials and Docker data."""
import copy
import datetime
import importlib.util
import json
from pathlib import Path
import types
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("diagnostic", Path(__file__).with_name("diagnose-loki-s3-host.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def require(condition, reason):
    if not condition:
        raise ValueError("synthetic-private-error")


def envmap(container):
    return dict(row.split("=", 1) for row in container["Config"]["Env"])


class DiagnosticTests(unittest.TestCase):
    def setUp(self):
        self.now = datetime.datetime.now(datetime.timezone.utc)
        self.values = {"LOKI_S3_JOB_UUID": m.JOB, "LOKI_S3_JOB_ID": "123", "LOKI_S3_PAYLOAD_JSON": "synthetic-private-payload"}
        self.data = {"expected_old_access": "synthetic-old-access", "expected_old_secret": "synthetic-old-secret",
                     "new_access": "synthetic-new-access", "new_secret": "synthetic-new-secret"}
        self.own = {"Id": "a" * 64, "State": {"Running": True}, "Config": {
            "Env": [k + "=" + v for k, v in self.values.items()], "Labels": {
                "coolify.managed": "true", "coolify.serviceUuid": m.JOB,
                "com.docker.compose.project": m.JOB, "com.docker.compose.service": "maintenance"}}}
        self.loki = {"Image": "sha256:" + "b" * 64, "Config": {"Env": [
            "S3_ACCESS_KEY=synthetic-old-access", "S3_SECRET_KEY=synthetic-old-secret"], "StopTimeout": 10},
            "HostConfig": {"Binds": ["synthetic-private-bind"]},
            "Mounts": [{"Destination": "/loki", "RW": True, "Mode": ""}],
            "State": {"Running": True, "Health": {"Status": "healthy", "Log": [{
                "End": self.now.isoformat(), "ExitCode": 0, "Output": "synthetic-private-health-output"}]}}}
        self.compose = {"services": {"loki": {"volumes": [{"target": "/loki", "read_only": False}]}}}
        self.calls = []
        self.helper = types.SimpleNamespace(_deadline=None, APP="synthetic-app", require=require,
            payload_input=Mock(return_value=self.data), command=Mock(side_effect=self.command), envmap=envmap,
            discover=Mock(return_value=self.loki), sources=Mock(return_value=("/safe/compose.yaml", "/safe")),
            document=Mock())

    def command(self, args):
        self.calls.append(args)
        if args[:2] == ["docker", "inspect"]:
            return json.dumps([self.own])
        if args[:3] == ["docker", "image", "inspect"]:
            return json.dumps([{"Id": self.loki["Image"], "Config": {}}])
        self.assertEqual(args[:2], ["docker", "compose"])
        self.assertEqual(args[-3:], ["config", "--format", "json"])
        return json.dumps(self.compose)

    def invoke(self):
        with patch.object(m.os, "geteuid", return_value=0), patch.object(m.socket, "gethostname", return_value="a" * 12):
            result = m.observe(self.helper, self.values)
        self.assertIsNotNone(m.closed_report(result))
        rendered = json.dumps(result)
        for value in [*self.data.values(), "synthetic-private-health-output", "synthetic-private-bind", "synthetic-private-error"]:
            self.assertNotIn(value, rendered)
        self.assertIsNone(self.helper._deadline)
        return result

    def test_operational_old_pair_and_fresh_health_pass_without_mutation(self):
        result = self.invoke()
        self.assertEqual(result["status"], "PASS")
        self.assertTrue(result["old_pair_matches"])
        self.assertFalse(result["new_pair_matches"])
        self.assertTrue(result["ready"])
        self.assertEqual(result["canonical_source_attestation"], "PASS")
        self.assertFalse(result["original_projection_available"])
        self.assertTrue(result["mount_mode_default_equivalence"])
        self.assertEqual(result["host_mount_representation"], "binds_only")
        self.assertTrue(all("up" not in args and "start" not in args and "network" not in args for args in self.calls))

    def test_healthcheck_stale_future_or_nonzero_is_not_ready(self):
        for seconds, code in ((31, 0), (-5, 0), (0, 1), (0, False)):
            changed = copy.deepcopy(self.loki)
            changed["State"]["Health"]["Log"][-1].update(End=(self.now - datetime.timedelta(seconds=seconds)).isoformat(), ExitCode=code)
            self.assertFalse(m.fresh_health(changed, self.now))

    def test_new_pair_never_passes_old_rollback_observation(self):
        self.loki["Config"]["Env"] = ["S3_ACCESS_KEY=synthetic-new-access", "S3_SECRET_KEY=synthetic-new-secret"]
        result = self.invoke()
        self.assertEqual(result["status"], "FAIL")
        self.assertFalse(result["old_pair_matches"])
        self.assertTrue(result["new_pair_matches"])

    def test_canonical_drift_and_private_backend_errors_remain_fixed_failures(self):
        self.helper.document.side_effect = ValueError("synthetic-private-error")
        result = self.invoke()
        self.assertEqual(result["status"], "FAIL")
        self.assertEqual(result["canonical_source_attestation"], "FAIL")

    def test_wrong_worker_identity_blocks_all_loki_reads(self):
        self.own["Config"]["Labels"]["com.docker.compose.project"] = "another"
        result = self.invoke()
        self.assertEqual(result["status"], "FAIL")
        self.assertFalse(result["job_binding"])
        self.helper.discover.assert_not_called()

    def test_closed_report_rejects_secret_fields_and_fabricated_projection_claim(self):
        result = self.invoke()
        self.assertIsNone(m.closed_report({**result, "raw_env": "synthetic-private"}))
        self.assertIsNone(m.closed_report({**result, "original_projection_available": True}))
        self.assertIsNone(m.closed_report({**result, "old_pair_matches": False}))


if __name__ == "__main__":
    unittest.main()
