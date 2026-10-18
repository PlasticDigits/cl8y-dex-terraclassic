#!/usr/bin/env python3
"""Stage replacement Loki S3 credentials in OpenBao, with hidden local prompts.

DEX #1544: this does not change running Loki or revoke its current credential.
An agent must verify Loki adoption before the operator revokes the previous key.
"""
import argparse
import datetime
import getpass
import hashlib
import hmac
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request
import warnings

APP = "iyi1s7ihy8icleonjdivpac9"
PENDING_PATH = "secret/data/groups/cl8y-logs/s3_rotation_pending"
STATES = frozenset(("staged_unverified", "staged_verified_read"))
SAFE_REASONS = frozenset((
    "interactive_hidden_prompt_required", "coolify_origin", "environment_bound",
    "environment_shape", "environment_duplicate", "replacement_required",
    "s3_endpoint_identity", "s3_bucket_identity", "s3_region", "credential_shape",
    "openbao_unavailable", "openbao_write_denied", "openbao_stage_failed",
    "openbao_readback_failed", "pending_exists_use_resume", "pending_missing",
    "pending_shape", "pending_identity_changed", "bao_admin_required", "new_key_probe",
))


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


OPENER = urllib.request.build_opener(NoRedirect())


def bao_read():
    result = subprocess.run(["bao-admin", "read", "-format=json", PENDING_PATH],
                            text=True, capture_output=True, timeout=15)
    if result.returncode:
        if "No value found" in result.stderr:
            return None
        raise ValueError("openbao_unavailable")
    return json.loads(result.stdout)["data"]


def signing_request(endpoint, bucket, region, access_key, secret_key, now=None):
    endpoint = endpoint if endpoint.startswith("https://") else "https://" + endpoint
    parsed = urllib.parse.urlsplit(endpoint)
    if (parsed.scheme != "https" or not parsed.hostname or parsed.username or parsed.password
            or parsed.query or parsed.fragment or parsed.path not in ("", "/")
            or not parsed.hostname.endswith(".your-objectstorage.com")):
        raise ValueError("s3_endpoint_identity")
    if not re.fullmatch(r"[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]", bucket):
        raise ValueError("s3_bucket_identity")
    if region not in ("nbg1", "fsn1", "hel1"):
        raise ValueError("s3_region")
    for value in (access_key, secret_key):
        if not isinstance(value, str) or not 16 <= len(value) <= 256 or any(c.isspace() for c in value):
            raise ValueError("credential_shape")
    stamp = (now or datetime.datetime.now(datetime.timezone.utc)).strftime("%Y%m%dT%H%M%SZ")
    day, path = stamp[:8], "/" + urllib.parse.quote(bucket, safe="")
    digest = hashlib.sha256(b"").hexdigest()
    headers = {"Host": parsed.netloc, "X-Amz-Date": stamp, "X-Amz-Content-Sha256": digest}
    canonical_headers = f"host:{parsed.netloc}\nx-amz-content-sha256:{digest}\nx-amz-date:{stamp}\n"
    signed = "host;x-amz-content-sha256;x-amz-date"
    canonical = "\n".join(("HEAD", path, "", canonical_headers, signed, digest))
    scope = f"{day}/{region}/s3/aws4_request"
    to_sign = "\n".join(("AWS4-HMAC-SHA256", stamp, scope, hashlib.sha256(canonical.encode()).hexdigest()))
    key = ("AWS4" + secret_key).encode()
    for part in (day, region, "s3", "aws4_request"):
        key = hmac.new(key, part.encode(), hashlib.sha256).digest()
    signature = hmac.new(key, to_sign.encode(), hashlib.sha256).hexdigest()
    headers["Authorization"] = f"AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed}, Signature={signature}"
    return urllib.request.Request(endpoint.rstrip("/") + path, method="HEAD", headers=headers)


def environment():
    """Read only this application's storage fields; never return raw API objects."""
    base, token = os.environ["COOLIFY_URL"].rstrip("/"), os.environ["COOLIFY_WIRE_TOKEN"]
    parsed = urllib.parse.urlsplit(base)
    if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise ValueError("coolify_origin")
    request = urllib.request.Request(base + "/api/v1/applications/" + APP + "/envs",
                                     headers={"Authorization": "Bearer " + token, "Accept": "application/json"})
    with OPENER.open(request, timeout=10) as response:
        raw = response.read(1024 * 1024 + 1)
    if len(raw) > 1024 * 1024:
        raise ValueError("environment_bound")
    rows = json.loads(raw)
    if not isinstance(rows, list) or len(rows) > 512:
        raise ValueError("environment_shape")
    names = {"S3_ENDPOINT", "S3_BUCKET", "S3_REGION", "S3_ACCESS_KEY", "S3_SECRET_KEY"}
    env = {}
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError("environment_shape")
        if row.get("key") not in names or row.get("is_preview"):
            continue
        if row["key"] in env:
            raise ValueError("environment_duplicate")
        if not isinstance(row.get("value"), str) or not 0 < len(row["value"]) <= 512:
            raise ValueError("environment_shape")
        env[row["key"]] = row["value"]
    # Match the existing production Compose default, rather than guessing a new region.
    env.setdefault("S3_REGION", "nbg1")
    if set(env) != names:
        raise ValueError("environment_shape")
    signing_request(env["S3_ENDPOINT"], env["S3_BUCKET"], env["S3_REGION"],
                    env["S3_ACCESS_KEY"], env["S3_SECRET_KEY"])
    return env


def writable():
    """Check authority before prompting; do not test writes with fake live records."""
    result = subprocess.run(["bao-admin", "token", "capabilities", PENDING_PATH],
                            capture_output=True, text=True, timeout=15)
    capabilities = set(result.stdout.replace(",", " ").split())
    if result.returncode or not {"create", "read", "update"} <= capabilities or "deny" in capabilities:
        raise ValueError("openbao_write_denied")


def validate_pending(data, env):
    fields = {"access_key", "secret_key", "endpoint", "bucket", "region", "state", "staged_at_utc"}
    if (not isinstance(data, dict) or set(data) - (fields | {"verified_at_utc"})
            or not fields <= set(data) or data.get("state") not in STATES):
        raise ValueError("pending_shape")
    for key, source in (("endpoint", "S3_ENDPOINT"), ("bucket", "S3_BUCKET"), ("region", "S3_REGION")):
        if data[key] != env[source]:
            raise ValueError("pending_identity_changed")
    for key in ("staged_at_utc", "verified_at_utc"):
        if key in data:
            if not isinstance(data[key], str) or len(data[key]) > 40:
                raise ValueError("pending_shape")
            if datetime.datetime.fromisoformat(data[key]).tzinfo is None:
                raise ValueError("pending_shape")
    if data["access_key"] == env["S3_ACCESS_KEY"] or data["secret_key"] == env["S3_SECRET_KEY"]:
        raise ValueError("replacement_required")
    return signing_request(data["endpoint"], data["bucket"], data["region"],
                           data["access_key"], data["secret_key"])


def store(data, previous):
    """CAS write plus private readback; stdout/stderr from bao-admin stay captured."""
    payload = {"options": {"cas": previous["metadata"]["version"] if previous else 0}, "data": data}
    result = subprocess.run(["bao-admin", "write", "-format=json", PENDING_PATH, "-"],
                            input=json.dumps(payload), capture_output=True, text=True, timeout=15)
    if result.returncode:
        raise ValueError("openbao_stage_failed")
    after = bao_read()
    if not isinstance(after, dict) or after.get("data") != data:
        raise ValueError("openbao_readback_failed")
    return after


def valid_version(previous):
    if previous is not None:
        if not isinstance(previous, dict) or not isinstance(previous.get("metadata"), dict):
            raise ValueError("pending_shape")
        version = previous["metadata"].get("version")
        if type(version) is not int or version < 1:
            raise ValueError("pending_shape")


def stage(*, resume=False, replace_pending=False, progress=None, preflight=False):
    progress = {} if progress is None else progress
    progress.update(phase="configuration", candidate_retention="none", prior_candidate_present=None)
    env = environment()
    progress["phase"] = "openbao_preflight"
    previous = bao_read()
    writable()
    valid_version(previous)
    progress["prior_candidate_present"] = previous is not None
    if previous:
        if not replace_pending:
            validate_pending(previous["data"], env)
            progress["candidate_retention"] = "confirmed"
    if preflight:
        return {"status": "PASS", "scope": "configuration and OpenBao authority only",
                "pending_candidate_present": previous is not None, "runtime_changed": False}
    if resume:
        if previous is None:
            raise ValueError("pending_missing")
        data = dict(previous["data"])
        if data["state"] == "staged_verified_read":
            # A failed recheck must not leave the older positive eligible for adoption.
            data.update(state="staged_unverified")
            data.pop("verified_at_utc", None)
            progress.update(phase="candidate_storage", candidate_retention="unknown")
            previous = store(data, previous)
            progress["candidate_retention"] = "confirmed"
    else:
        if previous is not None and not replace_pending:
            raise ValueError("pending_exists_use_resume")
        progress.update(phase="hidden_input", candidate_retention="none")
        if not sys.stdin.isatty():
            raise ValueError("interactive_hidden_prompt_required")
        # getpass must never fall back to echoed input if terminal control fails.
        with warnings.catch_warnings():
            warnings.simplefilter("error", getpass.GetPassWarning)
            access = getpass.getpass("New Hetzner S3 access key (hidden): ")
            secret = getpass.getpass("New Hetzner S3 secret key (hidden): ")
        data = {"access_key": access, "secret_key": secret, "endpoint": env["S3_ENDPOINT"],
                "bucket": env["S3_BUCKET"], "region": env["S3_REGION"], "state": "staged_unverified",
                "staged_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat()}
        progress["phase"] = "candidate_validation"
        validate_pending(data, env)
        progress["phase"] = "candidate_storage"
        progress["candidate_retention"] = "unknown"
        previous = store(data, previous)
        progress["candidate_retention"] = "confirmed"
    progress["phase"] = "bucket_verification"
    probe = validate_pending(data, env)
    with OPENER.open(probe, timeout=10) as response:
        if response.status != 200:
            raise ValueError("new_key_probe")
    # An unverified candidate never authorizes adoption. Promote only after real HEAD200.
    progress["phase"] = "verification_storage"
    data.update(state="staged_verified_read", verified_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
    store(data, previous)
    return {"status": "PASS", "candidate_state": "staged_verified_read", "candidate_retention": "confirmed",
            "runtime_changed": False, "current_key_revoked": False,
            "next_action": "agent adoption verification before old-key revocation"}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="Check dependency without reading credentials")
    mode.add_argument("--preflight", action="store_true", help="Read configuration and verify staging authority; no prompts or writes")
    mode.add_argument("--resume", action="store_true", help="Verify the privately retained candidate without prompting")
    mode.add_argument("--replace-pending", action="store_true", help="Explicitly replace a pending candidate through hidden prompts")
    args = parser.parse_args(argv)
    progress = {"phase": "dependency_check", "candidate_retention": "none", "prior_candidate_present": None}
    try:
        if args.check:
            import shutil
            if not shutil.which("bao-admin"):
                raise ValueError("bao_admin_required")
            result = {"status": "PASS", "scope": "dependency only; no secrets read or changed"}
        else:
            result = stage(resume=args.resume, replace_pending=args.replace_pending,
                           progress=progress, preflight=args.preflight)
        print(json.dumps(result, sort_keys=True))
        return 0
    except Exception as error:
        reason = str(error) if isinstance(error, ValueError) and str(error) in SAFE_REASONS else "operation_failed"
        result = {"status": "FAIL", "phase": progress["phase"], "reason": reason,
                  "candidate_retention": progress["candidate_retention"], "runtime_changed": False,
                  "prior_candidate_present": progress["prior_candidate_present"],
                  "current_key_revoked": False}
        if isinstance(error, urllib.error.HTTPError):
            result.update(reason="http_error", http_status=error.code)
        elif isinstance(error, getpass.GetPassWarning):
            result["reason"] = "hidden_input_unavailable"
        print(json.dumps(result, sort_keys=True))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
