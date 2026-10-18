#!/usr/bin/env python3
"""Reproducible, scoped Coolify/Grafana monitoring administration. No DEX writes."""
import argparse
import importlib.util
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import Request

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("monitor", ROOT / "scripts/qa/probe-swap-monitoring-1479-1480.py")
MONITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITOR)
NAME = "cl8y-dex-monitoring"
LOGS = "iyi1s7ihy8icleonjdivpac9"
SERVER = "ezqivkqptuds8gjnvfuw18jx"
PROJECT = "4ezx2qu5zsjpw3eybiie7rum"
ENVIRONMENT = "j9znbhbow3funogzkohzd98b"
DESTINATION = "n3nyhm5drw6a6plmcbjgjj3k"
UID = "cl8y-dex-prometheus"
REPOSITORY = "https://git.cl8y.com/code/cl8y-dex-terraclassic.git"
ID = re.compile(r"[A-Za-z0-9_-]{1,128}")
SHA = re.compile(r"[0-9a-f]{40}")
DEPLOYMENT_STATES = {"queued", "in_progress", "finished", "failed", "cancelled", "cancelled-by-user", "canceled"}


def require(value, label):
    if not value:
        raise ValueError(label)


def exchange(opener, url, authorization, method="GET", payload=None):
    headers = {"Authorization": authorization, "Accept": "application/json", "Cache-Control": "no-cache"}
    data = None
    if payload is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(payload).encode()
    try:
        with opener.open(Request(url, data=data, headers=headers, method=method), timeout=10) as response:
            body = response.read(2 * 1024 * 1024 + 1)
    except HTTPError as error:
        # Response bodies may contain interpolated secrets. Publish only status.
        raise ValueError("http_status_" + str(error.code)) from None
    require(len(body) <= 2 * 1024 * 1024, "body_bound")
    return json.loads(body) if body else {}


class Coolify:
    def __init__(self):
        self.reader = MONITOR.ATTESTER.Reader()
        self.calls = 0

    def api(self, path, method="GET", payload=None):
        require(not path.startswith("/") and ".." not in path and len(path) <= 256, "api_path")
        self.calls += 1
        require(self.calls <= 12, "request_budget")
        return exchange(self.reader.opener, self.reader.api_base + path, "Bearer " + self.reader.token, method, payload)

    def owned(self, app):
        require(isinstance(app, str) and ID.fullmatch(app) and app != LOGS, "application_id")
        row = self.api("applications/" + app)
        require(row.get("uuid") == app and row.get("name") == NAME
                and row.get("git_repository") == REPOSITORY
                and row.get("build_pack") == "dockercompose"
                and row.get("git_branch") == "main"
                and SHA.fullmatch(str(row.get("git_commit_sha", "")))
                and row.get("docker_compose_location") == "/monitoring/dex/compose.yaml"
                and row.get("destination_id") == 0 and row.get("environment_id") == 19, "monitoring_application_binding")
        return row


def create(api, revision):
    require(SHA.fullmatch(revision or ""), "revision")
    compose = subprocess.run(["git", "show", revision + ":monitoring/dex/compose.yaml"], cwd=ROOT, check=True, capture_output=True).stdout.decode()
    require(0 < len(compose) <= 65536, "compose_bound")
    apps = api.api("applications")
    require(isinstance(apps, list) and len(apps) <= 512, "inventory_bound")
    existing = [a for a in apps if a.get("name") == NAME]
    require(len(existing) <= 1, "duplicate_monitor")
    if existing:
        row = api.owned(existing[0]["uuid"])
        require(row.get("git_commit_sha") == revision, "existing_revision_mismatch")
        return {"app_uuid": row["uuid"], "created": False, "revision": revision}
    logs = api.api("applications/" + LOGS)
    require(logs.get("name") == "cl8y-logs" and logs.get("destination_id") == 0
            and logs.get("environment_id") == 19, "existing_logs_binding")
    payload = {"name": NAME, "description": "DEX monitoring only; issue #1544; no DEX lifecycle authority",
               "project_uuid": PROJECT, "environment_uuid": ENVIRONMENT, "server_uuid": SERVER,
               "destination_uuid": DESTINATION, "git_repository": REPOSITORY, "git_branch": "main",
               "git_commit_sha": revision, "build_pack": "dockercompose", "base_directory": "/",
               "docker_compose_location": "/monitoring/dex/compose.yaml",
               "docker_compose_raw": compose,
               "is_preserve_repository_enabled": True, "is_auto_deploy_enabled": False,
               "instant_deploy": False, "autogenerate_domain": False}
    row = api.api("applications/public", "POST", payload)
    require(ID.fullmatch(str(row.get("uuid", ""))), "created_identity")
    api.owned(row["uuid"])
    return {"app_uuid": row["uuid"], "created": True, "revision": revision}


def status(api, app):
    row = api.owned(app)
    history = api.api("deployments/applications/" + app + "?skip=0&take=1")
    deployments = history.get("deployments", [])
    state = row.get("status", "")
    require(state in {a + ":" + b for a in ("running", "exited", "starting", "restarting", "degraded") for b in ("healthy", "unhealthy", "unknown")} , "application_status")
    require(SHA.fullmatch(str(row.get("git_commit_sha", ""))), "application_revision")
    require(isinstance(deployments, list) and len(deployments) <= 1, "deployment_bound")
    result = {"utc": datetime.now(timezone.utc).isoformat(), "app_uuid": app, "status": state, "revision": row["git_commit_sha"]}
    if deployments:
        latest = deployments[0]
        require(isinstance(latest, dict) and ID.fullmatch(str(latest.get("deployment_uuid", "")))
                and latest.get("status") in DEPLOYMENT_STATES and SHA.fullmatch(str(latest.get("commit", ""))), "deployment_identity")
        result["deployment"] = {k: latest[k] for k in ("deployment_uuid", "status", "commit")}
        detail = api.api("deployments/" + latest["deployment_uuid"])
        require(detail.get("deployment_uuid") == latest["deployment_uuid"]
                and detail.get("application_id") == row.get("id")
                and row.get("id") is not None and detail.get("commit") == latest["commit"], "deployment_application_binding")
        # Inspect raw logs in memory only; build environments can contain secrets.
        logs = detail.get("logs")
        if isinstance(logs, str):
            try:
                logs = json.loads(logs)
            except ValueError:
                raise ValueError("deployment_logs_unavailable") from None
        require(isinstance(logs, list) and len(logs) <= 4096 and all(isinstance(r, dict) and isinstance(r.get("output"), str) for r in logs), "deployment_logs_unavailable")
        result["deployment_log_entries"] = len(logs)
        result["deployment_error_entries"] = sum(bool(re.search(r"(?i)\b(error|fatal|panic)\b", r["output"])) for r in logs)
    return result


def provision():
    monitor = MONITOR.Monitor()
    def grafana(path, method="GET", payload=None):
        return exchange(monitor.reader.opener, MONITOR.GRAFANA_ORIGIN + path, monitor.authorization, method, payload)
    sources = grafana("/api/datasources")
    require(isinstance(sources, list) and len(sources) <= 32, "datasource_inventory")
    existing = [s for s in sources if s.get("uid") == UID]
    require(len(existing) <= 1, "datasource_identity")
    payload = {"uid": UID, "name": "CL8Y DEX Prometheus", "type": "prometheus", "access": "proxy",
               "url": "http://dex-prometheus:9090", "isDefault": False, "basicAuth": False,
               "jsonData": {"httpMethod": "POST", "timeInterval": "30s", "queryTimeout": "10s", "prometheusType": "Prometheus"}}
    if existing:
        require(existing[0].get("type") == "prometheus" and existing[0].get("url") == payload["url"], "existing_datasource_binding")
    grafana("/api/datasources/uid/" + UID if existing else "/api/datasources", "PUT" if existing else "POST", payload)
    dashboard = json.loads((ROOT / "monitoring/dex/dashboard.json").read_text())
    require(dashboard.get("uid") == "cl8y-dex-monitoring", "dashboard_identity")
    grafana("/api/dashboards/db", "POST", {"dashboard": dashboard, "overwrite": True, "message": "DEX monitoring #1544"})
    health = grafana("/api/datasources/uid/" + UID + "/health")
    require(health.get("status") == "OK", "datasource_health")
    return {"datasource_uid": UID, "health": "OK", "dashboard_uid": dashboard["uid"], "loki_changed": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    sub.add_parser("create").add_argument("--revision", required=True)
    for action in ("deploy", "status"):
        sub.add_parser(action).add_argument("--app", required=True)
    sub.add_parser("provision")
    sub.add_parser("verify")
    args = parser.parse_args()
    try:
        if args.action == "provision":
            result = provision()
        elif args.action == "verify":
            result = MONITOR.Monitor().collect()
        else:
            api = Coolify()
            if args.action == "create":
                result = create(api, args.revision)
            elif args.action == "status":
                result = status(api, args.app)
            else:
                api.owned(args.app)
                result = api.api("deploy?uuid=" + args.app + "&force=false")
                # Deployment response is restricted to UUIDs and fixed counters.
                result = {"app_uuid": args.app, "deployment_requested": True}
        print(json.dumps(result, indent=2))
        return 0
    except Exception as error:
        label = str(error) if isinstance(error, ValueError) and re.fullmatch(r"[a-z_0-9]+", str(error)) else type(error).__name__
        print(json.dumps({"overall": "FAIL", "failure_class": label}))
        return 1


if __name__ == "__main__":
    sys.exit(main())
