#!/usr/bin/env python3
"""Root-only, Loki-only S3 adoption. Secret input and subprocess output stay private.

This helper never calls Coolify deployment or revokes a key. Its caller must persist
replacement credentials in Coolify and verify actual Loki writes before revocation.
"""
import argparse
import copy
from decimal import Decimal
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import re
import resource
import socket
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
    "loki_flush_unconfirmed", "loki_marker_missing", "loki_history_missing", "engine_failed",
    "engine_response", "engine_identity", "engine_network_unsupported", "engine_mount_unsupported",
    "engine_rollback_unsafe"))
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


class UnixConnection(http.client.HTTPConnection):
    """Docker Engine HTTP over the sole authorized local Unix socket."""
    def __init__(self, timeout):
        super().__init__("localhost", timeout=timeout)

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect("/var/run/docker.sock")


class Engine:
    def __init__(self):
        self.version = ""
        version = self.call("GET", "/version")
        value = version.get("ApiVersion", "")
        require(bool(re.fullmatch(r"1\.[0-9]{2,3}", value)) and int(value.split(".")[1]) >= 44, "engine_response")
        self.version = "/v" + value

    def call(self, method, path, body=None, *, timeout=5, missing_ok=False):
        require(method in ("GET", "POST", "DELETE") and path.startswith("/") and len(path) <= 512, "engine_response")
        remaining = (_deadline or float("inf")) - time.monotonic()
        require(remaining > 0, "deadline_exhausted")
        connection = UnixConnection(min(timeout, remaining))
        try:
            encoded = json.dumps(body).encode() if body is not None else None
            connection.request(method, self.version + path, body=encoded, headers={"Content-Type": "application/json"})
            response = connection.getresponse()
            raw = response.read(MAX_OUTPUT + 1)
            require(len(raw) <= MAX_OUTPUT, "engine_response")
            if response.status == 404 and missing_ok:
                return None
            require(response.status in (200, 201, 204, 304), "engine_failed")
            return json.loads(raw) if raw else {}
        finally:
            connection.close()

    def inspect(self, identity, *, missing_ok=False):
        require(bool(re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,254}", identity)), "engine_identity")
        return self.call("GET", "/containers/" + identity + "/json", missing_ok=missing_ok)

    def stop(self, identity):
        self.call("POST", "/containers/" + identity + "/stop?t=10", timeout=15)

    def rename(self, identity, name):
        require(bool(re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]{0,254}", name)), "engine_identity")
        self.call("POST", "/containers/" + identity + "/rename?name=" + name)

    def create(self, name, spec):
        result = self.call("POST", "/containers/create?name=" + name, spec, timeout=10)
        require(bool(re.fullmatch(r"[a-f0-9]{64}", result.get("Id", ""))), "engine_identity")
        return result["Id"]

    def start(self, identity):
        self.call("POST", "/containers/" + identity + "/start", timeout=10)

    def remove(self, identity, *, force=False):
        require(bool(re.fullmatch(r"[a-f0-9]{64}", identity)), "engine_identity")
        # Never remove volumes, even when dropping the completed rollback copy.
        self.call("DELETE", "/containers/" + identity + "?v=false&force=" + str(force).lower(), timeout=10)


def clone_spec(container, data):
    """Clone daemon-native runtime configuration, without Compose reserialization."""
    require(owned(container) and container["State"].get("Running") is True, "engine_identity")
    require(bool(re.fullmatch(r"[a-f0-9]{64}", container.get("Id", "")))
            and bool(re.fullmatch(r"sha256:[a-f0-9]{64}", container.get("Image", ""))), "engine_identity")
    config, host = copy.deepcopy(container["Config"]), copy.deepcopy(container["HostConfig"])
    require(not host.get("AutoRemove") and not host.get("Links") and not host.get("VolumesFrom"), "engine_rollback_unsafe")
    require(not config.get("MacAddress"), "engine_network_unsupported")
    if config.get("Hostname") in (container["Id"][:12], container["Id"]):
        config["Hostname"] = ""  # Let the daemon assign the new generated ID.
    require(all(row.get("HostPort", "").isdigit() and 0 < int(row["HostPort"]) <= 65535
                for bindings in (host.get("PortBindings") or {}).values() for row in bindings or []), "engine_rollback_unsafe")
    env = envmap(container)
    require((env.get(KEYS[0]), env.get(KEYS[1])) == (data["expected_old_access"], data["expected_old_secret"]), "old_credential_mismatch")
    replacement = dict(zip(KEYS, (data["new_access"], data["new_secret"])))
    # Preserve ordering and every byte of every other environment assignment.
    config["Env"] = [key + "=" + replacement[key] if key in replacement else value
                     for value in config["Env"] for key in (value.split("=", 1)[0],)]
    config["Image"] = container["Image"]
    for mount in container.get("Mounts", []):
        if mount["Type"] == "volume":
            name, target = mount.get("Name"), mount["Destination"]
            referenced = any(row.startswith(str(name) + ":" + target + ":") or row == str(name) + ":" + target
                             for row in host.get("Binds") or [])
            referenced = referenced or any(row.get("Type") == "volume" and row.get("Source") == name
                    and row.get("Target") == target for row in host.get("Mounts") or [])
            require(name and referenced, "engine_mount_unsupported")
    endpoints = {}
    for name, endpoint in container["NetworkSettings"]["Networks"].items():
        ipam = endpoint.get("IPAMConfig")
        require(not ipam or not any(ipam.get(key) for key in ("IPv4Address", "IPv6Address", "LinkLocalIPs")), "engine_network_unsupported")
        require(not endpoint.get("Links"), "engine_network_unsupported")
        endpoints[name] = {key: copy.deepcopy(endpoint[key]) for key in ("IPAMConfig", "DriverOpts", "NetworkID") if key in endpoint}
        endpoints[name]["Aliases"] = [alias for alias in endpoint.get("Aliases") or []
                                      if alias not in (container["Id"], container["Id"][:12])]
    require(endpoints, "network_identity")
    return {**config, "HostConfig": host, "NetworkingConfig": {"EndpointsConfig": endpoints}}


def restore_original(engine, before, new_id, data, immutable):
    """Restore the original container itself, rather than recreating its config."""
    name = before["Name"].removeprefix("/")
    # Never delete a working replacement unless the retained original is still
    # present and bound. In particular, deletion-response ambiguity must not
    # turn successful retirement into loss of both containers.
    original = engine.inspect(before["Id"], missing_ok=True)
    require(original is not None and original["Id"] == before["Id"] and owned(original), "engine_rollback_unsafe")
    if new_id is None:
        possible = engine.inspect(name, missing_ok=True)
        if possible is not None and possible["Id"] != before["Id"]:
            require(owned(possible) and possible["Image"] == before["Image"]
                    and (envmap(possible).get(KEYS[0]), envmap(possible).get(KEYS[1])) ==
                    (data["new_access"], data["new_secret"]), "engine_rollback_unsafe")
            new_id = possible["Id"]
    if new_id is not None:
        require(new_id != before["Id"], "engine_rollback_unsafe")
        try:
            engine.remove(new_id, force=True)
        except Exception:
            require(engine.inspect(new_id, missing_ok=True) is None, "engine_rollback_unsafe")
    original = engine.inspect(before["Id"])
    require(original["Id"] == before["Id"] and owned(original), "engine_rollback_unsafe")
    if original["Name"] != before["Name"]:
        engine.rename(before["Id"], name)
    if not original["State"].get("Running"):
        engine.start(before["Id"])
    restored = engine.inspect(before["Id"])
    identity_ok = restored["Id"] == before["Id"] and projection(restored) == immutable
    pair_ok = (envmap(restored).get(KEYS[0]), envmap(restored).get(KEYS[1])) == (data["expected_old_access"], data["expected_old_secret"])
    # These checks are independent: drift must not hide old-key/health evidence.
    available = ready(restored)
    return {"rollback_identity_verified": identity_ok, "rollback_old_pair_verified": pair_ok,
            "rollback_ready": available, "ready": available,
            "rollback_verified": identity_ok and pair_ok and available}


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
    overall_deadline = time.monotonic() + 200
    previous_deadline = _deadline
    # Reserve the full bounded replacement/readiness rollback budget.
    _deadline = overall_deadline - 70
    report = {"status": "FAIL", "phase": "preflight", "runtime_changed": False,
              "rollback_attempted": False, "rollback_verified": False, "ready": False}
    before = None
    engine = None
    new_id = None
    immutable = None
    try:
        require(os.geteuid() == 0, "root_required")
        before = discover()
        old = envmap(before)
        require((old.get("S3_ENDPOINT"), old.get("S3_BUCKET"), old.get("S3_REGION", "nbg1")) ==
                (data["endpoint"], data["bucket"], data["region"]), "storage_identity")
        require(ready(before), "readiness_failed")
        report["ready"] = True
        if (old.get(KEYS[0]), old.get(KEYS[1])) == (data["new_access"], data["new_secret"]):
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
        document(before, json.loads(rendered), images[0]["Config"])  # Source attestation only.
        immutable = projection(before)
        replacement = clone_spec(before, data)
        engine = Engine()
        report["phase"] = "historical_preflight"
        canary = historical_canary(before)
        if preflight:
            report.update(status="PASS", phase="preflight", loki_count=1, source_file=Path(source).name)
            return report
        require(time.monotonic() + 105 <= _deadline, "deadline_exhausted")
        current = engine.inspect(before["Id"])
        require(current["Id"] == before["Id"] and current["State"].get("Running") is True
                and projection(current) == immutable and envmap(current) == old, "runtime_binding_changed")
        report.update(phase="replacement", runtime_changed=True)
        name = before["Name"].removeprefix("/")
        quarantine = name + "-s3-rollback-" + uuid.uuid4().hex[:12]
        require(len(quarantine) <= 255, "engine_identity")
        engine.stop(before["Id"])
        engine.rename(before["Id"], quarantine)
        new_id = engine.create(name, replacement)
        engine.start(new_id)
        report["phase"] = "adoption_verification"
        after = engine.inspect(new_id)
        require(after["Id"] == new_id and owned(after) and after["State"].get("Running") is True, "engine_identity")
        actual_projection = projection(after)
        if actual_projection != immutable:
            report["immutable_drift"] = sorted(key for key in immutable if actual_projection[key] != immutable[key])
            raise ValueError("runtime_binding_changed")
        require((envmap(after).get(KEYS[0]), envmap(after).get(KEYS[1])) == (data["new_access"], data["new_secret"]), "adoption_mismatch")
        require(ready(after), "readiness_failed")
        report.update(phase="storage_verification", ready=True)
        report.update(storage_proof(after, canary))
        rollback_copy = engine.inspect(before["Id"])
        require(rollback_copy["Id"] == before["Id"] and owned(rollback_copy)
                and rollback_copy["State"].get("Running") is False and rollback_copy["Name"] == "/" + quarantine,
                "engine_rollback_unsafe")
        try:
            engine.remove(before["Id"])
        except Exception:
            # A successful DELETE with a lost response has already committed
            # the verified replacement. Confirm absence instead of rolling it
            # back into a now-deleted original.
            require(engine.inspect(before["Id"], missing_ok=True) is None, "engine_failed")
        report.update(status="PASS", phase="adopted", ready=True)
    except Exception as error:
        reason = str(error) if isinstance(error, ValueError) else "operation_failed"
        report["failure_reason"] = reason if reason in FAILURE_REASONS else "operation_failed"
        if report["runtime_changed"] and before is not None and engine is not None:
            _deadline = overall_deadline
            report.update(rollback_attempted=True, rollback_identity_verified=False,
                          rollback_old_pair_verified=False, rollback_ready=False, ready=False)
            try:
                report.update(restore_original(engine, before, new_id, data, immutable))
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
