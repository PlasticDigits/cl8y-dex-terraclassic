#!/usr/bin/env python3
"""Synthetic Console-target custody checks; no real Vault, browser or credentials."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("revocation", Path(__file__).with_name("check-logs-s3-revocation-target.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)
worker = m.load_worker()


def state():
    receipt = worker.initial_report("adopt")
    receipt.update(status="PASS", phase="adopted", runtime_changed=True, network_checked=True,
        adoption_latched=True, ready=True, backing_storage_write_verified=True,
        marker_readback_verified=True, historical_readback_verified=True, flush_success_delta=1,
        flush_failure_delta=0, attempt_nonce="0123456789abcdef")
    return {"app_uuid": m.APP, "phase": "config_persisted", "transaction": "fedcba9876543210",
            "attempt_nonce": "0123456789abcdef", "adoption_receipt": receipt,
            "payload": {"expected_old_access": "synthetic-old-access", "new_access": "synthetic-new-access",
                        "expected_old_secret": "synthetic-private-secret"}}


class CustodyTests(unittest.TestCase):
    def invoke(self, current=None, selected="synthetic-old-access"):
        output = io.StringIO()
        with patch.object(m.sys.stdin, "isatty", return_value=True), \
                patch.object(m, "read_state", return_value=current if current is not None else state()), \
                patch.object(m, "load_worker", return_value=worker), \
                patch.object(m.getpass, "getpass", return_value=selected) as prompt, \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            code = m.main()
        rendered = output.getvalue()
        for private in ("synthetic-old-access", "synthetic-new-access", "synthetic-private-secret", "synthetic-private-error"):
            self.assertNotIn(private, rendered)
        return code, json.loads(rendered), prompt

    def test_exact_retired_entry_is_the_only_authorized_selection_after_full_adoption(self):
        code, result, prompt = self.invoke()
        self.assertEqual(code, 0)
        self.assertEqual(result, {"status": "PASS", "selected_key_is_retired_loki_key": True,
                                  "replacement_selected": False, "may_revoke_selected_console_entry": True})
        prompt.assert_called_once()
        current = state()
        current["phase"] = "cleaned"
        self.assertEqual(self.invoke(current)[0], 0)

    def test_replacement_unknown_and_malformed_ids_are_denied_without_reflection(self):
        for selected, reason in (("synthetic-new-access", "replacement_selected"),
                                 ("synthetic-unknown-access", "selected_key_mismatch"),
                                 ("x" * 257, "input_shape"), (" space synthetic-access", "input_shape")):
            with self.subTest(reason=reason):
                code, result, _ = self.invoke(selected=selected)
                self.assertEqual(code, 1)
                self.assertEqual(result, {"status": "FAIL", "reason": reason})

    def test_incomplete_unverified_wrong_schema_and_nonce_deny_before_prompt(self):
        changes = [lambda s: s.update(phase="adoption_failed"),
                   lambda s: s.update(app_uuid="another"),
                   lambda s: s["adoption_receipt"].update(backing_storage_write_verified=False),
                   lambda s: s["adoption_receipt"].update(schema="another"),
                   lambda s: s["adoption_receipt"].update(attempt_nonce="aaaaaaaaaaaaaaaa"),
                   lambda s: s["adoption_receipt"].pop("attempt_nonce"),
                   lambda s: s["adoption_receipt"].update(flush_failure_delta=1)]
        for change in changes:
            current = state()
            change(current)
            code, result, prompt = self.invoke(current)
            self.assertEqual(code, 1)
            self.assertEqual(result["status"], "FAIL")
            prompt.assert_not_called()

    def test_non_tty_is_rejected_before_vault_or_prompt(self):
        output = io.StringIO()
        with patch.object(m.sys.stdin, "isatty", return_value=False), patch.object(m, "read_state") as read, \
                patch.object(m.getpass, "getpass") as prompt, contextlib.redirect_stdout(output):
            self.assertEqual(m.main(), 1)
        read.assert_not_called()
        prompt.assert_not_called()
        self.assertEqual(json.loads(output.getvalue())["reason"], "interactive_hidden_prompt_required")

    def test_malformed_vault_response_and_timeout_details_never_escape(self):
        cases = [subprocess.CompletedProcess([], 0, "synthetic-private-secret", ""),
                 subprocess.TimeoutExpired(["bao-admin"], 15, output="synthetic-private-secret",
                                           stderr="synthetic-private-error")]
        for case in cases:
            output = io.StringIO()
            response = {"side_effect": case} if isinstance(case, Exception) else {"return_value": case}
            with patch.object(m.sys.stdin, "isatty", return_value=True), patch.object(m.subprocess, "run", **response), \
                    patch.object(m.getpass, "getpass") as prompt, \
                    contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                self.assertEqual(m.main(), 1)
            self.assertEqual(json.loads(output.getvalue())["status"], "FAIL")
            self.assertNotIn("synthetic-private", output.getvalue())
            prompt.assert_not_called()

    def test_getpass_warning_fails_before_any_echo_fallback(self):
        output = io.StringIO()
        with patch.object(m.sys.stdin, "isatty", return_value=True), patch.object(m, "read_state", return_value=state()), \
                patch.object(m.getpass, "getpass", side_effect=m.getpass.GetPassWarning("synthetic-private-error")), \
                contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            self.assertEqual(m.main(), 1)
        self.assertEqual(json.loads(output.getvalue()), {"status": "FAIL", "reason": "hidden_input_unavailable"})

    def test_child_errors_and_environment_stay_private(self):
        failure = subprocess.CompletedProcess([], 1, "synthetic-private-secret", "synthetic-private-error")
        output = io.StringIO()
        with patch.object(m.sys.stdin, "isatty", return_value=True), \
                patch.dict(m.os.environ, {"COOLIFY_WIRE_TOKEN": "synthetic-private-secret", "VAULT_TOKEN": "synthetic-private-secret"}), \
                patch.object(m.subprocess, "run", return_value=failure) as run, \
                patch.object(m.getpass, "getpass") as prompt, contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            self.assertEqual(m.main(), 1)
        self.assertEqual(json.loads(output.getvalue()), {"status": "FAIL", "reason": "journal_unavailable"})
        self.assertTrue(run.call_args.kwargs["capture_output"])
        self.assertEqual(run.call_args.kwargs["timeout"], 15)
        self.assertNotIn("VAULT_TOKEN", run.call_args.kwargs["env"])
        self.assertNotIn("COOLIFY_WIRE_TOKEN", run.call_args.kwargs["env"])
        prompt.assert_not_called()


if __name__ == "__main__":
    unittest.main()
