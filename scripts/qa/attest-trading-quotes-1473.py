#!/usr/bin/env python3
"""Bounded read-only deployment provenance for the public trading quote matrix."""

import argparse
import hashlib
import json
import os
import re
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

FRONTEND = "https://dex.cl8y.com"
BACKEND = "https://indexer.dex.cl8y.com"
REVISION = re.compile(r"[0-9a-f]{40}")
APP_ID = re.compile(r"[A-Za-z0-9_-]{1,128}")
MAX_READS = 16


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


class Reader:
    def __init__(self):
        base = os.environ.get("COOLIFY_URL", "").strip().rstrip("/")
        parsed = urllib.parse.urlsplit(base)
        self.token = os.environ.get("COOLIFY_WIRE_TOKEN", "").strip()
        # The scoped operator configuration may use an enrolled private HTTP
        # broker. Never obtain this origin from a response or follow redirects.
        if (parsed.scheme not in {"http", "https"} or not parsed.hostname or parsed.username
                or parsed.password or parsed.query or parsed.fragment or not self.token):
            raise ValueError("scoped_coolify_credentials_required")
        self.api_base = base + "/api/v1/"
        self.opener = urllib.request.build_opener(NoRedirect())
        self.reads = 0

    def read(self, url, limit, private=False):
        self.reads += 1
        if self.reads > MAX_READS:
            raise ValueError("read_budget")
        headers = {"Accept": "application/json" if private else "*/*", "Cache-Control": "no-cache"}
        if private:
            headers["Authorization"] = "Bearer " + self.token
        request = urllib.request.Request(url, headers=headers, method="GET")
        with self.opener.open(request, timeout=15) as response:
            data = response.read(limit + 1)
            if len(data) > limit:
                raise ValueError("body_bound")
            return data

    def api(self, api_path):
        return json.loads(self.read(self.api_base + api_path, 8 * 1024 * 1024, True))


def rows(value):
    if isinstance(value, list):
        return value
    if isinstance(value, dict):
        for key in ("applications", "deployments", "data"):
            if isinstance(value.get(key), list):
                return value[key]
    return []


def discover(reader, requested):
    apps = rows(reader.api("applications"))
    if len(apps) > 512:
        raise ValueError("application_inventory_bound")
    selected = {}
    for label, domain in requested.items():
        matches = []
        for app in apps:
            if not isinstance(app, dict) or not isinstance(app.get("fqdn"), str):
                continue
            if any(urllib.parse.urlsplit(url.strip()).hostname == domain for url in app["fqdn"].split(",")):
                matches.append(app.get("uuid"))
        if len(matches) != 1 or not isinstance(matches[0], str) or not APP_ID.fullmatch(matches[0]):
            raise ValueError("application_identity_unavailable")
        selected[label] = matches[0]
    return selected


def utc_time(value):
    if not isinstance(value, str) or len(value) > 64:
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        return parsed.astimezone(timezone.utc).isoformat() if parsed.tzinfo else None
    except ValueError:
        return None


def deployment(reader, app_id, domain):
    encoded = urllib.parse.quote(app_id, safe="")
    app = reader.api("applications/" + encoded)
    if not isinstance(app, dict) or not isinstance(app.get("fqdn"), str) or not any(
        urllib.parse.urlsplit(url.strip()).hostname == domain for url in app["fqdn"].split(",")
    ):
        raise ValueError("application_domain_mismatch")
    candidates = rows(reader.api("deployments/applications/" + encoded + "?skip=0&take=5"))
    if len(candidates) > 20:
        raise ValueError("deployment_inventory_bound")
    completed = [row for row in candidates if isinstance(row, dict) and row.get("status") == "finished"
                 and REVISION.fullmatch(str(row.get("commit", ""))) and utc_time(row.get("created_at"))]
    latest = max(completed, key=lambda row: utc_time(row["created_at"]), default=None)
    if latest is None:
        raise ValueError("completed_deployment_unavailable")
    return {"source_sha": latest["commit"], "deployment_status": "finished",
            "deployment_finished_at_utc": utc_time(latest.get("finished_at"))}


def artifacts(reader):
    html = reader.read(FRONTEND + "/", 128 * 1024)
    paths = re.findall(rb'<script\b[^>]*\bsrc=["\']([^"\']+)["\']', html)
    paths = [value.decode("utf8") for value in paths if value.startswith(b"/assets/") and value.endswith(b".js")]
    if not paths or len(paths) > 8:
        raise ValueError("root_artifact_bound")
    result = []
    for asset in paths:
        if not re.fullmatch(r"/assets/[A-Za-z0-9_.-]+\.js", asset):
            raise ValueError("artifact_path_shape")
        payload = reader.read(FRONTEND + asset, 8 * 1024 * 1024)
        result.append({"asset": asset.rsplit("/", 1)[-1], "sha256": hashlib.sha256(payload).hexdigest(), "bytes": len(payload)})
    return {"html_sha256": hashlib.sha256(html).hexdigest(), "root_scripts": result}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--frontend-app-uuid")
    parser.add_argument("--backend-app-uuid")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not REVISION.fullmatch(args.revision):
        parser.error("exact public revision required")
    for app_id in (args.frontend_app_uuid, args.backend_app_uuid):
        if app_id is not None and not APP_ID.fullmatch(app_id):
            parser.error("invalid application identity")
    result = {"issue": 1473, "expected_revision": args.revision, "utc": datetime.now(timezone.utc).isoformat(), "read_only": True}
    try:
        reader = Reader()
        selected = {"frontend": args.frontend_app_uuid, "backend": args.backend_app_uuid}
        missing = {label: urllib.parse.urlsplit(FRONTEND if label == "frontend" else BACKEND).hostname
                   for label, app_id in selected.items() if app_id is None}
        if missing:
            selected.update(discover(reader, missing))
        result["frontend"] = deployment(reader, selected["frontend"], "dex.cl8y.com")
        result["frontend"]["artifact"] = artifacts(reader)
        result["frontend"]["revision_matches"] = result["frontend"]["source_sha"] == args.revision
        result["backend_deployment"] = deployment(reader, selected["backend"], "indexer.dex.cl8y.com")
        health = json.loads(reader.read(BACKEND + "/health", 4096))
        backend_sha = health.get("git_sha") if isinstance(health, dict) else None
        if not isinstance(backend_sha, str) or not REVISION.fullmatch(backend_sha):
            raise ValueError("backend_revision_unavailable")
        result["backend"] = {"git_sha": backend_sha, "revision_matches": backend_sha == args.revision}
        result["overall"] = "PASS" if (result["frontend"]["revision_matches"] and result["backend"]["revision_matches"]
            and result["backend_deployment"]["source_sha"] == args.revision) else "FAIL"
        result["read_count"] = reader.reads
        result["limits"] = {"no_raw_private_bodies": True, "no_private_endpoints": True,
            "frontend_source_attestation": "completed deployment metadata and independently hashed served root artifacts",
            "backend_source_attestation": "public health revision and completed deployment metadata",
            "unverified": ["lazy-loaded frontend artifacts", "settlement", "historical economics", "physical devices"]}
    except Exception as error:
        result.update({"overall": "FAIL", "failure_class": type(error).__name__})
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as output:
        os.chmod(args.output, 0o600)
        output.write(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result.get(key) for key in ("overall", "expected_revision", "failure_class")}))
    return int(result["overall"] != "PASS")


if __name__ == "__main__":
    raise SystemExit(main())
