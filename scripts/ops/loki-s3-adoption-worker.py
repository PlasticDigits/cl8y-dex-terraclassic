#!/usr/bin/env python3
"""One-shot, Loki-only maintenance worker; secret payload never becomes output."""
import importlib.util
import contextlib
import io
import json
import os
from pathlib import Path
import re
import resource
import socket
import stat
import subprocess
import time

APP = "iyi1s7ihy8icleonjdivpac9"
APPDIR = "/data/coolify/applications/" + APP
COMMAND_ENV = {"PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
               "HOME": "/root", "LC_ALL": "C", "DOCKER_HOST": "unix:///var/run/docker.sock"}
WORKER_REASONS = frozenset(("worker_input", "worker_identity", "worker_mounts", "worker_isolation",
    "worker_state_shared", "worker_network", "worker_deadline", "worker_command", "worker_report",
    "worker_latch_exists", "worker_operation_failed", "worker_state"))
PHASES = frozenset(("worker_preflight", "worker_transaction", "worker_setup", "preflight", "already_adopted",
    "replacement", "adoption_verification", "adopted", "input", "storage_verification", "historical_preflight"))
BOOL_FIELDS = frozenset(("runtime_changed", "network_checked", "network_detached", "adoption_latched",
    "rollback_attempted", "rollback_verified", "ready", "backing_storage_write_verified",
    "marker_readback_verified", "historical_readback_verified"))


def load_helper():
    spec = importlib.util.spec_from_file_location("host_adoption", Path(__file__).with_name("replace-loki-s3-host.py"))
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)
    return helper


def closed_report(value):
    """Validate tool/log receipts without ever reflecting arbitrary log content."""
    try:
        require(isinstance(value, dict), "worker_report")
        required = {"schema", "mode", "status", "phase", "runtime_changed", "network_checked",
                    "network_detached", "adoption_latched"}
        allowed = required | BOOL_FIELDS | {"failure_reason", "source_file", "loki_count", "immutable_drift",
                                            "flush_success_delta", "flush_failure_delta"}
        require(required <= set(value) <= allowed and value["schema"] == "loki_s3_adoption_v1"
                and value["mode"] in ("preflight", "adopt") and value["status"] in ("PASS", "FAIL")
                and value["phase"] in PHASES, "worker_report")
        require(all(type(value[k]) is bool for k in BOOL_FIELDS & set(value)), "worker_report")
        if "failure_reason" in value:
            require(value["failure_reason"] in WORKER_REASONS | load_helper().FAILURE_REASONS | {"operation_failed"}, "worker_report")
        if "source_file" in value:
            require(value["source_file"] in ("compose.yaml", "docker-compose.yaml", "docker-compose.yml"), "worker_report")
        if "loki_count" in value:
            require(type(value["loki_count"]) is int and value["loki_count"] == 1, "worker_report")
        for key in ("flush_success_delta", "flush_failure_delta"):
            if key in value:
                require(type(value[key]) is int and 0 <= value[key] <= 1_000_000_000, "worker_report")
        if "immutable_drift" in value:
            require(isinstance(value["immutable_drift"], list) and len(value["immutable_drift"]) <= 6
                    and set(value["immutable_drift"]) <= {"image", "config", "host", "mounts", "networks", "name"}, "worker_report")
        if value["status"] == "PASS":
            require(value["network_checked"] and value["network_detached"] and "failure_reason" not in value,
                    "worker_report")
            if value["mode"] == "preflight":
                require(value["phase"] == "preflight" and value["runtime_changed"] is False
                        and value.get("loki_count") == 1 and value["adoption_latched"] is False, "worker_report")
            else:
                require(value["phase"] in ("adopted", "already_adopted") and value["adoption_latched"]
                        and value.get("ready") is True and value.get("backing_storage_write_verified") is True
                        and value.get("marker_readback_verified") is True and value.get("historical_readback_verified") is True
                        and value.get("flush_success_delta", 0) >= 1 and value.get("flush_failure_delta") == 0, "worker_report")
        return dict(value)
    except Exception:
        return None


def initial_report(mode):
    return {"schema": "loki_s3_adoption_v1", "mode": mode, "status": "FAIL", "phase": "worker_preflight",
            "runtime_changed": False, "network_checked": False, "network_detached": True,
            "adoption_latched": False, "rollback_attempted": False, "rollback_verified": False, "ready": False}


class QuietOutput:
    """Discard unexpected helper diagnostics instead of buffering secret text."""
    def write(self, value):
        return len(value)

    def flush(self):
        pass


def private_call(function, *args, **kwargs):
    with contextlib.redirect_stdout(QuietOutput()), contextlib.redirect_stderr(QuietOutput()):
        return function(*args, **kwargs)


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def command(args, deadline):
    remaining = deadline - time.monotonic()
    require(remaining > 0, "worker_deadline")
    result = subprocess.run(["docker", *args], capture_output=True, text=True,
                            timeout=min(8, remaining), env=COMMAND_ENV)
    require(result.returncode == 0 and len(result.stdout) <= 4 * 1024 * 1024
            and len(result.stderr) <= 4 * 1024 * 1024, "worker_command")
    return result.stdout


def inputs():
    # Clear every inherited variable before Docker or helper subprocesses run.
    values = {key: os.environ.get(key, "") for key in
              ("LOKI_S3_PAYLOAD_JSON", "LOKI_S3_MODE", "LOKI_S3_JOB_UUID", "LOKI_S3_JOB_ID", "LOKI_S3_NETWORK")}
    inherited = set(os.environ)
    os.environ.clear()
    os.environ.update(COMMAND_ENV)
    require(not any(k.startswith(("BAO_", "VAULT_")) or
                    (k.startswith("COOLIFY_") and any(s in k for s in ("TOKEN", "PASSWORD", "SECRET")))
                    for k in inherited), "worker_input")
    require(values["LOKI_S3_MODE"] in ("preflight", "adopt")
            and re.fullmatch(r"[a-z0-9]{8,64}", values["LOKI_S3_JOB_UUID"])
            and values["LOKI_S3_JOB_UUID"] != APP
            and re.fullmatch(r"[1-9][0-9]*", values["LOKI_S3_JOB_ID"])
            and values["LOKI_S3_NETWORK"] == APP
            and 0 < len(values["LOKI_S3_PAYLOAD_JSON"]) <= 16384, "worker_input")
    return values


def self_identity(container, values, hostname):
    identity = container.get("Id", "")
    config = container.get("Config", {})
    labels = config.get("Labels") or {}
    rows = config.get("Env") or []
    env = {}
    for row in rows:
        key, separator, value = row.partition("=")
        require(separator and key not in env, "worker_identity")
        env[key] = value
    modern = labels.get("coolify.serviceUuid")
    legacy = labels.get("coolify.serviceId")
    uuid, numeric = values["LOKI_S3_JOB_UUID"], values["LOKI_S3_JOB_ID"]
    require(re.fullmatch(r"[0-9a-f]{64}", identity) and identity.startswith(hostname)
            and container.get("State", {}).get("Running") is True
            and labels.get("coolify.managed") == "true"
            and labels.get("com.docker.compose.project") == uuid
            and labels.get("com.docker.compose.service") == "maintenance"
            and labels.get("coolify.type", "service") == "service"
            and labels.get("coolify.service.subName", "maintenance") == "maintenance"
            and (modern == uuid or str(legacy) == numeric)
            and (modern is None or modern == uuid)
            and (legacy is None or str(legacy) == numeric)
            and not labels.get("coolify.applicationUuid") and not labels.get("coolify.applicationId")
            and all(env.get(key) == value for key, value in values.items()), "worker_identity")
    return identity


def isolation(container):
    host = container.get("HostConfig", {})
    require(host.get("RestartPolicy", {}).get("Name") == "no"
            and not host.get("RestartPolicy", {}).get("MaximumRetryCount")
            and not host.get("Privileged") and not host.get("CapAdd")
            and sorted(host.get("CapDrop") or []) == ["ALL"]
            and not host.get("Devices") and not host.get("DeviceRequests")
            and not host.get("VolumesFrom") and not host.get("PortBindings")
            and host.get("NetworkMode") not in ("host", "container")
            and not str(host.get("NetworkMode", "")).startswith("container:")
            and host.get("PidMode", "") != "host" and host.get("IpcMode", "") != "host"
            and not host.get("PublishAllPorts"), "worker_isolation")
    mounts = container.get("Mounts", [])
    require(len(mounts) == 3 and len({m.get("Destination") for m in mounts}) == 3, "worker_mounts")
    by_target = {m.get("Destination"): m for m in mounts}
    require(set(by_target) == {"/var/run/docker.sock", APPDIR, "/state"}, "worker_mounts")
    sock, source, state = (by_target[k] for k in ("/var/run/docker.sock", APPDIR, "/state"))
    require(sock.get("Type") == "bind" and sock.get("Source") == "/var/run/docker.sock" and sock.get("RW") is True
            and source.get("Type") == "bind" and source.get("Source") == APPDIR and source.get("RW") is False
            and state.get("Type") == "volume" and state.get("RW") is True
            and re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}", state.get("Name", "")), "worker_mounts")
    return state["Name"]


def latch(state_dir):
    require(state_dir.is_dir() and not state_dir.is_symlink(), "worker_state")
    try:
        descriptor = os.open(state_dir / "adoption-started", os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600)
    except FileExistsError:
        raise ValueError("worker_latch_exists") from None
    with os.fdopen(descriptor, "w") as stream:
        stream.write("one-shot adoption started\n")
        stream.flush()
        os.fsync(stream.fileno())
    directory = os.open(state_dir, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def local_authority():
    require(os.geteuid() == 0 and stat.S_ISSOCK(os.stat("/var/run/docker.sock").st_mode), "worker_isolation")


def sanitized_helper(report, helper):
    require(isinstance(report, dict), "worker_report")
    required = {"status", "phase", "runtime_changed", "rollback_attempted", "rollback_verified", "ready"}
    require(required <= set(report) and report["status"] in ("PASS", "FAIL")
            and report["phase"] in ("preflight", "already_adopted", "replacement", "adoption_verification", "adopted", "input",
                                    "storage_verification", "historical_preflight")
            and all(type(report[k]) is bool for k in required - {"status", "phase"}), "worker_report")
    result = {key: report[key] for key in required}
    if "failure_reason" in report:
        require(report["failure_reason"] in helper.FAILURE_REASONS | {"operation_failed"}, "worker_report")
        result["failure_reason"] = report["failure_reason"]
    if "source_file" in report:
        require(report["source_file"] in ("compose.yaml", "docker-compose.yaml", "docker-compose.yml"), "worker_report")
        result["source_file"] = report["source_file"]
    if "loki_count" in report:
        require(type(report["loki_count"]) is int and report["loki_count"] == 1, "worker_report")
        result["loki_count"] = 1
    if "immutable_drift" in report:
        require(isinstance(report["immutable_drift"], list) and set(report["immutable_drift"]) <=
                {"image", "config", "host", "mounts", "networks", "name"}, "worker_report")
        result["immutable_drift"] = sorted(set(report["immutable_drift"]))
    for key in ("backing_storage_write_verified", "marker_readback_verified", "historical_readback_verified"):
        if key in report:
            require(type(report[key]) is bool, "worker_report")
            result[key] = report[key]
    for key in ("flush_success_delta", "flush_failure_delta"):
        if key in report:
            require(type(report[key]) is int and 0 <= report[key] <= 1_000_000_000, "worker_report")
            result[key] = report[key]
    return result


def execute(helper, values, *, state_dir=Path("/state")):
    started = time.monotonic()
    deadline = started + 25
    report = initial_report(values["LOKI_S3_MODE"])
    joined, identity = False, None
    try:
        local_authority()
        data = helper.payload_input(io.StringIO(values["LOKI_S3_PAYLOAD_JSON"]))
        hostname = socket.gethostname()
        require(re.fullmatch(r"[0-9a-f]{12,64}", hostname), "worker_identity")
        own = json.loads(command(["inspect", hostname], deadline))
        require(isinstance(own, list) and len(own) == 1, "worker_identity")
        identity = self_identity(own[0], values, hostname)
        volume = isolation(own[0])
        users = command(["ps", "-aq", "--no-trunc", "--filter", "volume=" + volume], deadline).split()
        require(users == [identity], "worker_state_shared")
        before = private_call(helper.discover)
        require(values["LOKI_S3_NETWORK"] in before["NetworkSettings"]["Networks"], "worker_network")
        report["network_checked"] = True
        own_networks = own[0].get("NetworkSettings", {}).get("Networks", {})
        report["network_detached"] = values["LOKI_S3_NETWORK"] not in own_networks
        require(report["network_detached"], "worker_network")
        # Exact own ID and existing network only; never reconnect production containers.
        joined = True
        report["network_detached"] = False
        command(["network", "connect", values["LOKI_S3_NETWORK"], identity], deadline)
        require(time.monotonic() < deadline, "worker_deadline")
        if values["LOKI_S3_MODE"] == "adopt":
            latch(state_dir)
            report["adoption_latched"] = True
            # If the helper throws unexpectedly after mutation, conservatively
            # report possible runtime change rather than claim no transaction.
            report["runtime_changed"] = True
        report["phase"] = "worker_transaction"
        outcome = private_call(helper.run, data, preflight=values["LOKI_S3_MODE"] == "preflight")
        report.update(sanitized_helper(outcome, helper))
    except Exception as error:
        reason = str(error) if isinstance(error, ValueError) and str(error) in WORKER_REASONS else "worker_operation_failed"
        report.update(status="FAIL", failure_reason=reason)
    finally:
        if joined and identity:
            try:
                command(["network", "disconnect", values["LOKI_S3_NETWORK"], identity], started + 218)
                report["network_detached"] = True
            except Exception:
                report.update(status="FAIL", failure_reason="worker_network")
    if closed_report(report) is None:
        report.update(status="FAIL", failure_reason="worker_report")
    return report


def persist(report, state_dir=Path("/state")):
    require(closed_report(report) is not None, "worker_report")
    # A refused replay must not erase the original terminal adoption evidence.
    if (state_dir / "adoption-started").exists() and (state_dir / "report.json").exists():
        descriptor = os.open(state_dir / "report.json", os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(descriptor) as previous:
            body = previous.read(16385)
        require(len(body) <= 16384, "worker_report")
        original = closed_report(json.loads(body))
        require(original is not None, "worker_report")
        if original["mode"] == "adopt" and original["adoption_latched"]:
            return
    descriptor = os.open(state_dir / "report.tmp", os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as stream:
        os.fchmod(stream.fileno(), 0o600)
        json.dump(report, stream, sort_keys=True)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(state_dir / "report.tmp", state_dir / "report.json")
    directory = os.open(state_dir, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.umask(0o077)
    report = None
    try:
        values = inputs()
        helper = load_helper()
        report = execute(helper, values)
    except Exception:
        report = initial_report("preflight")
        report.update(phase="worker_setup", failure_reason="worker_operation_failed")
    try:
        persist(report)
    except Exception:
        report.update(status="FAIL", failure_reason="worker_state")
    print(json.dumps(report, sort_keys=True), flush=True)
    # No more Docker calls, writes or transactions. The owner collects the receipt
    # and explicitly deletes this one-shot application.
    while True:
        time.sleep(3600)


if __name__ == "__main__":
    main()
