"""Bounded migration-job transport/phase tests; real SQLx coverage is separate."""
import ast
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('migration_ops', ROOT / 'scripts/ops/cohort-migration-1211.py')
ops = importlib.util.module_from_spec(spec); spec.loader.exec_module(ops)
SHA = '1' * 40
BASE = '# Synthetic credential-free guard fixture, long enough for the encoded-source boundary.\nTABLES=set()\nEXPECTED_CHECKSUMS={}\nclass Job: pass\n'
ROUTINE = '''from pathlib import Path
REVISION = 'old'
NEW_VERSION = 1
ALLOWED_MIGRATIONS = {1}
TABLES = {'old'}
BASE_PATH = Path('/old.py')
def make_job(module):
    class RoutineJob(module.Job): pass
    return RoutineJob(Path('/old-state-' + REVISION[:12]), '/usr/local/bin/cl8y-dex-indexer')
'''
TEMPLATE = '\n'.join(['FROM old AS build', 'RUN old-source-build', 'FROM postgres:16-alpine',
    'RUN install-python3', 'RUN printf %s ' + base64.b64encode(BASE.encode()).decode() + ' | base64 -d > /old-base.py',
    'RUN printf %s ' + base64.b64encode(ROUTINE.encode()).decode() + ' | base64 -d > /old-routine.py', 'CMD ["sleep","infinity"]'])
COMPILER = 'FROM rust:1.99-bookworm@sha256:' + 'a' * 64 + ' AS builder'
CHECKSUMS = {20261016120000:'b' * 96, ops.pre.TARGET:'c' * 96}


class FakeApi:
    def __init__(self):
        scope = {'environment_id':1, 'environment':{'uuid':'env','project':{'uuid':'project'}},
            'destination':{'uuid':'dest','server':{'uuid':'server'}}}
        self.template = {**scope, 'dockerfile':TEMPLATE}
        self.backend = {**scope, 'fqdn':'https://indexer.dex.cl8y.com', 'status':'running:healthy'}
        self.database = {**scope, 'postgres_user':'postgres', 'postgres_db':'postgres',
            'postgres_password':'SYNTHETIC_PASSWORD', 'internal_db_url':'postgresql://postgres:SYNTHETIC_PASSWORD@' + ops.pre.DATABASE + ':5432/postgres'}
        self.calls, self.env, self.volumes, self.logs, self.deployments = [], [], [], [], []
        self.start_returns_uuid = True
        self.job = {}

    def call(self, path, method='GET', payload=None):
        self.calls.append((path,method,payload))
        if path=='applications/'+ops.pre.TEMPLATE: return self.template
        if path=='applications/'+ops.pre.BACKEND: return self.backend
        if path=='databases/'+ops.pre.DATABASE: return self.database
        if path=='applications/'+ops.pre.BACKEND+'/envs': return [{'key':'DATABASE_URL','is_runtime':True,'is_buildtime':False}]
        if path=='applications/dockerfile':
            self.job={'name':payload['name'],'description':payload['description'],'dockerfile':base64.b64decode(payload['dockerfile']).decode(),'status':'running:unknown'}
            return {'uuid':'new-job'}
        if path=='applications/new-job': return self.job
        if path=='applications/new-job/envs':
            if method=='POST': self.env.append({**payload,'uuid':'env-'+str(len(self.env))}); return {}
            if method=='PATCH':
                for row in self.env:
                    if row['key']==payload['key'] and not row.get('is_preview'): row.update(payload); return {}
                raise AssertionError('missing variable')
            return self.env
        if path.startswith('applications/new-job/envs/') and method=='DELETE':
            self.env=[row for row in self.env if row['uuid']!=path.rsplit('/',1)[-1]]; return {}
        if path=='applications/new-job/storages':
            if method=='POST':
                storage={**payload,'name':'new-job-'+payload['name'],'uuid':'volume','host_path':None,
                    'resource_id':71,'resource_type':'App\\Models\\Application'}
                self.volumes.append(storage); return dict(storage)
            return {'persistent_storages':self.volumes,'file_storages':[]}
        if path=='applications/new-job/logs?lines=60': return {'logs':[{'output':json.dumps(row)} for row in self.logs]}
        if path=='applications/new-job/start' and method=='POST':
            identity='deployment-'+str(len(self.deployments)+1)
            self.deployments.append({'deployment_uuid':identity,'application_id':'71',
                'application_name':self.job['name'],'application':{'uuid':'new-job','name':self.job['name']},'status':'finished'})
            return {'deployment_uuid':identity} if self.start_returns_uuid else {}
        if path=='deployments/applications/new-job?skip=0&take=5':
            return {'deployments':list(reversed(self.deployments))[:5]}
        if path.startswith('deployments/'):
            identity=path.split('/')[1]
            row=next(row for row in self.deployments if row['deployment_uuid']==identity)
            if path.endswith('/cancel') and method=='POST': row['status']='cancelled-by-user'; return {}
            return row
        if method=='POST' and path.endswith(('/start','/stop')): return {}
        raise AssertionError('unexpected route')


class MigrationOpsTests(unittest.TestCase):
    def setUp(self):
        patches=[patch.object(ops.pre,'TEMPLATE_HASH',hashlib.sha256(TEMPLATE.encode()).hexdigest()),
            patch.object(ops.pre,'BASE_HASH',hashlib.sha256(BASE.encode()).hexdigest()),
            patch.object(ops,'ROUTINE_HASH',hashlib.sha256(ROUTINE.encode()).hexdigest()),
            patch.object(ops.pre,'manifest',return_value=CHECKSUMS),patch.object(ops,'release_manifest',return_value=CHECKSUMS),
            patch.object(ops,'git',return_value=COMPILER.encode())]
        for item in patches: item.start(); self.addCleanup(item.stop)
        self.temp=tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.journal=Path(self.temp.name)/'journal.json'

    def built(self):
        api=FakeApi(); state=ops.build(api,SHA,self.journal)
        return api,state

    def test_plan_gets_only_and_pins_git_compiler_guard_constants_without_secret(self):
        api=FakeApi(); payload,credential,plan=ops.prepare(api,SHA)
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls))
        recipe=base64.b64decode(payload['dockerfile']).decode()
        self.assertIn('git fetch --depth=1 origin '+SHA,recipe)
        self.assertIn('test "$(git rev-parse HEAD)" = '+SHA,recipe)
        self.assertIn(COMPILER.rsplit(' AS ',1)[0],recipe)
        self.assertNotIn('SYNTHETIC_PASSWORD',json.dumps(payload)+json.dumps(plan))
        self.assertIn('SYNTHETIC_PASSWORD',credential)
        sources={}
        for line in recipe.splitlines():
            import re
            match=re.search(r'printf %s ([A-Za-z0-9+/=]+) \| base64 -d > /ops/([A-Za-z0-9_.-]+)',line)
            if match: sources[match[2]]=base64.b64decode(match[1]).decode()
        assignments={n.targets[0].id:ast.literal_eval(n.value) for n in ast.parse(sources['routine.py']).body if isinstance(n,ast.Assign) and isinstance(n.targets[0],ast.Name) and n.targets[0].id in {'REVISION','NEW_VERSION','ALLOWED_MIGRATIONS','TABLES'}}
        self.assertEqual(assignments['REVISION'],SHA)
        self.assertEqual(assignments['ALLOWED_MIGRATIONS'],{ops.pre.TARGET})
        self.assertEqual(assignments['TABLES'],ops.pre.TABLES)
        self.assertIn("Path('/state/cohort-' + REVISION[:12])",sources['routine.py'])
        self.assertEqual(plan['only_allowed_migration'],ops.pre.TARGET)

    def test_build_is_waiting_credential_free_with_unique_private_persistent_volume(self):
        api,state=self.built()
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))
        self.assertEqual(next(row['value'] for row in api.env if row['key']=='OPS_ROUTINE_ACTION'),'waiting')
        self.assertTrue(all(row['is_runtime'] and not row['is_buildtime'] for row in api.env))
        self.assertEqual(api.volumes[0]['type'],'persistent')
        self.assertEqual(api.volumes[0]['name'],'new-job-cohort-new-job')
        self.assertEqual(api.volumes[0]['mount_path'],'/state')
        self.assertIsNone(api.volumes[0]['host_path'])
        self.assertTrue(state['snapshot_storage_configured'])
        self.assertNotIn('SYNTHETIC_PASSWORD',self.journal.read_text())
        self.assertFalse(any(path.startswith('applications/'+ops.pre.BACKEND) and method!='GET' for path,method,_ in api.calls))

    def test_storage_readback_refuses_wrong_scope_uuid_path_host_or_ambiguous_mount(self):
        for mutation in ['name','uuid','path','host','duplicate','extra','file','nestedfile','fileops']:
            with self.subTest(mutation=mutation):
                api=FakeApi(); original=api.call
                def call(path,method='GET',payload=None):
                    result=original(path,method,payload)
                    if path=='applications/new-job/storages' and method=='GET':
                        volume=api.volumes[0]
                        if mutation=='name': volume['name']='other-job-cohort-new-job'
                        elif mutation=='uuid': volume['uuid']='other-volume'
                        elif mutation=='path': volume['mount_path']='/other'
                        elif mutation=='host': volume['host_path']='/host/private'
                        elif mutation=='duplicate': api.volumes.append(dict(volume))
                        elif mutation=='extra': api.volumes.append({**volume,'uuid':'extra-volume','mount_path':'/ops'})
                        elif mutation=='file': result['file_storages']=[{'mount_path':'/state'}]
                        elif mutation=='nestedfile': result['file_storages']=[{'mount_path':'/state/subpath'}]
                        elif mutation=='fileops': result['file_storages']=[{'mount_path':'/ops'}]
                    return result
                api.call=call
                journal=Path(self.temp.name)/(mutation+'.json')
                with self.assertRaisesRegex(ops.pre.GuardError,'private_snapshot_storage_readback'):
                    ops.build(api,SHA,journal)
                self.assertFalse(any(path.endswith('/start') for path,_,_ in api.calls))
                self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))

    def test_apply_refuses_running_serving_app_before_credentials_or_actions(self):
        api,state=self.built(); state['inspection_passed']=True
        calls=len(api.calls)
        with self.assertRaises(ValueError): ops.start_action(api,state,'apply')
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[calls:]))

    def test_inspect_transient_credential_scope_run_binding_and_cleanup(self):
        api,state=self.built()
        ops.start_action(api,state,'inspect')
        secret=next(row for row in api.env if row['key']=='DB_ADMIN_URL')
        self.assertTrue(secret['is_shown_once'] and secret['is_runtime'] and not secret['is_buildtime'])
        row={'schema':'cohort_migration_job_v1','action':'inspect','run_id':state['active_run_id'],'revision':SHA,'overall':'PASS','history_count':1,'pending_versions':[ops.pre.TARGET],'database_read_only_enforced':True}
        api.logs=[row]
        report=ops.status(api,state)
        self.assertTrue(state['inspection_passed'] and report['credential_removed'])
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))

    def test_stale_phase_receipt_cannot_stop_current_migration(self):
        api,state=self.built(); state.update(active_action='apply',active_run_id='current')
        api.logs=[{'schema':'cohort_migration_job_v1','action':'inspect','run_id':'old','revision':SHA,'overall':'PASS','history_count':1,'pending_versions':[ops.pre.TARGET],'database_read_only_enforced':True}]
        calls=len(api.calls)
        result=ops.status(api,state)
        self.assertEqual(result['receipts'],[])
        self.assertFalse(any(method!='GET' for _,method,_ in api.calls[calls:]))
        self.assertFalse(state.get('migration_verified',False))

    def test_full_apply_receipt_validates_backup_denials_and_preservation_before_verified(self):
        api,state=self.built(); state.update(active_action='apply',active_run_id='current')
        row={'schema':'cohort_migration_job_v1','action':'apply','run_id':'current','revision':SHA,'overall':'PASS','history_count':2,'schema_marker':ops.pre.TARGET,'complete_history_preserved':True,'runtime_table_grants':True,'actual_permission_denials':4,'runtime_login_tested':False,'roles_credentials_memberships_unchanged':True,'backup':{'bytes':10000,'sha256':'a'*64,'observed_at_utc':'2026-10-18T00:00:00Z','structurally_valid':True}}
        api.logs=[row]; result=ops.status(api,state)
        self.assertTrue(state['migration_verified'] and result['credential_removed'])
        self.assertEqual(result['receipts'][0]['backup']['sha256'],'a'*64)

    def test_missing_apply_proof_is_terminal_refusal_after_safe_cleanup(self):
        api,state=self.built(); state.update(active_action='apply',active_run_id='current')
        api.logs=[{'schema':'cohort_migration_job_v1','action':'apply','run_id':'current','revision':SHA,'overall':'PASS'}]
        with self.assertRaises(ValueError): ops.status(api,state)
        self.assertFalse(state.get('migration_verified',False))
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))

    def test_log_sanitization_rejects_known_field_injection_and_keeps_nested_safe_backup(self):
        body={'schema':'cohort_migration_job_v1','action':'apply','run_id':'current','revision':SHA,'overall':'FAIL','failure_class':'postgresql://private:SECRET@internal/target','actor':'terra1private'}
        result=ops.receipts({'logs':[json.dumps(body)]})
        self.assertEqual(result[0]['failure_class'],'Error')
        self.assertNotIn('SECRET',json.dumps(result));self.assertNotIn('terra1',json.dumps(result))
        body['history_count']='postgresql://secret'
        self.assertEqual(ops.receipts({'logs':[json.dumps(body)]}),[])

    def test_wrong_recipe_prevents_secret_or_start_action(self):
        api,state=self.built(); api.job['dockerfile']+='\n# modified'
        calls=len(api.calls)
        with self.assertRaises(ValueError): ops.start_action(api,state,'inspect')
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[calls:]))

    def test_queued_or_building_current_action_never_stops_exited_old_container(self):
        for action in ['waiting','inspect','apply','verify']:
            for phase in ['queued','in_progress']:
                with self.subTest(action=action,phase=phase):
                    api,state=self.built()
                    if action!='waiting':
                        api.backend['status']='stopped:unhealthy';state['inspection_passed']=True
                        ops.start_action(api,state,action)
                    api.job['status']='exited:unhealthy';api.deployments[-1]['status']=phase
                    api.logs=[{'schema':'cohort_migration_job_v1','action':action,'run_id':'old','revision':SHA,'overall':'PASS'}]
                    start=len(api.calls); result=ops.status(api,state)
                    self.assertEqual(result['overall'],'PENDING')
                    self.assertEqual(result['receipts'],[])
                    self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))
                    self.assertFalse(state.get('migration_verified',False))
                    self.journal.unlink()

    def test_missing_start_uuid_discovers_only_new_owned_deployment(self):
        api,state=self.built(); old=state['active_deployment_uuid']
        api.start_returns_uuid=False
        ops.start_action(api,state,'inspect')
        self.assertNotIn('active_deployment_uuid',state)
        api.deployments[-1]['status']='in_progress'
        result=ops.status(api,state)
        self.assertEqual(result['overall'],'PENDING')
        self.assertNotEqual(state['active_deployment_uuid'],old)
        # An eventually consistent inventory showing only the previous finished
        # deployment cannot expose a stale receipt or clean up the queued action.
        state.pop('active_deployment_uuid');api.deployments.pop()
        start=len(api.calls);self.assertEqual(ops.status(api,state)['overall'],'PENDING')
        self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))

    def test_deployment_binding_refuses_foreign_application_or_id_before_mutations(self):
        for field in ['uuid','id']:
            api,state=self.built()
            original=api.call
            def call(path,method='GET',payload=None):
                result=original(path,method,payload)
                if path=='deployments/'+state['active_deployment_uuid']:
                    result={**result,'application':dict(result['application'])}
                    if field=='uuid': result['application']['uuid']='other-app'
                    else: result['application_id']='other-id'
                return result
            api.call=call;start=len(api.calls)
            with self.assertRaisesRegex(ops.pre.GuardError,'current_deployment_binding'):
                ops.status(api,state)
            self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))
            self.journal.unlink()

    def test_failed_build_is_terminal_and_cleans_owned_job(self):
        api,state=self.built();api.deployments[-1]['status']='failed'
        result=ops.status(api,state)
        self.assertEqual(result['overall'],'FAIL')
        self.assertEqual(result['cleanup_overall'],'PASS')

    def test_legacy_waiting_build_adoption_is_read_only_and_storage_bound(self):
        for valid in [True,False]:
            api,state=self.built()
            for key in ['active_deployment_uuid','previous_deployment_uuids','job_resource_id']: state.pop(key)
            api.deployments[-1]['status']='in_progress'
            if not valid: api.volumes[0]['resource_id']=72
            start=len(api.calls)
            if valid:
                self.assertEqual(ops.status(api,state)['overall'],'PENDING')
                self.assertEqual(state['active_deployment_uuid'],api.deployments[-1]['deployment_uuid'])
                self.assertEqual(state['job_resource_id'],'71')
            else:
                with self.assertRaisesRegex(ops.pre.GuardError,'current_deployment_binding'):
                    ops.status(api,state)
            self.assertTrue(all(method=='GET' for _,method,_ in api.calls[start:]))
            self.journal.unlink()

    def test_explicit_cleanup_cancels_only_current_owned_pending_deployment(self):
        api,state=self.built();ops.start_action(api,state,'inspect')
        current=state['active_deployment_uuid'];api.deployments[-1]['status']='in_progress'
        result=ops.status(api,state,cleanup=True)
        self.assertEqual(result['cleanup_overall'],'PASS')
        cancelled=[path for path,method,_ in api.calls if method=='POST' and path.endswith('/cancel')]
        self.assertEqual(cancelled,['deployments/'+current+'/cancel'])
        self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))

    def test_pending_cleanup_with_missing_identity_or_cancel_failure_still_removes_credential(self):
        for failure in ['missing','cancel']:
            api,state=self.built();ops.start_action(api,state,'inspect')
            api.deployments[-1]['status']='in_progress'
            original=api.call
            def call(path,method='GET',payload=None):
                if path.endswith('/cancel'): raise OSError('synthetic transport failure')
                return original(path,method,payload)
            api.call=call
            if failure=='missing': api.deployments.pop()
            with self.assertRaises((ops.pre.GuardError,OSError)):
                ops.status(api,state,cleanup=True)
            self.assertEqual(state['cleanup_overall'],'FAIL')
            self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))
            self.journal.unlink()

    def test_explicit_cleanup_binding_failure_still_cleans_owned_application(self):
        for failure in ['inventory','foreign_app','application_id']:
            api,state=self.built();ops.start_action(api,state,'inspect')
            if failure=='inventory': api.deployments[-1]['application_name']='other-app'
            elif failure=='foreign_app': api.deployments[-1]['application']['uuid']='other-app'
            else: api.deployments[-1]['application_id']='72'
            start=len(api.calls)
            with self.assertRaises(ops.pre.GuardError): ops.status(api,state,cleanup=True)
            self.assertEqual(state['cleanup_overall'],'FAIL')
            self.assertFalse(any(row['key']=='DB_ADMIN_URL' for row in api.env))
            self.assertIn(('applications/new-job/stop','POST',None),api.calls[start:])
            self.assertFalse(any(path.endswith('/cancel') for path,_,_ in api.calls[start:]))
            self.journal.unlink()


if __name__=='__main__': unittest.main()
