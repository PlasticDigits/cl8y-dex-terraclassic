#!/usr/bin/env python3
"""Journal and deploy a one-shot Loki credential transaction through Coolify.

Run noninteractively under bao-exec with only COOLIFY_URL,COOLIFY_WIRE_TOKEN.
Secrets remain in private OpenBao records and runtime-only shown-once variables.
This never deploys the logs application or revokes either S3 credential.
"""
import argparse
import base64
import copy
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import resource
import secrets
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
APP = "iyi1s7ihy8icleonjdivpac9"
SERVER = "ezqivkqptuds8gjnvfuw18jx"
JOURNAL = "secret/data/groups/cl8y-logs/s3_rotation_adoption"
ID = re.compile(r"[a-z0-9]{20,32}")
SHA = re.compile(r"[a-f0-9]{40}")
BASE = "docker:27.5.1-cli@sha256:851f91d241214e7c6db86513b270d58776379aacc5eb9c4a87e5b47115e3065c"
PROTECTED = {APP, "s14i3w45r5g6ulyu6nsxzm0n", "o4htirce3mwp89dzip3vdikt"}
SOURCE_OVERRIDES = {"source_file": "compose.yaml", "workdir": "/data/coolify/applications/" + APP}
SAFE_PHASES = {"prepared", "reset_prepared", "preflight_started", "adoption_started", "adoption_failed", "runtime_adopted", "config_persisted", "cleaned"}


class GuardError(ValueError):
    """Fixed local reason only."""


def require(condition, reason):
    if not condition:
        raise GuardError(reason)


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def vault_read():
    result = subprocess.run(["bao-admin", "read", "-format=json", JOURNAL],
                            capture_output=True, text=True, timeout=15)
    if result.returncode:
        require("No value found" in result.stderr, "journal_read_failed")
        return None
    result = json.loads(result.stdout)["data"]
    require(type(result["metadata"]["version"]) is int, "journal_shape")
    return result


def save(data, previous):
    body = {"options": {"cas": previous["metadata"]["version"] if previous else 0}, "data": data}
    result = subprocess.run(["bao-admin", "write", "-format=json", JOURNAL, "-"],
                            input=json.dumps(body), capture_output=True, text=True, timeout=15)
    require(result.returncode == 0, "journal_cas_failed")
    readback = vault_read()
    require(readback is not None and readback["data"] == data, "journal_readback_failed")
    return readback


class Api:
    def __init__(self):
        self.reader = load("adoption_reader", ROOT / "scripts/qa/attest-trading-quotes-1473.py").Reader()
        self.calls = 0

    def call(self, path, method="GET", payload=None):
        require(not path.startswith("/") and ".." not in path and len(path) < 256, "api_path")
        self.calls += 1
        require(self.calls <= 24, "api_budget")
        require(method in {"GET", "POST", "PATCH", "DELETE"}, "api_method")
        request = urllib.request.Request(self.reader.api_base + path, method=method,
            data=json.dumps(payload).encode() if payload is not None else None,
            headers={"Authorization": "Bearer " + self.reader.token, "Content-Type": "application/json"})
        try:
            with self.reader.opener.open(request, timeout=12) as response:
                raw = response.read(8 * 1024 * 1024 + 1)
        except urllib.error.HTTPError as error:
            # Never retain or interpolate an HTTP error body.
            raise GuardError("api_http_" + str(error.code)) from None
        require(len(raw) <= 8 * 1024 * 1024, "api_body_bound")
        return json.loads(raw) if raw else {}


def recipe(revision):
    lines = ["FROM " + BASE, "RUN apk add --no-cache python3", "WORKDIR /ops"]
    for name in ("replace-loki-s3-host.py", "loki-s3-adoption-worker.py"):
        path = "scripts/ops/" + name
        result = subprocess.run(["git", "show", revision + ":" + path], cwd=ROOT,
                                capture_output=True, timeout=5)
        require(result.returncode == 0, "committed_worker_required")
        source = result.stdout
        require(source == (ROOT / path).read_bytes(), "worker_source_changed")
        # These are reviewed source bytes only; no runtime values enter the image.
        lines.append("RUN printf %s " + base64.b64encode(source).decode() + " | base64 -d > /ops/" + name)
    lines += ["ENTRYPOINT []", 'CMD ["python3", "/ops/loki-s3-adoption-worker.py"]']
    return "\n".join(lines) + "\n"


def compose_document(dockerfile, transaction):
    require(re.fullmatch(r"[a-f0-9]{16}", transaction), "transaction_identity")
    directory = "/data/coolify/applications/" + APP
    state_volume = "transaction-state-" + transaction
    keys = ("LOKI_S3_JOB_UUID", "LOKI_S3_JOB_ID", "LOKI_S3_MODE", "LOKI_S3_NETWORK", "LOKI_S3_PAYLOAD_JSON")
    return {"services": {"maintenance": {
        "image": "cl8y-loki-s3-adoption:" + transaction,
        "build": {"context": ".", "dockerfile_inline": dockerfile},
        "restart": "no", "cap_drop": ["ALL"], "mem_limit": "256m", "cpus": "0.5",
        "environment": {key: "${" + key + ":-disabled}" for key in keys},
        "volumes": ["/var/run/docker.sock:/var/run/docker.sock:rw", directory + ":" + directory + ":ro",
                    state_volume + ":/state"]}},
        "volumes": {state_volume: {"name": "dex-loki-s3-" + transaction}}}


def compose_source(dockerfile, transaction):
    # JSON is valid YAML and preserves the reviewed source byte-for-byte.
    return json.dumps(compose_document(dockerfile, transaction), separators=(",", ":"), sort_keys=True)


def binding(app):
    require(app.get("uuid") == APP and app.get("name") == "cl8y-logs"
            and app.get("build_pack") == "dockercompose", "logs_app_binding")
    require(app["destination"]["server"]["uuid"] == SERVER, "logs_server_binding")
    require(type(app.get("environment_id")) is int and app["environment_id"] > 0, "logs_environment_binding")
    return {"project_uuid": app["environment"]["project"]["uuid"], "server_uuid": SERVER,
            "environment_uuid": app["environment"]["uuid"], "destination_uuid": app["destination"]["uuid"],
            "environment_id": app["environment_id"]}


def validate(state):
    require(isinstance(state, dict) and state.get("phase") in SAFE_PHASES
            and state.get("app_uuid") == APP and SHA.fullmatch(state.get("revision", ""))
            and re.fullmatch(r"[a-f0-9]{64}", state.get("recipe_sha256", "")), "journal_binding")
    require(bool(re.fullmatch(r"[a-f0-9]{16}", state.get("transaction", "")))
            and bool(re.fullmatch(r"[a-f0-9]{64}", state.get("compose_sha256", ""))), "journal_binding")
    if state.get("job_uuid"):
        require(ID.fullmatch(state["job_uuid"]) and state["job_uuid"] not in PROTECTED, "job_identity")
    if "attempt_nonce" in state:
        require(isinstance(state["attempt_nonce"], str)
                and re.fullmatch(r"[a-f0-9]{16}", state["attempt_nonce"]), "journal_binding")
    if "runtime_container_removal_verified" in state:
        require(type(state["runtime_container_removal_verified"]) is bool, "journal_binding")


def bind_job(app, state):
    validate(state)
    require(app.get("uuid") == state["job_uuid"] and app.get("name") == state["job_name"]
            and app.get("description") == "Loki-only S3 adoption #1544; " + state["transaction"], "owned_job_binding")
    raw = app.get("docker_compose_raw")
    require(isinstance(raw, str) and hashlib.sha256(raw.encode()).hexdigest() == state["compose_sha256"], "owned_job_recipe")
    require(app["destination"]["server"]["uuid"] == SERVER
            and app.get("environment_id") == state["binding"]["environment_id"]
            and not app.get("connect_to_docker_network"), "owned_job_scope")
    children = app.get("applications")
    require(isinstance(children, list) and len(children) == 1 and not app.get("databases"), "owned_job_scope")
    child = children[0]
    require(child.get("name") == "maintenance" and ID.fullmatch(child.get("uuid", ""))
            and not child.get("fqdn") and not child.get("ports_mappings")
            and not child.get("is_public"), "owned_job_scope")
    require(type(child.get("service_id")) is int and child["service_id"] > 0
            and ("job_id" not in state or state["job_id"] == child["service_id"]), "job_numeric_identity")
    return child


def service_env_value(key, value, hidden=False):
    # Services persist only runtime environment references; no build args exist
    # in the reviewed Compose document or embedded Dockerfile.
    return {"key": key, "value": value, "is_literal": True, "is_shown_once": hidden}


def env_value(key, value, hidden=False):
    return {"key": key, "value": value, "is_runtime": True, "is_buildtime": False,
            "is_preview": False, "is_literal": True, "is_shown_once": hidden}


def create(api, revision):
    require(SHA.fullmatch(revision), "revision_shape")
    require(vault_read() is None, "journal_exists_use_status")
    current_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, timeout=5)
    require(current_sha.returncode == 0 and current_sha.stdout.strip() == revision, "revision_binding")
    scope = binding(api.call("applications/" + APP))
    stage = load("s3_stage", ROOT / "scripts/ops/stage-logs-s3-credentials.py")
    old, record = stage.environment(), stage.bao_read()
    stage.valid_version(record)
    require(record is not None and record["data"].get("state") == "staged_verified_read", "candidate_unverified")
    stage.validate_pending(record["data"], old)
    candidate = record["data"]
    payload = {"app_uuid": APP, "expected_old_access": old["S3_ACCESS_KEY"], "expected_old_secret": old["S3_SECRET_KEY"],
        "new_access": candidate["access_key"], "new_secret": candidate["secret_key"], "endpoint": candidate["endpoint"],
        "bucket": candidate["bucket"], "region": candidate["region"]}
    transaction, dockerfile = secrets.token_hex(8), recipe(revision)
    raw = compose_source(dockerfile, transaction)
    state = {"phase": "prepared", "app_uuid": APP, "transaction": transaction, "binding": scope,
        "revision": revision, "recipe_sha256": hashlib.sha256(dockerfile.encode()).hexdigest(),
        "compose_sha256": hashlib.sha256(raw.encode()).hexdigest(),
        "job_name": "loki-s3-adopt-" + transaction, "payload": payload, "pending_version": record["metadata"]["version"],
        "created_at_utc": datetime.now(timezone.utc).isoformat()}
    previous = save(state, None)  # Old-key rollback record must precede ANY deployment.
    return create_service_job(api, state, previous, raw)


def create_service_job(api, state, previous, raw):
    create_scope = {key: state["binding"][key] for key in ("project_uuid", "server_uuid", "environment_uuid", "destination_uuid")}
    result = api.call("services", "POST", {**create_scope,
        "docker_compose_raw": base64.b64encode(raw.encode()).decode(), "name": state["job_name"],
        "description": "Loki-only S3 adoption #1544; " + state["transaction"],
        "instant_deploy": False})
    job = result.get("uuid")
    require(isinstance(job, str) and ID.fullmatch(job) and job not in PROTECTED, "job_identity")
    state["job_uuid"] = job
    previous = save(state, previous)
    app = api.call("services/" + job)
    child = bind_job(app, state)
    state["job_id"], state["subapp_uuid"] = child["service_id"], child["uuid"]
    values = {"LOKI_S3_JOB_UUID": job, "LOKI_S3_JOB_ID": str(state["job_id"]), "LOKI_S3_MODE": "preflight",
              "LOKI_S3_NETWORK": APP, "LOKI_S3_PAYLOAD_JSON": json.dumps(state["payload"])}
    rows = api.call("services/" + job + "/envs")
    require(isinstance(rows, list) and len(rows) <= 64
            and all(sum(row.get("key") == key for row in rows) == 1 for key in values), "job_environment_shape")
    for key, value in values.items():
        api.call("services/" + job + "/envs", "PATCH", service_env_value(key, value, key == "LOKI_S3_PAYLOAD_JSON"))
    state["phase"] = "preflight_started"
    save(state, previous)  # Journal the ambiguous start before sending it.
    api.call("services/" + job + "/start", "POST")
    return summary(state)



def resume_create(api, previous):
    """Retry a rejected create only after proving the owned name is absent."""
    state = previous["data"]
    validate(state)
    require(state["phase"] == "prepared" and "job_uuid" not in state, "resume_create_phase")
    unchanged_candidate(api, state)
    rows = api.call("services")
    require(isinstance(rows, list) and len(rows) <= 512 and all(isinstance(row, dict) for row in rows), "recovery_inventory_bound")
    require(not any(row.get("name") == state["job_name"] for row in rows), "owned_job_name_exists_use_recover")
    dockerfile = recipe(state["revision"])
    raw = compose_source(dockerfile, state["transaction"])
    require(hashlib.sha256(dockerfile.encode()).hexdigest() == state["recipe_sha256"]
            and hashlib.sha256(raw.encode()).hexdigest() == state["compose_sha256"], "journal_recipe_changed")
    return create_service_job(api, state, previous, raw)


def unchanged_candidate(api, state):
    """Revalidate the original application, pending version and canonical old pair."""
    require(binding(api.call("applications/" + APP)) == state["binding"], "logs_binding_changed")
    stage = load("s3_stage", ROOT / "scripts/ops/stage-logs-s3-credentials.py")
    old, pending = stage.environment(), stage.bao_read()
    stage.valid_version(pending)
    require(pending is not None and pending["metadata"]["version"] == state["pending_version"]
            and pending["data"].get("state") == "staged_verified_read", "candidate_changed")
    stage.validate_pending(pending["data"], old)
    for field, source in (("access_key", "new_access"), ("secret_key", "new_secret"),
                          ("endpoint", "endpoint"), ("bucket", "bucket"), ("region", "region")):
        require(pending["data"].get(field) == state["payload"][source], "candidate_changed")
    require(old["S3_ACCESS_KEY"] == state["payload"]["expected_old_access"]
            and old["S3_SECRET_KEY"] == state["payload"]["expected_old_secret"], "configuration_changed")


def recovery_observation(state):
    """Require bounded, source-bound observed recovery; never infer old projection."""
    value = state.get("recovery_observation")
    fields = {"report", "revision", "recipe_sha256", "job_uuid", "recorded_at_utc", "task_uuid", "execution_uuid"}
    require(isinstance(value, dict) and set(value) == fields, "recovery_observation_required")
    require(all(value[key] == state[key] for key in ("revision", "recipe_sha256", "job_uuid")), "recovery_observation_binding")
    require(all(isinstance(value[key], str) and ID.fullmatch(value[key]) for key in ("task_uuid", "execution_uuid")),
            "recovery_observation_binding")
    stamp = value["recorded_at_utc"]
    require(isinstance(stamp, str) and len(stamp) <= 40, "recovery_observation_binding")
    try:
        require(datetime.fromisoformat(stamp).tzinfo is not None, "recovery_observation_binding")
    except ValueError:
        raise GuardError("recovery_observation_binding") from None
    diagnostic = load("s3_recovery_schema", ROOT / "scripts/ops/diagnose-loki-s3-host.py")
    report = diagnostic.closed_report(value["report"])
    required = {"status": "PASS", "phase": "complete", "job_binding": True,
                "old_pair_matches": True, "new_pair_matches": False, "runtime_running": True,
                "ready": True, "readiness_source": "docker_healthcheck", "canonical_source_attestation": "PASS",
                "canonical_source_file": SOURCE_OVERRIDES["source_file"],
                "original_projection_available": False, "limitation": "original_projection_unavailable"}
    require(report is not None and all(type(report.get(key)) is type(expected) and report.get(key) == expected
                                      for key, expected in required.items()), "recovery_not_verified")
    return copy.deepcopy(value)


def active_receipts(state, found):
    """A fresh image/state nonce prevents stale logs authorizing another attempt."""
    if "attempt_nonce" not in state:
        return found
    return [row for row in found if row.get("attempt_nonce") == state["attempt_nonce"]]


def reset_preflight(api, previous, revision):
    """Upgrade only the owned maintenance job after independently observed recovery.

    Persist the replacement plan before PATCH. A retry reconciles only its exact
    old/new raw Compose hashes, keeping the old latch, receipt and source archive.
    No invocation replays an adoption or changes the Loki runtime.
    """
    state = copy.deepcopy(previous["data"])
    validate(state)
    require(state["phase"] in {"adoption_started", "adoption_failed", "reset_prepared"}, "reset_phase")
    require(isinstance(revision, str) and SHA.fullmatch(revision), "revision_shape")
    require(revision != state["revision"], "replacement_source_required")
    current_sha = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True, text=True, timeout=5)
    require(current_sha.returncode == 0 and current_sha.stdout.strip() == revision, "revision_binding")
    observation = recovery_observation(state)
    history = state.get("attempt_archive", [])
    require(isinstance(history, list) and len(history) < 8, "attempt_archive_bound")
    unchanged_candidate(api, state)
    app = api.call("services/" + state["job_uuid"])
    dockerfile = recipe(revision)
    if state["phase"] != "reset_prepared":
        bind_job(app, state)
        found = active_receipts(state, receipts(api.call(
            "services/" + state["job_uuid"] + "/logs?sub_service_name=maintenance&lines=60")))
        require(found and found[-1].get("mode") == "adopt" and found[-1].get("status") == "FAIL",
                "failed_attempt_receipt_required")
        nonce = secrets.token_hex(8)
        used = {state.get("attempt_nonce", state["transaction"])}
        used.update(row.get("attempt_nonce") for row in history if isinstance(row, dict))
        require(nonce not in used, "fresh_attempt_required")
        raw = compose_source(dockerfile, nonce)
        archive = {"phase": state["phase"], "revision": state["revision"],
                   "recipe_sha256": state["recipe_sha256"], "compose_sha256": state["compose_sha256"],
                   "attempt_nonce": state.get("attempt_nonce", state["transaction"]),
                   "terminal_receipt": found[-1], "recovery_observation": observation,
                   "archived_at_utc": datetime.now(timezone.utc).isoformat()}
        # The original preflight attested this canonical file. Failed stdin
        # Compose rollback can replace generated config_files labels with '-'.
        # Keep strict helper path checks; carry only these fixed owned paths.
        state["payload"] = {**state["payload"], **SOURCE_OVERRIDES}
        state["reset_plan"] = {"revision": revision, "attempt_nonce": nonce, "payload": copy.deepcopy(state["payload"]),
                               "recipe_sha256": hashlib.sha256(dockerfile.encode()).hexdigest(),
                               "compose_sha256": hashlib.sha256(raw.encode()).hexdigest(), "previous_attempt": archive}
        state["phase"] = "reset_prepared"
        previous = save(state, previous)
    plan = state.get("reset_plan")
    fields = {"revision", "attempt_nonce", "recipe_sha256", "compose_sha256", "previous_attempt", "payload"}
    require(isinstance(plan, dict) and set(plan) == fields and plan["revision"] == revision,
            "reset_plan_binding")
    require(plan["payload"] == state["payload"]
            and all(state["payload"].get(key) == value for key, value in SOURCE_OVERRIDES.items()), "reset_plan_binding")
    raw = compose_source(dockerfile, plan["attempt_nonce"])
    require(plan["recipe_sha256"] == hashlib.sha256(dockerfile.encode()).hexdigest()
            and plan["compose_sha256"] == hashlib.sha256(raw.encode()).hexdigest(), "reset_plan_binding")
    expected_archive = plan["previous_attempt"]
    require(isinstance(expected_archive, dict)
            and all(expected_archive.get(key) == state[key] for key in ("revision", "recipe_sha256", "compose_sha256"))
            and expected_archive.get("recovery_observation") == observation
            and expected_archive.get("terminal_receipt", {}).get("mode") == "adopt"
            and expected_archive["terminal_receipt"].get("status") == "FAIL"
            and plan["attempt_nonce"] != state.get("attempt_nonce", state["transaction"]), "reset_plan_binding")
    require(all(isinstance(row, dict) and row.get("attempt_nonce") != plan["attempt_nonce"] for row in history),
            "reset_plan_binding")
    replacement = {**state, "revision": revision, "recipe_sha256": plan["recipe_sha256"],
                   "compose_sha256": plan["compose_sha256"], "attempt_nonce": plan["attempt_nonce"]}
    actual_hash = hashlib.sha256(app.get("docker_compose_raw", "").encode()).hexdigest()
    if actual_hash == state["compose_sha256"]:
        bind_job(app, state)
        api.call("services/" + state["job_uuid"], "PATCH", {"docker_compose_raw": base64.b64encode(raw.encode()).decode()})
    else:
        bind_job(app, replacement)  # An unknown hash/ownership never gets overwritten.
    bind_job(api.call("services/" + state["job_uuid"]), replacement)
    api.call("services/" + state["job_uuid"] + "/envs", "PATCH",
             service_env_value("LOKI_S3_PAYLOAD_JSON", json.dumps(replacement["payload"]), True))
    api.call("services/" + state["job_uuid"] + "/envs", "PATCH", service_env_value("LOKI_S3_MODE", "preflight"))
    replacement["attempt_archive"] = [*state.get("attempt_archive", []), copy.deepcopy(expected_archive)]
    replacement["phase"] = "preflight_started"
    for key in ("reset_plan", "recovery_observation", "adoption_receipt"):
        replacement.pop(key, None)
    save(replacement, previous)  # An ambiguous restart cannot re-enter reset-preflight.
    api.call("services/" + state["job_uuid"] + "/restart", "POST")
    return {**summary(replacement), "attempt_nonce": replacement["attempt_nonce"], "preflight_reset": True}

def receipts(raw):
    """Ignore all raw log text except complete closed worker JSON reports."""
    output = []
    if isinstance(raw, dict):
        lines = raw.get("logs", [])
    else:
        lines = raw
    # API shapes differ by Coolify version. Iterate bounded strings, never emit them.
    if isinstance(lines, str):
        texts = lines.splitlines()[-100:]
    elif isinstance(lines, list) and len(lines) <= 200:
        texts = [x if isinstance(x, str) else x.get("output", "") for x in lines if isinstance(x, (str, dict))]
    else:
        texts = []
    for line in texts:
        if not isinstance(line, str) or len(line) > 8192:
            continue
        start = line.find('{"')
        if start < 0:
            continue
        try:
            value = json.loads(line[start:])
        except ValueError:
            continue
        # Full field/type validation is shared with the reviewed worker module.
        worker = load("s3_worker_schema", ROOT / "scripts/ops/loki-s3-adoption-worker.py")
        checked = worker.closed_report(value)
        if checked is not None:
            output.append(checked)
    return output[-2:]


def summary(state):
    validate(state)
    result = {"status": "PASS", "phase": state["phase"], "old_key_revoked": False,
              "candidate_retained": True}
    for key in ("job_uuid", "subapp_uuid", "revision"):
        if key in state:
            require((SHA if key == "revision" else ID).fullmatch(state[key]), "summary_identity")
            result[key] = state[key]
    return result


def recover(api, previous):
    """Reconcile an ambiguous create without starting or recreating anything."""
    state = previous["data"]
    validate(state)
    require(state["phase"] == "prepared" and "job_uuid" not in state, "recovery_phase")
    rows = api.call("services")
    require(isinstance(rows, list) and len(rows) <= 512, "recovery_inventory_bound")
    matches = []
    for row in rows:
        if (isinstance(row, dict) and row.get("name") == state["job_name"]
                and row.get("description") == "Loki-only S3 adoption #1544; " + state["transaction"]
                and row.get("environment_id") == state["binding"]["environment_id"]
                and row.get("destination", {}).get("server", {}).get("uuid") == SERVER
                and isinstance(row.get("docker_compose_raw"), str)
                and hashlib.sha256(row["docker_compose_raw"].encode()).hexdigest() == state["compose_sha256"]):
            matches.append(row)
    require(matches, "job_recovery_not_found")
    require(len(matches) == 1, "job_recovery_ambiguous")
    job = matches[0].get("uuid")
    require(isinstance(job, str) and ID.fullmatch(job) and job not in PROTECTED, "job_identity")
    recovered = {**state, "job_uuid": job}
    child = bind_job(api.call("services/" + job), recovered)
    recovered.update(job_id=child["service_id"], subapp_uuid=child["uuid"])
    save(recovered, previous)
    return {**summary(recovered), "job_recovered": True, "deployment_started": False}


def status(api, previous):
    state = previous["data"]
    validate(state)
    require("job_uuid" in state, "create_outcome_requires_inventory")
    app = api.call("services/" + state["job_uuid"])
    bind_job(app, state)
    found = active_receipts(state, receipts(api.call(
        "services/" + state["job_uuid"] + "/logs?sub_service_name=maintenance&lines=60")))
    if state["phase"] == "adoption_started" and found:
        last = found[-1]
        if (last.get("mode") == "adopt" and last.get("status") == "FAIL"
                and (last.get("runtime_changed") is False or last.get("rollback_verified") is True)):
            state["phase"], state["adoption_receipt"] = "adoption_failed", last
            save(state, previous)
    result = summary(state)
    result["receipts"] = found
    return result


def adopt(api, previous):
    state = previous["data"]
    require(state["phase"] == "preflight_started", "adoption_phase")
    report = status(api, previous)
    require(any(
        x.get("status") == "PASS" and x.get("mode") == "preflight"
        for x in active_receipts(state, report["receipts"])), "preflight_not_passed")
    stage = load("s3_stage", ROOT / "scripts/ops/stage-logs-s3-credentials.py")
    pending = stage.bao_read()
    require(pending is not None and pending["metadata"]["version"] == state["pending_version"]
            and pending["data"].get("state") == "staged_verified_read"
            and pending["data"]["access_key"] == state["payload"]["new_access"]
            and pending["data"]["secret_key"] == state["payload"]["new_secret"], "candidate_changed")
    binding(api.call("applications/" + APP))
    old = stage.environment()
    require(old["S3_ACCESS_KEY"] == state["payload"]["expected_old_access"]
            and old["S3_SECRET_KEY"] == state["payload"]["expected_old_secret"], "configuration_changed")
    api.call("services/" + state["job_uuid"] + "/envs", "PATCH", service_env_value("LOKI_S3_MODE", "adopt"))
    state["phase"] = "adoption_started"
    previous = save(state, previous)  # Ambiguous deployment is never blindly retried.
    api.call("services/" + state["job_uuid"] + "/restart", "POST")
    return summary(state)


def finalize(api, previous):
    state = previous["data"]
    require(state["phase"] in {"adoption_started", "runtime_adopted", "config_persisted"}, "finalize_phase")
    report = status(api, previous)
    passed = [x for x in active_receipts(state, report["receipts"])
              if x.get("status") == "PASS" and x.get("mode") == "adopt"]
    require(passed, "adoption_not_passed")
    if state["phase"] == "adoption_started":
        state["phase"] = "runtime_adopted"
        state["adoption_receipt"] = passed[-1]
        previous = save(state, previous)
    binding(api.call("applications/" + APP))
    # Persist only these two fields; a configuration edit never redeploys the stack.
    stage = load("s3_stage", ROOT / "scripts/ops/stage-logs-s3-credentials.py")
    before = stage.environment()
    for key, source in (("S3_ACCESS_KEY", "new_access"), ("S3_SECRET_KEY", "new_secret")):
        require(before[key] in (state["payload"][source], state["payload"]["expected_old_access" if source == "new_access" else "expected_old_secret"]), "configuration_changed")
        api.call("applications/" + APP + "/envs", "PATCH", env_value(key, state["payload"][source]))
    after = stage.environment()
    require(after["S3_ACCESS_KEY"] == state["payload"]["new_access"]
            and after["S3_SECRET_KEY"] == state["payload"]["new_secret"], "configuration_readback_failed")
    require(all(after[key] == before[key] for key in ("S3_ENDPOINT", "S3_BUCKET", "S3_REGION")), "storage_binding_changed")
    state["phase"] = "config_persisted"
    save(state, previous)
    return {**summary(state), "canonical_configuration_verified": True, "stack_deployed": False}


def container_removed(api, state):
    """Prove Docker absence while its exact Service and child still exist.

    This known logs route uses Docker's all-container inventory, including
    stopped containers. Only its HTTP404 after a bound parent read is proof;
    the Service record's own HTTP404 never establishes physical removal.
    """
    base = "services/" + state["job_uuid"]
    deadline = time.monotonic() + 15
    for attempt in range(3):
        if time.monotonic() >= deadline:
            return False
        try:
            parent = api.call(base)
        except Exception:
            return False
        bind_job(parent, state)  # Ownership drift never authorizes deletion.
        if time.monotonic() >= deadline:
            return False
        try:
            api.call(base + "/logs?sub_service_name=maintenance&lines=1")
        except GuardError as error:
            if str(error) == "api_http_404":
                return True
            if str(error) != "api_http_400":
                return False
        except Exception:
            return False
        # HTTP200 or a stopped-but-not-removed HTTP400 remains unproven.
        if attempt < 2:
            time.sleep(max(0, min(1, deadline - time.monotonic())))
    return False


def cleanup(api, previous):
    state = previous["data"]
    validate(state)
    if state["phase"] == "adoption_started":
        status(api, previous)
        refreshed = vault_read()
        require(refreshed is not None and refreshed["data"].get("job_uuid") == state.get("job_uuid"), "journal_binding")
        previous, state = refreshed, refreshed["data"]
    if state["phase"] == "adoption_failed":
        terminal = state.get("adoption_receipt", {})
        require(terminal.get("mode") == "adopt" and terminal.get("status") == "FAIL"
                and (terminal.get("runtime_changed") is False or terminal.get("rollback_verified") is True), "ambiguous_adoption_requires_status")
    require("job_uuid" in state and state["phase"] != "adoption_started", "ambiguous_adoption_requires_status")
    job = state["job_uuid"]
    bind_job(api.call("services/" + job), state)
    # First erase the saved secret. Deletion never targets shared networks/volumes.
    rows = api.call("services/" + job + "/envs")
    require(isinstance(rows, list) and len(rows) <= 64, "job_environment_shape")
    for row in rows:
        if row.get("key") == "LOKI_S3_PAYLOAD_JSON":
            require(ID.fullmatch(row.get("uuid", "")), "job_environment_identity")
            api.call("services/" + job + "/envs/" + row["uuid"], "DELETE")
    require(not any(x.get("key") == "LOKI_S3_PAYLOAD_JSON" for x in api.call("services/" + job + "/envs")), "job_secret_removal_failed")
    stopped_error = None
    try:
        api.call("services/" + job + "/stop?docker_cleanup=false", "POST")
    except GuardError as error:
        require(str(error) == "api_http_400", "job_stop_failed")
        stopped_error = error
    removed = container_removed(api, state)
    if stopped_error is not None and not removed:
        raise stopped_error
    api.call("services/" + job + "?delete_volumes=false&delete_connected_networks=false&docker_cleanup=false", "DELETE")
    state.update(phase="cleaned", runtime_container_removal_verified=removed)
    save(state, previous)
    return {**summary(state), "saved_payload_removed": True, "job_deletion_queued": True,
            "shared_volumes_and_networks_protected": True, "runtime_container_removal_verified": removed}


def cleanup_status(api, previous):
    """Confirm API record deletion; never equate HTTP404 with Docker removal."""
    state = previous["data"]
    validate(state)
    require(state["phase"] == "cleaned" and "job_uuid" in state, "cleanup_phase")
    removed = False
    try:
        bind_job(api.call("services/" + state["job_uuid"]), state)
    except GuardError as error:
        require(str(error) == "api_http_404", "cleanup_status_failed")
        removed = True
    return {**summary(state), "job_record_removed": removed,
            "runtime_container_removal_verified": state.get("runtime_container_removal_verified", False)}


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("create", "resume-create", "recover", "reset-preflight", "status", "adopt", "finalize", "cleanup", "cleanup-status"))
    parser.add_argument("--revision")
    args = parser.parse_args()
    try:
        api = Api()
        if args.phase == "create":
            result = create(api, args.revision or "")
        else:
            previous = vault_read()
            require(previous is not None, "journal_missing")
            if args.phase == "reset-preflight":
                result = reset_preflight(api, previous, args.revision or "")
            else:
                result = globals()[args.phase.replace("-", "_")](api, previous)
        print(json.dumps(result, sort_keys=True))
        return 0
    except Exception as error:
        # Only locally raised fixed guards can enter a report. Everything else is opaque.
        reason = str(error) if isinstance(error, GuardError) else "operation_failed"
        print(json.dumps({"status": "FAIL", "phase": args.phase, "failure_reason": reason,
                          "old_key_revoked": False}, sort_keys=True))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
