#!/usr/bin/env python3
"""Local hidden prompts plus a noninteractive guarded staging subprocess.

Run directly in the operator's terminal, outside bao-exec. No raw-output mode,
credential arguments/environment/files, or unvalidated child-output replay.
"""
import getpass
import importlib.util
import json
from pathlib import Path
import resource
import subprocess
import sys
import warnings

HELPER = Path(__file__).with_name("stage-logs-s3-credentials.py")
SPEC = importlib.util.spec_from_file_location("s3_staging", HELPER)
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)
PHASES = frozenset(("dependency_check", "configuration", "openbao_preflight", "hidden_input",
                    "candidate_validation", "candidate_storage", "bucket_verification", "verification_storage"))
REASONS = STAGE.SAFE_REASONS | {"operation_failed", "http_error", "hidden_input_unavailable"}


class OperatorError(Exception):
    """Only fixed local codes, never subprocess/HTTP/credential contents."""


def report(payload):
    """Validate every field before any child bytes can reach the terminal."""
    if not isinstance(payload, dict) or payload.get("runtime_changed") is not False:
        raise OperatorError("report_withheld")
    if payload.get("status") == "PASS":
        preflight = {"status", "scope", "pending_candidate_present", "runtime_changed"}
        success = {"status", "candidate_state", "candidate_retention", "runtime_changed", "current_key_revoked", "next_action"}
        if set(payload) == preflight:
            if (payload["scope"] != "configuration and OpenBao authority only"
                    or type(payload["pending_candidate_present"]) is not bool):
                raise OperatorError("report_withheld")
        elif set(payload) == success:
            if (payload["candidate_state"] != "staged_verified_read" or payload["candidate_retention"] != "confirmed"
                    or payload["current_key_revoked"] is not False
                    or payload["next_action"] != "agent adoption verification before old-key revocation"):
                raise OperatorError("report_withheld")
        else:
            raise OperatorError("report_withheld")
    elif payload.get("status") == "FAIL":
        fields = {"status", "phase", "reason", "candidate_retention", "runtime_changed", "prior_candidate_present", "current_key_revoked"}
        if (set(payload) not in (fields, fields | {"http_status"}) or payload["phase"] not in PHASES
                or payload["reason"] not in REASONS or payload["candidate_retention"] not in {"none", "unknown", "confirmed"}
                or payload["current_key_revoked"] is not False
                or payload["prior_candidate_present"] is not None and type(payload["prior_candidate_present"]) is not bool):
            raise OperatorError("report_withheld")
        if "http_status" in payload and (payload["reason"] != "http_error" or type(payload["http_status"]) is not int
                                         or not 100 <= payload["http_status"] <= 599):
            raise OperatorError("report_withheld")
    else:
        raise OperatorError("report_withheld")
    return payload


def guarded_call(mode, credentials=None):
    if mode not in {"--preflight", "--resume", "--input-json"}:
        raise OperatorError("invalid_operation")
    command = ["bao-exec", "--output=guarded", "--only", "COOLIFY_URL,COOLIFY_WIRE_TOKEN",
               "--", sys.executable, str(HELPER), mode]
    # Leave deadline/descendant cleanup to the guard (300s) before our outer limit.
    kwargs = {"stdout": subprocess.PIPE, "stderr": subprocess.PIPE, "timeout": 420, "check": False}
    if credentials is None:
        kwargs["stdin"] = subprocess.DEVNULL
    else:
        if mode != "--input-json":
            raise OperatorError("invalid_operation")
        kwargs["input"] = json.dumps(credentials, ensure_ascii=True).encode("ascii")
    result = subprocess.run(command, **kwargs)
    # Child output is already guard-bounded; impose the smaller report bound too.
    if len(result.stdout) > 4096 or len(result.stderr) > 4096:
        raise OperatorError("report_withheld")
    if result.stderr:
        if result.stderr.startswith(b"bao-exec: Direct OpenBao access failed."):
            raise OperatorError("certificate_login_required")
        if result.stderr == b"bao-exec: output withheld by guard.\n":
            raise OperatorError("guard_withheld")
        raise OperatorError("report_withheld")
    parsed = report(json.loads(result.stdout))
    if (parsed["status"] == "PASS") != (result.returncode == 0):
        raise OperatorError("report_withheld")
    if parsed["status"] == "PASS" and ((mode == "--preflight") != ("pending_candidate_present" in parsed)):
        raise OperatorError("report_withheld")
    return parsed


def main(argv=None):
    phase = "terminal_check"
    try:
        # The prompting parent also holds keys; match bao-exec's no-core policy.
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        if (sys.argv[1:] if argv is None else argv):
            raise OperatorError("invalid_operation")
        if not sys.stdin.isatty():
            raise OperatorError("operator_terminal_required")
        phase = "preflight"
        result = guarded_call("--preflight")
        if result["status"] == "PASS":
            if result["pending_candidate_present"]:
                phase = "resume"
                result = guarded_call("--resume")
            else:
                phase = "hidden_input"
                with warnings.catch_warnings():
                    warnings.simplefilter("error", getpass.GetPassWarning)
                    access = getpass.getpass("New Hetzner S3 access key (hidden): ")
                    secret = getpass.getpass("New Hetzner S3 secret key (hidden): ")
                if any(not isinstance(value, str) or not 16 <= len(value) <= 256 for value in (access, secret)):
                    raise OperatorError("credential_input_shape")
                phase = "staging"
                result = guarded_call("--input-json", {"access_key": access, "secret_key": secret})
    except (Exception, KeyboardInterrupt) as error:
        allowed = {"operator_terminal_required", "certificate_login_required", "guard_withheld",
                   "report_withheld", "credential_input_shape", "invalid_operation"}
        reason = str(error) if isinstance(error, OperatorError) and str(error) in allowed else "operation_failed"
        result = {"status": "FAIL", "phase": phase, "reason": reason, "runtime_changed": False}
    print(json.dumps(result, sort_keys=True))
    return int(result["status"] != "PASS")


if __name__ == "__main__":
    raise SystemExit(main())
