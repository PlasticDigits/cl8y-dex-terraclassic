#!/usr/bin/env python3
"""Synthetic safety regressions: no Docker, remote service or credentials required."""
import copy
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("replace_loki_s3_host", Path(__file__).with_name("replace-loki-s3-host.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def payload():
    return {"app_uuid": m.APP, "expected_old_access": "old-access-synthetic", "expected_old_secret": "old-secret-synthetic",
            "new_access": "new-access-synthetic", "new_secret": "new-$secret-${literal}",
            "endpoint": "nbg1.your-objectstorage.com", "bucket": "logs", "region": "nbg1"}


def fixture():
    data = payload()
    environment = dict(zip(m.KEYS, (data["expected_old_access"], data["expected_old_secret"])))
    environment.update(S3_ENDPOINT=data["endpoint"], S3_BUCKET="logs", S3_REGION="nbg1", OTHER="untouched-$value")
    labels = {"coolify.managed": "true", "coolify.applicationId": "54", "com.docker.compose.project": m.APP,
              "com.docker.compose.service": "loki", "custom": "unchanged",
              "com.docker.compose.config-hash": "oldhash",
              "com.docker.compose.version": "2.20.0",
              "com.docker.compose.project.config_files": "/data/coolify/applications/" + m.APP + "/docker-compose.yaml",
              "com.docker.compose.project.working_dir": "/data/coolify/applications/" + m.APP}
    service = {"image": "grafana/loki:existing", "command": ["-config.file=/etc/loki/config.yaml", "-config.expand-env=true"],
               "user": "10001", "environment": environment, "labels": {k: v for k, v in labels.items() if not k.startswith("com.docker.compose.")},
               "mem_limit": "536870912", "restart": "unless-stopped",
               "volumes": [{"type": "volume", "source": "loki-data", "target": "/loki"},
                           {"type": "bind", "source": "/srv/logs/loki.yaml", "target": "/etc/loki/config.yaml", "read_only": True}],
               "networks": {"default": {"aliases": ["loki"]}}}
    container = {"Id": "a" * 64, "Name": "/logs-loki-1", "Image": "sha256:" + "b" * 64,
                 "State": {"Running": True},
                 "Config": {"Env": [key + "=" + value for key, value in environment.items()], "Image": service["image"],
                            "Hostname": "a" * 12, "Labels": labels, "Cmd": service["command"], "Entrypoint": ["/usr/bin/loki"], "User": "10001"},
                 "HostConfig": {"Binds": ["existing-loki-data:/loki:rw", "/srv/logs/loki.yaml:/etc/loki/config.yaml:ro"], "RestartPolicy": {"Name": "unless-stopped", "MaximumRetryCount": 0},
                                "Memory": 536870912, "NetworkMode": "logs-net", "ReadonlyRootfs": False},
                 "Mounts": [{"Type": "volume", "Name": "existing-loki-data", "Source": "/var/lib/docker/volumes/existing-loki-data/_data",
                             "Destination": "/loki", "Driver": "local", "Mode": "rw", "RW": True, "Propagation": ""},
                            {"Type": "bind", "Source": "/srv/logs/loki.yaml", "Destination": "/etc/loki/config.yaml",
                             "Mode": "ro", "RW": False, "Propagation": "rprivate"}],
                 "NetworkSettings": {"Networks": {"logs-net": {"IPAddress": "172.18.0.4", "Aliases": ["logs-loki-1", "loki", "a" * 12],
                                                             "IPAMConfig": None, "Links": None, "DriverOpts": None}}}}
    compose = {"services": {"loki": service, "grafana": {"image": "must-never-be-touched"},
                            "alloy": {"image": "must-never-be-touched"}},
               "networks": {"default": {"name": "logs-net"}}, "volumes": {"loki-data": {"name": "existing-loki-data"}}}
    return data, container, compose


def image_defaults(container):
    return {key: copy.deepcopy(container["Config"].get(key)) for key in ("Cmd", "Entrypoint", "User")}


class ScopeTests(unittest.TestCase):
    def setUp(self):
        self.data, self.container, self.compose = fixture()

    def test_only_loki_and_existing_storage_survive_render(self):
        original = copy.deepcopy(self.compose)
        doc = m.document(self.container, self.compose, image_defaults(self.container))
        self.assertEqual(set(doc["services"]), {"loki"})
        self.assertEqual(doc["services"]["loki"]["image"], self.container["Image"])
        self.assertEqual(doc["volumes"], {"existing_loki_0": {"external": True, "name": "existing-loki-data"}})
        self.assertEqual(doc["networks"], {"default": {"external": True, "name": "logs-net"}})
        self.assertEqual(self.compose, original)
        self.assertEqual(doc["services"]["loki"]["command"], self.compose["services"]["loki"]["command"])

    def test_source_env_drift_preserves_running_environment(self):
        self.compose["services"]["loki"]["environment"]["OTHER"] = "different"
        doc = m.document(self.container, self.compose, image_defaults(self.container))
        self.assertEqual(doc["services"]["loki"]["environment"]["OTHER"], "untouched-$value")

    def test_canonical_command_resources_user_and_label_drift_are_rejected(self):
        for key, value in (("command", ["-wrong-command"]), ("mem_limit", "1"), ("cpus", "1"),
                           ("user", "root"), ("labels", {"custom": "different"}), ("entrypoint", ["/wrong"]),
                           ("privileged", True), ("cap_add", ["SYS_ADMIN"])):
            full = copy.deepcopy(self.compose)
            full["services"]["loki"][key] = value
            with self.assertRaisesRegex(ValueError, "compose_runtime_drift"):
                m.document(self.container, full, image_defaults(self.container))

    def test_changed_mount_or_network_is_rejected_before_mutation(self):
        for alter in (lambda c: c["services"]["loki"]["volumes"][1].update(source="/wrong"),
                      lambda c: c["networks"]["default"].update(name="wrong-network"),
                      lambda c: c["services"]["loki"].update(volumes=[])):
            full = copy.deepcopy(self.compose)
            alter(full)
            with self.assertRaises(ValueError):
                m.document(self.container, full, image_defaults(self.container))

    def test_unexpected_container_owner_rejected(self):
        self.assertTrue(m.owned(self.container))
        for key, value in (("coolify.applicationUuid", "another"), ("coolify.applicationId", "nonnumeric"),
                           ("com.docker.compose.project", "another"), ("com.docker.compose.service", "grafana")):
            changed = copy.deepcopy(self.container)
            changed["Config"]["Labels"][key] = value
            self.assertFalse(m.owned(changed))

    def test_unsafe_feature_not_silently_changed(self):
        for field in ("volumes_from", "post_start", "pre_stop", "devices", "use_api_socket", "pid"):
            full = copy.deepcopy(self.compose)
            full["services"]["loki"][field] = ["unreviewed"]
            with self.assertRaisesRegex(ValueError, "unsupported_compose"):
                m.document(self.container, full, image_defaults(self.container))

    def test_engine_clone_preserves_runtime_and_literal_secret_bytes(self):
        original = copy.deepcopy(self.container)
        spec = m.clone_spec(self.container, self.data)
        self.assertEqual(spec["HostConfig"], original["HostConfig"])
        self.assertEqual(spec["Image"], original["Image"])
        self.assertEqual(spec["Hostname"], "")
        self.assertEqual(spec["Env"], [row if not row.startswith("S3_ACCESS_KEY=") and not row.startswith("S3_SECRET_KEY=")
            else row.split("=", 1)[0] + "=" + self.data["new_access" if row.startswith("S3_ACCESS_KEY=") else "new_secret"]
            for row in original["Config"]["Env"]])
        self.assertIn("S3_SECRET_KEY=new-$secret-${literal}", spec["Env"])
        endpoints = spec["NetworkingConfig"]["EndpointsConfig"]
        self.assertEqual(set(endpoints), {"logs-net"})
        self.assertNotIn("IPAddress", endpoints["logs-net"])
        self.assertNotIn("a" * 12, endpoints["logs-net"]["Aliases"])
        self.assertEqual(self.container, original)

    def test_generated_hostname_normalizes_but_explicit_hostname_is_preserved(self):
        changed = copy.deepcopy(self.container)
        changed["Id"] = "c" * 64
        changed["Config"]["Hostname"] = "c" * 12
        changed["NetworkSettings"]["Networks"]["logs-net"]["Aliases"] = ["logs-loki-1", "loki", "c" * 12]
        self.assertEqual(m.projection(self.container), m.projection(changed))
        self.container["Config"]["Hostname"] = "loki-explicit-host"
        self.assertEqual(m.clone_spec(self.container, self.data)["Hostname"], "loki-explicit-host")
        self.assertNotEqual(m.projection(self.container), m.projection(changed))

    def test_engine_clone_rejects_features_that_prevent_exact_original_rollback(self):
        for alter, reason in ((lambda c: c["HostConfig"].update(AutoRemove=True), "engine_rollback_unsafe"),
                              (lambda c: c["HostConfig"].update(Binds=[]), "engine_mount_unsupported"),
                              (lambda c: c["Config"].update(MacAddress="static"), "engine_network_unsupported"),
                              (lambda c: c["NetworkSettings"]["Networks"]["logs-net"].update(IPAMConfig={"IPv4Address": "172.18.0.5"}), "engine_network_unsupported")):
            changed = copy.deepcopy(self.container)
            alter(changed)
            with self.assertRaisesRegex(ValueError, reason):
                m.clone_spec(changed, self.data)

    def test_engine_api_deletion_never_removes_volumes_or_accepts_a_name(self):
        engine = m.Engine.__new__(m.Engine)
        engine.version = "/v1.47"
        connection = Mock()
        connection.getresponse.return_value = Mock(status=204, read=Mock(return_value=b""))
        with patch.object(m, "UnixConnection", return_value=connection):
            engine.remove("c" * 64, force=True)
        self.assertEqual(connection.request.call_args.args[0:2],
                         ("DELETE", "/v1.47/containers/" + "c" * 64 + "?v=false&force=true"))
        connection.close.assert_called_once()
        with self.assertRaisesRegex(ValueError, "engine_identity"):
            engine.remove("loki")

    def test_engine_error_body_is_never_reflected(self):
        engine = m.Engine.__new__(m.Engine)
        engine.version = "/v1.47"
        connection = Mock()
        connection.getresponse.return_value = Mock(status=500, read=Mock(return_value=b"SYNTHETIC_DOWNSTREAM_SECRET"))
        with patch.object(m, "UnixConnection", return_value=connection), self.assertRaisesRegex(ValueError, "^engine_failed$"):
            engine.start("c" * 64)
        connection.close.assert_called_once()

    def test_immutable_projection_rejects_resource_and_userlabel_changes(self):
        changed = copy.deepcopy(self.container)
        changed["Config"]["Env"] = [row if not row.startswith("S3_SECRET_KEY=") else "S3_SECRET_KEY=new" for row in changed["Config"]["Env"]]
        changed["Config"]["Labels"]["com.docker.compose.config-hash"] = "newhash"
        changed["Config"]["Labels"]["com.docker.compose.version"] = "2.32.4"
        self.assertEqual(m.projection(changed), m.projection(self.container))
        changed["HostConfig"]["Memory"] += 1
        self.assertNotEqual(m.projection(changed), m.projection(self.container))
        changed = copy.deepcopy(self.container)
        changed["Config"]["Labels"]["custom"] = "changed"
        self.assertNotEqual(m.projection(changed), m.projection(self.container))

    def test_path_override_cannot_escape_canonical_source(self):
        for source in ("../docker-compose.yaml", "/etc/shadow", "random.yaml"):
            with self.assertRaises(ValueError):
                m.sources(self.container, {**self.data, "source_file": source})

    def test_input_bound_and_unknown_fields(self):
        with self.assertRaises(ValueError):
            m.payload_input(io.StringIO("x" * 16385))
        with self.assertRaises(ValueError):
            m.payload_input(io.StringIO(json.dumps({**self.data, "command": "bad"})))
        self.assertEqual(m.payload_input(io.StringIO(json.dumps(self.data))), self.data)

    def test_command_cannot_inherit_maintenance_credentials_or_remote_daemon(self):
        result = m.subprocess.CompletedProcess(["docker"], 0, stdout="{}", stderr="")
        with patch.dict(m.os.environ, {"COOLIFY_TOKEN": "synthetic-private", "S3_SECRET_KEY": "wrong-inherited",
                                      "DOCKER_HOST": "tcp://wrong-daemon:2375"}), \
             patch.object(m.subprocess, "run", return_value=result) as call:
            self.assertEqual(m.command(["docker", "inspect", "exact"]), "{}")
        environment = call.call_args.kwargs["env"]
        self.assertEqual(set(environment), {"PATH", "HOME", "LC_ALL", "DOCKER_HOST"})
        self.assertEqual(environment["DOCKER_HOST"], "unix:///var/run/docker.sock")
        self.assertNotIn("synthetic-private", str(environment))


class TransactionTests(unittest.TestCase):
    def exercise(self, *, preflight=False, immutable_failure=False, readiness=True, command_failure=False, already=False,
                 storage_failure=False, retirement_ambiguous=False, rollback_projection_drift=False):
        data, container, compose = fixture()
        objects = {container["Id"]: copy.deepcopy(container)}
        calls = []
        new_id = "c" * 64
        if already:
            objects[container["Id"]]["Config"]["Env"] = [row if not row.startswith("S3_ACCESS_KEY=") and not row.startswith("S3_SECRET_KEY=")
                else row.split("=", 1)[0] + "=" + data["new_access" if row.startswith("S3_ACCESS_KEY=") else "new_secret"]
                for row in objects[container["Id"]]["Config"]["Env"]]

        class FakeEngine:
            def inspect(self, identity, missing_ok=False):
                item = objects.get(identity) or next((v for v in objects.values() if v["Name"] == "/" + identity), None)
                if item is None:
                    if missing_ok:
                        return None
                    raise ValueError("engine_failed")
                return copy.deepcopy(item)

            def stop(self, identity):
                calls.append(("stop", identity))
                objects[identity]["State"]["Running"] = False

            def rename(self, identity, name):
                calls.append(("rename", identity))
                objects[identity]["Name"] = "/" + name

            def create(self, name, spec):
                calls.append(("create", new_id))
                item = copy.deepcopy(container)
                item.update(Id=new_id, Name="/" + name, State={"Running": False})
                item["Config"] = {key: copy.deepcopy(value) for key, value in spec.items() if key not in ("HostConfig", "NetworkingConfig")}
                item["HostConfig"] = copy.deepcopy(spec["HostConfig"])
                for network, endpoint in spec["NetworkingConfig"]["EndpointsConfig"].items():
                    item["NetworkSettings"]["Networks"][network].update(copy.deepcopy(endpoint))
                if immutable_failure:
                    item["HostConfig"]["Memory"] = 1
                objects[new_id] = item
                if command_failure:
                    raise ValueError("raw secret must never escape: " + data["new_secret"])
                return new_id

            def start(self, identity):
                calls.append(("start", identity))
                objects[identity]["State"]["Running"] = True
                if rollback_projection_drift and identity == container["Id"]:
                    objects[identity]["HostConfig"]["Memory"] += 1

            def remove(self, identity, force=False):
                calls.append(("remove", identity, force))
                objects.pop(identity)
                if retirement_ambiguous and identity == container["Id"]:
                    raise TimeoutError()

        with patch.object(m.os, "geteuid", return_value=0), \
             patch.object(m, "discover", side_effect=lambda: copy.deepcopy(objects[container["Id"]])), \
             patch.object(m, "sources", return_value=("/trusted/docker-compose.yaml", "/trusted")), \
             patch.object(m, "command", side_effect=lambda args: json.dumps(
                 [{"Id": container["Image"], "Config": image_defaults(container)}]) if args[1] == "image" else json.dumps(compose)), \
             patch.object(m, "Engine", return_value=FakeEngine()), \
             patch.object(m, "historical_canary", return_value={"stream": {"job": "docker"}, "timestamp": 1, "line": "old"}), \
             patch.object(m, "storage_proof", side_effect=ValueError("loki_flush_failed") if storage_failure else None,
                 return_value={"backing_storage_write_verified": True, "marker_readback_verified": True,
                 "historical_readback_verified": True, "flush_success_delta": 1, "flush_failure_delta": 0}), \
             patch.object(m, "ready", side_effect=lambda current: readiness or current["Id"] == container["Id"]):
            report = m.run(data, preflight)
        self.assertNotIn(data["new_secret"], json.dumps(report))
        self.assertNotIn(data["expected_old_secret"], json.dumps(report))
        live = next(value for value in objects.values() if value["State"]["Running"])
        return report, calls, live

    def test_preflight_is_read_only(self):
        report, calls, _ = self.exercise(preflight=True)
        self.assertEqual(report["status"], "PASS")
        self.assertFalse(report["runtime_changed"])
        self.assertTrue(report["ready"])
        self.assertEqual(report["source_file"], "docker-compose.yaml")
        self.assertEqual(calls, [])

    def test_success_preserves_actual_config_and_retires_original_without_volumes(self):
        report, calls, live = self.exercise()
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(calls[-1], ("remove", "a" * 64, False))
        self.assertEqual(live["Id"], "c" * 64)
        self.assertEqual(m.envmap(live)["S3_SECRET_KEY"], payload()["new_secret"])
        self.assertEqual(m.projection(live), m.projection(fixture()[1]))

    def test_lost_create_response_recovers_by_exact_name_then_restores_original_id(self):
        report, calls, live = self.exercise(command_failure=True)
        self.assertEqual(report["status"], "FAIL")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(report["failure_reason"], "operation_failed")
        self.assertEqual(live["Id"], "a" * 64)
        self.assertIn(("remove", "c" * 64, True), calls)
        self.assertNotIn(("remove", "a" * 64, False), calls)

    def test_runtime_resource_drift_restores_the_original_container(self):
        report, calls, live = self.exercise(immutable_failure=True)
        self.assertEqual(report["status"], "FAIL")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(report["immutable_drift"], ["host"])
        self.assertEqual(live["Id"], "a" * 64)

    def test_readiness_or_storage_failure_restores_original(self):
        for kwargs, reason in (({"readiness": False}, "readiness_failed"), ({"storage_failure": True}, "loki_flush_failed")):
            report, _, live = self.exercise(**kwargs)
            self.assertEqual(report["status"], "FAIL")
            self.assertEqual(report["failure_reason"], reason)
            self.assertTrue(report["rollback_verified"])
            self.assertEqual(live["Id"], "a" * 64)

    def test_rollback_projection_drift_does_not_hide_old_pair_and_readiness(self):
        report, _, live = self.exercise(storage_failure=True, rollback_projection_drift=True)
        self.assertFalse(report["rollback_verified"])
        self.assertFalse(report["rollback_identity_verified"])
        self.assertTrue(report["rollback_old_pair_verified"])
        self.assertTrue(report["rollback_ready"])
        self.assertTrue(report["ready"])
        self.assertEqual(live["Id"], "a" * 64)

    def test_lost_retirement_response_cannot_delete_the_working_replacement(self):
        report, calls, live = self.exercise(retirement_ambiguous=True)
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(live["Id"], "c" * 64)
        self.assertNotIn(("remove", "c" * 64, True), calls)

    def test_retry_after_success_never_recreates_loki(self):
        report, calls, _ = self.exercise(already=True)
        self.assertEqual(report["phase"], "already_adopted")
        self.assertEqual(report["status"], "PASS")
        self.assertFalse(report["runtime_changed"])
        self.assertEqual(calls, [])

    def test_already_adopted_preflight_stays_read_only(self):
        report, calls, _ = self.exercise(already=True, preflight=True)
        self.assertEqual(report["phase"], "preflight")
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["loki_count"], 1)
        self.assertNotIn("backing_storage_write_verified", report)
        self.assertEqual(calls, [])

    def test_nonroot_cannot_mutate(self):
        with patch.object(m.os, "geteuid", return_value=1000), patch.object(m, "command") as call:
            report = m.run(payload())
        self.assertEqual(report["status"], "FAIL")
        call.assert_not_called()

    def test_primary_work_cannot_consume_rollback_reservation(self):
        with patch.object(m.time, "monotonic", side_effect=[0, 100]):
            report, calls, _ = self.exercise()
        self.assertEqual(report["failure_reason"], "deadline_exhausted")
        self.assertFalse(report["runtime_changed"])
        self.assertEqual(calls, [])

    def test_nonroot_main_does_not_read_secret_stdin(self):
        stream = io.StringIO()
        with patch.object(m.os, "geteuid", return_value=1000), patch.object(m.resource, "setrlimit"), \
             patch.object(m.os, "umask"), patch.object(m.sys, "argv", ["replace-loki-s3-host.py"]), \
             patch.object(m.sys.stdin, "read", side_effect=AssertionError("must not read")), \
             patch.object(m.sys, "stdout", stream):
            self.assertEqual(m.main(), 1)
        self.assertEqual(json.loads(stream.getvalue())["failure_reason"], "root_required")

    def test_readiness_has_one_absolute_bound_across_all_networks(self):
        _, container, _ = fixture()
        container["NetworkSettings"]["Networks"]["another"] = {"IPAddress": "172.19.0.4"}
        elapsed = [0]

        class Opener:
            def open(self, _url, timeout):
                elapsed[0] += timeout
                raise TimeoutError()

        with patch.object(m.time, "monotonic", side_effect=lambda: elapsed[0]), \
             patch.object(m.time, "sleep", side_effect=lambda delay: elapsed.__setitem__(0, elapsed[0] + delay)), \
             patch.object(m.urllib.request, "build_opener", return_value=Opener()):
            self.assertFalse(m.ready(container))
        self.assertLessEqual(elapsed[0], 25)


class StorageProofTests(unittest.TestCase):
    @staticmethod
    def metrics(stored=0, failures=0, flushed=0):
        return ('# TYPE loki_ingester_chunks_stored_total counter\n'
                '# TYPE loki_ingester_chunks_flush_failures_total counter\n'
                '# TYPE loki_ingester_chunks_flushed_total counter\n'
                f'loki_ingester_chunks_stored_total{{tenant="fake"}} {stored}\n'
                f'loki_ingester_chunks_flush_failures_total {failures}\n'
                f'loki_ingester_chunks_flushed_total{{reason="forced"}} {flushed}\n').encode()

    def exercise(self, counters, missing_marker=False, missing_history=False):
        _, container, _ = fixture()
        canary = {"stream": {"job": "docker", "component": "docker"}, "timestamp": 111, "line": "private historical sample"}
        pushes = []
        metrics = iter(counters)

        def http(_container, path, **kwargs):
            if path == "/metrics":
                return 200, next(metrics)
            if path == "/loki/api/v1/push":
                pushes.append(kwargs["body"])
            self.assertIn(path, ("/flush", "/loki/api/v1/push"))
            return 204, b""

        def query(_container, selector, start, end, **_kwargs):
            if start == canary["timestamp"]:
                return [] if missing_history else [canary]
            stream = pushes[0]["streams"][0]
            self.assertEqual(end - start, 1)
            self.assertEqual(stream["stream"], {"job": "cl8y-s3-rotation-proof", "service_name": "cl8y-s3-rotation-proof"})
            return [] if missing_marker else [{"stream": stream["stream"], "timestamp": int(stream["values"][0][0]),
                                                "line": stream["values"][0][1]}]

        with patch.object(m, "loki_http", side_effect=http), patch.object(m, "query_samples", side_effect=query), \
             patch.object(m.time, "sleep"):
            return m.storage_proof(container, canary)

    def test_async_flush_requires_success_counters_and_distinct_reads(self):
        report = self.exercise([self.metrics(), self.metrics(), self.metrics(1, 0, 1), self.metrics(1, 0, 1)])
        self.assertEqual(report, {"backing_storage_write_verified": True, "marker_readback_verified": True,
                                  "historical_readback_verified": True, "flush_success_delta": 1, "flush_failure_delta": 0})

    def test_http204_and_marker_acceptance_without_actual_flush_cannot_pass(self):
        with self.assertRaisesRegex(ValueError, "loki_flush_unconfirmed"):
            self.exercise([self.metrics()] * 7)

    def test_uninitialized_vectors_are_zero_only_until_actual_flush_samples_arrive(self):
        baseline = (b'# TYPE loki_ingester_chunks_flush_failures_total counter\n'
                    b'loki_ingester_chunks_flush_failures_total 0\n')
        report = self.exercise([baseline, baseline, self.metrics(1, 0, 1), self.metrics(1, 0, 1)])
        self.assertTrue(report["backing_storage_write_verified"])
        with self.assertRaisesRegex(ValueError, "loki_flush_unconfirmed"):
            self.exercise([baseline] * 7)

    def test_renamed_or_non_counter_success_metrics_cannot_pass(self):
        renamed = self.metrics(100, 0, 100).replace(b"chunks_stored_total", b"chunks_stored_renamed").replace(
            b"chunks_flushed_total", b"chunks_flushed_renamed")
        with self.assertRaisesRegex(ValueError, "loki_flush_unconfirmed"):
            self.exercise([renamed] * 7)
        wrong_type = self.metrics(1, 0, 1).replace(b"chunks_stored_total counter", b"chunks_stored_total gauge")
        with self.assertRaisesRegex(ValueError, "loki_metrics_missing"):
            self.exercise([self.metrics(), wrong_type])

    def test_any_new_storage_failure_fails_even_when_success_counter_rises(self):
        with self.assertRaisesRegex(ValueError, "loki_flush_failed"):
            self.exercise([self.metrics(), self.metrics(1, 1, 1)])

    def test_counter_reset_cannot_masquerade_as_success(self):
        with self.assertRaisesRegex(ValueError, "loki_flush_unconfirmed"):
            self.exercise([self.metrics(2, 0, 2), self.metrics(1, 0, 1)])

    def test_delayed_failure_during_reads_is_not_hidden(self):
        with self.assertRaisesRegex(ValueError, "loki_flush_failed"):
            self.exercise([self.metrics(), self.metrics(1, 0, 1), self.metrics(1, 1, 1)])

    def test_missing_marker_or_history_fails_after_successful_storage_write(self):
        for options, reason in (({"missing_marker": True}, "loki_marker_missing"),
                                ({"missing_history": True}, "loki_history_missing")):
            with self.assertRaisesRegex(ValueError, reason):
                self.exercise([self.metrics(), self.metrics(1, 0, 1)], **options)

    def test_missing_failure_metric_and_nonfinite_counters_fail_closed(self):
        _, container, _ = fixture()
        for body, reason in ((b"unrelated_metric 1\n", "loki_metrics_missing"),
                             (self.metrics(failures="NaN"), "loki_response_shape")):
            with patch.object(m, "loki_http", return_value=(200, body)), self.assertRaisesRegex(ValueError, reason):
                m.flush_counters(container, 999999999999)

    def test_historical_content_never_enters_query_url(self):
        _, container, _ = fixture()
        response = {"status": "success", "data": {"resultType": "streams", "result": [
            {"stream": {"job": "docker"}, "values": [["111", "synthetic sensitive historical line"]]}]}}
        with patch.object(m, "loki_http", return_value=(200, json.dumps(response).encode())) as call:
            sample = m.query_sample(container, '{job="docker"}', 111, 112)
        self.assertEqual(sample["line"], "synthetic sensitive historical line")
        self.assertNotIn("historical", call.call_args.args[1])

    def test_categorized_queries_keep_metadata_out_of_marker_and_history_selectors(self):
        _, container, _ = fixture()
        stamp = 1760000000123456789
        stream = {"job": "cl8y-s3-rotation-proof", "service_name": "cl8y-s3-rotation-proof"}
        expected = {"timestamp": stamp, "stream": stream, "line": "synthetic owned proof exact content"}
        paths = []

        class Response:
            status = 200

            def __init__(self, categorized):
                self.categorized = categorized

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def read(self, _bound):
                result_stream = dict(stream) if self.categorized else {**stream, "detected_level": "unknown"}
                value = [str(stamp), expected["line"]]
                if self.categorized:
                    value.append({"structuredMetadata": {"detected_level": "unknown"}})
                return json.dumps({"status": "success", "data": {"resultType": "streams", "result": [
                    {"stream": result_stream, "values": [value]}]}}).encode()

        class Opener:
            def open(self, request, **_kwargs):
                paths.append(request.full_url)
                headers = {key.lower(): value for key, value in request.header_items()}
                return Response(headers.get("x-loki-response-encoding-flags") == "categorize-labels")

        with patch.object(m.urllib.request, "build_opener", return_value=Opener()):
            sample = m.query_sample(container, m.selector_for(stream), stamp, stamp + 1)
            self.assertEqual(sample["stream"], stream)
            self.assertNotIn("detected_level", sample["stream"])
            self.assertTrue(m.find_sample(container, expected))
            # A retained historical sample uses the same indexed-label-only
            # selector and exact nanosecond lookup, independent of metadata.
            self.assertTrue(m.find_sample(container, sample))
        parameters = m.urllib.parse.parse_qs(m.urllib.parse.urlsplit(paths[-1]).query)
        self.assertEqual(int(parameters["start"][0]), stamp)
        self.assertEqual(int(parameters["end"][0]), stamp + 1)
        self.assertEqual(parameters["limit"], ["64"])
        self.assertTrue(all("detected_level" not in path and expected["line"] not in path for path in paths))

    def test_private_lookup_never_accepts_a_nearby_or_different_line(self):
        _, container, _ = fixture()
        expected = {"timestamp": 1760000000123456789, "stream": {"job": "docker"}, "line": "exact historical content"}
        alternatives = [{**expected, "timestamp": expected["timestamp"] + 1},
                        {**expected, "line": "different content"},
                        {**expected, "stream": {"job": "different"}}]
        with patch.object(m, "query_samples", return_value=alternatives):
            self.assertFalse(m.find_sample(container, expected))

    def test_private_query_enforces_one_global_result_count_bound(self):
        _, container, _ = fixture()
        response = {"status": "success", "data": {"resultType": "streams", "result": [
            {"stream": {"job": "docker"}, "values": [["111", "synthetic"]] * 65}]}}
        with patch.object(m, "loki_http", return_value=(200, json.dumps(response).encode())), \
                self.assertRaisesRegex(ValueError, "loki_response_shape"):
            m.query_samples(container, '{job="docker"}', 111, 112)

    def test_history_requires_old_bounded_range_and_store_involvement(self):
        _, container, _ = fixture()
        sample = {"stream": {"job": "docker"}, "timestamp": 1, "line": "old", "stats": {
            "querier": {"store": {"totalChunksDownloaded": 1}}}}
        with patch.object(m.time, "time_ns", return_value=10**15), \
             patch.object(m, "query_sample", return_value=sample) as call:
            self.assertEqual(m.historical_canary(container), sample)
        self.assertEqual(call.call_args.args[2:4], (10**15 - 4 * 3600 * 10**9, 10**15 - 3600 * 10**9))
        sample["stats"]["querier"]["store"]["totalChunksDownloaded"] = 0
        with patch.object(m, "query_sample", return_value=sample), self.assertRaisesRegex(ValueError, "loki_history_missing"):
            m.historical_canary(container)


if __name__ == "__main__":
    unittest.main()
