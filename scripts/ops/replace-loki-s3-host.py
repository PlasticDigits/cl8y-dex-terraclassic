#!/usr/bin/env python3
"""Root-only, Loki-only S3 adoption. Secret input and subprocess output stay private.

This helper never calls Coolify deployment or revokes a key. Its caller must persist
replacement credentials in Coolify and verify actual Loki writes before revocation.
"""
import argparse
import copy
from decimal import Decimal
import hashlib
import json
import math
import os
from pathlib import Path
import re
import resource
import subprocess
import sys
import time
import urllib.request
import urllib.parse
import urllib.error
import uuid

APP = "iyi1s7ihy8icleonjdivpac9"
KEYS = ("S3_ACCESS_KEY", "S3_SECRET_KEY")
GENERATED_LABELS = frozenset(("com.docker.compose.config-hash", "com.docker.compose.project.config_files",
    "com.docker.compose.project.working_dir", "com.docker.compose.replace", "com.docker.compose.depends_on",
    "com.docker.compose.version"))
MAX_OUTPUT = 4 * 1024 * 1024
COMMAND_ENV = {"PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
               "HOME": "/root", "LC_ALL": "C", "DOCKER_HOST": "unix:///var/run/docker.sock"}
FAILURE_REASONS = frozenset(("root_required", "input_bound", "input_shape", "replacement_required",
    "command_failed", "output_bound", "loki_identity", "environment_shape", "source_identity",
    "compose_runtime_drift", "compose_shape", "unsupported_compose", "mount_identity",
    "unsupported_mount", "network_identity", "readiness_address", "readiness_failed",
    "storage_identity", "old_credential_mismatch", "configuration_changed", "image_identity",
    "runtime_binding_changed", "adoption_mismatch", "deadline_exhausted", "loki_http_failed",
    "loki_response_bound", "loki_response_shape", "loki_metrics_missing", "loki_flush_failed",
    "loki_flush_unconfirmed", "loki_marker_missing", "loki_history_missing"))
_deadline = None


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def command(args, body=None, timeout=5):
    if _deadline is not None:
        remaining = _deadline - time.monotonic()
        require(remaining > 0, "deadline_exhausted")
        timeout = min(timeout, remaining)
    # A maintenance container's inherited Coolify variables must never override
    # the production project's .env or redirect commands to another Docker host.
    result = subprocess.run(args, input=body, capture_output=True, text=True, timeout=timeout,
                            env=COMMAND_ENV)
    require(result.returncode == 0, "command_failed")
    require(len(result.stdout) <= MAX_OUTPUT and len(result.stderr) <= MAX_OUTPUT, "output_bound")
    return result.stdout


def payload_input(stream):
    body = stream.read(16385)
    require(len(body) <= 16384, "input_bound")
    data = json.loads(body)
    fields = {"app_uuid", "expected_old_access", "expected_old_secret", "new_access", "new_secret",
              "endpoint", "bucket", "region"}
    require(isinstance(data, dict) and set(data) <= fields | {"expected_config_sha", "source_file", "workdir"}
            and fields <= set(data) and data["app_uuid"] == APP, "input_shape")
    for key in fields - {"app_uuid"}:
        require(isinstance(data[key], str) and 0 < len(data[key]) <= 512 and "\x00" not in data[key]
                and "\n" not in data[key] and "\r" not in data[key], "input_shape")
    require((data["new_access"], data["new_secret"]) != (data["expected_old_access"], data["expected_old_secret"]), "replacement_required")
    return data


def envmap(container):
    result = {}
    for value in container["Config"].get("Env", []):
        key, separator, content = value.partition("=")
        require(separator and key not in result, "environment_shape")
        result[key] = content
    return result


def labels(container):
    return container["Config"].get("Labels") or {}


def owned(container):
    meta = labels(container)
    return (meta.get("com.docker.compose.project") == APP and meta.get("com.docker.compose.service") == "loki"
            and meta.get("coolify.managed") == "true"
            and meta.get("coolify.applicationUuid", APP) == APP
            and (meta.get("coolify.applicationUuid") == APP or
                 bool(re.fullmatch(r"[1-9][0-9]*", str(meta.get("coolify.applicationId", ""))))))


def discover():
    ids = command(["docker", "ps", "-aq", "--filter", "label=com.docker.compose.project=" + APP,
                   "--filter", "label=com.docker.compose.service=loki"]).split()
    require(len(ids) == 1 and re.fullmatch(r"[0-9a-f]{12,64}", ids[0]), "loki_identity")
    items = json.loads(command(["docker", "inspect", ids[0]]))
    require(len(items) == 1 and owned(items[0]) and items[0]["State"].get("Running") is True, "loki_identity")
    return items[0]


def sources(container, data):
    meta = labels(container)
    base = Path("/data/coolify/applications") / APP
    source = str(base / data["source_file"]) if "source_file" in data else meta.get("com.docker.compose.project.config_files", "")
    directory = data.get("workdir", meta.get("com.docker.compose.project.working_dir", ""))
    require(isinstance(source, str) and isinstance(directory, str), "source_identity")
    # Coolify's canonical application directory, not arbitrary root-readable input.
    require(Path(source).is_absolute() and Path(directory).is_absolute() and "," not in source
            and Path(source).parent == base and Path(source).name in ("compose.yaml", "docker-compose.yaml", "docker-compose.yml")
            and Path(source).resolve().parent == base and Path(directory) == base and Path(directory).resolve() == base
            and not any(parent.is_symlink() for parent in (*base.parents, base)),
            "source_identity")
    require(Path(source).is_file() and not Path(source).is_symlink(), "source_identity")
    return source, directory


def projection(container):
    config = copy.deepcopy(container["Config"])
    config.pop("Image", None)  # Compare immutable image ID instead of its mutable tag.
    hostname = config.pop("Hostname", None)
    if hostname and hostname not in (container["Id"][:12], container["Id"]):
        config["Hostname"] = hostname
    config["Env"] = {k: v for k, v in envmap(container).items() if k not in KEYS}
    config["Labels"] = {k: v for k, v in labels(container).items() if k not in GENERATED_LABELS}
    networks = {}
    for name, value in container["NetworkSettings"]["Networks"].items():
        networks[name] = {key: value.get(key) for key in ("IPAMConfig", "Links", "DriverOpts")}
        networks[name]["Aliases"] = sorted(alias for alias in (value.get("Aliases") or [])
                                           if alias not in (container["Id"], container["Id"][:12]))
    mounts = [{key: value.get(key) for key in ("Type", "Name", "Source", "Destination", "Driver", "Mode", "RW", "Propagation")}
              for value in container.get("Mounts", [])]
    return {"image": container["Image"], "config": config, "host": container["HostConfig"],
            "mounts": sorted(mounts, key=lambda row: row["Destination"]), "networks": networks,
            "name": container["Name"]}


def validate_service(container, service, image):
    """Catch canonical configuration drift before allowing any container mutation."""
    current = container["Config"]
    for source, actual in (("command", "Cmd"), ("entrypoint", "Entrypoint")):
        expected = service.get(source)
        if expected is None:
            expected = image.get(actual)
        require(expected == current.get(actual), "compose_runtime_drift")
    require(str(service.get("user", image.get("User", ""))) == current.get("User", ""), "compose_runtime_drift")
    combined_labels = {**(image.get("Labels") or {}), **(service.get("labels") or {})}
    require({k: v for k, v in combined_labels.items() if not k.startswith("com.docker.compose.")} ==
            {k: v for k, v in labels(container).items() if not k.startswith("com.docker.compose.")}, "compose_runtime_drift")
    host = container["HostConfig"]
    for field, actual in (("privileged", "Privileged"), ("read_only", "ReadonlyRootfs"), ("init", "Init")):
        require(bool(service.get(field, False)) == bool(host.get(actual, False)), "compose_runtime_drift")
    for field, actual in (("cap_add", "CapAdd"), ("cap_drop", "CapDrop"), ("security_opt", "SecurityOpt")):
        require(sorted(service.get(field) or []) == sorted(host.get(actual) or []), "compose_runtime_drift")
    limits = service.get("deploy", {}).get("resources", {}).get("limits", {})
    require(int(service.get("mem_limit", limits.get("memory", 0))) == host.get("Memory", 0), "compose_runtime_drift")
    require(int(Decimal(str(service.get("cpus", limits.get("cpus", 0)))) * 1000000000) == host.get("NanoCpus", 0), "compose_runtime_drift")
    restart = str(service.get("restart", "no")).split(":")
    require(restart[0] == host.get("RestartPolicy", {}).get("Name", "no") and
            (int(restart[1]) if len(restart) == 2 else 0) == host.get("RestartPolicy", {}).get("MaximumRetryCount", 0), "compose_runtime_drift")


def document(container, full, image):
    require(isinstance(full, dict) and isinstance(full.get("services"), dict) and "loki" in full["services"], "compose_shape")
    service = copy.deepcopy(full["services"]["loki"])
    require(not any(key in service for key in ("build", "configs", "secrets", "extends", "volumes_from", "network_mode", "develop", "provider",
                                              "post_start", "pre_stop", "use_api_socket", "devices", "device_cgroup_rules",
                                              "pid", "ipc", "uts", "userns_mode", "cgroup", "cgroup_parent")), "unsupported_compose")
    require(service.get("scale", 1) == 1 and service.get("deploy", {}).get("replicas", 1) == 1
            and service.get("deploy", {}).get("mode", "replicated") == "replicated", "unsupported_compose")
    validate_service(container, service, image)
    # Canonical .env can legitimately differ for unused inherited credentials.
    # Only the running Loki environment is authoritative for this transaction.
    service["environment"] = envmap(container)
    service["image"] = container["Image"]
    service["container_name"] = container["Name"].removeprefix("/")
    service.pop("depends_on", None)
    result = {"services": {"loki": service}, "volumes": {}, "networks": {}}
    mounts = {item["Destination"]: item for item in container.get("Mounts", [])}
    volume_targets = set()
    for number, volume in enumerate(service.get("volumes", [])):
        require(isinstance(volume, dict) and volume.get("target") in mounts, "mount_identity")
        mount = mounts[volume["target"]]
        volume_targets.add(volume["target"])
        require(volume.get("type") == mount["Type"], "mount_identity")
        if mount["Type"] == "volume":
            key = "existing_loki_" + str(number)
            require(mount.get("Name"), "mount_identity")
            result["volumes"][key] = {"external": True, "name": mount["Name"]}
            volume["source"] = key
        elif mount["Type"] == "bind":
            require(volume.get("source") == mount["Source"], "mount_identity")
        else:
            require(False, "unsupported_mount")
        require(bool(volume.get("read_only", False)) == (not mount["RW"]), "mount_identity")
    require(volume_targets == set(mounts), "mount_identity")
    networks = service.get("networks", {})
    require(isinstance(networks, dict) and networks, "network_identity")
    seen = set()
    for name in networks:
        config = full.get("networks", {}).get(name, {})
        actual_name = config.get("name")
        require(actual_name in container["NetworkSettings"]["Networks"], "network_identity")
        result["networks"][name] = {"external": True, "name": actual_name}
        seen.add(actual_name)
    require(seen == set(container["NetworkSettings"]["Networks"]), "network_identity")
    return result


def escaped(value):
    # Compose would interpolate dollar signs a second time when reading stdin.
    if isinstance(value, str):
        return value.replace("$", "$$")
    if isinstance(value, list):
        return [escaped(item) for item in value]
    if isinstance(value, dict):
        return {key: escaped(item) for key, item in value.items()}
    return value


def apply(compose, directory):
    body = json.dumps(escaped(compose))
    command(["docker", "compose", "--env-file", "/dev/null", "--project-name", APP,
             "--project-directory", directory, "--file", "-", "up", "-d", "--no-deps",
             "--force-recreate", "--pull", "never", "loki"], body, timeout=20)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


def loki_http(container, path, *, method="GET", body=None, deadline=None, bound=262144):
    """Internal, inspected-IP API only. Never returns bodies to an external caller."""
    require(path in ("/metrics", "/flush", "/loki/api/v1/push") or path.startswith("/loki/api/v1/query_range?"), "loki_response_shape")
    networks = container["NetworkSettings"]["Networks"]
    names = sorted(networks, key=lambda name: (name != APP, name))
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    for name in names[:2]:
        address = networks[name].get("IPAddress", "")
        require(bool(re.fullmatch(r"[0-9.]+", address)), "readiness_address")
        remaining = min(deadline or float("inf"), _deadline or float("inf")) - time.monotonic()
        require(remaining > 0, "deadline_exhausted")
        request = urllib.request.Request("http://" + address + ":3100" + path, method=method,
                    data=json.dumps(body).encode() if body is not None else (b"" if method == "POST" else None),
                    headers={"Content-Type": "application/json"})
        try:
            with opener.open(request, timeout=min(2, remaining)) as response:
                raw = response.read(bound + 1)
                require(len(raw) <= bound, "loki_response_bound")
                return response.status, raw
        except (urllib.error.URLError, TimeoutError, OSError):
            pass
    raise ValueError("loki_http_failed")


def query_sample(container, selector, start, end, *, deadline=None):
    query = urllib.parse.urlencode({"query": selector, "start": str(start), "end": str(end),
                                   "limit": "1", "direction": "forward"})
    status, raw = loki_http(container, "/loki/api/v1/query_range?" + query, deadline=deadline, bound=65536)
    require(status == 200, "loki_http_failed")
    result = json.loads(raw)
    require(isinstance(result, dict) and result.get("status") == "success" and isinstance(result.get("data"), dict), "loki_response_shape")
    data = result["data"]
    require(data.get("resultType") == "streams" and isinstance(data.get("result"), list) and len(data["result"]) <= 1, "loki_response_shape")
    if not data["result"]:
        return None
    row = data["result"][0]
    stream = row.get("stream")
    values = row.get("values")
    require(isinstance(stream, dict) and 0 < len(stream) <= 16 and isinstance(values, list) and len(values) == 1, "loki_response_shape")
    require(all(re.fullmatch(r"[a-zA-Z_][a-zA-Z0-9_]*", key) and isinstance(value, str) and len(value) <= 256
                for key, value in stream.items()), "loki_response_shape")
    value = values[0]
    require(isinstance(value, list) and len(value) >= 2 and isinstance(value[0], str) and value[0].isdigit()
            and start <= int(value[0]) < end and isinstance(value[1], str) and len(value[1]) <= 4096, "loki_response_shape")
    # Exact timestamp/stream lookup avoids copying a possibly sensitive existing
    # log line into a query URL or Loki's query logs.
    return {"stream": stream, "timestamp": int(value[0]), "line": value[1], "stats": data.get("stats", {})}


def selector_for(stream):
    return "{" + ",".join(key + "=" + json.dumps(value) for key, value in sorted(stream.items())) + "}"


def historical_canary(container):
    now = time.time_ns()
    deadline = min(time.monotonic() + 12, _deadline or float("inf"))
    for job in ("docker", "logfile", "stack-probe"):
        sample = query_sample(container, '{job="' + job + '"}', now - 4 * 3600 * 10**9,
                              now - 3600 * 10**9, deadline=deadline)
        if sample:
            # Loki 3.4.2 stats.proto: querier.store counters show historical
            # store involvement. They do not prove a fresh S3 GET past caches.
            store = sample["stats"].get("querier", {}).get("store", {})
            require(isinstance(store.get("totalChunksDownloaded"), int) and store["totalChunksDownloaded"] > 0,
                    "loki_history_missing")
            return sample
    raise ValueError("loki_history_missing")


def flush_counters(container, deadline):
    status, raw = loki_http(container, "/metrics", deadline=deadline, bound=4 * 1024 * 1024)
    require(status == 200, "loki_http_failed")
    metrics = raw.decode()
    names = ("loki_ingester_chunks_stored_total", "loki_ingester_chunks_flush_failures_total",
             "loki_ingester_chunks_flushed_total")
    counters = {}
    for name in names:
        rows = re.findall(r"(?m)^" + re.escape(name) + r"(?:\{[^\n]*\})?\s+([^\s]+)(?:\s+[^\n]+)?$", metrics)
        if name == "loki_ingester_chunks_flush_failures_total":
            # A plain Counter is eagerly initialized in Loki 3.4.2. Unlike the
            # tenant/reason CounterVecs, this sample must always exist once.
            require(len(rows) == 1, "loki_metrics_missing")
        # CounterVec families may be entirely absent before the first flush.
        # Both exact successful families must subsequently rise, so a renamed
        # or permanently absent vector can never establish successful storage.
        if rows:
            require(bool(re.search(r"(?m)^# TYPE " + re.escape(name) + r" counter$", metrics)), "loki_metrics_missing")
        numbers = [float(row) for row in rows]
        require(all(math.isfinite(value) and value >= 0 and value.is_integer() for value in numbers), "loki_response_shape")
        counters[name] = sum(int(value) for value in numbers)
    return counters


def storage_proof(container, canary):
    """Prove actual successful Loki store.Put plus distinct marker/read checks.

    Loki 3.4.2 FlushHandler is asynchronous: HTTP204 alone is insufficient.
    flush.go increments stored/flushed counters only after successful store.Put;
    failures increment a separate counter. A marker read may use the ingester,
    so this report deliberately does not claim exact-marker object durability.
    """
    deadline = min(time.monotonic() + 35, _deadline or float("inf"))
    baseline = flush_counters(container, deadline)
    stamp = time.time_ns()
    marker = "cl8y S3 credential adoption proof " + uuid.uuid4().hex
    stream = {"job": "cl8y-s3-rotation-proof"}
    status, _ = loki_http(container, "/loki/api/v1/push", method="POST", deadline=deadline,
                         body={"streams": [{"stream": stream, "values": [[str(stamp), marker]]}]})
    require(status == 204, "loki_http_failed")
    status, _ = loki_http(container, "/flush", method="POST", deadline=deadline)
    require(status == 204, "loki_http_failed")
    stored, failures, flushed = ("loki_ingester_chunks_stored_total", "loki_ingester_chunks_flush_failures_total",
                                "loki_ingester_chunks_flushed_total")
    after = baseline
    for attempt in range(6):
        after = flush_counters(container, deadline)
        require(all(after[key] >= baseline[key] for key in baseline), "loki_flush_unconfirmed")
        require(after[failures] == baseline[failures], "loki_flush_failed")
        if after[stored] > baseline[stored] and after[flushed] > baseline[flushed]:
            break
        require(attempt < 5, "loki_flush_unconfirmed")
        time.sleep(max(0, min(2, deadline - time.monotonic())))
    found = query_sample(container, selector_for(stream), stamp, stamp + 1, deadline=deadline)
    require(found is not None and found["timestamp"] == stamp and found["line"] == marker and found["stream"] == stream,
            "loki_marker_missing")
    historical = query_sample(container, selector_for(canary["stream"]), canary["timestamp"], canary["timestamp"] + 1,
                              deadline=deadline)
    require(historical is not None and historical["timestamp"] == canary["timestamp"]
            and historical["line"] == canary["line"] and historical["stream"] == canary["stream"], "loki_history_missing")
    # A final sample rejects errors that occurred while validating the reads.
    final = flush_counters(container, deadline)
    require(final[failures] == baseline[failures], "loki_flush_failed")
    return {"backing_storage_write_verified": True, "marker_readback_verified": True,
            "historical_readback_verified": True, "flush_success_delta": after[stored] - baseline[stored],
            "flush_failure_delta": 0}


def ready(container):
    networks = container["NetworkSettings"]["Networks"]
    addresses = [item.get("IPAddress") for item in networks.values() if item.get("IPAddress")]
    require(addresses, "readiness_address")
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    deadline = min(time.monotonic() + 25, _deadline or float("inf"))
    for _ in range(12):
        for address in addresses[:2]:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return False
            require(re.fullmatch(r"[0-9.]+", address), "readiness_address")
            try:
                with opener.open("http://" + address + ":3100/ready", timeout=min(2, remaining)) as response:
                    if response.status == 200 and response.read(128).strip() == b"ready":
                        return True
            except Exception:
                pass
        time.sleep(max(0, min(2, deadline - time.monotonic())))
    return False


def run(data, preflight=False):
    global _deadline
    overall_deadline = time.monotonic() + 180
    previous_deadline = _deadline
    # Reserve the full bounded replacement/readiness rollback budget.
    _deadline = overall_deadline - 70
    report = {"status": "FAIL", "phase": "preflight", "runtime_changed": False,
              "rollback_attempted": False, "rollback_verified": False, "ready": False}
    original = None
    directory = None
    immutable = None
    try:
        require(os.geteuid() == 0, "root_required")
        before = discover()
        old = envmap(before)
        require((old.get("S3_ENDPOINT"), old.get("S3_BUCKET"), old.get("S3_REGION", "nbg1")) ==
                (data["endpoint"], data["bucket"], data["region"]), "storage_identity")
        if (old.get(KEYS[0]), old.get(KEYS[1])) == (data["new_access"], data["new_secret"]):
            require(ready(before), "readiness_failed")
            report.update(phase="historical_preflight", ready=True)
            canary = historical_canary(before)
            if not preflight:
                report["phase"] = "storage_verification"
                report.update(storage_proof(before, canary))
            report.update(status="PASS", phase="preflight" if preflight else "already_adopted", ready=True, loki_count=1)
            return report
        require((old.get(KEYS[0]), old.get(KEYS[1])) == (data["expected_old_access"], data["expected_old_secret"]), "old_credential_mismatch")
        source, directory = sources(before, data)
        rendered = command(["docker", "compose", "--project-name", APP, "--project-directory", directory,
                            "--file", source, "config", "--format", "json"])
        digest = hashlib.sha256(rendered.encode()).hexdigest()
        require("expected_config_sha" not in data or data["expected_config_sha"] == digest, "configuration_changed")
        images = json.loads(command(["docker", "image", "inspect", before["Image"]]))
        require(len(images) == 1 and images[0]["Id"] == before["Image"], "image_identity")
        original = document(before, json.loads(rendered), images[0]["Config"])
        immutable = projection(before)
        replacement = copy.deepcopy(original)
        replacement["services"]["loki"]["environment"].update(dict(zip(KEYS, (data["new_access"], data["new_secret"]))))
        report["phase"] = "historical_preflight"
        canary = historical_canary(before)
        if preflight:
            report.update(status="PASS", phase="preflight", loki_count=1, source_file=Path(source).name)
            return report
        require(time.monotonic() + 90 <= _deadline, "deadline_exhausted")
        report.update(phase="replacement", runtime_changed=True)
        apply(replacement, directory)
        report["phase"] = "adoption_verification"
        after = discover()
        actual_projection = projection(after)
        if actual_projection != immutable:
            report["immutable_drift"] = sorted(key for key in immutable if actual_projection[key] != immutable[key])
            raise ValueError("runtime_binding_changed")
        require((envmap(after).get(KEYS[0]), envmap(after).get(KEYS[1])) == (data["new_access"], data["new_secret"]), "adoption_mismatch")
        require(ready(after), "readiness_failed")
        report.update(phase="storage_verification", ready=True)
        report.update(storage_proof(after, canary))
        report.update(status="PASS", phase="adopted", ready=True)
    except Exception as error:
        reason = str(error) if isinstance(error, ValueError) else "operation_failed"
        report["failure_reason"] = reason if reason in FAILURE_REASONS else "operation_failed"
        if report["runtime_changed"] and original is not None:
            _deadline = overall_deadline
            report["rollback_attempted"] = True
            try:
                apply(original, directory)
                restored = discover()
                require(projection(restored) == immutable, "rollback_binding_changed")
                require((envmap(restored).get(KEYS[0]), envmap(restored).get(KEYS[1])) ==
                        (data["expected_old_access"], data["expected_old_secret"]), "rollback_mismatch")
                require(ready(restored), "rollback_unready")
                report["rollback_verified"] = True
            except Exception:
                pass
    finally:
        _deadline = previous_deadline
    return report


def main():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--preflight", action="store_true")
    args = parser.parse_args()
    if os.geteuid() != 0:
        print(json.dumps({"status": "FAIL", "phase": "preflight", "runtime_changed": False,
                          "failure_reason": "root_required"}, sort_keys=True))
        return 1
    try:
        report = run(payload_input(sys.stdin), args.preflight)
    except Exception as error:
        reason = str(error) if isinstance(error, ValueError) else "input_shape"
        report = {"status": "FAIL", "phase": "input", "runtime_changed": False,
                  "failure_reason": reason if reason in FAILURE_REASONS else "input_shape"}
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
