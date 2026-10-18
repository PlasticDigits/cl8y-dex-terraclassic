#!/usr/bin/env python3
"""Synthetic worker-boundary tests; never opens Docker or live credentials."""
import contextlib
import copy
import importlib.util
import json
import io
import os
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("worker", Path(__file__).with_name("loki-s3-adoption-worker.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)
ORIGINAL_COMMAND = m.command


def values(mode="adopt"):
    return {"LOKI_S3_MODE": mode, "LOKI_S3_JOB_UUID": "syntheticjob1234", "LOKI_S3_JOB_ID": "123",
            "LOKI_S3_NETWORK": m.APP, "LOKI_S3_PAYLOAD_JSON": '{"synthetic":"private-payload"}'}


def container():
    return {"Id": "a" * 64, "State": {"Running": True},
            "Config": {"Env": [k + "=" + v for k, v in values().items()],
                       "Labels": {"coolify.managed": "true", "coolify.serviceUuid": values()["LOKI_S3_JOB_UUID"],
                                  "coolify.type": "service", "coolify.service.subName": "maintenance",
                                  "com.docker.compose.project": values()["LOKI_S3_JOB_UUID"],
                                  "com.docker.compose.service": "maintenance"}},
            "HostConfig": {"RestartPolicy": {"Name": "no", "MaximumRetryCount": 0}, "CapDrop": ["ALL"]},
            "Mounts": [{"Type": "bind", "Source": "/var/run/docker.sock", "Destination": "/var/run/docker.sock", "RW": True},
                       {"Type": "bind", "Source": m.APPDIR, "Destination": m.APPDIR, "RW": False},
                       {"Type": "volume", "Name": "private-worker-state", "Destination": "/state", "RW": True}],
            "NetworkSettings": {"Networks": {"platform": {}}}}


def helper_report():
    return {"status": "PASS", "phase": "adopted", "runtime_changed": True,
            "rollback_attempted": False, "rollback_verified": False, "ready": True,
            "backing_storage_write_verified": True, "marker_readback_verified": True,
            "historical_readback_verified": True, "flush_success_delta": 1, "flush_failure_delta": 0}


def completed_report():
    return {**m.initial_report("adopt"), **helper_report(), "adoption_latched": True, "network_checked": True}


class BoundaryTests(unittest.TestCase):
    def test_input_clears_private_payload_and_all_inherited_environment_before_commands(self):
        with patch.dict(os.environ, {**values(), "EXTRANEOUS": "synthetic-secret"}, clear=True):
            self.assertEqual(m.inputs(), values())
            self.assertEqual(dict(os.environ), m.COMMAND_ENV)
        with patch.dict(os.environ, {**values(), "VAULT_TOKEN": "synthetic-secret"}, clear=True):
            with self.assertRaisesRegex(ValueError, "worker_input"):
                m.inputs()
            self.assertNotIn("VAULT_TOKEN", os.environ)

    def test_self_requires_exact_coolify_service_identity_hostname_and_payload_binding(self):
        own = container()
        self.assertEqual(m.self_identity(own, values(), "a" * 12), "a" * 64)
        for change in (lambda c: c["Config"]["Labels"].update({"coolify.serviceId": "124"}),
                       lambda c: c["Config"]["Labels"].update({"coolify.serviceUuid": "different"}),
                       lambda c: c["Config"]["Labels"].update({"com.docker.compose.project": "different"}),
                       lambda c: c["Config"]["Labels"].update({"com.docker.compose.service": "loki"}),
                       lambda c: c["Config"]["Labels"].update({"coolify.applicationId": "123"}),
                       lambda c: c["Config"].update(Env=[])):
            changed = copy.deepcopy(own)
            change(changed)
            with self.assertRaisesRegex(ValueError, "worker_identity"):
                m.self_identity(changed, values(), "a" * 12)
        with self.assertRaises(ValueError):
            m.self_identity(own, values(), "b" * 12)

    def test_legacy_service_label_requires_exact_numeric_id_and_uuid_project(self):
        own = container()
        own["Config"]["Labels"].pop("coolify.serviceUuid")
        own["Config"]["Labels"]["coolify.serviceId"] = "123"
        self.assertEqual(m.self_identity(own, values(), "a" * 12), "a" * 64)
        own["Config"]["Labels"]["com.docker.compose.project"] = "different"
        with self.assertRaises(ValueError):
            m.self_identity(own, values(), "a" * 12)

    def test_isolation_rejects_restart_privilege_ports_extra_mounts_and_writable_appdir(self):
        self.assertEqual(m.isolation(container()), "private-worker-state")
        for field, value in (("Privileged", True), ("CapAdd", ["SYS_ADMIN"]),
                             ("RestartPolicy", {"Name": "always"}), ("PortBindings", {"80/tcp": []}),
                             ("NetworkMode", "host"), ("PidMode", "host")):
            changed = container()
            changed["HostConfig"][field] = value
            with self.assertRaises(ValueError):
                m.isolation(changed)
        for change in (lambda c: c["Mounts"].append({"Destination": "/unreviewed"}),
                       lambda c: c["Mounts"][1].update(RW=True)):
            changed = container()
            change(changed)
            with self.assertRaises(ValueError):
                m.isolation(changed)

    def test_receipt_filters_unknown_fields_and_rejects_unbounded_reason(self):
        helper = types.SimpleNamespace(FAILURE_REASONS=frozenset({"readiness_failed"}))
        result = m.sanitized_helper({**helper_report(), "secret": "synthetic-private"}, helper)
        self.assertNotIn("secret", result)
        with self.assertRaises(ValueError):
            m.sanitized_helper({**helper_report(), "failure_reason": "synthetic-private"}, helper)

    def test_latch_is_exclusive_and_private(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            m.latch(path)
            self.assertEqual((path / "adoption-started").stat().st_mode & 0o777, 0o600)
            with self.assertRaisesRegex(ValueError, "worker_latch_exists"):
                m.latch(path)

    def test_closed_report_rejects_arbitrary_strings_unknowns_and_wrong_types(self):
        report = m.initial_report("adopt")
        self.assertEqual(m.closed_report(report), report)
        for key, value in (("secret", "synthetic-secret"), ("phase", "synthetic-secret"),
                           ("failure_reason", "synthetic-secret"), ("mode", "other"),
                           ("runtime_changed", 1), ("source_file", "/etc/shadow")):
            self.assertIsNone(m.closed_report({**report, key: value}))

    def test_private_terminal_receipt_survives_refused_replay(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory)
            m.latch(state)
            original = completed_report()
            m.persist(original, state)
            refused = m.initial_report("adopt")
            refused.update(failure_reason="worker_latch_exists")
            m.persist(refused, state)
            self.assertEqual(json.loads((state / "report.json").read_text()), original)
            self.assertEqual((state / "report.json").stat().st_mode & 0o777, 0o600)

    def test_adoption_terminal_receipt_replaces_earlier_preflight_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory)
            m.persist(m.initial_report("preflight"), state)
            m.latch(state)
            original = completed_report()
            m.persist(original, state)
            self.assertEqual(json.loads((state / "report.json").read_text()), original)

    def test_pass_receipts_require_all_actual_mode_specific_evidence(self):
        valid = completed_report()
        self.assertIsNotNone(m.closed_report(valid))
        for key, value in (("network_detached", False), ("adoption_latched", False), ("ready", False),
                           ("backing_storage_write_verified", False), ("historical_readback_verified", False),
                           ("marker_readback_verified", False), ("flush_success_delta", 0), ("flush_failure_delta", 1)):
            self.assertIsNone(m.closed_report({**valid, key: value}))
        preflight = {**m.initial_report("preflight"), "status": "PASS", "phase": "preflight",
                     "network_checked": True, "loki_count": 1}
        self.assertIsNotNone(m.closed_report(preflight))
        self.assertIsNone(m.closed_report({**preflight, "runtime_changed": True}))


class TransactionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)
        self.calls = []
        self.own = container()
        self.helper = types.SimpleNamespace(payload_input=Mock(return_value={"private": "synthetic-secret"}),
            discover=Mock(return_value={"NetworkSettings": {"Networks": {m.APP: {}}}}),
            run=Mock(return_value=helper_report()), FAILURE_REASONS=frozenset())
        stack = contextlib.ExitStack()
        self.addCleanup(stack.close)
        stack.enter_context(patch.object(m, "local_authority"))
        stack.enter_context(patch.object(m.socket, "gethostname", return_value="a" * 12))
        stack.enter_context(patch.object(m, "command", side_effect=self.command))

    def command(self, args, deadline):
        self.calls.append(args)
        if args[0] == "inspect":
            return json.dumps([self.own])
        if args[0] == "ps":
            return "a" * 64 + "\n"
        self.assertEqual(args[:2], ["network", args[1]])
        self.assertIn(args[1], ("connect", "disconnect"))
        self.assertEqual(args[2:], [m.APP, "a" * 64])
        return ""

    def test_adoption_calls_helper_once_detaches_self_and_refuses_replay(self):
        result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["status"], "PASS")
        self.assertTrue(result["adoption_latched"])
        self.assertTrue(result["network_detached"])
        self.helper.run.assert_called_once()
        result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["failure_reason"], "worker_latch_exists")
        self.helper.run.assert_called_once()
        self.assertNotIn("synthetic-secret", json.dumps(result))

    def test_preflight_does_not_create_latch_or_mutate_through_helper(self):
        current = values("preflight")
        self.own["Config"]["Env"] = [k + "=" + v for k, v in current.items()]
        self.helper.run.return_value = {**helper_report(), "phase": "preflight", "runtime_changed": False, "loki_count": 1}
        result = m.execute(self.helper, current, state_dir=self.path)
        self.assertEqual(result["status"], "PASS")
        self.helper.run.assert_called_once_with({"private": "synthetic-secret"}, preflight=True)
        self.assertFalse((self.path / "adoption-started").exists())

    def test_unexpected_helper_exception_reports_ambiguity_and_leaves_replay_latched(self):
        self.helper.run.side_effect = RuntimeError("synthetic-secret")
        result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["status"], "FAIL")
        self.assertTrue(result["runtime_changed"])
        self.assertTrue(result["network_detached"])
        self.assertTrue((self.path / "adoption-started").exists())
        self.assertNotIn("synthetic-secret", json.dumps(result))

    def test_wrong_network_or_shared_volume_blocks_helper_before_adoption(self):
        self.helper.discover.return_value = {"NetworkSettings": {"Networks": {"wrong": {}}}}
        result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["failure_reason"], "worker_network")
        self.helper.run.assert_not_called()
        self.assertFalse((self.path / "adoption-started").exists())
        self.helper.discover.return_value = {"NetworkSettings": {"Networks": {m.APP: {}}}}
        with patch.object(m, "command", side_effect=[json.dumps([self.own]), "a" * 64 + "\n" + "b" * 64]):
            result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["failure_reason"], "worker_state_shared")
        self.helper.run.assert_not_called()

    def test_preexisting_application_network_is_not_misreported_as_detached(self):
        self.own["NetworkSettings"]["Networks"][m.APP] = {}
        result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["status"], "FAIL")
        self.assertFalse(result["network_detached"])
        self.helper.run.assert_not_called()
        self.assertFalse((self.path / "adoption-started").exists())

    def test_unexpected_helper_output_cannot_escape_to_worker_streams(self):
        def noisy(*args, **kwargs):
            print("synthetic-secret")
            return helper_report()
        self.helper.run.side_effect = noisy
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            result = m.execute(self.helper, values(), state_dir=self.path)
        self.assertEqual(result["status"], "PASS")
        self.assertEqual(output.getvalue(), "")

    def test_subprocess_never_inherits_runtime_payload_or_daemon_override(self):
        result = m.subprocess.CompletedProcess([], 0, "{}", "")
        with patch.dict(os.environ, {"LOKI_S3_PAYLOAD_JSON": "synthetic-secret", "DOCKER_HOST": "tcp://untrusted"}), \
                patch.object(m.subprocess, "run", return_value=result) as run:
            self.assertEqual(ORIGINAL_COMMAND(["inspect", "a" * 12], m.time.monotonic() + 10), "{}")
        self.assertEqual(run.call_args.kwargs["env"], m.COMMAND_ENV)
        self.assertNotIn("synthetic-secret", str(run.call_args))


if __name__ == "__main__":
    unittest.main()
