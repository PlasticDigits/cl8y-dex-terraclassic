#!/usr/bin/env python3
"""Verify frozen, precompiled serving continuity in owned local infrastructure.

Capture the source manifest before compiling. Compilation is separate; every
actual test child has a57s owner. This runner performs no production operation.
"""
import argparse,hashlib,importlib.util,json,os,re,subprocess,time
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]

def verify(build_report, source_manifest, OUT):
    OUT.mkdir(mode=0o700,parents=True,exist_ok=True)
    assert OUT.stat().st_uid==os.getuid(), 'owned_output_directory_required'
    OUT.chmod(0o700)
    spec=importlib.util.spec_from_file_location('eligibility',ROOT/'scripts/ops/api-rolling-eligibility.py');e=importlib.util.module_from_spec(spec);spec.loader.exec_module(e)
    before=e.application_manifest(ROOT)
    assert e.read_json(source_manifest)==before, 'frozen_build_source_manifest_mismatch'
    e.startup_healthcheck({p:(ROOT/p).read_bytes() for p in ['docker/indexer/Dockerfile']})
    artifacts={}
    for line in build_report.read_text().splitlines():
        try:r=json.loads(line)
        except ValueError:continue
        if r.get('reason')=='compiler-artifact' and r.get('executable') and r.get('profile',{}).get('test'):
            if r['target']['name'] in {'cl8y_dex_indexer','cohort_online_compat','indexer_fee_cohorts','indexer_protocol_fees','route_local_quotes_1452'}:
                assert Path(r['manifest_path']).resolve()==(ROOT/'indexer/Cargo.toml').resolve(), 'build_report_checkout_mismatch'
                assert Path(r['executable']).is_file() and os.access(r['executable'],os.X_OK), 'precompiled_test_required'
                artifacts[r['target']['name']]=r['executable']
    assert set(['cl8y_dex_indexer','cohort_online_compat','indexer_fee_cohorts','indexer_protocol_fees','route_local_quotes_1452'])<=set(artifacts)
    lib=artifacts['cl8y_dex_indexer'];route=artifacts['route_local_quotes_1452']
    cases=[
    ('schema_compatibility',['python3','scripts/qa/test-cohort-online-compat-1211.py','--test-binary',artifacts['cohort_online_compat']]),
    ('paced_proxy_handoff',[route,'api_rolling_handoff::rolling_proxy_keeps_old_until_standby_native_admission_then_real_block_handoff','--exact','--nocapture','--test-threads=1']),
    ('readiness_latch',[route,'api_rolling_readiness::api_rolling_readiness_standby_native_cache_and_owned_leader_handoff','--exact','--nocapture','--test-threads=1']),
    ('readiness_contract',[lib,'api::rolling_readiness::tests','--test-threads=1']),
    ('missing_trace',[lib,'api::local_quote::missing_trace_tests','--test-threads=1']),
    ('trace_eight_slot_cap',[lib,'api::prerequisite_trace::tests','--test-threads=1']),
    ('original_stale_trace',[lib,'issue1505_http_stale_origins_are_bounded_private_and_correlated','--test-threads=1']),
    ('cohort_regression',[artifacts['indexer_fee_cohorts'],'--test-threads=1']),
    ('protocol_fee_regression',[artifacts['indexer_protocol_fees'],'--test-threads=1'])]
    for filename in ['test_cohort_preflight_1211.py','test_cohort_migration_ops.py','test_cohort_online_worker_1211.py','test_cohort_online_ops.py','test_api_rolling_eligibility.py','test_api_rolling_release.py']:
        cases.append((filename,['python3','scripts/tests/'+filename]))
    results=[];handoff=None
    for name,args in cases:
        started=time.monotonic();clock=datetime.now(timezone.utc).isoformat()
        try:
            process=subprocess.run(args,cwd=ROOT,text=True,capture_output=True,timeout=57)
            output=process.stdout+process.stderr;code=process.returncode
        except subprocess.TimeoutExpired as error:
            output=(error.stdout or b'').decode(errors='replace')+(error.stderr or b'').decode(errors='replace');code=124
        elapsed=time.monotonic()-started
        p=OUT/('combined-'+name.replace('.py','')+'.log');p.touch(mode=0o600,exist_ok=True);p.chmod(0o600);p.write_text(output)
        matches=re.findall(r'test result: ok\. (\d+) passed; 0 failed;',output)
        py=re.search(r'Ran (\d+) tests? in ',output)
        passed=code==0 and elapsed<59 and (any(int(x)>0 for x in matches) or py and int(py[1])>0 and '\nOK' in output)
        if name=='paced_proxy_handoff' and passed:
            receipts=[json.loads(line.split('ROLLING_HANDOFF_RECEIPT ',1)[1]) for line in output.splitlines() if line.startswith('ROLLING_HANDOFF_RECEIPT ')]
            assert len(receipts)==1;handoff=receipts[0]
            assert handoff['measurement_profile']=='paced_native_200ms_total_client_cap150_v1' and handoff['client_http_requests']<=150 and handoff['http_failures']==0
        row={'name':name,'overall':'PASS' if passed else 'FAIL','return_code':code,'elapsed_seconds':round(elapsed,3),'utc':clock,'output_sha256':hashlib.sha256(output.encode()).hexdigest(),'passed_tests':int(py[1]) if py else sum(map(int,matches))}
        results.append(row);print(json.dumps(row),flush=True)
        if not passed:break
    unchanged=e.application_manifest(ROOT)==before
    ok=len(results)==len(cases) and all(r['overall']=='PASS' for r in results) and unchanged
    report={'schema':'combined_serving_tests_v1','overall':'PASS' if ok else 'FAIL','source_manifest':before,'source_unchanged':unchanged,'tests':results,'handoff':handoff,'scope':'Owned local verification only; retained stress failure and full live1505 acceptance remain unresolved.'}
    p=OUT/'combined-test-results.json';e.write(p,report)
    proof={'schema':'api_rolling_eligibility_proof_v1','overall':'FAIL','source_manifest':before}
    if ok:
        proof.update(overall='PASS',controls={key:True for key in e.PROOF_CONTROLS},measurements={key:handoff[key] for key in ['proxy_requests','native_quote_successes','http_failures','request_lcd_calls','parsed_blocks_after_handoff']},scope='owned independent HTTP/state, production block indexing and proxy forwarding; not OS-process or actual Coolify deployment',tests=results,measurement_profile=handoff['measurement_profile'],retained_unresolved_stress_failure=True,full_issue_1505_acceptance=False)
        e.proof_ready(proof,before)
    e.write(OUT/'api-owned-proof.json',proof)
    print(json.dumps({'overall':report['overall'],'groups':len(results),'source_unchanged':unchanged,'source_manifest_sha256':before['sha256']}))
    return 0 if ok else 1

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-report',type=Path,required=True)
    parser.add_argument('--source-manifest',type=Path,required=True)
    parser.add_argument('--output-dir',type=Path,required=True)
    args=parser.parse_args()
    return verify(args.build_report,args.source_manifest,args.output_dir)

if __name__=='__main__':
    raise SystemExit(main())
