"""Offline controls for online-only migration boundaries; never contact production."""
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
from urllib.parse import parse_qs, urlsplit

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('online_worker', ROOT / 'scripts/ops/cohort-online-worker-1211.py')
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)
REVISION = 'a' * 40

class BaseJob:
    def __init__(self, directory, binary):
        self.directory, self.binary = directory, binary
        directory.mkdir(parents=True, exist_ok=True)
        self.admin = urlsplit('postgresql://postgres:SYNTHETIC_ONLY@127.0.0.1:12345/postgres')
        self.database, self.runtime, self.migrator = 'postgres', 'dex_indexer_runtime_20261006', 'dex_indexer_migrator_20261006'
    def connection(self):
        return {'PATH': '/usr/bin:/bin', 'PGPASSWORD': 'SYNTHETIC_ONLY'}
    def write(self, name, data):
        (self.directory / name).write_text(json.dumps(data))
    def transition(self):
        raise AssertionError('online worker may not transition roles')
    def assert_stopped(self):
        raise AssertionError('online worker may not require a stopped API')
    def apply(self):
        raise AssertionError('online worker may not call legacy restore-capable workflow')

BASE = SimpleNamespace(Job=BaseJob, literal=lambda s: "'" + s.replace("'", "''") + "'",
                       identifier=lambda s: '"' + s + '"')

class Response:
    status = 200
    def __init__(self, value): self.data = json.dumps(value).encode()
    def read(self, limit): return self.data[:limit]
    def __enter__(self): return self
    def __exit__(self, *args): return False

class OnlineControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
    def job(self, readiness=lambda: None):
        return worker.make_job(BASE, self.directory, '/owned/indexer', readiness)

    def test_serving_gate_binds_exact_revision_and_useful_readiness(self):
        calls = []
        def open(request, timeout):
            calls.append((request.full_url, timeout))
            return Response({'status':'ok', 'git_sha':REVISION} if request.full_url.endswith('/health') else {'ready':True})
        worker.serving_ready(REVISION, SimpleNamespace(open=open))
        self.assertEqual(calls, [('https://indexer.dex.cl8y.com/health',5), ('https://indexer.dex.cl8y.com/ready-api',5)])
        for value, guard in [({'status':'ok','git_sha':'b'*40}, 'serving_revision_binding'),
                             ({'status':'bad','git_sha':REVISION}, 'serving_revision_binding')]:
            with self.assertRaisesRegex(RuntimeError, guard):
                worker.serving_ready(REVISION, SimpleNamespace(open=lambda *a, **k: Response(value)))
        with self.assertRaisesRegex(RuntimeError, 'serving_useful_readiness'):
            worker.serving_ready(REVISION, SimpleNamespace(open=lambda request, **k: Response(
                {'status':'ok','git_sha':REVISION} if request.full_url.endswith('/health') else {'ready':False})))

    def test_waiting_has_no_database_or_http_work_and_source_mismatch_refuses(self):
        (self.directory/'source-revision').write_text(REVISION)
        with patch.object(worker, 'load', side_effect=AssertionError('no DB loader')), \
             patch.object(worker.urllib.request, 'build_opener', side_effect=AssertionError('no network')):
            result = worker.run('waiting', {'revision':REVISION}, self.directory, self.directory)
            self.assertEqual(result, {'overall':'WAITING','database_access':False})
            with self.assertRaisesRegex(RuntimeError, 'compiled_source_revision'):
                worker.run('apply', {'revision':'b'*40}, self.directory, self.directory)

    def test_owner_dsn_encodes_role_and_low_lock_statement_deadlines(self):
        job = self.job()
        parsed = urlsplit(job.owner_session_url())
        self.assertEqual(parsed.hostname, '127.0.0.1')
        self.assertEqual(parse_qs(parsed.query)['options'], [
            '-c role=dex_indexer_migrator_20261006 -c lock_timeout=250ms -c statement_timeout=5000ms'])
        with patch.object(worker.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,'t','')) as command:
            self.assertEqual(job.owner_sql('SELECT 1'), 't')
        self.assertIn('role=dex_indexer_migrator_20261006', command.call_args.kwargs['env']['PGOPTIONS'])
        self.assertEqual(command.call_args.kwargs['timeout'], 10)
        self.assertNotIn('SYNTHETIC_ONLY', ' '.join(command.call_args.args[0]))

    def test_readiness_failure_precedes_snapshot_or_migration(self):
        ready = Mock(side_effect=RuntimeError('not_ready'))
        job = self.job(ready)
        job.inspect, job.backup = Mock(), Mock()
        with patch.object(worker.subprocess, 'run') as command, self.assertRaisesRegex(RuntimeError,'not_ready'):
            job.apply_online()
        job.inspect.assert_called_once()
        job.backup.assert_not_called()
        command.assert_not_called()

    def test_migrator_subprocess_is_scoped_and_failure_has_no_restore(self):
        job = self.job()
        job.inspect, job.backup, job.verify = Mock(), Mock(), Mock(return_value={'overall':'PASS'})
        with patch.dict(os.environ, {'DB_ADMIN_URL':'SYNTHETIC_UNRELATED', 'COOLIFY_TOKEN':'SYNTHETIC_UNRELATED'}), \
             patch.object(worker.subprocess, 'run', return_value=subprocess.CompletedProcess([],0,'','')) as command:
            self.assertEqual(job.apply_online()['overall'],'PASS')
        self.assertEqual(command.call_args.args[0], ['/owned/indexer','migrate'])
        self.assertEqual(set(command.call_args.kwargs['env']), {'PATH','DATABASE_URL','INDEXER_RUNTIME_ROLE'})
        self.assertEqual(command.call_args.kwargs['timeout'],900)
        job.verify.reset_mock()
        with patch.object(worker.subprocess, 'run', return_value=subprocess.CompletedProcess([],1,'','SYNTHETIC_PRIVATE_ERROR')), \
             self.assertRaisesRegex(RuntimeError,'scoped_migration_cli_failed'):
            job.apply_online()
        job.verify.assert_not_called()

    def test_readiness_drift_after_snapshot_refuses_before_migration(self):
        ready = Mock(side_effect=[None, RuntimeError('readiness_changed')])
        job = self.job(ready)
        job.inspect, job.backup = Mock(), Mock()
        with patch.object(worker.subprocess, 'run') as command, self.assertRaisesRegex(RuntimeError,'readiness_changed'):
            job.apply_online()
        self.assertEqual(job.inspect.call_count,2)
        job.backup.assert_called_once()
        command.assert_not_called()

    def test_inspection_readonly_options_exist_before_base_construction(self):
        (self.directory/'source-revision').write_text(REVISION)
        base = SimpleNamespace(EXPECTED_CHECKSUMS={worker.TARGET:'fixture'})
        job = SimpleNamespace(sql=lambda query: 't', inspect=lambda: {'migrations':[]})
        def factory(module, directory, binary, ready):
            self.assertIn('default_transaction_read_only=on',os.environ['PGOPTIONS'])
            return job
        with patch.object(worker,'load',return_value=base), patch.object(worker,'make_job',side_effect=factory), \
             patch.dict(os.environ,{},clear=True), patch.object(worker.urllib.request,'build_opener',side_effect=AssertionError('no HTTP')):
            result=worker.run('inspect',{'revision':REVISION},self.directory,self.directory,readiness=lambda:None)
        self.assertTrue(result['database_read_only_enforced'])
        self.assertTrue(job.readonly)

    def test_legacy_stopped_flag_is_refused_before_database_calls(self):
        job=self.job()
        with patch.dict(os.environ,{'OPS_ONLINE_MODE':worker.MODE,'OPS_OLD_INDEXER_STOPPED':'1'}), \
             self.assertRaisesRegex(RuntimeError,'stopped_serving_flag_forbidden'):
            job.inspect()

    def test_snapshot_retry_reuses_exact_original_dump_and_rejects_tamper(self):
        job = self.job()
        job.history = lambda: [{'version':worker.PREVIOUS}]
        job.existing_acl = lambda: {'owned':'fixture'}
        job.role_fingerprint = lambda: 'aggregate-only'
        calls = []
        def command(args, **kwargs):
            calls.append(args)
            if args[0] == 'pg_dump': kwargs['stdout'].write(b'owned-fixture' * 200)
            self.assertTrue(args[0] in {'pg_dump','pg_restore'})
            if args[0] == 'pg_restore': self.assertEqual(args[1], '--list')
            return subprocess.CompletedProcess(args,0,b'',b'')
        with patch.object(worker.subprocess, 'run', side_effect=command):
            first = job.backup()
            second = job.backup()
        self.assertEqual(first,second)
        self.assertEqual(len(calls),2)
        self.assertFalse(first['automatic_database_restore_allowed'])
        (self.directory/'online-before.dump').write_bytes(b'modified')
        with self.assertRaisesRegex(RuntimeError,'immutable_backup_identity'):
            job.backup()

if __name__ == '__main__':
    unittest.main()
