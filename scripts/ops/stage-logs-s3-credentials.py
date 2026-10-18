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

APP = "iyi1s7ihy8icleonjdivpac9"
PENDING_PATH = "secret/data/groups/cl8y-logs/s3_rotation_pending"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


OPENER = urllib.request.build_opener(NoRedirect())


def bao_read():
    result = subprocess.run(["bao-admin", "read", "-format=json", PENDING_PATH],
                            text=True, capture_output=True, timeout=30)
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


def stage():
    if not sys.stdin.isatty():
        raise ValueError("interactive_hidden_prompt_required")
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
    env = {row["key"]: row["value"] for row in json.loads(raw) if not row.get("is_preview")}
    access = getpass.getpass("New Hetzner S3 access key (hidden): ")
    secret = getpass.getpass("New Hetzner S3 secret key (hidden): ")
    if access == env["S3_ACCESS_KEY"] or secret == env["S3_SECRET_KEY"]:
        raise ValueError("replacement_required")
    probe = signing_request(env["S3_ENDPOINT"], env["S3_BUCKET"], env["S3_REGION"], access, secret)
    with OPENER.open(probe, timeout=10) as response:
        if response.status != 200:
            raise ValueError("new_key_probe")
    previous = bao_read()
    data = {"access_key": access, "secret_key": secret, "endpoint": env["S3_ENDPOINT"],
            "bucket": env["S3_BUCKET"], "region": env["S3_REGION"], "state": "staged_verified_read",
            "staged_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat()}
    payload = {"options": {"cas": previous["metadata"]["version"] if previous else 0}, "data": data}
    result = subprocess.run(["bao-admin", "write", "-format=json", PENDING_PATH, "-"],
                            input=json.dumps(payload), capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise ValueError("openbao_stage_failed")
    if bao_read()["data"] != data:
        raise ValueError("openbao_readback_failed")
    print("PASS: replacement key verified and staged in OpenBao. Keep the old key active until agent verification confirms Loki adoption.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Check script and dependencies without prompting or reading credentials")
    args = parser.parse_args()
    if args.check:
        import shutil
        if not shutil.which("bao-admin"):
            raise ValueError("bao_admin_required")
        print("PASS: hidden-prompt staging helper is available; no secrets read or changed.")
    else:
        stage()


if __name__ == "__main__":
    try:
        main()
    except Exception:
        print("FAIL: credentials were not printed; keep the current key active and report staging failure.")
        raise SystemExit(1)
