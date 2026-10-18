"""Transport and credential boundaries for the finite read-only maintenance job."""
import base64
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import sys
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('cohort_ops', ROOT / 'scripts/ops/cohort-preflight-1211.py')
ops = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ops)

BASE = "# Deterministic synthetic maintenance guard for transport tests; contains no credentials.\nTABLES=set()\nEXPECTED_CHECKSUMS={}\nclass Job: pass\n"
TEMPLATE = 'FROM builder\nRUN never-copy-builder\nFROM runtime\nRUN apt-get install python3 postgresql-client\nCOPY --from=builder /binary /binary\nRUN printf %s ' + base64.b64encode(BASE.encode()).decode() + ' | base64 -d > /base.py\nCMD ["sleep","infinity"]\n'


class FakeApi:
    def __init__(self):
        self.calls = []
        self.env = []
        scope = {'environment_id': 1, 'environment': {'uuid': 'environment', 'project': {'uuid': 'project'}},
                 'destination': {'uuid': 'destination', 'server': {'uuid': 'server'}}}
        self.template = {**scope, 'dockerfile': TEMPLATE}
        self.backend = {**scope, 'fqdn': 'https://indexer.dex.cl8y.com'}
        self.database = {**scope, 'postgres_db': 'postgres', 'postgres_user': 'postgres',
            'postgres_password': 'SYNTHETIC_PRIVATE_PASSWORD',
            'internal_db_url': 'postgresql://postgres:SYNTHETIC_PRIVATE_PASSWORD@' + ops.DATABASE + ':5432/postgres'}
        self.job = {'name': '', 'description': '', 'status': 'exited:unhealthy'}

    def call(self, path, method='GET', payload=None):
        self.calls.append((path, method, payload))
        if path == 'applications/' + ops.TEMPLATE: return self.template
        if path == 'applications/' + ops.BACKEND: return self.backend
        if path == 'databases/' + ops.DATABASE: return self.database
        if path == 'applications/' + ops.BACKEND + '/envs':
            return [{'key': 'DATABASE_URL', 'is_runtime': True, 'is_buildtime': False}]
        if path == 'applications/dockerfile':
            self.job.update(name=payload['name'], description=payload['description'], dockerfile=base64.b64decode(payload['dockerfile']).decode())
            return {'uuid': 'owned-job'}
        if path == 'applications/owned-job': return self.job
        if path == 'applications/owned-job/envs':
            if method == 'POST':
                self.env.append({**payload, 'uuid': 'secret-env'})
                return {}
            return self.env
        if path == 'applications/owned-job/envs/secret-env' and method == 'DELETE':
            self.env = [row for row in self.env if row['key'] != 'DB_ADMIN_URL']; return {}
        if path == 'applications/owned-job/logs?lines=40':
            return {'logs': [{'output': json.dumps({'schema': 'cohort_read_only_preflight_v1', 'overall': 'PASS', 'runtime_session_seen': True, 'untrusted': 'SYNTHETIC_PRIVATE_PASSWORD'})}]}
        if path.endswith(('/start', '/stop')) and method == 'POST': return {}
        raise AssertionError('Unexpected route')


class CohortPreflightTests(unittest.TestCase):
    def setUp(self):
        self.patches = [patch.object(ops, 'TEMPLATE_HASH', hashlib.sha256(TEMPLATE.encode()).hexdigest()),
            patch.object(ops, 'BASE_HASH', hashlib.sha256(BASE.encode()).hexdigest()),
            patch.object(ops, 'manifest', return_value={ops.TARGET: 'a' * 96})]
        for item in self.patches: item.start(); self.addCleanup(item.stop)

    def test_plan_is_read_only_no_credentials_exported_and_runtime_only_recipe(self):
        api = FakeApi()
        payload, secret, summary = ops.prepare(api, '1' * 40)
        self.assertTrue(all(method == 'GET' for _, method, _ in api.calls))
        self.assertEqual(secret, api.database['internal_db_url'])
        self.assertNotIn('SYNTHETIC_PRIVATE_PASSWORD', json.dumps(payload) + json.dumps(summary))
        recipe = base64.b64decode(payload['dockerfile'], validate=True).decode()
        self.assertNotIn('builder', recipe)
        self.assertNotIn('infinity', recipe)
        self.assertNotIn('DB_ADMIN_URL=', recipe)
        guard_blob = __import__('re').findall(r'[A-Za-z0-9+/=]{120,}', recipe)[0]
        guard = base64.b64decode(guard_blob).decode()
        self.assertTrue(guard.startswith(BASE))
        __import__('ast').parse(guard, feature_version=(3, 11))
        self.assertFalse(summary['runtime_dsn_api_visible'])
        self.assertFalse(payload['instant_deploy'])
        self.assertFalse(payload['autogenerate_domain'])

    def test_template_checksum_or_resource_binding_change_fails_before_mutation(self):
        for failure in ['template', 'server', 'password', 'database']:
            api = FakeApi()
            if failure == 'template': api.template['dockerfile'] += '# changed'
            elif failure == 'server': api.database['destination'] = {'server': {'uuid': 'other'}}
            elif failure == 'password': api.database['postgres_password'] = 'different'
            else: api.database['postgres_db'] = 'other'
            with self.subTest(failure=failure), self.assertRaises(ValueError): ops.prepare(api, '1' * 40)
            self.assertTrue(all(method == 'GET' for _, method, _ in api.calls))

    def test_create_credential_runtime_scope_and_owned_cleanup_never_touch_backend(self):
        api = FakeApi()
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'journal.json'
            state = ops.create(api, '1' * 40, journal)
            self.assertNotIn('SYNTHETIC_PRIVATE_PASSWORD', journal.read_text())
            self.assertEqual(journal.stat().st_mode & 0o777, 0o600)
            injected = api.env[0]
            self.assertTrue(injected['is_runtime'] and injected['is_shown_once'])
            self.assertFalse(injected['is_buildtime'])
            report = ops.status(api, journal)
            self.assertTrue(report['credential_removed'])
            self.assertFalse(any(row['key'] == 'DB_ADMIN_URL' for row in api.env))
            self.assertEqual({row['key'] for row in api.env}, {'OPS_APPLICATION_DB', 'INDEXER_RUNTIME_ROLE', 'INDEXER_MIGRATOR_ROLE'})
            self.assertNotIn('SYNTHETIC_PRIVATE_PASSWORD', json.dumps(report))
            self.assertFalse(any(path.startswith('applications/' + ops.BACKEND) and method != 'GET' for path, method, _ in api.calls))
            self.assertEqual(state['job_uuid'], 'owned-job')

    def test_changed_job_owner_prevents_cleanup(self):
        api = FakeApi()
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'journal.json'
            ops.create(api, '1' * 40, journal)
            api.job['name'] = 'other'
            with self.assertRaises(ValueError): ops.status(api, journal, True)
            self.assertFalse(any(path.endswith('/stop') for path, _, _ in api.calls))

    def test_lost_start_ack_stops_owned_job_before_removing_credential(self):
        api = FakeApi()
        original = api.call
        def lost_start(path, method='GET', payload=None):
            response = original(path, method, payload)
            if path.endswith('/start'): raise RuntimeError('synthetic lost acknowledgement')
            return response
        api.call = lost_start
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'journal.json'
            with self.assertRaises(RuntimeError): ops.create(api, '1' * 40, journal)
            saved = json.loads(journal.read_text())
            self.assertTrue(saved['credential_removed'] and saved['stop_acknowledged'])
            paths = [path for path, method, _ in api.calls if method != 'GET']
            self.assertLess(paths.index('applications/owned-job/stop'), paths.index('applications/owned-job/envs/secret-env'))

    def test_stop_failure_still_removes_credential_and_records_residual_scope(self):
        api = FakeApi()
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'journal.json'
            ops.create(api, '1' * 40, journal)
            original = api.call
            def failed_stop(path, method='GET', payload=None):
                if path.endswith('/stop'): raise RuntimeError('synthetic stop failure')
                return original(path, method, payload)
            api.call = failed_stop
            result = ops.status(api, journal, True)
            self.assertTrue(result['credential_removed'])
            self.assertFalse(result['stop_acknowledged'])
            self.assertEqual(result['cleanup_overall'], 'FAIL')

    def test_wrong_created_resource_is_bound_before_secret_injection(self):
        api = FakeApi()
        original = api.call
        def wrong_owner(path, method='GET', payload=None):
            response = original(path, method, payload)
            if path == 'applications/owned-job': return {**response, 'name': 'wrong'}
            return response
        api.call = wrong_owner
        with tempfile.TemporaryDirectory() as folder:
            with self.assertRaises(ValueError): ops.create(api, '1' * 40, Path(folder) / 'journal.json')
        self.assertFalse(any(method == 'POST' and path.endswith('/envs') for path, method, _ in api.calls))

    def test_cli_reports_nonzero_when_owned_cleanup_leaves_residual_process(self):
        api = FakeApi()
        with tempfile.TemporaryDirectory() as folder:
            journal = Path(folder) / 'journal.json'
            ops.create(api, '1' * 40, journal)
            original = api.call
            def failed_stop(path, method='GET', payload=None):
                if path.endswith('/stop'): raise RuntimeError('synthetic stop failure')
                return original(path, method, payload)
            api.call = failed_stop
            with patch.object(ops, 'Api', return_value=api), patch.object(ops, 'load_reader'), patch.object(sys, 'argv', ['ops', 'cleanup', '--revision', '1' * 40, '--journal', str(journal)]), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(ops.main(), 1)

    def test_worker_executes_only_selects_and_requires_serving_runtime_session(self):
        worker_spec = importlib.util.spec_from_file_location('cohort_worker', ROOT / 'scripts/ops/cohort-preflight-worker-1211.py')
        worker = importlib.util.module_from_spec(worker_spec)
        worker_spec.loader.exec_module(worker)
        class Job:
            runtime, migrator, database = 'dex_indexer_runtime_20261006', 'dex_indexer_migrator_20261006', 'postgres'
            def __init__(self, seen): self.seen = seen; self.statements = []
            def inspect(self): return {'objects': [{'owner': self.migrator}], 'migrations': [{'version': 20261016120000}]}
            def sql(self, query):
                self.statements.append(query)
                if 'pg_get_userbyid' in query: return self.migrator
                if 'SELECT version' in query: return '20261016120000'
                if 'pg_stat_activity' in query and not self.seen: return 'f'
                return 't'
        for seen in [True, False]:
            job = Job(seen)
            module = SimpleNamespace()
            loader = SimpleNamespace(exec_module=lambda target: target.__dict__.update(Job=lambda *args: job, literal=lambda x: "'" + x + "'", EXPECTED_CHECKSUMS={20261016120000: 'old', ops.TARGET: 'new'}))
            output = io.StringIO()
            with patch.object(worker.importlib.util, 'spec_from_file_location', return_value=SimpleNamespace(loader=loader)), patch.object(worker.importlib.util, 'module_from_spec', return_value=module), patch.object(worker.signal, 'signal'), patch.object(worker.signal, 'alarm'), contextlib.redirect_stdout(output):
                if seen:
                    worker.main()
                    self.assertTrue(json.loads(output.getvalue())['runtime_session_seen'])
                else:
                    with self.assertRaises(ValueError): worker.main()
            self.assertTrue(job.statements and all(query.startswith('SELECT ') for query in job.statements))

    def test_worker_has_no_migration_or_transition_execution(self):
        source = (ROOT / 'scripts/ops/cohort-preflight-worker-1211.py').read_text()
        compile(source, 'worker', 'exec')
        self.assertNotIn('.migrate(', source)
        self.assertNotIn('.transition(', source)
        self.assertNotIn('.assert_stopped(', source)
        self.assertNotIn('CREATE ', source)
        self.assertNotIn('DELETE ', source)
        self.assertNotIn('UPDATE ', source)
        self.assertIn('signal.alarm(55)', source)


if __name__ == '__main__':
    unittest.main()
