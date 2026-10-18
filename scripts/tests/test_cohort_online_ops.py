"""Offline online-coordinator controls; all service transports are synthetic."""
import base64
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

from test_cohort_migration_ops import FakeApi, TEMPLATE, BASE, ROUTINE, COMPILER, CHECKSUMS, SHA
ROOT=Path(__file__).resolve().parents[2]
RELEASE_WORKER=(ROOT/'scripts/ops/cohort-online-worker-1211.py').read_bytes()+b'\n# synthetic exact-release source sentinel\n'
spec=importlib.util.spec_from_file_location('online_ops',ROOT/'scripts/ops/cohort-online-migration-1211.py')
ops=importlib.util.module_from_spec(spec);spec.loader.exec_module(ops)

class ProtectedApi(FakeApi):
    def __init__(self):
        super().__init__();self.backend['uuid']=ops.pre.BACKEND
    def call(self,path,method='GET',payload=None):
        if method!='GET' and any(path.startswith('applications/'+identity) for identity in [ops.pre.BACKEND,ops.pre.TEMPLATE]):
            raise AssertionError('protected serving/template mutation forbidden')
        return super().call(path,method,payload)

class OnlineOpsTests(unittest.TestCase):
    def setUp(self):
        def git(*args):
            if args==('show',SHA+':docker/indexer/Dockerfile'):return COMPILER.encode()
            if args==('show',SHA+':scripts/ops/cohort-online-worker-1211.py'):return RELEASE_WORKER
            raise AssertionError('unexpected Git input')
        for item in [patch.object(ops.pre,'TEMPLATE_HASH',hashlib.sha256(TEMPLATE.encode()).hexdigest()),
            patch.object(ops.pre,'BASE_HASH',hashlib.sha256(BASE.encode()).hexdigest()),
            patch.object(ops.migration,'ROUTINE_HASH',hashlib.sha256(ROUTINE.encode()).hexdigest()),
            patch.object(ops.pre,'manifest',return_value=CHECKSUMS),
            patch.object(ops.migration,'release_manifest',return_value=CHECKSUMS),
            patch.object(ops.migration,'git',side_effect=git),
            patch.object(ops.worker,'serving_ready',return_value=None)]:
            item.start();self.addCleanup(item.stop)
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.counter=0
    def built(self):
        self.counter+=1
        journal=Path(self.temp.name)/(str(self.counter)+'.json')
        api=ProtectedApi();state=ops.build(api,SHA,journal)
        return api,state
    def cleaned(self):
        api,state=self.built()
        api.logs=[{'schema':'cohort_migration_job_v1','mode':ops.worker.MODE,'action':'waiting','run_id':'build',
            'revision':SHA,'overall':'WAITING','database_access':False}]
        ops.migration.status(api,state)
        self.assertEqual(state['cleanup_overall'],'PASS')
        return api,state
    def row(self,state):
        return {'schema':'cohort_migration_job_v1','mode':ops.worker.MODE,'action':state['active_action'],
            'run_id':state['active_run_id'],'revision':SHA,'overall':'PASS','history_count':len(CHECKSUMS),
            'schema_marker':ops.pre.TARGET,'complete_history_preserved':True,'runtime_table_grants':True,
            'actual_permission_denials':4,'runtime_login_tested':False,'roles_credentials_memberships_unchanged':True,
            'existing_acl_unchanged':True,'serving_compatibility_verified':True,
            'backup':{'bytes':10000,'sha256':'a'*64,'observed_at_utc':'2026-10-18T00:00:00Z',
                'structurally_valid':True,'mode':'concurrent_mvcc','automatic_database_restore_allowed':False}}

    def test_build_uses_online_worker_credential_free_waiting_and_bound_volume(self):
        api,state=self.built()
        self.assertEqual(state['mode'],ops.worker.MODE)
        self.assertNotIn('SYNTHETIC_PASSWORD',json.dumps(state))
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))
        self.assertEqual(next(row['value'] for row in api.env if row['key']=='OPS_ONLINE_MODE'),ops.worker.MODE)
        self.assertNotIn(' > /ops/routine.py',api.job['dockerfile'])
        encoded=next(line.split()[3] for line in api.job['dockerfile'].splitlines() if line.endswith(' > /ops/cohort-migrate.py'))
        self.assertEqual(base64.b64decode(encoded),RELEASE_WORKER)
        self.assertEqual(api.volumes[0]['name'],'new-job-cohort-new-job')
        self.assertNotEqual(state.get('cleanup_overall'),'PASS')

    def test_unresolved_prior_operation_refuses_before_any_api_or_credential_access(self):
        for broken in [{},{'cleanup_overall':'PASS','credential_removed':False,'stop_acknowledged':True},
            {'cleanup_overall':'PASS','credential_removed':True,'stop_acknowledged':False}]:
            api,state=self.built();state.update(broken);start=len(api.calls)
            with patch.object(ops.pre,'prepare') as credential, self.assertRaisesRegex(ops.pre.GuardError,'prior_online_operation_unresolved'):
                ops.start_action(api,state,'inspect')
            self.assertEqual(len(api.calls),start);credential.assert_not_called()

    def test_useful_exact_revision_gate_precedes_credential_retrieval(self):
        api,state=self.cleaned();start=len(api.calls)
        with patch.object(ops.worker,'serving_ready',side_effect=RuntimeError('not_useful')) as ready, \
             patch.object(ops.pre,'prepare') as credential, self.assertRaisesRegex(RuntimeError,'not_useful'):
            ops.start_action(api,state,'inspect')
        ready.assert_called_once_with(SHA);credential.assert_not_called()
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))

    def test_successful_start_never_stops_serving_and_reentrant_start_is_refused(self):
        api,state=self.cleaned();state['inspection_passed']=True;start=len(api.calls)
        ops.start_action(api,state,'apply')
        self.assertEqual(state['cleanup_overall'],'PENDING')
        self.assertFalse(state['credential_removed']);self.assertFalse(state['stop_acknowledged'])
        self.assertFalse(any(path.endswith('/stop') for path,_,_ in api.calls[start:]))
        secret=next(row for row in api.env if row['key']=='DB_ADMIN_URL')
        self.assertTrue(secret['is_shown_once'] and secret['is_runtime'] and not secret['is_buildtime'])
        later=len(api.calls)
        with self.assertRaisesRegex(ops.pre.GuardError,'prior_online_operation_unresolved'):
            ops.start_action(api,state,'verify')
        self.assertEqual(len(api.calls),later)

    def test_accepted_then_transport_throw_keeps_owner_for_exact_status_reconciliation(self):
        api,state=self.cleaned();state['inspection_passed']=True
        previous=state['active_deployment_uuid'];original=api.call
        def ambiguous(path,method='GET',payload=None):
            result=original(path,method,payload)
            if path=='applications/new-job/start' and method=='POST':raise TimeoutError('dispatch_ambiguous')
            return result
        api.call=ambiguous;start=len(api.calls)
        try:ops.start_action(api,state,'apply')
        except TimeoutError:pass
        self.assertEqual(state['cleanup_overall'],'PENDING')
        self.assertFalse(state['credential_removed']);self.assertFalse(state['stop_acknowledged'])
        self.assertNotIn('active_deployment_uuid',state)
        self.assertIn(previous,state['previous_deployment_uuids'])
        self.assertFalse(any(path.endswith('/stop') or method=='DELETE' for path,method,_ in api.calls[start:]))
        self.assertTrue(any(row['key']=='DB_ADMIN_URL' for row in api.env))
        report=ops.migration.status(api,state)
        self.assertEqual(report['overall'],'PENDING')
        self.assertNotEqual(state['active_deployment_uuid'],previous)
        later=len(api.calls)
        with self.assertRaisesRegex(ops.pre.GuardError,'prior_online_operation_unresolved'):
            ops.start_action(api,state,'apply')
        self.assertEqual(len(api.calls),later)

    def test_online_and_offline_action_journals_cannot_cross(self):
        api,state=self.cleaned();start=len(api.calls)
        with self.assertRaisesRegex(ops.pre.GuardError,'offline_action_mode'):
            ops.migration.start_action(api,state,'inspect')
        state.pop('mode')
        with self.assertRaisesRegex(ops.pre.GuardError,'online_action_mode'):
            ops.start_action(api,state,'inspect')
        self.assertEqual(len(api.calls),start)
        journal=Path(self.temp.name)/'offline.json';journal.write_text(json.dumps(state))
        with patch('sys.argv',['online','status','--revision',SHA,'--journal',str(journal)]), \
             patch.object(ops.pre,'load_reader',return_value=None), patch.object(ops.migration,'Api',return_value=api), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(ops.main(),1)
        self.assertIn('online_journal_binding',output.getvalue())
        self.assertEqual(len(api.calls),start)

    def test_online_pass_requires_all_compatibility_acl_and_no_restore_proofs(self):
        mutations=['compatibility','acl','restore','snapshot_mode']
        for mutation in [None,*mutations]:
            with self.subTest(mutation=mutation):
                api,state=self.cleaned();state['inspection_passed']=True;ops.start_action(api,state,'apply')
                row=self.row(state)
                if mutation=='compatibility':row.pop('serving_compatibility_verified')
                if mutation=='acl':row.pop('existing_acl_unchanged')
                if mutation=='restore':row['backup']['automatic_database_restore_allowed']=True
                if mutation=='snapshot_mode':row['backup']['mode']='offline'
                api.logs=[row]
                if mutation:
                    with self.assertRaises(ops.pre.GuardError):ops.migration.status(api,state)
                    self.assertFalse(state.get('migration_verified',False))
                else:
                    result=ops.migration.status(api,state)
                    self.assertTrue(state['migration_verified'] and result['credential_removed'])
                self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))

    def test_wrong_mode_receipt_cannot_verify_and_wait_is_finite(self):
        api,state=self.cleaned();state['inspection_passed']=True;ops.start_action(api,state,'apply')
        row=self.row(state);row.pop('mode');api.logs=[row];start=len(api.calls)
        with patch.object(ops.migration.time,'time',return_value=1000):
            result=ops.migration.status(api,state)
        self.assertEqual(result['overall'],'PENDING');self.assertEqual(result['receipts'],[])
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))
        with patch.object(ops.migration.time,'time',return_value=1915):
            result=ops.migration.status(api,state)
        self.assertEqual(result['overall'],'FAIL');self.assertEqual(result['failure_class'],'terminal_receipt_timeout')
        self.assertFalse(state.get('migration_verified',False))
        self.assertEqual(result['cleanup_overall'],'PASS')

    def test_storage_and_deployment_binding_refuse_foreign_owned_job_surfaces(self):
        api=ProtectedApi();original=api.call
        def changed(path,method='GET',payload=None):
            value=original(path,method,payload)
            if path=='applications/new-job/storages' and method=='GET':value['persistent_storages'][0]['host_path']='/foreign'
            return value
        api.call=changed
        with self.assertRaisesRegex(ops.pre.GuardError,'private_snapshot_storage_readback'):
            ops.build(api,SHA,Path(self.temp.name)/'storage.json')
        self.assertFalse(any(path.endswith('/start') for path,_,_ in api.calls))
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))
        api,state=self.built();api.deployments[-1]['application']['uuid']='foreign-app';start=len(api.calls)
        with self.assertRaisesRegex(ops.pre.GuardError,'current_deployment_binding'):
            ops.migration.status(api,state)
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))

    def test_protected_cleanup_and_owned_binding_refuse_before_any_api(self):
        for identity in [ops.pre.BACKEND,ops.pre.TEMPLATE]:
            api=Mock()
            with self.assertRaises(ops.pre.GuardError):ops.pre.cleanup_owned(api,identity)
            api.call.assert_not_called()
            with self.assertRaises(ops.pre.GuardError):ops.migration.owned(api,{'job_uuid':identity})
            api.call.assert_not_called()

    def test_forbidden_stop_serving_returns_before_reader_or_api_construction(self):
        with patch('sys.argv',['offline','stop-serving','--revision',SHA,'--journal','/nonexistent']), \
             patch.object(ops.migration.pre,'load_reader',side_effect=AssertionError('no credential lookup')), \
             contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(ops.migration.main(),1)
        self.assertIn('production_api_stop_forbidden',output.getvalue())

if __name__=='__main__':unittest.main()
