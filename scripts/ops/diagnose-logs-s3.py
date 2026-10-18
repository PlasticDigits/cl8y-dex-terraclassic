#!/usr/bin/env python3
"""Run one disabled, read-only diagnostic in the already-owned maintenance job.

No production container is restarted and no secret enters the command or logs.
The fixed diagnostic source is read from the selected committed Git revision.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PATH = "scripts/ops/diagnose-loki-s3-host.py"


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def source_command(m, revision):
    m.require(isinstance(revision, str) and m.SHA.fullmatch(revision), "diagnostic_revision")
    result = subprocess.run(["git", "show", revision + ":" + PATH], cwd=ROOT,
                            capture_output=True, timeout=5)
    m.require(result.returncode == 0 and result.stdout == (ROOT / PATH).read_bytes(), "diagnostic_source_binding")
    # Installed Coolify stores commands in varchar(255). Fetch public source at
    # the complete immutable Git SHA, then verify its digest BEFORE execution.
    # Only public code is written in the owned container's /ops working directory.
    url = ("https://git.cl8y.com/code/cl8y-dex-terraclassic/raw/commit/" + revision
           + "/scripts/ops/diagnose-loki-s3-host.py")
    digest = hashlib.sha256(result.stdout).hexdigest()
    command = "wget -T10 -qO d " + url + "&&echo " + digest + " d|sha256sum -c&&python3 d"
    m.require(len(command) <= 255, "diagnostic_command_bound")
    return command


def bind(m, api, state):
    m.validate(state)
    m.require(state.get("job_uuid") == "9tksfavan8o2tcdjovzunumt", "diagnostic_job_binding")
    m.bind_job(api.call("services/" + state["job_uuid"]), state)
    return "services/" + state["job_uuid"] + "/scheduled-tasks"


def begin(m, api, previous, revision):
    state = previous["data"]
    prefix = bind(m, api, state)
    if state.get("diagnostic"):
        prior = state["diagnostic"]
        if prior.get("stage") == "removed":
            history = state.setdefault("diagnostic_history", [])
            m.require(isinstance(history, list) and len(history) < 8, "diagnostic_history_bound")
            history.append({"diagnostic": prior, "observation": state.pop("recovery_observation", None)})
            state.pop("diagnostic")
            prior = None
        if prior is not None:
            m.require(prior.get("stage") == "prepared" and not prior.get("task_uuid")
                      and prior.get("source_revision") == revision, "diagnostic_already_prepared")
            tasks = api.call(prefix)
            m.require(isinstance(tasks, list) and len(tasks) <= 64
                      and not any(row.get("name") == "loki-s3-read-only-recovery-1544" for row in tasks),
                      "diagnostic_create_outcome_unknown")
    report = m.status(api, previous)
    m.require(any(row.get("mode") == "adopt" and row.get("status") == "FAIL" for row in report["receipts"]), "failed_attempt_required")
    command = source_command(m, revision)
    task = {"name": "loki-s3-read-only-recovery-1544", "command": command,
            "frequency": "0 0 1 1 *", "container": "maintenance", "timeout": 55, "enabled": False}
    state["diagnostic"] = {"source_revision": revision, "command_sha256": hashlib.sha256(command.encode()).hexdigest(),
                           "stage": "prepared"}
    previous = m.save(state, previous)
    response = api.call(prefix, "POST", task)
    identity = response.get("uuid")
    m.require(isinstance(identity, str) and m.ID.fullmatch(identity), "diagnostic_task_identity")
    state["diagnostic"].update(task_uuid=identity, stage="created")
    previous = m.save(state, previous)
    m.require(response.get("enabled") is False and response.get("container") == "maintenance"
              and response.get("command") == command, "diagnostic_task_binding")
    state["diagnostic"].update(task_uuid=identity, stage="execution_requested")
    m.save(state, previous)  # An ambiguous manual execution is never blindly repeated.
    api.call(prefix + "/" + identity + "/execute", "POST")
    return {"status": "PASS", "phase": "diagnostic_requested", "task_uuid": identity,
            "automatic_schedule_enabled": False, "runtime_changed": False}


def status(m, api, previous):
    state = previous["data"]
    prefix = bind(m, api, state)
    diagnostic = state.get("diagnostic", {})
    identity = diagnostic.get("task_uuid", "")
    m.require(isinstance(identity, str) and m.ID.fullmatch(identity), "diagnostic_task_identity")
    tasks = api.call(prefix)
    m.require(isinstance(tasks, list) and len(tasks) <= 64, "diagnostic_task_bound")
    matches = [row for row in tasks if row.get("uuid") == identity]
    m.require(len(matches) == 1 and matches[0].get("enabled") is False
              and matches[0].get("container") == "maintenance" and isinstance(matches[0].get("command"), str)
              and hashlib.sha256(matches[0]["command"].encode()).hexdigest() == diagnostic["command_sha256"], "diagnostic_task_binding")
    executions = api.call(prefix + "/" + identity + "/executions")
    m.require(isinstance(executions, list) and len(executions) <= 8, "diagnostic_execution_bound")
    schema = load("s3_diagnostic_schema", ROOT / PATH)
    results = []
    for execution in executions:
        raw = execution.get("message")
        if not isinstance(raw, str) or len(raw) > 16384 or execution.get("status") != "success":
            continue
        for line in raw.splitlines()[-4:]:
            try:
                report = schema.closed_report(json.loads(line))
            except ValueError:
                continue
            if report is not None:
                execution_id = execution.get("uuid", "")
                m.require(isinstance(execution_id, str) and m.ID.fullmatch(execution_id), "diagnostic_execution_identity")
                results.append((execution_id, report))
    m.require(len(results) <= 1, "diagnostic_duplicate_execution")
    if not results:
        return {"status": "PENDING", "phase": "diagnostic_observation", "runtime_changed": False}
    execution_id, report = results[0]
    state["recovery_observation"] = {"report": report, "revision": state["revision"],
        "recipe_sha256": state["recipe_sha256"], "job_uuid": state["job_uuid"],
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(), "task_uuid": identity, "execution_uuid": execution_id}
    state["diagnostic"]["stage"] = "observed"
    m.save(state, previous)
    return {"status": report["status"], "phase": "diagnostic_observation", "report": report,
            "runtime_changed": False}


def cleanup(m, api, previous):
    state = previous["data"]
    prefix = bind(m, api, state)
    m.require(state.get("diagnostic", {}).get("stage") == "observed", "diagnostic_not_observed")
    identity = state["diagnostic"]["task_uuid"]
    m.require(m.ID.fullmatch(identity), "diagnostic_task_identity")
    api.call(prefix + "/" + identity, "DELETE")
    rows = api.call(prefix)
    m.require(isinstance(rows, list) and len(rows) <= 64
              and not any(row.get("uuid") == identity for row in rows), "diagnostic_cleanup_readback")
    state["diagnostic"]["stage"] = "removed"
    m.save(state, previous)
    return {"status": "PASS", "phase": "diagnostic_removed", "runtime_changed": False}


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("begin", "status", "cleanup"))
    parser.add_argument("--revision")
    args = parser.parse_args()
    m = load("adoption", Path(__file__).with_name("adopt-logs-s3.py"))
    try:
        api, previous = m.Api(), m.vault_read()
        m.require(previous is not None, "journal_missing")
        result = begin(m, api, previous, args.revision) if args.phase == "begin" else globals()[args.phase](m, api, previous)
        print(json.dumps(result, sort_keys=True))
        return 0 if result["status"] in ("PASS", "PENDING") else 1
    except Exception as error:
        print(json.dumps({"status": "FAIL", "phase": args.phase,
            "failure_reason": str(error) if isinstance(error, m.GuardError) else "diagnostic_operation_failed",
            "runtime_changed": False}, sort_keys=True))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
