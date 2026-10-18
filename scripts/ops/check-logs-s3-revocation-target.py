#!/usr/bin/env python3
"""Privately verify the selected Console entry is Loki's retired S3 access key.

Run directly in a real local terminal. This helper never revokes credentials,
changes configuration, opens a browser, or modifies clipboard contents.
"""
import getpass
import hmac
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import sys
import warnings

APP = "iyi1s7ihy8icleonjdivpac9"
JOURNAL = "secret/data/groups/cl8y-logs/s3_rotation_adoption"
REASONS = frozenset(("interactive_hidden_prompt_required", "hidden_input_unavailable", "journal_unavailable",
    "journal_shape", "adoption_incomplete", "adoption_unverified", "attempt_mismatch", "input_shape",
    "replacement_selected", "selected_key_mismatch", "operation_failed"))


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load_worker():
    spec = importlib.util.spec_from_file_location("revocation_worker_schema", Path(__file__).with_name("loki-s3-adoption-worker.py"))
    worker = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(worker)
    return worker


def read_state():
    # The operator wrapper obtains its own local certificate identity. Never
    # supply input, Coolify credentials, Vault tokens, or raw errors to it.
    env = {key: os.environ[key] for key in ("HOME", "PATH", "LANG", "LC_ALL") if key in os.environ}
    result = subprocess.run(["bao-admin", "read", "-format=json", JOURNAL], capture_output=True,
                            text=True, timeout=15, env=env)
    require(result.returncode == 0 and len(result.stdout) <= 1024 * 1024
            and len(result.stderr) <= 65536, "journal_unavailable")
    parsed = json.loads(result.stdout)
    require(isinstance(parsed, dict) and isinstance(parsed.get("data"), dict)
            and isinstance(parsed["data"].get("data"), dict), "journal_shape")
    return parsed["data"]["data"]


def validated_keys(state, worker):
    require(isinstance(state, dict) and state.get("app_uuid") == APP, "journal_shape")
    require(state.get("phase") in ("config_persisted", "cleaned"), "adoption_incomplete")
    nonce = state.get("attempt_nonce", state.get("transaction"))
    require(isinstance(nonce, str) and re.fullmatch(r"[a-f0-9]{16}", nonce), "journal_shape")
    receipt = worker.closed_report(state.get("adoption_receipt"))
    require(receipt is not None and receipt.get("status") == "PASS" and receipt.get("mode") == "adopt"
            and receipt.get("ready") is True and receipt.get("backing_storage_write_verified") is True
            and receipt.get("marker_readback_verified") is True and receipt.get("historical_readback_verified") is True
            and receipt.get("flush_failure_delta") == 0 and receipt.get("flush_success_delta", 0) >= 1,
            "adoption_unverified")
    # The latest nonce is mandatory after any reset. Older archived reports may
    # omit a nonce, but cannot override a state's explicitly selected attempt.
    if "attempt_nonce" in state or "attempt_nonce" in receipt:
        require(receipt.get("attempt_nonce") == nonce, "attempt_mismatch")
    payload = state.get("payload")
    require(isinstance(payload, dict), "journal_shape")
    old, new = payload.get("expected_old_access"), payload.get("new_access")
    require(all(isinstance(key, str) and 16 <= len(key) <= 256 and key.isascii()
                and not any(c.isspace() or ord(c) < 32 or ord(c) == 127 for c in key) for key in (old, new))
            and old != new, "journal_shape")
    return old, new


def check_selected(selected, old, new):
    require(isinstance(selected, str) and 16 <= len(selected) <= 256 and selected.isascii()
            and not any(c.isspace() or ord(c) < 32 or ord(c) == 127 for c in selected), "input_shape")
    require(not hmac.compare_digest(selected, new), "replacement_selected")
    require(hmac.compare_digest(selected, old), "selected_key_mismatch")
    return {"status": "PASS", "selected_key_is_retired_loki_key": True, "replacement_selected": False,
            "may_revoke_selected_console_entry": True}


def main():
    try:
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        os.umask(0o077)
        require(sys.stdin.isatty(), "interactive_hidden_prompt_required")
        old, new = validated_keys(read_state(), load_worker())
        with warnings.catch_warnings():
            warnings.simplefilter("error", getpass.GetPassWarning)
            selected = getpass.getpass("Selected Console S3 access key ID (hidden; not the secret key): ")
        report = check_selected(selected, old, new)
        code = 0
    except Exception as error:
        if isinstance(error, getpass.GetPassWarning):
            reason = "hidden_input_unavailable"
        else:
            reason = str(error) if isinstance(error, ValueError) and str(error) in REASONS else "operation_failed"
        report, code = {"status": "FAIL", "reason": reason}, 1
    print(json.dumps(report, sort_keys=True))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
