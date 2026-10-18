#!/usr/bin/env python3
"""Offline mutation-boundary and ambiguous-ack controls; never contact Coolify."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('release',ROOT/'scripts/ops/api-rolling-release.py')
ops=importlib.util.module_from_spec(spec);spec.loader.exec_module(ops)
e=ops.e
TOKEN='terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
PROFILE=[{'token_in':'uusd','token_out':TOKEN,'amount_in':'1000000','max_spread':'0.01'}]
OLD='0'*40;NEW='1'*40
MANIFEST={'sha256':'a'*64}

class FakeResponse:
    def __init__(self,value):self.data=json.dumps(value).encode()
    def __enter__(self):return self
    def __exit__(self,*args):pass
    def read(self,limit):return self.data[:limit]

class FakeReader:
    api_base='https://owned.invalid/api/v1/';token='PRIVATE_TOKEN'
    def __init__(self):
        self.reads=0;self.calls=[];self.writes=[];self.opener=self;self.ack=None;self.journal=None;self.corrupt_readback=False
        self.app={'uuid':e.APP,'name':e.NAME,'fqdn':'https://'+e.DOMAIN,'git_repository':e.REPOSITORY,'git_branch':'main','git_commit_sha':OLD,'status':'running:healthy',
            'destination_type':'App\\Models\\StandaloneDocker','ports_mappings':None,'custom_docker_run_options':None,'health_check_enabled':False,'custom_healthcheck_found':True,
            'settings':{'is_consistent_container_name_enabled':False,'is_auto_deploy_enabled':False,'use_build_secrets':True},'destination':{'server':{'proxy':{'type':'TRAEFIK','status':'running'}}}}
        self.history=[{'deployment_uuid':'old-deploy','application_id':'71','status':'finished','commit':OLD}]
        self.details={'old-deploy':{**self.history[0],'application':{'uuid':e.APP,'name':e.NAME},'logs':[]}}
        self.env=[{'key':'API_READY_SCENARIOS','value':e.canonical(PROFILE).decode(),'is_preview':False,'is_runtime':True,'is_buildtime':False,'is_literal':True,'is_shown_once':False},
            {'key':'DATABASE_URL','value':'PRIVATE_DB_PASSWORD','is_buildtime':False,'is_runtime':True}]
        self.health={'status':'ok','git_sha':OLD};self.ready={'ready':True};self.active=[]
    def api(self,path):
        self.reads+=1;self.calls.append(('GET',path))
        if path=='applications/'+e.APP:return copy.deepcopy(self.app)
        if path=='applications/'+e.APP+'/envs':return copy.deepcopy(self.env)
        if path=='deployments/applications/'+e.APP+'?skip=0&take=3':return {'deployments':copy.deepcopy(self.history)}
        if path=='deployments':return copy.deepcopy(self.active)
        if path.startswith('deployments/') and path[12:] in self.details:return copy.deepcopy(self.details[path[12:]])
        raise AssertionError('unscoped GET: '+path)
    def read(self,url,limit,private=False):
        self.reads+=1;self.calls.append(('GET',url))
        if url==self.api_base+'version':return b'4.4.6'
        if url=='https://'+e.DOMAIN+'/health':return json.dumps(self.health).encode()
        if url=='https://'+e.DOMAIN+'/ready-api':return json.dumps(self.ready).encode()
        raise AssertionError('unscoped public GET')
    def open(self,request,timeout):
        path=request.full_url.removeprefix(self.api_base)
        payload=json.loads(request.data) if request.data else None
        self.writes.append((request.method,path,payload));self.calls.append((request.method,path))
        if path=='applications/'+e.APP+'/envs':
            if not self.corrupt_readback:self.env=[row for row in self.env if row['key']!='API_READY_SCENARIOS']+[copy.deepcopy(payload)]
        elif path=='applications/'+e.APP:
            if not self.corrupt_readback:self.app.update(payload)
        elif path=='deploy?uuid='+e.APP+'&force=false':
            assert self.journal is not None and e.read_json(self.journal,private=True)['deploy_requested'] is True
            if isinstance(self.ack,Exception):raise self.ack
            return FakeResponse(self.ack or {'deployments':[{'deployment_uuid':'new-deploy','resource_uuid':e.APP}]})
        else:raise AssertionError('unscoped mutation')
        return FakeResponse({})
    def finished(self,phase='finished'):
        row={'deployment_uuid':'new-deploy','application_id':'71','status':phase,'commit':NEW}
        self.history.insert(0,row);self.details['new-deploy']={**row,'application':{'uuid':e.APP,'name':e.NAME},'logs':[{'output':'normal PRIVATE_LOG_VALUE'}]}
        if phase=='finished':self.health['git_sha']=NEW

class ReleaseControls(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.journal=Path(self.temp.name)/'journal.json';self.r=FakeReader();self.r.journal=self.journal
        self.state={'schema':'api_rolling_release_v1','application_uuid':e.APP,'revision':NEW,'source_manifest_sha256':MANIFEST['sha256'],
            'profile_sha256':__import__('hashlib').sha256(e.canonical(PROFILE)).hexdigest(),'old_commit':OLD,'prior_git_commit_sha':OLD}
    def writer(self):return ops.Writer(self.r)
    def pinned(self):self.r.app['git_commit_sha']=NEW;self.state['pinned']=True
    def test_writer_refuses_every_nonwhitelisted_mutation_without_transport(self):
        invalid=[('applications/'+e.APP+'/stop','POST',None),('applications/'+e.APP+'/restart','POST',None),('deployments/x/cancel','POST',None),
            ('applications/'+e.APP+'/rollback','POST',{'commit':OLD}),('deploy?uuid='+e.APP+'&force=true','POST',None),
            ('applications/foreign','PATCH',{'git_commit_sha':NEW}),('applications/'+e.APP,'PATCH',{'git_commit_sha':NEW,'health_check_enabled':True}),
            ('applications/'+e.APP+'/envs','POST',{'key':'DATABASE_URL','value':'PRIVATE'})]
        for args in invalid:
            with self.subTest(args=args),self.assertRaises(e.Refusal):self.writer().call(*args)
        self.assertEqual(self.r.writes,[])
    def test_writer_allows_only_one_mutation_per_instance(self):
        writer=self.writer();writer.call('applications/'+e.APP,'PATCH',{'git_commit_sha':NEW})
        with self.assertRaisesRegex(e.Refusal,'one_release_mutation'):writer.call('applications/'+e.APP,'PATCH',{'git_commit_sha':NEW})
        self.assertEqual(len(self.r.writes),1)
    def test_plan_is_get_only_and_journal_binds_revision_proof_and_profile(self):
        self.r.env.pop(0)
        self.assertEqual(ops.plan(self.r,NEW,MANIFEST,PROFILE,self.journal)['overall'],'PREPARED')
        self.assertEqual(self.r.writes,[]);self.assertEqual(self.journal.stat().st_mode&0o777,0o600)
        ops.state_for(self.journal,NEW,MANIFEST,PROFILE)
        for revision,manifest,profile in [(OLD,MANIFEST,PROFILE),(NEW,{'sha256':'b'*64},PROFILE),(NEW,MANIFEST,PROFILE*2)]:
            with self.assertRaises(e.Refusal):ops.state_for(self.journal,revision,manifest,profile)
    def test_profile_is_only_runtime_public_key_readback_and_never_restarts(self):
        self.r.env.pop(0);db=copy.deepcopy(self.r.env)
        result=ops.configure_profile(self.r,self.writer(),self.state,PROFILE)
        self.assertFalse(result['restart_requested']);self.assertTrue(self.state['profile_configured'])
        self.assertEqual(len(self.r.writes),1);method,path,payload=self.r.writes[0]
        self.assertEqual((method,path),('POST','applications/'+e.APP+'/envs'))
        self.assertEqual(payload,{'key':'API_READY_SCENARIOS','value':e.canonical(PROFILE).decode(),'is_runtime':True,'is_buildtime':False,'is_preview':False,'is_literal':True,'is_shown_once':False})
        self.assertEqual([row for row in self.r.env if row['key']=='DATABASE_URL'],db)
    def test_profile_readback_failure_does_not_claim_success(self):
        self.r.env.pop(0);self.r.corrupt_readback=True
        with self.assertRaisesRegex(e.Refusal,'profile_readback'):ops.configure_profile(self.r,self.writer(),self.state,PROFILE)
        self.assertNotIn('profile_configured',self.state);self.assertEqual(len(self.r.writes),1)
    def test_unsafe_revision_settings_or_privileged_key_refuse_before_mutation(self):
        for changed in ['source','pin','privileged','active']:
            self.setUp()
            if changed=='source':self.state['old_commit']='2'*40
            elif changed=='pin':self.r.app['git_commit_sha']='2'*40
            elif changed=='privileged':self.r.env.append({'key':'MIGRATION_DATABASE_URL','value':'PRIVATE'})
            else:self.r.active=[{'application_id':'71','status':'queued'}]
            with self.assertRaises(e.Refusal):ops.pin(self.r,self.writer(),self.state,PROFILE)
            self.assertEqual(self.r.writes,[])
    def test_pin_exact_revision_readback_no_restart_and_idempotent(self):
        self.assertFalse(ops.pin(self.r,self.writer(),self.state,PROFILE)['restart_requested'])
        self.assertEqual(self.r.writes,[('PATCH','applications/'+e.APP,{'git_commit_sha':NEW})])
        self.r.reads=0  # A second CLI phase constructs a fresh Reader/budget.
        ops.pin(self.r,self.writer(),self.state,PROFILE);self.assertEqual(len(self.r.writes),1)
    def test_deploy_refuses_without_journal_pin_and_actual_exact_pin(self):
        for actual,pinned in [(OLD,False),(OLD,True),(NEW,False)]:
            self.r.app['git_commit_sha']=actual;self.state['pinned']=pinned
            with self.assertRaises(e.Refusal):ops.deploy(self.r,self.writer(),self.state,PROFILE,self.journal)
        self.assertEqual(self.r.writes,[])
    def test_deploy_persists_before_post_and_ack_never_redeploys(self):
        self.pinned();result=ops.deploy(self.r,self.writer(),self.state,PROFILE,self.journal)
        self.assertEqual(result['deployment_uuid'],'new-deploy')
        self.assertEqual(self.r.writes,[('POST','deploy?uuid='+e.APP+'&force=false',None)])
        self.assertEqual(ops.deploy(self.r,self.writer(),self.state,PROFILE,self.journal)['overall'],'ALREADY_REQUESTED')
        self.assertEqual(len(self.r.writes),1)
    def test_lost_or_foreign_ack_is_ambiguous_and_cannot_retry_or_cancel(self):
        for ack in [TimeoutError('PRIVATE_NETWORK_ERROR'),{'deployments':[{'deployment_uuid':'new-deploy','resource_uuid':'foreign'}]}, {'deployments':[]}]:
            self.setUp();self.pinned();self.r.ack=ack
            with self.assertRaises((TimeoutError,e.Refusal)):ops.deploy(self.r,self.writer(),self.state,PROFILE,self.journal)
            self.assertTrue(e.read_json(self.journal,private=True)['deploy_requested'])
            with self.assertRaisesRegex(e.Refusal,'ambiguous_deployment'):ops.deploy(self.r,self.writer(),self.state,PROFILE,self.journal)
            self.assertEqual(len(self.r.writes),1)
    def test_scoped_history_reconciles_only_one_new_exact_release(self):
        self.state.update(deploy_requested=True,previous_deployment_uuids=['old-deploy'])
        self.assertTrue(ops.status(self.r,self.state)['acknowledgement_unresolved'])
        self.r.finished('in_progress');result=ops.status(self.r,self.state)
        self.assertEqual(result['deployment_uuid'],'new-deploy');self.assertEqual(result['overall'],'PENDING');self.assertEqual(self.r.writes,[])
    def test_reconcile_ambiguous_or_old_candidates_cannot_be_adopted(self):
        self.state.update(deploy_requested=True,previous_deployment_uuids=['old-deploy'])
        self.r.history[0]['commit']=NEW
        self.assertTrue(ops.status(self.r,self.state)['acknowledgement_unresolved'])
        self.r.finished('in_progress');self.r.history.append({**self.r.history[0],'deployment_uuid':'other-new'})
        with self.assertRaisesRegex(e.Refusal,'ambiguous_release_history'):ops.status(self.r,self.state)
    def test_status_binds_ack_detail_and_numeric_application_identity(self):
        self.state['deployment_uuid']='new-deploy';self.r.finished()
        for field,value in [('deployment_uuid','foreign'),('application_id','72'),('application',{'uuid':'foreign','name':e.NAME})]:
            detail=copy.deepcopy(self.r.details['new-deploy']);self.r.details['new-deploy'][field]=value
            with self.assertRaisesRegex(e.Refusal,'deployment_application_binding'):ops.status(self.r,self.state)
            self.r.details['new-deploy']=detail
        self.assertEqual(self.r.writes,[])
    def test_finished_requires_exact_source_and_useful_health_without_exporting_logs(self):
        self.state['deployment_uuid']='new-deploy';self.r.finished()
        result=ops.status(self.r,self.state);self.assertEqual(result['overall'],'PASS')
        self.assertNotIn('PRIVATE',json.dumps(result));self.assertEqual(result['log_failure_pattern_count'],0)
        for field in ['commit','health','ready','application']:
            r=copy.deepcopy(self.r)
            if field=='commit':r.details['new-deploy']['commit']=OLD
            elif field=='health':r.health['git_sha']=OLD
            elif field=='ready':r.ready['ready']=False
            else:r.app['status']='running:unhealthy'
            with self.assertRaises(e.Refusal):ops.status(r,self.state)
        self.assertEqual(self.r.writes,[])
    def test_proof_inputs_bind_exact_source_profile_and_default_main_gate(self):
        docker=(ROOT/'docker/indexer/Dockerfile').read_bytes()
        proof={'schema':'api_rolling_eligibility_proof_v1','overall':'PASS','source_manifest':MANIFEST,
            'controls':{key:True for key in e.PROOF_CONTROLS},
            'measurements':{'proxy_requests':10,'native_quote_successes':4,'http_failures':0,'request_lcd_calls':0,'parsed_blocks_after_handoff':1},
            'scope':'owned independent HTTP/state, production block indexing and proxy forwarding; not OS-process or actual Coolify deployment'}
        profile_path=Path(self.temp.name)/'profile.json';proof_path=Path(self.temp.name)/'proof.json'
        e.write(profile_path,PROFILE);e.write(proof_path,proof)
        with patch.object(e,'revision_manifest',return_value=(MANIFEST,{'docker/indexer/Dockerfile':docker}))as revision:
            ops.proof_inputs(NEW,profile_path,proof_path);revision.assert_called_once_with(NEW,exact_main=True)
            revision.reset_mock();ops.proof_inputs(NEW,profile_path,proof_path,exact_main=False)
            revision.assert_called_once_with(NEW,exact_main=False)
            proof['source_manifest']={'sha256':'b'*64};e.write(proof_path,proof)
            with self.assertRaisesRegex(e.Refusal,'owned_source_manifest_binding'):ops.proof_inputs(NEW,profile_path,proof_path)
        with patch.object(e,'revision_manifest',side_effect=e.Refusal('exact_remote_main_required')):
            with self.assertRaisesRegex(e.Refusal,'exact_remote_main_required'):ops.proof_inputs(NEW,profile_path,proof_path)
        self.assertEqual(self.r.writes,[])
    def test_failed_deployment_checks_old_api_without_mutating_anything(self):
        self.state['deployment_uuid']='new-deploy';self.r.finished('failed')
        result=ops.status(self.r,self.state)
        self.assertEqual(result['overall'],'FAIL');self.assertTrue(result['old_api_still_serving']);self.assertEqual(self.r.writes,[])
    def test_finished_unavailable_log_inspection_cannot_claim_pass(self):
        self.state['deployment_uuid']='new-deploy';self.r.finished()
        for logs in ['not-json',{'output':'error: hidden'},None,[],[{'unrelated':'missing output'}]]:
            with self.subTest(logs=logs):
                self.r.details['new-deploy']['logs']=logs
                try:result=ops.status(self.r,self.state)
                except e.Refusal:continue
                self.assertEqual(result['overall'],'FAIL')
    def test_finished_failure_log_markers_are_fail_counts_only(self):
        self.state['deployment_uuid']='new-deploy';self.r.finished();self.r.details['new-deploy']['logs']=[{'output':'error: PRIVATE_URL_PASSWORD'}]
        result=ops.status(self.r,self.state);self.assertEqual(result['overall'],'FAIL');self.assertEqual(result['log_failure_pattern_count'],1)
        self.assertNotIn('PRIVATE',json.dumps(result));self.assertEqual(self.r.writes,[])

if __name__=='__main__':unittest.main()
