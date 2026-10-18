#!/usr/bin/env python3
"""Synthetic journal/deployment boundary checks; no network or production secrets."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location("adopt", Path(__file__).with_name("adopt-logs-s3.py"))
m = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(m)


def state(phase="preflight_started"):
    return {"phase": phase, "app_uuid": m.APP, "revision": "a" * 40, "transaction": "b" * 16,
            "recipe_sha256": hashlib.sha256(b"synthetic recipe").hexdigest(),
            "compose_sha256": hashlib.sha256(m.compose_source("synthetic recipe", "b" * 16).encode()).hexdigest(), "job_uuid": "c" * 24,
            "job_name": "synthetic-job", "binding": {"environment_uuid": "d" * 24, "environment_id": 101},
            "payload": {"new_access": "NEW_SYNTHETIC_ACCESS", "new_secret": "NEW_SYNTHETIC_SECRET",
                        "expected_old_access": "OLD_SYNTHETIC_ACCESS", "expected_old_secret": "OLD_SYNTHETIC_SECRET",
                        "endpoint": "nbg1.your-objectstorage.com", "bucket": "logs", "region": "nbg1"},
            "subapp_uuid": "e" * 24, "job_id": 303, "pending_version": 3}


def job(s):
    return {"uuid": s["job_uuid"], "name": s["job_name"],
            "docker_compose_raw": m.compose_source("synthetic recipe", s["transaction"]),
            "description": "Loki-only S3 adoption #1544; " + s["transaction"],
            "connect_to_docker_network": False, "databases": [],
            "applications": [{"uuid": s["subapp_uuid"], "name": "maintenance", "service_id": 303, "fqdn": None}],
            "destination": {"server": {"uuid": m.SERVER}}, "environment_id": s["binding"]["environment_id"]}


class Tests(unittest.TestCase):
    def test_summary_never_copies_payload_or_nested_state(self):
        s = state()
        s["unexpected"] = "FOREIGN_SYNTHETIC_SECRET"
        body = json.dumps(m.summary(s))
        for value in ("SYNTHETIC", "unexpected", "payload", "binding"):
            self.assertNotIn(value, body)
        self.assertFalse(m.summary(s)["old_key_revoked"])

    def test_protected_job_refused(self):
        for target in m.PROTECTED:
            s = state()
            s["job_uuid"] = target
            with self.assertRaises(m.GuardError):
                m.validate(s)

    def test_compose_contains_exact_mounts_no_ports_and_no_build_secrets(self):
        for tx in ("a --privileged", "../escape", "", "b" * 17):
            with self.assertRaises(m.GuardError):
                m.compose_document("reviewed recipe", tx)
        document = m.compose_document("reviewed recipe", "b" * 16)
        self.assertEqual(set(document["services"]), {"maintenance"})
        service = document["services"]["maintenance"]
        self.assertEqual(service["restart"], "no")
        self.assertEqual(service["cap_drop"], ["ALL"])
        self.assertEqual(len(service["volumes"]), 3)
        self.assertEqual(service["volumes"][0], "/var/run/docker.sock:/var/run/docker.sock:rw")
        self.assertIn(m.APP + ":/data/coolify/applications/" + m.APP + ":ro", service["volumes"][1])
        self.assertNotIn("ports", service)
        self.assertNotIn("privileged", service)
        self.assertEqual(set(service["build"]), {"context", "dockerfile_inline"})
        self.assertNotIn("args", service["build"])
        self.assertNotIn("NEW_SYNTHETIC_SECRET", json.dumps(document))
        self.assertTrue(all(value.endswith(":-disabled}") for value in service["environment"].values()))

    def test_owned_recipe_and_scope_required(self):
        s = state()
        m.bind_job(job(s), s)
        for key, value in (("docker_compose_raw", "different"), ("environment_id", 999),
                           ("connect_to_docker_network", True), ("name", "unrelated")):
            row = job(s)
            row[key] = value
            with self.assertRaises(m.GuardError):
                m.bind_job(row, s)
        for alter in (lambda row: row["applications"].append(copy.deepcopy(row["applications"][0])),
                      lambda row: row["applications"][0].update(fqdn="https://wrong.invalid"),
                      lambda row: row["applications"][0].update(service_id=404),
                      lambda row: row.update(databases=[{}])):
            row = job(s)
            alter(row)
            with self.assertRaises(m.GuardError):
                m.bind_job(row, s)

    def test_service_payload_is_literal_shown_once_without_build_flags(self):
        value = m.service_env_value("LOKI_S3_PAYLOAD_JSON", "synthetic", True)
        self.assertEqual(set(value), {"key", "value", "is_literal", "is_shown_once"})
        self.assertTrue(value["is_shown_once"])
        self.assertTrue(value["is_literal"])

    def test_journal_cas_must_read_back_exact_private_data(self):
        s = state()
        prior = {"metadata": {"version": 4}, "data": {}}
        with patch.object(m.subprocess, "run", return_value=Mock(returncode=0)) as run, \
                patch.object(m, "vault_read", return_value={"metadata": {"version": 5}, "data": s}):
            result = m.save(s, prior)
            sent = json.loads(run.call_args.kwargs["input"])
            self.assertEqual(sent["options"]["cas"], 4)
            self.assertEqual(result["data"], s)
            self.assertTrue(run.call_args.kwargs["capture_output"])
        with patch.object(m.subprocess, "run", return_value=Mock(returncode=0)), \
                patch.object(m, "vault_read", return_value={"data": {}}):
            with self.assertRaisesRegex(m.GuardError, "journal_readback_failed"):
                m.save(s, prior)

    def test_adoption_requires_completed_real_preflight(self):
        for receipt in ([], [{"mode": "preflight", "status": "FAIL"}], [{"mode": "adopt", "status": "PASS"}]):
            with patch.object(m, "status", return_value={"deployment_status": "finished", "receipts": receipt}):
                with self.assertRaisesRegex(m.GuardError, "preflight_not_passed"):
                    m.adopt(Mock(), {"data": state()})

    def test_adoption_cannot_replay_started_job(self):
        with self.assertRaisesRegex(m.GuardError, "adoption_phase"):
            m.adopt(Mock(), {"data": state("adoption_started")})

    def test_finalize_requires_live_success_receipt(self):
        for phase in ("adoption_started", "runtime_adopted"):
            with patch.object(m, "status", return_value={"receipts": [{"mode": "adopt", "status": "FAIL"}]}):
                api = Mock()
                with self.assertRaisesRegex(m.GuardError, "adoption_not_passed"):
                    m.finalize(api, {"data": state(phase)})
                api.call.assert_not_called()

    def test_finalize_persists_only_two_keys_with_readback_and_no_deployment(self):
        s = state("runtime_adopted")
        stage = Mock()
        before = {"S3_ACCESS_KEY": s["payload"]["expected_old_access"], "S3_SECRET_KEY": s["payload"]["expected_old_secret"],
                  "S3_ENDPOINT": "s3.invalid", "S3_BUCKET": "synthetic-bucket", "S3_REGION": "nbg1"}
        after = {**before, "S3_ACCESS_KEY": s["payload"]["new_access"], "S3_SECRET_KEY": s["payload"]["new_secret"]}
        stage.environment.side_effect = [before, after]
        api = Mock()
        with patch.object(m, "status", return_value={"receipts": [{"mode": "adopt", "status": "PASS"}]}), \
                patch.object(m, "binding"), patch.object(m, "load", return_value=stage), patch.object(m, "save"):
            result = m.finalize(api, {"data": s})
        writes = [x for x in api.call.call_args_list if len(x.args) > 1]
        self.assertEqual(len(writes), 2)
        self.assertEqual({x.args[2]["key"] for x in writes}, {"S3_ACCESS_KEY", "S3_SECRET_KEY"})
        self.assertTrue(all(x.args[0] == "applications/" + m.APP + "/envs" and x.args[1] == "PATCH" for x in writes))
        self.assertTrue(result["canonical_configuration_verified"])
        self.assertFalse(result["stack_deployed"])

    def test_cleanup_refuses_ambiguous_running_adoption(self):
        previous = {"data": state("adoption_started")}
        with patch.object(m, "status"), patch.object(m, "vault_read", return_value=previous), \
                self.assertRaisesRegex(m.GuardError, "ambiguous_adoption_requires_status"):
            m.cleanup(Mock(), previous)

    def test_cleanup_erases_saved_secret_and_protects_shared_networks_volumes(self):
        s = state("config_persisted")
        api = Mock()
        api.call.side_effect = [job(s), [{"key": "LOKI_S3_PAYLOAD_JSON", "uuid": "f" * 24}], {}, [], {}, {}]
        with patch.object(m, "save"):
            result = m.cleanup(api, {"data": s})
        calls = api.call.call_args_list
        self.assertEqual(calls[2].args[0], "services/" + s["job_uuid"] + "/envs/" + "f" * 24)
        path = calls[-1].args[0]
        self.assertIn("delete_volumes=false", path)
        self.assertIn("delete_connected_networks=false", path)
        self.assertIn("docker_cleanup=false", path)
        self.assertTrue(result["saved_payload_removed"])

    def test_uncommitted_source_cannot_enter_recipe(self):
        with patch.object(m.subprocess, "run", return_value=Mock(returncode=1, stdout=b"")):
            with self.assertRaisesRegex(m.GuardError, "committed_worker_required"):
                m.recipe("a" * 40)

    def test_create_uses_service_compose_and_patches_only_runtime_refs(self):
        s = state("prepared")
        stage = Mock()
        stage.environment.return_value = {"S3_ACCESS_KEY": "OLD_SYNTHETIC_ACCESS", "S3_SECRET_KEY": "OLD_SYNTHETIC_SECRET"}
        stage.bao_read.return_value = {"metadata": {"version": 3}, "data": {
            "state": "staged_verified_read", "access_key": "NEW_SYNTHETIC_ACCESS", "secret_key": "NEW_SYNTHETIC_SECRET",
            "endpoint": "nbg1.your-objectstorage.com", "bucket": "logs", "region": "nbg1"}}
        saved, calls = [], []

        def save(value, _previous):
            saved.append(copy.deepcopy(value))
            return {"data": copy.deepcopy(value), "metadata": {"version": len(saved)}}

        def call(path, method="GET", value=None):
            calls.append((path, method, value))
            if path == "applications/" + m.APP:
                return {"uuid": m.APP, "name": "cl8y-logs", "build_pack": "dockercompose", "environment_id": 101,
                        "environment": {"uuid": "d" * 24, "project": {"uuid": "p" * 24}},
                        "destination": {"uuid": "t" * 24, "server": {"uuid": m.SERVER}}}
            if path == "services" and method == "POST":
                self.assertNotIn("connect_to_docker_network", value)
                self.assertEqual(len(saved), 1)
                self.assertNotIn("job_uuid", saved[0])
                document = json.loads(m.base64.b64decode(value["docker_compose_raw"]))
                self.assertEqual(set(document["services"]), {"maintenance"})
                self.assertNotIn("NEW_SYNTHETIC_SECRET", json.dumps(value))
                return {"uuid": s["job_uuid"]}
            if path == "services/" + s["job_uuid"] and method == "GET":
                pending = {**saved[-1], "subapp_uuid": s["subapp_uuid"]}
                return job(pending)
            if path.endswith("/envs") and method == "GET":
                return [{"key": key} for key in m.compose_document("", "b" * 16)["services"]["maintenance"]["environment"]]
            return {}

        api = Mock()
        api.call.side_effect = call
        with patch.object(m, "vault_read", return_value=None), patch.object(m, "save", side_effect=save), \
             patch.object(m, "recipe", return_value="synthetic recipe"), patch.object(m, "load", return_value=stage), \
             patch.object(m.secrets, "token_hex", return_value="b" * 16), \
             patch.object(m.subprocess, "run", return_value=Mock(returncode=0, stdout="a" * 40)):
            result = m.create(api, "a" * 40)
        self.assertEqual(result["phase"], "preflight_started")
        self.assertEqual(calls[-1][:2], ("services/" + s["job_uuid"] + "/start", "POST"))
        self.assertEqual(saved[-1]["phase"], "preflight_started")
        self.assertFalse(any(path.startswith("deployments") or path == "applications/dockerfile" for path, _, _ in calls))
        patches = [value for path, method, value in calls if path.endswith("/envs") and method == "PATCH"]
        self.assertEqual(len(patches), 5)
        self.assertTrue(next(value for value in patches if value["key"] == "LOKI_S3_PAYLOAD_JSON")["is_shown_once"])
        self.assertTrue(all(set(value) == {"key", "value", "is_literal", "is_shown_once"} for value in patches))

    def test_recover_reconciles_exact_owned_service_without_deployment(self):
        s = state("prepared")
        missing = {key: value for key, value in s.items() if key not in ("job_uuid", "job_id", "subapp_uuid")}
        api = Mock()
        api.call.side_effect = [[job(s)], job(s)]
        with patch.object(m, "save") as save:
            result = m.recover(api, {"data": missing})
        self.assertTrue(result["job_recovered"])
        self.assertFalse(result["deployment_started"])
        self.assertEqual(save.call_args.args[0]["job_uuid"], s["job_uuid"])
        self.assertTrue(all(len(call.args) == 1 for call in api.call.call_args_list))
        for rows, reason in (([], "job_recovery_not_found"), ([job(s), job(s)], "job_recovery_ambiguous"),
                             ([{**job(s), "docker_compose_raw": "foreign"}], "job_recovery_not_found")):
            api.call.side_effect = [rows]
            with self.assertRaisesRegex(m.GuardError, reason):
                m.recover(api, {"data": missing})

    def test_status_records_only_verified_safe_terminal_failures(self):
        for runtime_changed, rollback_verified, terminal in ((False, False, True), (True, True, True), (True, False, False)):
            s = state("adoption_started")
            receipt = {"mode": "adopt", "status": "FAIL", "runtime_changed": runtime_changed,
                       "rollback_verified": rollback_verified}
            api = Mock()
            api.call.side_effect = [job(s), {}]
            with patch.object(m, "receipts", return_value=[receipt]), patch.object(m, "save") as save:
                result = m.status(api, {"data": s})
            self.assertEqual(result["phase"], "adoption_failed" if terminal else "adoption_started")
            self.assertEqual(save.called, terminal)
            self.assertEqual(api.call.call_args.args[0], "services/" + s["job_uuid"] + "/logs?sub_service_name=maintenance&lines=60")

    def test_resume_create_revalidates_candidate_recipe_binding_and_absence(self):
        s = state("prepared")
        for key in ("job_uuid", "job_id", "subapp_uuid"):
            s.pop(key)
        stage = Mock()
        stage.environment.return_value = {"S3_ACCESS_KEY": s["payload"]["expected_old_access"],
                                          "S3_SECRET_KEY": s["payload"]["expected_old_secret"]}
        pending = {"metadata": {"version": s["pending_version"]}, "data": {
            "state": "staged_verified_read", "access_key": s["payload"]["new_access"],
            "secret_key": s["payload"]["new_secret"], "endpoint": s["payload"]["endpoint"],
            "bucket": s["payload"]["bucket"], "region": s["payload"]["region"]}}
        stage.bao_read.return_value = pending
        api = Mock()
        api.call.side_effect = [{}, []]
        with patch.object(m, "binding", return_value=s["binding"]), patch.object(m, "load", return_value=stage), \
             patch.object(m, "recipe", return_value="synthetic recipe") as recipe, \
             patch.object(m, "create_service_job", return_value={"resumed": True}) as create:
            self.assertEqual(m.resume_create(api, {"data": s}), {"resumed": True})
        recipe.assert_called_once_with(s["revision"])
        self.assertEqual(create.call_args.args[3], m.compose_source("synthetic recipe", s["transaction"]))
        for rows, change, reason in (([{"name": s["job_name"]}], None, "owned_job_name_exists_use_recover"),
                                     ([], "candidate", "candidate_changed"),
                                     ([], "recipe", "journal_recipe_changed"),
                                     ([{}] * 513, None, "recovery_inventory_bound")):
            record = copy.deepcopy(pending)
            if change == "candidate":
                record["data"]["secret_key"] = "CHANGED_SYNTHETIC_SECRET"
            stage.bao_read.return_value = record
            api.call.side_effect = [{}, rows]
            with patch.object(m, "binding", return_value=s["binding"]), patch.object(m, "load", return_value=stage), \
                 patch.object(m, "recipe", return_value="changed" if change == "recipe" else "synthetic recipe"), \
                 patch.object(m, "create_service_job") as create, self.assertRaisesRegex(m.GuardError, reason):
                m.resume_create(api, {"data": s})
            create.assert_not_called()

    def test_cleanup_accepts_verified_rollback_but_preserves_failed_rollback(self):
        s = state("adoption_failed")
        s["adoption_receipt"] = {"mode": "adopt", "status": "FAIL", "runtime_changed": True, "rollback_verified": True}
        api = Mock()
        api.call.side_effect = [job(s), [], [], {}, {}]
        with patch.object(m, "save"):
            self.assertTrue(m.cleanup(api, {"data": s})["saved_payload_removed"])
        s["phase"] = "adoption_failed"
        s["adoption_receipt"]["rollback_verified"] = False
        with self.assertRaisesRegex(m.GuardError, "ambiguous_adoption_requires_status"):
            m.cleanup(Mock(), {"data": s})

    def test_cleanup_confirmation_distinguishes_record_404_from_container_removal(self):
        s = state("cleaned")
        api = Mock()
        api.call.side_effect = m.GuardError("api_http_404")
        result = m.cleanup_status(api, {"data": s})
        self.assertTrue(result["job_record_removed"])
        self.assertFalse(result["runtime_container_removal_verified"])
        api.call.side_effect = None
        api.call.return_value = job(s)
        self.assertFalse(m.cleanup_status(api, {"data": s})["job_record_removed"])
        api.call.side_effect = m.GuardError("api_http_403")
        with self.assertRaisesRegex(m.GuardError, "cleanup_status_failed"):
            m.cleanup_status(api, {"data": s})


if __name__ == "__main__":
    unittest.main()
