#!/usr/bin/env python3
"""Synthetic safety regressions: no Docker, remote service or credentials required."""
import copy
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

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
                 "HostConfig": {"RestartPolicy": {"Name": "unless-stopped", "MaximumRetryCount": 0},
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

    def test_apply_never_mentions_other_service_and_escapes_interpolation(self):
        doc = m.document(self.container, self.compose, image_defaults(self.container))
        doc["services"]["loki"]["environment"]["S3_SECRET_KEY"] = self.data["new_secret"]
        with patch.object(m, "command") as call:
            m.apply(doc, "/trusted")
        args, body = call.call_args.args
        self.assertEqual(args[-1], "loki")
        self.assertIn("--no-deps", args)
        self.assertEqual(args[args.index("--pull") + 1], "never")
        self.assertEqual(args[args.index("--file") + 1], "-")
        serialized = json.loads(body)
        self.assertEqual(set(serialized["services"]), {"loki"})
        self.assertEqual(serialized["services"]["loki"]["environment"]["S3_SECRET_KEY"], "new-$$secret-$${literal}")
        self.assertNotIn(self.data["new_secret"], " ".join(args))

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
                 storage_failure=False):
        data, container, compose = fixture()
        live = [copy.deepcopy(container)]
        applications = []
        if already:
            env = m.envmap(live[0])
            env.update(dict(zip(m.KEYS, (data["new_access"], data["new_secret"]))))
            live[0]["Config"]["Env"] = [key + "=" + value for key, value in env.items()]

        def apply(doc, _directory):
            applications.append(copy.deepcopy(doc))
            if command_failure and len(applications) == 1:
                raise ValueError("raw secret must never escape: " + data["new_secret"])
            live[0] = copy.deepcopy(container)
            env = doc["services"]["loki"]["environment"]
            live[0]["Config"]["Env"] = [key + "=" + value for key, value in env.items()]
            if immutable_failure and len(applications) == 1:
                live[0]["HostConfig"]["Memory"] = 1

        with patch.object(m.os, "geteuid", return_value=0), \
             patch.object(m, "discover", side_effect=lambda: copy.deepcopy(live[0])), \
             patch.object(m, "sources", return_value=("/trusted/docker-compose.yaml", "/trusted")), \
             patch.object(m, "command", side_effect=lambda args: json.dumps(
                 [{"Id": container["Image"], "Config": image_defaults(container)}]) if args[1] == "image" else json.dumps(compose)), \
             patch.object(m, "apply", side_effect=apply), \
             patch.object(m, "historical_canary", return_value={"stream": {"job": "docker"}, "timestamp": 1, "line": "old"}), \
             patch.object(m, "storage_proof", side_effect=ValueError("loki_flush_failed") if storage_failure else None,
                 return_value={"backing_storage_write_verified": True,
                 "marker_readback_verified": True, "historical_readback_verified": True,
                 "flush_success_delta": 1, "flush_failure_delta": 0}), \
             patch.object(m, "ready", side_effect=lambda _c: readiness or len(applications) > 1):
            report = m.run(data, preflight)
        self.assertNotIn(data["new_secret"], json.dumps(report))
        self.assertNotIn(data["expected_old_secret"], json.dumps(report))
        return report, applications, live[0]

    def test_preflight_is_read_only(self):
        report, calls, _ = self.exercise(preflight=True)
        self.assertEqual(report["status"], "PASS")
        self.assertFalse(report["runtime_changed"])
        self.assertEqual(report["source_file"], "docker-compose.yaml")
        self.assertEqual(calls, [])

    def test_success_changes_only_the_s3_pair(self):
        report, calls, live = self.exercise()
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(len(calls), 1)
        self.assertTrue(report["ready"])
        self.assertEqual(m.envmap(live)["S3_SECRET_KEY"], payload()["new_secret"])

    def test_ambiguous_command_failure_rolls_back_loki_only(self):
        report, calls, live = self.exercise(command_failure=True)
        self.assertEqual(report["status"], "FAIL")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(report["failure_reason"], "operation_failed")
        self.assertEqual(len(calls), 2)
        self.assertEqual(m.envmap(live)["S3_SECRET_KEY"], payload()["expected_old_secret"])
        self.assertTrue(all(set(doc["services"]) == {"loki"} for doc in calls))

    def test_runtime_resource_drift_rolls_back(self):
        report, calls, _ = self.exercise(immutable_failure=True)
        self.assertEqual(report["status"], "FAIL")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(report["failure_reason"], "runtime_binding_changed")
        self.assertEqual(report["immutable_drift"], ["host"])
        self.assertEqual(len(calls), 2)

    def test_readiness_failure_rolls_back(self):
        report, calls, _ = self.exercise(readiness=False)
        self.assertEqual(report["status"], "FAIL")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(len(calls), 2)

    def test_successful_push_but_failed_storage_proof_rolls_back(self):
        report, calls, _ = self.exercise(storage_failure=True)
        self.assertEqual(report["status"], "FAIL")
        self.assertEqual(report["phase"], "storage_verification")
        self.assertEqual(report["failure_reason"], "loki_flush_failed")
        self.assertTrue(report["rollback_verified"])
        self.assertEqual(len(calls), 2)

    def test_retry_after_success_never_recreates_loki(self):
        report, calls, _ = self.exercise(already=True)
        self.assertEqual(report["phase"], "already_adopted")
        self.assertEqual(report["status"], "PASS")
        self.assertFalse(report["runtime_changed"])
        self.assertEqual(calls, [])

    def test_already_adopted_preflight_is_reported_as_read_only_preflight(self):
        report, calls, _ = self.exercise(already=True, preflight=True)
        self.assertEqual(report["phase"], "preflight")
        self.assertEqual(report["status"], "PASS")
        self.assertEqual(report["loki_count"], 1)
        self.assertFalse(report["runtime_changed"])
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
        self.assertEqual(report["status"], "FAIL")
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
                return None if missing_history else canary
            stream = pushes[0]["streams"][0]
            self.assertEqual(end - start, 1)
            self.assertEqual(set(stream["stream"]), {"job"})
            return None if missing_marker else {"stream": stream["stream"], "timestamp": int(stream["values"][0][0]),
                                                "line": stream["values"][0][1]}

        with patch.object(m, "loki_http", side_effect=http), patch.object(m, "query_sample", side_effect=query), \
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
