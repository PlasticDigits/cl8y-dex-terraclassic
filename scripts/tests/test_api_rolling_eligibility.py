#!/usr/bin/env python3
"""No network: semantic controls for GET-only rollout eligibility and provenance."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('eligibility',Path(__file__).resolve().parents[1]/'ops/api-rolling-eligibility.py')
ops=importlib.util.module_from_spec(spec);spec.loader.exec_module(ops)
TOKEN='terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3'
PROFILE=[{'token_in':'uusd','token_out':TOKEN,'amount_in':'1000000','max_spread':'0.01'}]
OLD='0'*40
class FakeReader:
    api_base='https://owned.invalid/api/v1/'
    def __init__(self):
        self.reads=0;self.calls=[]
        self.app={'uuid':ops.APP,'name':ops.NAME,'fqdn':'https://'+ops.DOMAIN,'git_repository':ops.REPOSITORY,'git_branch':'main','git_commit_sha':OLD,'status':'running:healthy',
            'destination_type':'App\\Models\\StandaloneDocker','ports_mappings':None,'custom_docker_run_options':None,'health_check_enabled':False,'custom_healthcheck_found':True,
            'settings':{'is_consistent_container_name_enabled':False,'is_auto_deploy_enabled':False,'use_build_secrets':True},'destination':{'server':{'proxy':{'type':'TRAEFIK','status':'running'}}}}
        self.history={'deployments':[{'deployment_uuid':'old-deploy','application_id':'71','status':'finished','commit':OLD}]}
        self.detail={**self.history['deployments'][0],'application':{'uuid':ops.APP,'name':ops.NAME},'logs':'SECRET_LOG_MUST_NOT_APPEAR'}
        self.env=[{'key':'API_READY_SCENARIOS','value':json.dumps(PROFILE),'is_preview':False,'is_runtime':True,'is_buildtime':False,'is_literal':True,'is_shown_once':False},
            {'key':'DATABASE_URL','value':'SECRET_RUNTIME_VALUE_MUST_NOT_APPEAR','is_runtime':True,'is_buildtime':False}]
        self.health={'status':'ok','git_sha':OLD};self.active=[]
    def api(self,path):
        self.reads+=1;self.calls.append(('GET',path))
        if path=='applications/'+ops.APP:return self.app
        if path=='applications/'+ops.APP+'/envs':return self.env
        if path.startswith('deployments/applications/'):return self.history
        if path=='deployments/old-deploy':return self.detail
        if path=='deployments':return self.active
        raise AssertionError('unscoped path')
    def read(self,url,limit,private=False):
        self.reads+=1;self.calls.append(('GET',url))
        if url==self.api_base+'version':return b'4.4.6'
        assert url=='https://'+ops.DOMAIN+'/health' and not private
        return json.dumps(self.health).encode()

class EligibilityControls(unittest.TestCase):
    def test_eligible_inventory_uses_get_only_and_exports_no_runtime_or_log_data(self):
        reader=FakeReader();result=ops.inventory(reader,PROFILE)
        self.assertTrue(all(result['checks'].values()));self.assertEqual(result['reads'],7)
        self.assertTrue(all(method=='GET'for method,_ in reader.calls))
        text=json.dumps(result)
        self.assertNotIn('SECRET_',text);self.assertNotIn(TOKEN,text);self.assertNotIn('amount_in',text)
    def test_rolling_breakers_and_unsafe_profile_metadata_never_become_eligible(self):
        for mutation in ['port','fixed','entrypoint','health','auto','unhealthy','identity','profilemissing','buildtime','shownonce','literal','privileged','compiler','proxy','pending']:
            with self.subTest(mutation=mutation):
                r=FakeReader()
                if mutation=='port':r.app['ports_mappings']='3001:3001'
                elif mutation=='fixed':r.app['settings']['is_consistent_container_name_enabled']=True
                elif mutation=='entrypoint':r.app['custom_docker_run_options']='--entrypoint true'
                elif mutation=='health':r.app['health_check_enabled']=True
                elif mutation=='auto':r.app['settings']['is_auto_deploy_enabled']=True
                elif mutation=='unhealthy':r.app['status']='running:unhealthy'
                elif mutation=='identity':r.health['git_sha']='1'*40
                elif mutation=='profilemissing':r.env.pop(0)
                elif mutation=='buildtime':r.env[0]['is_buildtime']=True
                elif mutation=='shownonce':r.env[0]['is_shown_once']=True
                elif mutation=='literal':r.env[0]['is_literal']=False
                elif mutation=='privileged':r.env.append({'key':'DB_ADMIN_URL','value':'SECRET_PRIVILEGED'})
                elif mutation=='compiler':r.env.append({'key':'RUSTFLAGS','value':'--cfg unreviewed','is_buildtime':True})
                elif mutation=='proxy':r.app['destination']['server']['proxy']['type']='NONE'
                elif mutation=='pending':r.active=[{'application_id':'71','status':'queued'}]
                self.assertFalse(all(ops.inventory(r,PROFILE)['checks'].values()))
    def test_foreign_app_or_deployment_identity_refuses_before_further_reads(self):
        for foreign in ['app','deployment','numeric']:
            r=FakeReader()
            if foreign=='app':r.app['uuid']='other-app'
            elif foreign=='deployment':r.detail['application']['uuid']='other-app'
            else:r.detail['application_id']='72'
            with self.assertRaises(ops.Refusal):ops.inventory(r,PROFILE)
            self.assertTrue(all(method=='GET'for method,_ in r.calls))
    def test_profile_parser_rejects_actors_bad_checksum_duplicates_and_amount_overflow(self):
        self.assertEqual(ops.public_profile(json.dumps(PROFILE)),PROFILE)
        variants=[]
        for key,value in [('trader',TOKEN),('token_out',TOKEN[:-1]+'q'),('amount_in','01'),('amount_in',str(2**128)),('max_spread','0.31')]:
            row=copy.deepcopy(PROFILE);row[0][key]=value;variants.append(json.dumps(row))
        variants += ['[]',json.dumps(PROFILE*2),'[{"token_in":"uusd","token_in":"uluna"}]']
        for value in variants:
            with self.assertRaises(ops.Refusal):ops.public_profile(value)
    def test_pure_manifest_binds_compiled_inputs_and_excludes_env_or_target(self):
        with tempfile.TemporaryDirectory()as directory:
            root=Path(directory)
            for name in ops.SOURCE_REQUIRED+ops.SOURCE_CRITICAL:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(name)
            initial=ops.application_manifest(root)
            (root/'indexer/.env').write_text('SECRET')
            (root/'indexer/target').mkdir();(root/'indexer/target/cache').write_text('irrelevant')
            self.assertEqual(initial,ops.application_manifest(root))
            (root/'indexer/src/main.rs').write_text('changed')
            self.assertNotEqual(initial['sha256'],ops.application_manifest(root)['sha256'])
            (root/'indexer/src/escape').symlink_to('/etc/hosts')
            with self.assertRaises(ops.Refusal):ops.application_manifest(root)
    def test_owned_proof_requires_exact_manifest_all_controls_and_numeric_success_counts(self):
        manifest={'schema':'indexer_compiled_inputs_v1','sha256':'1'*64}
        proof={'schema':'api_rolling_eligibility_proof_v1','overall':'PASS','source_manifest':manifest,
            'controls':{key:True for key in ops.PROOF_CONTROLS},'measurements':{'proxy_requests':10,'native_quote_successes':4,'http_failures':0,'request_lcd_calls':0,'parsed_blocks_after_handoff':1},
            'scope':'owned independent HTTP/state, production block indexing and proxy forwarding; not OS-process or actual Coolify deployment'}
        ops.proof_ready(proof,manifest)
        for changed in ['source','control','failure','bool']:
            invalid=copy.deepcopy(proof)
            if changed=='source':invalid['source_manifest']['sha256']='2'*64
            elif changed=='control':invalid['controls']['actual_block_handoff_after_lock_transfer']=False
            elif changed=='failure':invalid['measurements']['http_failures']=1
            else:invalid['measurements']['proxy_requests']=True
            with self.assertRaises(ops.Refusal):ops.proof_ready(invalid,manifest)
    def test_startup_health_must_be_actual_bounded_instruction_and_normal_runtime(self):
        docker=(ops.ROOT/'docker/indexer/Dockerfile').read_bytes()
        ops.startup_healthcheck({'docker/indexer/Dockerfile':docker})
        for invalid in [docker.replace(b'HEALTHCHECK --interval',b'# HEALTHCHECK --interval'),
            docker.replace(b'/health/serving',b'/ready-api'),docker.replace(b'|| exit 1',b'|| true'),
            docker+b'\nENTRYPOINT ["true"]\n']:
            with self.assertRaises(ops.Refusal):ops.startup_healthcheck({'docker/indexer/Dockerfile':invalid})
    def test_revision_manifest_is_strict_by_default_and_historical_still_hashes_exact_revision(self):
        calls=[]
        def fake_git(*args):
            calls.append(args)
            if args[0]=='ls-remote':return ('1'*40+' refs/heads/main\n').encode()
            if args[0]=='ls-tree':raise ops.Refusal('archive_inventory_reached')
            raise AssertionError(args)
        with patch.object(ops,'git',side_effect=fake_git):
            with self.assertRaisesRegex(ops.Refusal,'exact_remote_main_required'):ops.revision_manifest(OLD)
            self.assertEqual([call[0]for call in calls],['ls-remote']);calls.clear()
            with self.assertRaisesRegex(ops.Refusal,'archive_inventory_reached'):ops.revision_manifest(OLD,exact_main=False)
            self.assertEqual(calls[0][0],'ls-tree');self.assertIn(OLD,calls[0])
    def test_read_budget_cannot_contact_after_original_deadline(self):
        reader=FakeReader()
        with patch.object(ops.time,'monotonic',side_effect=[0,46]):
            budget=ops.ReadBudget(reader)
            with self.assertRaises(ops.Refusal):budget.api('applications/'+ops.APP)
        self.assertEqual(reader.calls,[])
if __name__=='__main__':unittest.main()
