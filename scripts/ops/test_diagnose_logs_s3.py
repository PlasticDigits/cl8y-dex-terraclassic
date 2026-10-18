#!/usr/bin/env python3
"""Synthetic scheduler bounds; no live API or credentials."""
import importlib.util
from pathlib import Path
import types
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("diagnose", Path(__file__).with_name("diagnose-logs-s3.py"))
d = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(d)


class GuardError(ValueError):
    pass


def require(condition, label):
    if not condition:
        raise GuardError(label)


class Tests(unittest.TestCase):
    def fixture(self):
        m = d.load("adopt_fixture", Path(__file__).with_name("adopt-logs-s3.py"))
        state = {"job_uuid": "9tksfavan8o2tcdjovzunumt", "revision": "a" * 40,
                 "recipe_sha256": "b" * 64}
        m.validate = Mock()
        m.bind_job = Mock()
        m.status = Mock(return_value={"receipts": [{"mode": "adopt", "status": "FAIL"}]})
        m.save = Mock(return_value={"data": state})
        return m, state

    def test_diagnostic_task_is_disabled_finite_exact_container_and_no_secret_command(self):
        m, state = self.fixture()
        api = Mock()
        command = "python3 -c 'synthetic source only'"
        api.call.side_effect = [{}, {"uuid": "c" * 24, "enabled": False,
                                    "container": "maintenance", "command": command}, {}]
        with patch.object(d, "source_command", return_value=command):
            result = d.begin(m, api, {"data": state}, "a" * 40)
        task = api.call.call_args_list[1].args[2]
        self.assertIs(task["enabled"], False)
        self.assertEqual(task["timeout"], 55)
        self.assertEqual(task["container"], "maintenance")
        self.assertEqual(api.call.call_args_list[-1].args[0].split("/")[-1], "execute")
        self.assertFalse(result["runtime_changed"])

    def test_refuse_other_job_before_any_task_creation(self):
        m, state = self.fixture()
        state["job_uuid"] = "c" * 24
        api = Mock()
        with self.assertRaises(m.GuardError):
            d.bind(m, api, state)
        api.call.assert_not_called()

    def test_ambiguous_task_cannot_be_blindly_rescheduled(self):
        m, state = self.fixture()
        state["diagnostic"] = {"stage": "prepared"}
        api = Mock()
        with self.assertRaisesRegex(m.GuardError, "diagnostic_already_prepared"):
            d.begin(m, api, {"data": state}, "a" * 40)
        self.assertEqual(len(api.call.call_args_list), 1)

    def test_task_with_automatic_schedule_enabled_is_rejected(self):
        m, state = self.fixture()
        api = Mock()
        command = "synthetic reviewed command"
        api.call.side_effect = [{}, {"uuid": "c" * 24, "enabled": True,
                                    "container": "maintenance", "command": command}]
        with patch.object(d, "source_command", return_value=command):
            with self.assertRaisesRegex(m.GuardError, "diagnostic_task_binding"):
                d.begin(m, api, {"data": state}, "a" * 40)
        self.assertNotIn("execute", api.call.call_args_list[-1].args[0])


if __name__ == "__main__":
    unittest.main()
