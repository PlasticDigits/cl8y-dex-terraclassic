import datetime
import contextlib
import copy
import importlib.util
import io
import json
import subprocess
from pathlib import Path
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

SPEC = importlib.util.spec_from_file_location("stage_s3", Path(__file__).with_name("stage-logs-s3-credentials.py"))
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)


class StagingTests(unittest.TestCase):
    def test_signed_read_is_deterministic_bounded_and_does_not_expose_secret(self):
        args = ("nbg1.your-objectstorage.com", "synthetic-logs", "nbg1", "synthetic-access-key", "synthetic-secret-key")
        now = datetime.datetime(2026, 10, 18, tzinfo=datetime.timezone.utc)
        request = STAGE.signing_request(*args, now=now)
        self.assertEqual(request.method, "HEAD")
        self.assertEqual(request.full_url, "https://nbg1.your-objectstorage.com/synthetic-logs")
        self.assertEqual(request.headers, STAGE.signing_request(*args, now=now).headers)
        self.assertNotIn("synthetic-secret-key", str(request.headers))
        self.assertIn("20261018/nbg1/s3/aws4_request", request.headers["Authorization"])
        self.assertIsNone(STAGE.NoRedirect().redirect_request(None, None, 302, "", {}, "https://private/"))

    def test_rejects_endpoint_credential_and_bucket_injection(self):
        args = ["https://nbg1.your-objectstorage.com", "synthetic-logs", "nbg1", "synthetic-access-key", "synthetic-secret-key"]
        for index, value in ((0, "http://private/"), (0, "https://user:pass@nbg1.your-objectstorage.com"),
                             (0, "https://nbg1.your-objectstorage.com/path"), (1, "private?secret"),
                             (2, "unbounded"), (3, "space secret key")):
            modified = list(args)
            modified[index] = value
            with self.assertRaises(ValueError):
                STAGE.signing_request(*modified)


class WorkflowTests(unittest.TestCase):
    def setUp(self):
        self.env = {"S3_ENDPOINT": "nbg1.your-objectstorage.com", "S3_BUCKET": "synthetic-logs",
                    "S3_REGION": "nbg1", "S3_ACCESS_KEY": "existing-access-key", "S3_SECRET_KEY": "existing-secret-key"}
        self.record = None
        self.writes = []
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(patch.object(STAGE, "environment", return_value=self.env))
        self.stack.enter_context(patch.object(STAGE, "bao_read", side_effect=lambda: copy.deepcopy(self.record)))
        self.stack.enter_context(patch.object(STAGE, "writable"))
        self.stack.enter_context(patch.object(STAGE.sys.stdin, "isatty", return_value=True))
        self.prompts = self.stack.enter_context(patch.object(STAGE.getpass, "getpass", side_effect=["candidate-access-key", "candidate-secret-key"]))
        self.stack.enter_context(patch.object(STAGE.subprocess, "run", side_effect=self.write))
        self.http = self.stack.enter_context(patch.object(STAGE.OPENER, "open", side_effect=self.head))

    def write(self, args, **kwargs):
        self.assertEqual(args, ["bao-admin", "write", "-format=json", STAGE.PENDING_PATH, "-"])
        self.assertTrue(kwargs["capture_output"])
        payload = json.loads(kwargs["input"])
        version = self.record["metadata"]["version"] if self.record else 0
        self.assertEqual(payload["options"], {"cas": version})
        self.record = {"data": payload["data"], "metadata": {"version": version + 1}}
        self.writes.append(copy.deepcopy(payload))
        return subprocess.CompletedProcess(args, 0, "{}", "")

    def head(self, request, **kwargs):
        self.assertEqual(request.method, "HEAD")
        self.assertEqual(self.record["data"]["state"], "staged_unverified")
        return type("Response", (), {"status": 200, "__enter__": lambda obj: obj,
                                     "__exit__": lambda *args: None})()

    def invoke(self, args):
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            code = STAGE.main(args)
        rendered = output.getvalue()
        for value in ("existing-access-key", "existing-secret-key", "candidate-access-key", "candidate-secret-key", "synthetic-private-error"):
            self.assertNotIn(value, rendered)
        return code, json.loads(rendered)

    def test_candidate_is_retained_before_head_and_only_then_promoted_with_cas(self):
        code, result = self.invoke([])
        self.assertEqual(code, 0)
        self.assertEqual([w["data"]["state"] for w in self.writes], ["staged_unverified", "staged_verified_read"])
        self.assertEqual(result["candidate_state"], "staged_verified_read")
        self.assertFalse(result["runtime_changed"])
        self.assertFalse(result["current_key_revoked"])

    def test_http_failure_retains_unverified_candidate_and_emits_only_phase_and_status(self):
        self.http.side_effect = HTTPError("https://private.invalid", 403, "synthetic-private-error", {}, io.BytesIO(b"synthetic-private-error"))
        code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["phase"], "bucket_verification")
        self.assertEqual(result["http_status"], 403)
        self.assertEqual(result["candidate_retention"], "confirmed")
        self.assertEqual(self.record["data"]["state"], "staged_unverified")
        self.assertEqual(len(self.writes), 1)

    def test_resume_after_failed_verification_does_not_prompt_or_replace_candidate(self):
        self.http.side_effect = TimeoutError("synthetic-private-error")
        self.invoke([])
        saved = copy.deepcopy(self.record["data"])
        self.prompts.reset_mock()
        self.http.side_effect = self.head
        code, result = self.invoke(["--resume"])
        self.assertEqual(code, 0)
        self.prompts.assert_not_called()
        self.assertEqual(self.record["data"]["access_key"], saved["access_key"])
        self.assertEqual(result["candidate_state"], "staged_verified_read")

    def test_new_run_cannot_silently_overwrite_an_existing_pending_candidate(self):
        self.http.side_effect = TimeoutError()
        self.invoke([])
        self.prompts.reset_mock()
        count = len(self.writes)
        code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["reason"], "pending_exists_use_resume")
        self.prompts.assert_not_called()
        self.assertEqual(len(self.writes), count)

    def test_resume_rejects_changed_storage_identity_before_any_probe_or_write(self):
        self.http.side_effect = TimeoutError()
        self.invoke([])
        self.env["S3_BUCKET"] = "different-bucket"
        self.http.reset_mock()
        count = len(self.writes)
        code, result = self.invoke(["--resume"])
        self.assertEqual(code, 1)
        self.assertEqual(result["reason"], "pending_identity_changed")
        self.http.assert_not_called()
        self.assertEqual(len(self.writes), count)

    def test_preflight_has_no_prompts_writes_or_s3_requests(self):
        code, result = self.invoke(["--preflight"])
        self.assertEqual(code, 0)
        self.assertFalse(result["pending_candidate_present"])
        self.prompts.assert_not_called()
        self.http.assert_not_called()
        self.assertEqual(self.writes, [])

    def test_failed_reverification_revokes_the_previously_verified_pending_state(self):
        code, _ = self.invoke([])
        self.assertEqual(code, 0)
        self.http.side_effect = HTTPError("https://private.invalid", 403, "synthetic-private-error", {}, None)
        self.prompts.reset_mock()
        code, result = self.invoke(["--resume"])
        self.assertEqual(code, 1)
        self.assertEqual(result["phase"], "bucket_verification")
        self.assertEqual(self.record["data"]["state"], "staged_unverified")
        self.assertNotIn("verified_at_utc", self.record["data"])
        self.prompts.assert_not_called()

    def test_terminal_failure_cannot_fall_back_to_echoed_input(self):
        self.prompts.side_effect = STAGE.getpass.GetPassWarning("synthetic-private-error")
        code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["phase"], "hidden_input")
        self.assertEqual(result["reason"], "hidden_input_unavailable")
        self.assertEqual(self.writes, [])
        self.http.assert_not_called()

    def test_ambiguous_first_write_reports_retention_unknown_and_never_probes(self):
        with patch.object(STAGE.subprocess, "run", side_effect=TimeoutError("synthetic-private-error")):
            code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["phase"], "candidate_storage")
        self.assertEqual(result["candidate_retention"], "unknown")
        self.http.assert_not_called()

    def test_explicit_replacement_can_recover_a_stale_pending_identity_with_cas(self):
        self.record = {"data": {"bucket": "old-bucket", "state": "old-format"}, "metadata": {"version": 7}}
        code, result = self.invoke(["--replace-pending"])
        self.assertEqual(code, 0)
        self.assertEqual(self.writes[0]["options"]["cas"], 7)
        self.assertEqual(self.record["data"]["bucket"], "synthetic-logs")
        self.assertEqual(result["candidate_state"], "staged_verified_read")

    def test_invalid_replacement_input_does_not_claim_the_new_value_was_saved(self):
        self.record = {"data": {"bucket": "old-bucket"}, "metadata": {"version": 7}}
        before = copy.deepcopy(self.record)
        self.prompts.side_effect = ["too-short", "candidate-secret-key"]
        code, result = self.invoke(["--replace-pending"])
        self.assertEqual(code, 1)
        self.assertEqual(result["candidate_retention"], "none")
        self.assertTrue(result["prior_candidate_present"])
        self.assertEqual(self.record, before)
        self.assertEqual(self.writes, [])

    def test_cas_conflict_never_overwrites_prior_candidate_or_prints_backend_body(self):
        self.record = {"data": {"bucket": "old-bucket"}, "metadata": {"version": 7}}
        before = copy.deepcopy(self.record)
        failure = subprocess.CompletedProcess([], 1, "synthetic-private-error", "synthetic-private-error")
        with patch.object(STAGE.subprocess, "run", return_value=failure):
            code, result = self.invoke(["--replace-pending"])
        self.assertEqual(code, 1)
        self.assertEqual(result["reason"], "openbao_stage_failed")
        self.assertEqual(result["candidate_retention"], "unknown")
        self.assertEqual(self.record, before)
        self.http.assert_not_called()

    def test_unconfirmed_readback_does_not_probe_or_claim_saved_input(self):
        wrong = {"data": {"wrong": "synthetic-private-error"}, "metadata": {"version": 1}}
        with patch.object(STAGE, "bao_read", side_effect=[None, wrong]):
            code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["reason"], "openbao_readback_failed")
        self.assertEqual(result["candidate_retention"], "unknown")
        self.assertEqual(self.record["data"]["state"], "staged_unverified")
        self.http.assert_not_called()

    def test_authority_failure_occurs_before_hidden_input_or_storage(self):
        with patch.object(STAGE, "writable", side_effect=ValueError("openbao_write_denied")):
            code, result = self.invoke([])
        self.assertEqual(code, 1)
        self.assertEqual(result["phase"], "openbao_preflight")
        self.prompts.assert_not_called()
        self.http.assert_not_called()
        self.assertEqual(self.writes, [])


if __name__ == "__main__":
    unittest.main()
