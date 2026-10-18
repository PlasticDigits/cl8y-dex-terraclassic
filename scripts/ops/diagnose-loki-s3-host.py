#!/usr/bin/env python3
"""Read-only #1544 rollback observation inside its existing maintenance worker.

No container/network changes, payload export, raw errors or raw log output.
The original pre-replacement projection is unavailable; source attestation
and fresh Docker health observations do not reconstruct that lost snapshot.
"""
import contextlib
import datetime
import importlib.util
import hashlib
import io
import json
import os
from pathlib import Path
import re
import resource
import socket
import time

JOB = "9tksfavan8o2tcdjovzunumt"
SCHEMA = "loki_s3_diagnostic_v1"
REPRESENTATIONS = frozenset(("binds_only", "mounts_only", "both", "neither"))


class QuietOutput:
    def write(self, value):
        return len(value)

    def flush(self):
        pass


def load_helper():
    spec = importlib.util.spec_from_file_location("held_host_helper", "/ops/replace-loki-s3-host.py")
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper


def fresh_health(container, now=None):
    health = container.get("State", {}).get("Health", {})
    entries = health.get("Log") or []
    if health.get("Status") != "healthy" or not entries or len(entries) > 16:
        return False
    last = entries[-1]
    try:
        end = datetime.datetime.fromisoformat(last["End"].replace("Z", "+00:00"))
        age = ((now or datetime.datetime.now(datetime.timezone.utc)) - end).total_seconds()
        return end.tzinfo is not None and 0 <= age <= 30 and type(last.get("ExitCode")) is int and last["ExitCode"] == 0
    except Exception:
        return False


def structural_observation(container, canonical, image):
    service = canonical["services"]["loki"]
    config, host = container["Config"], container["HostConfig"]
    binds, mounts = host.get("Binds") or [], host.get("Mounts") or []
    representation = "both" if binds and mounts else "binds_only" if binds else "mounts_only" if mounts else "neither"
    by_target = {entry["target"]: entry for entry in service.get("volumes", [])}
    modes_equivalent = True
    for mount in container.get("Mounts", []):
        volume = by_target.get(mount.get("Destination"))
        readonly = bool(volume.get("read_only", False)) if volume else None
        modes_equivalent &= volume is not None and mount.get("RW") is (not readonly)
        modes_equivalent &= mount.get("Mode", "") in (("ro",) if readonly else ("", "rw"))
    return {"config_stop_timeout_present": "StopTimeout" in config,
            "image_stop_timeout_present": "StopTimeout" in image,
            "canonical_stop_grace_period_present": "stop_grace_period" in service,
            "stop_timeout_matches_image": config.get("StopTimeout") == image.get("StopTimeout"),
            "host_mount_representation": representation,
            "host_binds_count": len(binds), "host_mounts_count": len(mounts),
            "mount_mode_default_equivalence": bool(modes_equivalent)}


def closed_report(value):
    required = {"schema", "status", "phase", "job_binding", "old_pair_matches", "new_pair_matches",
                "runtime_running", "ready", "readiness_source", "canonical_source_attestation",
                "original_projection_available", "limitation"}
    boolean = {"job_binding", "old_pair_matches", "new_pair_matches", "runtime_running", "ready",
               "original_projection_available", "config_stop_timeout_present", "image_stop_timeout_present",
               "canonical_stop_grace_period_present", "stop_timeout_matches_image", "mount_mode_default_equivalence"}
    optional = boolean | {"host_mount_representation", "host_binds_count", "host_mounts_count"}
    try:
        if not isinstance(value, dict) or not required <= set(value) <= required | optional:
            return None
        if not (value["schema"] == SCHEMA and value["status"] in ("PASS", "FAIL")
                and value["phase"] in ("worker_binding", "runtime_observation", "source_attestation", "complete")
                and value["readiness_source"] == "docker_healthcheck"
                and value["canonical_source_attestation"] in ("PASS", "FAIL", "UNAVAILABLE")
                and value["original_projection_available"] is False
                and value["limitation"] == "original_projection_unavailable"):
            return None
        if any(type(value[key]) is not bool for key in boolean & set(value)):
            return None
        for key in ("host_binds_count", "host_mounts_count"):
            if key in value and (type(value[key]) is not int or not 0 <= value[key] <= 64):
                return None
        if "host_mount_representation" in value and value["host_mount_representation"] not in REPRESENTATIONS:
            return None
        if value["status"] == "PASS" and not (value["job_binding"] and value["phase"] == "complete"
                and value["old_pair_matches"] and not value["new_pair_matches"] and value["ready"]
                and value["runtime_running"] and value["canonical_source_attestation"] == "PASS"):
            return None
        return dict(value)
    except Exception:
        return None


def observe(helper, values):
    report = {"schema": SCHEMA, "status": "FAIL", "phase": "worker_binding", "job_binding": False,
              "old_pair_matches": False, "new_pair_matches": False, "runtime_running": False,
              "ready": False, "readiness_source": "docker_healthcheck", "canonical_source_attestation": "UNAVAILABLE",
              "original_projection_available": False, "limitation": "original_projection_unavailable"}
    previous_deadline = helper._deadline
    helper._deadline = time.monotonic() + 45
    try:
        helper.require(os.geteuid() == 0 and values.get("LOKI_S3_JOB_UUID") == JOB
                       and bool(re.fullmatch(r"[1-9][0-9]*", values.get("LOKI_S3_JOB_ID", ""))), "diagnostic_binding")
        data = helper.payload_input(io.StringIO(values.get("LOKI_S3_PAYLOAD_JSON", "")))
        hostname = socket.gethostname()
        helper.require(bool(re.fullmatch(r"[0-9a-f]{12,64}", hostname)), "diagnostic_binding")
        inspected = json.loads(helper.command(["docker", "inspect", hostname]))
        helper.require(isinstance(inspected, list) and len(inspected) == 1, "diagnostic_binding")
        own = inspected[0]
        labels = own["Config"].get("Labels") or {}
        env = helper.envmap(own)
        modern, legacy = labels.get("coolify.serviceUuid"), labels.get("coolify.serviceId")
        helper.require(own["Id"].startswith(hostname) and own["State"].get("Running") is True
                       and labels.get("coolify.managed") == "true"
                       and labels.get("com.docker.compose.project") == JOB
                       and labels.get("com.docker.compose.service") == "maintenance"
                       and (modern == JOB or str(legacy) == values["LOKI_S3_JOB_ID"])
                       and (modern is None or modern == JOB)
                       and (legacy is None or str(legacy) == values["LOKI_S3_JOB_ID"])
                       and all(env.get(key) == value for key, value in values.items()), "diagnostic_binding")
        report.update(job_binding=True, phase="runtime_observation")
        current = helper.discover()
        current_env = helper.envmap(current)
        pair = (current_env.get("S3_ACCESS_KEY"), current_env.get("S3_SECRET_KEY"))
        report.update(old_pair_matches=pair == (data["expected_old_access"], data["expected_old_secret"]),
                      new_pair_matches=pair == (data["new_access"], data["new_secret"]),
                      runtime_running=current["State"].get("Running") is True,
                      ready=fresh_health(current), phase="source_attestation")
        source, directory = helper.sources(current, data)
        rendered = helper.command(["docker", "compose", "--project-name", helper.APP,
            "--project-directory", directory, "--file", source, "config", "--format", "json"])
        helper.require("expected_config_sha" not in data or
                       hashlib.sha256(rendered.encode()).hexdigest() == data["expected_config_sha"], "diagnostic_source")
        canonical = json.loads(rendered)
        images = json.loads(helper.command(["docker", "image", "inspect", current["Image"]]))
        helper.require(len(images) == 1 and images[0]["Id"] == current["Image"], "diagnostic_image")
        report.update(structural_observation(current, canonical, images[0]["Config"]))
        report["canonical_source_attestation"] = "FAIL"
        helper.document(current, canonical, images[0]["Config"])
        report.update(canonical_source_attestation="PASS", phase="complete")
        if report["old_pair_matches"] and not report["new_pair_matches"] and report["ready"] and report["runtime_running"]:
            report["status"] = "PASS"
    except Exception:
        pass
    finally:
        helper._deadline = previous_deadline
    return report


def main():
    report = None
    try:
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        os.umask(0o077)
        values = {key: os.environ.get(key, "") for key in ("LOKI_S3_JOB_UUID", "LOKI_S3_JOB_ID", "LOKI_S3_PAYLOAD_JSON")}
        os.environ.clear()
        helper = load_helper()
        os.environ.update(helper.COMMAND_ENV)
        with contextlib.redirect_stdout(QuietOutput()), contextlib.redirect_stderr(QuietOutput()):
            report = observe(helper, values)
    except Exception:
        pass
    if closed_report(report) is None:
        report = {"schema": SCHEMA, "status": "FAIL", "phase": "worker_binding", "job_binding": False,
                  "old_pair_matches": False, "new_pair_matches": False, "runtime_running": False, "ready": False,
                  "readiness_source": "docker_healthcheck", "canonical_source_attestation": "UNAVAILABLE",
                  "original_projection_available": False, "limitation": "original_projection_unavailable"}
    print(json.dumps(report, sort_keys=True), flush=True)
    return 0  # The platform stores the fixed diagnostic result even when checks fail.


if __name__ == "__main__":
    raise SystemExit(main())
