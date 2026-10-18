#!/usr/bin/env python3
"""Monitoring admin boundaries: fake API state only; no credentials or remote calls."""
import contextlib
import copy
import importlib.util
import io
import json
import subprocess
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from urllib.error import HTTPError

SPEC = importlib.util.spec_from_file_location('dex_monitoring_ops', Path(__file__).resolve().parents[1] / 'ops/dex-monitoring-1544.py')
OPS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OPS)
REVISION = 'a' * 40
APP = 'monitor1544'


def owned_row():
    return {'uuid': APP, 'name': OPS.NAME, 'git_repository': OPS.REPOSITORY,
            'build_pack': 'dockercompose', 'git_branch': 'main', 'docker_compose_location': '/monitoring/dex/compose.yaml',
            'destination_id': 0, 'environment_id': 19, 'git_commit_sha': REVISION,
            'status': 'running:healthy', 'id': 1544}


class FakeAPI(OPS.Coolify):
    def __init__(self, inventory=None):
        self.calls = []
        self.inventory = [] if inventory is None else inventory
        self.row = owned_row()
        self.history = {'deployments': []}
        self.detail = {}

    def api(self, path, method='GET', payload=None):
        self.calls.append((path, method, copy.deepcopy(payload)))
        if path == 'applications':
            return copy.deepcopy(self.inventory)
        if path == 'applications/' + OPS.LOGS:
            return {'uuid': OPS.LOGS, 'name': 'cl8y-logs', 'destination_id': 0, 'environment_id': 19}
        if path == 'applications/' + APP:
            return copy.deepcopy(self.row)
        if path == 'applications/public' and method == 'POST':
            return {'uuid': APP}
        if path == 'deployments/applications/' + APP + '?skip=0&take=1':
            return copy.deepcopy(self.history)
        if path.startswith('deployments/'):
            return copy.deepcopy(self.detail)
        raise AssertionError('unexpected fake API route')


class GuardTests(unittest.TestCase):
    def test_rejects_logs_and_malformed_app_ids_before_any_request(self):
        for app in (OPS.LOGS, '../indexer', 'indexer?force=true', '', None):
            with self.subTest(app=app):
                api = FakeAPI()
                with self.assertRaises(ValueError):
                    api.owned(app)
                self.assertEqual(api.calls, [])

    def test_rejects_other_resource_bindings(self):
        for field, wrong in [('uuid', 'actual-indexer'), ('name', 'cl8y-dex-indexer'),
                             ('git_repository', 'https://example.invalid/other.git'),
                             ('build_pack', 'dockerfile'), ('git_branch', 'feature/unreviewed'), ('git_commit_sha', 'main'), ('docker_compose_location', '/compose.yaml'),
                             ('destination_id', 1), ('environment_id', 20)]:
            with self.subTest(field=field):
                api = FakeAPI()
                api.row[field] = wrong
                with self.assertRaises(ValueError):
                    api.owned(APP)
                self.assertTrue(all(method == 'GET' for _, method, _ in api.calls))

    def test_duplicate_monitor_names_cannot_create_or_deploy(self):
        api = FakeAPI([{'name': OPS.NAME, 'uuid': APP}, {'name': OPS.NAME, 'uuid': 'duplicate'}])
        with patch.object(OPS.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, stdout=b'fixture')):
            with self.assertRaisesRegex(ValueError, 'duplicate_monitor'):
                OPS.create(api, REVISION)
        self.assertTrue(all(method == 'GET' for _, method, _ in api.calls))

    def test_existing_monitor_revision_mismatch_is_read_only(self):
        api = FakeAPI([{'name': OPS.NAME, 'uuid': APP}])
        api.row['git_commit_sha'] = 'b' * 40
        with patch.object(OPS.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, stdout=b'fixture')):
            with self.assertRaisesRegex(ValueError, 'existing_revision_mismatch'):
                OPS.create(api, REVISION)
        self.assertTrue(all(method == 'GET' for _, method, _ in api.calls))

    def test_create_payload_uses_requested_revision_tree_not_working_tree(self):
        api = FakeAPI()
        immutable_compose = '# exact requested revision\nservices: {}\n'
        def git_run(argv, **kwargs):
            if argv[:2] == ['git', 'show']:
                self.assertIn(REVISION + ':monitoring/dex/compose.yaml', argv)
            return subprocess.CompletedProcess(argv, 0, stdout=immutable_compose.encode())
        with patch.object(OPS.subprocess, 'run', side_effect=git_run):
            result = OPS.create(api, REVISION)
        payload = next(payload for _, method, payload in api.calls if method == 'POST')
        self.assertEqual(payload['git_commit_sha'], REVISION)
        self.assertEqual(payload['docker_compose_raw'], immutable_compose)
        self.assertFalse(payload['instant_deploy'])
        self.assertFalse(payload['is_auto_deploy_enabled'])
        self.assertEqual(result['app_uuid'], APP)
        self.assertFalse(any('deploy?' in path for path, _, _ in api.calls))

    def test_bad_revision_is_rejected_before_any_api_request(self):
        for revision in ('main', 'a' * 39, 'a' * 40 + ';bad', None):
            api = FakeAPI()
            with self.subTest(revision=revision), self.assertRaises(ValueError):
                OPS.create(api, revision)
            self.assertEqual(api.calls, [])

    def test_status_cannot_follow_unvalidated_deployment_identity(self):
        api = FakeAPI()
        api.history = {'deployments': [{'deployment_uuid': '../other?secret=credential',
                                        'status': 'finished', 'commit': REVISION}]}
        with self.assertRaises(ValueError):
            OPS.status(api, APP)
        self.assertFalse(any('secret=' in path for path, _, _ in api.calls))

    def test_status_rejects_unexpected_classes_and_foreign_deployment(self):
        for field, bad in [('status', 'credential-private-body'), ('git_commit_sha', 'credential-private-body')]:
            api = FakeAPI()
            api.row[field] = bad
            with self.subTest(field=field), self.assertRaises(ValueError):
                OPS.status(api, APP)
        api = FakeAPI()
        api.history = {'deployments': [{'deployment_uuid': 'deployment1544', 'status': 'finished', 'commit': REVISION}]}
        api.detail = {'deployment_uuid': 'deployment1544', 'application_id': 999, 'commit': REVISION}
        with self.assertRaisesRegex(ValueError, 'deployment_application_binding'):
            OPS.status(api, APP)

    def test_status_exports_only_fixed_fields_and_log_counts(self):
        api = FakeAPI()
        api.history = {'deployments': [{'deployment_uuid': 'deployment1544', 'status': 'finished', 'commit': REVISION,
                                       'created_at': 'credential-private-body'}]}
        api.detail = {'deployment_uuid': 'deployment1544', 'application_id': 1544, 'commit': REVISION,
                      'logs': json.dumps([{'output': 'ERROR credential-private-body'}, {'output': 'private-actor'}])}
        result = OPS.status(api, APP)
        self.assertEqual(result['deployment_log_entries'], 2)
        self.assertEqual(result['deployment_error_entries'], 1)
        self.assertNotIn('private', json.dumps(result))
        self.assertEqual(set(result['deployment']), {'deployment_uuid', 'status', 'commit'})

    def test_unavailable_or_malformed_deployment_logs_are_not_an_empty_success(self):
        for logs in (None, '', 'not-json', {}, [{}], [{'output': 7}], [{'output': 'x'}] * 4097):
            api = FakeAPI()
            api.history = {'deployments': [{'deployment_uuid': 'deployment1544', 'status': 'finished', 'commit': REVISION}]}
            api.detail = {'deployment_uuid': 'deployment1544', 'application_id': 1544, 'commit': REVISION, 'logs': logs}
            with self.subTest(shape=type(logs).__name__), self.assertRaisesRegex(ValueError, 'deployment_logs_unavailable'):
                OPS.status(api, APP)

    def test_provision_rejects_duplicate_or_wrong_datasource_before_mutation(self):
        for sources in ([{'uid': OPS.UID}, {'uid': OPS.UID}],
                        [{'uid': OPS.UID, 'type': 'loki', 'url': 'http://dex-prometheus:9090'}],
                        [{'uid': OPS.UID, 'type': 'prometheus', 'url': 'https://wrong.invalid'}]):
            calls = []
            def exchange(opener, url, authorization, method='GET', payload=None):
                calls.append((url, method))
                self.assertEqual(method, 'GET', 'must reject inventory before any Grafana writes')
                return sources
            fake = SimpleNamespace(reader=SimpleNamespace(opener=object()), authorization='fixture-private')
            with self.subTest(sources=sources), patch.object(OPS.MONITOR, 'Monitor', return_value=fake), patch.object(OPS, 'exchange', side_effect=exchange):
                with self.assertRaises(ValueError):
                    OPS.provision()
            self.assertEqual(len(calls), 1)

    def test_http_error_never_reads_or_prints_sensitive_body(self):
        body = io.BytesIO(b'credential-private-body')
        error = HTTPError('https://private.invalid', 403, 'credential-private-message', {}, body)
        class Opener:
            def open(self, *args, **kwargs):
                raise error
        with self.assertRaisesRegex(ValueError, '^http_status_403$'):
            OPS.exchange(Opener(), 'https://private.invalid', 'private-auth')
        self.assertEqual(body.tell(), 0)

    def test_main_diagnostics_never_include_exception_bodies(self):
        for error in (RuntimeError('credential-private-body'), ValueError('credential-private-body')):
            output = io.StringIO()
            with patch.object(sys, 'argv', ['ops', 'verify']), patch.object(OPS.MONITOR, 'Monitor', side_effect=error), contextlib.redirect_stdout(output):
                code = OPS.main()
            self.assertEqual(code, 1)
            self.assertNotIn('credential', output.getvalue())
            self.assertEqual(json.loads(output.getvalue())['overall'], 'FAIL')

    def test_exchange_bounds_success_body(self):
        class Response:
            def __enter__(self): return self
            def __exit__(self, *args): return None
            def read(self, limit): return b'x' * limit
        class Opener:
            def open(self, *args, **kwargs): return Response()
        with self.assertRaisesRegex(ValueError, 'body_bound'):
            OPS.exchange(Opener(), 'https://private.invalid', 'private-auth')


if __name__ == '__main__':
    unittest.main()
