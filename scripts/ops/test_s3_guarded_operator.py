#!/usr/bin/env python3
"""Synthetic terminal/guard contract tests; never read Vault or live services.

The fake fetch supplies only invented Coolify values. The installed OpenBao
workstation implementation still launches and guards the child normally.
"""
import errno
import importlib.util
import json
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import termios
import textwrap
import time
import unittest
from unittest.mock import patch


OPERATOR = Path(__file__).with_name("stage-logs-s3-operator.py")
INSTALLED = Path.home() / "openbao-ops"
ACCESS = b"synthetic-access-key-1544"
SECRET = b"synthetic-secret-key-1544"
DOWNSTREAM = b"synthetic-downstream-private-value-1544"

FIXTURE = r'''
import json, os, pathlib, sys
assert not any(s.isatty() for s in (sys.stdin, sys.stdout, sys.stderr))
mode = os.environ.get("FIXTURE_MODE", "new")
preflight = "--preflight" in sys.argv
if preflight:
    assert sys.stdin.read() == ""
    result = {"status": "PASS", "scope": "configuration and OpenBao authority only",
              "pending_candidate_present": mode == "resume", "runtime_changed": False}
else:
    if mode == "resume":
        assert "--resume" in sys.argv
        assert sys.stdin.read() == ""
    else:
        assert "--input-json" in sys.argv
        payload = json.load(sys.stdin)
        expected = {"access_key": "synthetic-" + "access-key-1544",
                    "secret_key": "synthetic-" + "secret-key-1544"}
        assert payload == expected
        for secret in expected.values():
            assert all(secret not in value for value in sys.argv)
            assert all(secret not in value for value in os.environ.values())
            assert all(secret.encode() not in path.read_bytes()
                       for path in pathlib.Path(__file__).parent.rglob("*") if path.is_file())
    result = {"status": "PASS", "candidate_state": "staged_verified_read",
              "candidate_retention": "confirmed", "runtime_changed": False,
              "current_key_revoked": False,
              "next_action": "agent adoption verification before old-key revocation"}
if mode == "unknown":
    result["unapproved"] = "synthetic-" + "downstream-private-value-1544"
elif mode == "stderr":
    print("synthetic-" + "downstream-private-value-1544", file=sys.stderr)
elif mode == "invalid-enum":
    result["scope"] = "synthetic-" + "downstream-private-value-1544"
elif mode == "failure":
    result = {"status": "FAIL", "phase": "bucket_verification", "reason": "http_error",
              "candidate_retention": "confirmed", "runtime_changed": False,
              "prior_candidate_present": False, "current_key_revoked": False,
              "http_status": 403}
elif mode == "failure-secret":
    result = {"status": "FAIL", "phase": "bucket_verification",
              "reason": "synthetic-" + "downstream-private-value-1544",
              "candidate_retention": "confirmed", "runtime_changed": False,
              "prior_candidate_present": False, "current_key_revoked": False}
print(json.dumps(result))
raise SystemExit(1 if result["status"] == "FAIL" else 0)
'''


class OperatorBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("s3_operator_contract", OPERATOR)
        cls.operator = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.operator)

    def test_report_rejects_extra_fields_and_open_text_values(self):
        valid = {"status": "PASS", "scope": "configuration and OpenBao authority only",
                 "pending_candidate_present": False, "runtime_changed": False}
        self.assertEqual(self.operator.report(valid), valid)
        for changed in (dict(valid, secret=DOWNSTREAM.decode()),
                        dict(valid, scope=DOWNSTREAM.decode()),
                        dict(valid, pending_candidate_present=0),
                        dict(valid, runtime_changed=0)):
            with self.subTest(changed=list(changed)):
                with self.assertRaises(self.operator.OperatorError) as raised:
                    self.operator.report(changed)
                self.assertNotIn(DOWNSTREAM.decode(), str(raised.exception))

    def test_credential_transfer_uses_only_stdin_and_captures_both_streams(self):
        report = {"status": "PASS", "candidate_state": "staged_verified_read",
                  "candidate_retention": "confirmed", "runtime_changed": False,
                  "current_key_revoked": False,
                  "next_action": "agent adoption verification before old-key revocation"}
        completed = subprocess.CompletedProcess([], 0, json.dumps(report).encode(), b"")
        payload = {"access_key": ACCESS.decode(), "secret_key": SECRET.decode()}
        with patch.object(self.operator.subprocess, "run", return_value=completed) as child:
            self.assertEqual(self.operator.guarded_call("--input-json", payload), report)
        args, kwargs = child.call_args
        self.assertIn("--output=guarded", args[0])
        self.assertNotIn("--output=raw", args[0])
        self.assertEqual(kwargs["stdout"], subprocess.PIPE)
        self.assertEqual(kwargs["stderr"], subprocess.PIPE)
        self.assertEqual(json.loads(kwargs["input"]), payload)
        self.assertGreater(kwargs["timeout"], 0)
        for private in (ACCESS.decode(), SECRET.decode()):
            self.assertNotIn(private, str(args))
            self.assertNotIn(private, str(kwargs.get("env", {})))
        with patch.object(self.operator.subprocess, "run", return_value=completed) as child:
            self.operator.guarded_call("--resume")
        self.assertEqual(child.call_args.kwargs["stdin"], subprocess.DEVNULL)
        self.assertNotIn("input", child.call_args.kwargs)

    def test_unexpected_stderr_is_not_replayed_or_treated_as_success(self):
        completed = subprocess.CompletedProcess([], 0, b"{}", DOWNSTREAM)
        with patch.object(self.operator.subprocess, "run", return_value=completed):
            with self.assertRaises(self.operator.OperatorError) as raised:
                self.operator.guarded_call("--preflight")
        self.assertNotIn(DOWNSTREAM.decode(), str(raised.exception))


class GuardedOperatorTests(unittest.TestCase):
    def setUp(self):
        if not (INSTALLED / "output_guard.py").is_file():
            self.skipTest("installed OpenBao output guard unavailable")
        self.temp = tempfile.TemporaryDirectory(prefix="s3-guard-contract-")
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        self.operator = self.directory / OPERATOR.name
        shutil.copyfile(OPERATOR, self.operator)
        (self.directory / "stage-logs-s3-credentials.py").write_text(
            'SAFE_REASONS = frozenset({"openbao_write_denied", "pending_missing"})\n'
            'def fixture():\n' + textwrap.indent(FIXTURE, "    ")
            + '\nif __name__ == "__main__":\n    fixture()\n')
        executable = self.directory / "bao-exec"
        executable.write_text(f'''#!{sys.executable}
import sys
sys.path.insert(0, {str(INSTALLED)!r})
import workstation
workstation.fetch = lambda names: {{"COOLIFY_URL": "https://synthetic-coolify.invalid",
                                    "COOLIFY_WIRE_TOKEN": "synthetic-wire-token-1544"}}
workstation.main()
''')
        executable.chmod(0o700)
        self.env = {"PATH": str(self.directory) + os.pathsep + os.defpath,
                    "HOME": self.temp.name, "PYTHONDONTWRITEBYTECODE": "1",
                    "PYTHONUNBUFFERED": "1", "LC_ALL": "C.UTF-8"}

    def terminal(self, command, mode="new", inputs=()):
        """Use a real controlling terminal; abort every process within 8 sec."""
        pid, master = pty.fork()
        if pid == 0:
            try:
                os.execve(command[0], command, dict(self.env, FIXTURE_MODE=mode))
            except BaseException:
                os._exit(126)
        chunks = bytearray()
        pending = list(inputs)
        sent = 0
        status = None
        deadline = time.monotonic() + 8
        try:
            while time.monotonic() < deadline:
                ready, _, _ = select.select([master], [], [], 0.05)
                if ready:
                    try:
                        chunk = os.read(master, 65536)
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        break
                    if not chunk:
                        break
                    chunks.extend(chunk)
                    # Both getpass prompts end with this suffix. Checking ECHO
                    # before writing catches accidental plain input() fallback.
                    prompts = bytes(chunks).count(b"(hidden): ")
                    if prompts > sent and pending:
                        self.assertFalse(termios.tcgetattr(master)[3] & termios.ECHO)
                        os.write(master, pending.pop(0) + b"\n")
                        sent += 1
                observed, child_status = os.waitpid(pid, os.WNOHANG)
                if observed:
                    status = child_status
                    # Drain until terminal EOF even when process already exited.
                    while select.select([master], [], [], 0)[0]:
                        try:
                            chunk = os.read(master, 65536)
                        except OSError as error:
                            if error.errno != errno.EIO:
                                raise
                            break
                        if not chunk:
                            break
                        chunks.extend(chunk)
                    break
            else:
                self.fail("synthetic operator exceeded 8-second terminal deadline")
        finally:
            if status is None:
                observed, child_status = os.waitpid(pid, os.WNOHANG)
                if observed:
                    status = child_status
                else:
                    os.kill(pid, signal.SIGKILL)
                    _, status = os.waitpid(pid, 0)
            os.close(master)
        rendered = bytes(chunks)
        for private in (ACCESS, SECRET, DOWNSTREAM):
            self.assertNotIn(private, rendered)
        self.assertNotIn(b"Traceback", rendered)
        return os.waitstatus_to_exitcode(status), rendered, sent

    def test_legacy_guarded_terminal_is_rejected_before_child_launch(self):
        marker = self.directory / "child-launched"
        command = [str(self.directory / "bao-exec"), "--output=guarded", "--only",
                   "COOLIFY_URL,COOLIFY_WIRE_TOKEN", "--", sys.executable, "-c",
                   f"from pathlib import Path; Path({str(marker)!r}).touch()"]
        code, output, _ = self.terminal(command)
        self.assertEqual(code, 125)
        self.assertEqual(output.strip(), b"bao-exec: output withheld by guard.")
        self.assertFalse(marker.exists())

    def test_hidden_terminal_prompts_cross_real_guard_over_pipes(self):
        code, output, sent = self.terminal([sys.executable, str(self.operator)],
                                           inputs=(ACCESS, SECRET))
        self.assertEqual(code, 0, output)
        self.assertEqual(sent, 2)
        self.assertIn(b"staged_verified_read", output)

    def test_retained_candidate_resumes_without_prompting(self):
        code, output, sent = self.terminal([sys.executable, str(self.operator)], mode="resume")
        self.assertEqual(code, 0, output)
        self.assertEqual(sent, 0)
        self.assertNotIn(b"(hidden):", output)
        self.assertIn(b"staged_verified_read", output)

    def test_unknown_downstream_output_never_reaches_terminal(self):
        for mode in ("unknown", "stderr", "invalid-enum", "failure-secret"):
            with self.subTest(mode=mode):
                code, output, sent = self.terminal([sys.executable, str(self.operator)], mode=mode)
                self.assertNotEqual(code, 0)
                self.assertEqual(sent, 0)
                self.assertLess(len(output), 1024)

    def test_safe_failure_report_preserves_status_without_echoing_body(self):
        code, output, sent = self.terminal([sys.executable, str(self.operator)], mode="failure")
        self.assertNotEqual(code, 0)
        self.assertEqual(sent, 0)
        self.assertIn(b"403", output)
        self.assertIn(b"bucket_verification", output)

    def test_operator_rejects_nonterminal_without_launching_secret_fetch(self):
        executable = self.directory / "bao-exec"
        marker = self.directory / "unexpected-fetch"
        executable.write_text(f"#!{sys.executable}\nfrom pathlib import Path\nPath({str(marker)!r}).touch()\n")
        result = subprocess.run([sys.executable, str(self.operator)], stdin=subprocess.DEVNULL,
                                capture_output=True, env=self.env, timeout=8)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(marker.exists())
        self.assertNotIn(b"Traceback", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
