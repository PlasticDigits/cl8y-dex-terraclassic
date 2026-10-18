#!/usr/bin/env python3
"""One owned 67->68 online PostgreSQL rehearsal; no production HTTP/DB or restore.

Compiled executables are separately pinned before the <59s actual test run.
Fresh random credentials enter only private process environments/stdin.
"""
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import tempfile
import threading
import time
import urllib.parse
from unittest.mock import patch

from test_cohort_migration_local import load, patch_env, process, require, PG

ROOT = Path(__file__).resolve().parents[2]
BINARY = Path('/tmp/dex-online-1211-pinned/indexer')
BASELINE = Path('/tmp/dex-online-1211-pinned/baseline')
BASE = Path('/tmp/dex-issues-1206-1512/migration-pinned-base.py')
MIGRATOR = 'dex_indexer_migrator_20261006'
RUNTIME = 'dex_indexer_runtime_20261006'
PHASE = 'initialization'


def run():
    global PHASE
    require(all(path.is_file() for path in [BINARY,BASELINE,BASE]), 'pinned_inputs_missing')
    worker = load('owned_online_worker', ROOT/'scripts/ops/cohort-online-worker-1211.py')
    pre = load('owned_preflight', ROOT/'scripts/ops/cohort-preflight-1211.py')
    base = load('owned_private_base', BASE)
    base.EXPECTED_CHECKSUMS = pre.manifest() # reviewed source only; no SQLx history editing
    base.TABLES.update(pre.TABLES)
    require(len(base.EXPECTED_CHECKSUMS)==68, 'exact_source_manifest')
    marker_id = secrets.token_hex(12)
    marker = 'cl8y-dex:cohort-rehearsal:' + marker_id
    container = 'dex-cohort-online-' + marker_id
    passwords = [secrets.token_urlsafe(32) for _ in range(3)]
    with socket.socket() as reserved:
        reserved.bind(('127.0.0.1',0)); port=reserved.getsockname()[1]
    environment = {'PATH':PG+':'+os.environ['PATH'],'HOME':os.environ['HOME']}
    pg_env = {**environment,'PGHOST':'127.0.0.1','PGPORT':str(port),'PGDATABASE':'postgres',
              'PGUSER':'postgres','PGPASSWORD':passwords[0],'PGCONNECT_TIMEOUT':'2'}
    runtime_env = {**pg_env,'PGUSER':RUNTIME,'PGPASSWORD':passwords[1],
                   'PGOPTIONS':'-c lock_timeout=250ms -c statement_timeout=2000ms'}
    dsn = 'postgresql://postgres:' + urllib.parse.quote(passwords[0],safe='') + '@127.0.0.1:' + str(port) + '/postgres'
    owner_dsn = dsn + '?' + urllib.parse.urlencode({'options':'-c role='+MIGRATOR})
    owned=False; stop=threading.Event(); writes=[]; errors=[]; thread=None; holder=None
    def sql(query, env=pg_env):
        return process([PG+'/psql','-X','-qAt','-v','ON_ERROR_STOP=1'],env=env,input=query,seconds=5).stdout.strip()
    def writer():
        while not stop.is_set():
            result = process([PG+'/psql','-X','-qAt','-v','ON_ERROR_STOP=1'],env=runtime_env,
                input='UPDATE global_stats_24h SET total_trades=total_trades+1 WHERE id=1 RETURNING total_trades; SELECT version FROM indexer_schema_version;',ok=False,seconds=5)
            if result.returncode:
                errors.append('runtime_read_write_failed'); return
            rows=result.stdout.strip().splitlines()
            if len(rows)!=2 or int(rows[1]) not in {worker.PREVIOUS,worker.TARGET}:
                errors.append('runtime_schema_marker_failed'); return
            writes.append((time.monotonic(),int(rows[0]),int(rows[1])))
            stop.wait(0.015)
    try:
        PHASE='owned_postgres_provision'
        process(['docker','run','-d','--name',container,'--label','cl8y.cohort-online-rehearsal='+marker_id,
                 '--memory','256m','--cpus','1','-p','127.0.0.1:'+str(port)+':5432','-e','POSTGRES_PASSWORD',
                 'postgres:16-alpine'],env={**environment,'POSTGRES_PASSWORD':passwords[0]})
        owned=True
        for _ in range(40):
            if process([PG+'/psql','-X','-qAt','-c','SELECT 1'],env=pg_env,ok=False).stdout.strip()=='1': break
            time.sleep(0.15)
        else: raise RuntimeError('owned_postgres_not_ready')
        PHASE='owned_roles_and_real_baseline'
        sql('CREATE ROLE '+MIGRATOR+' LOGIN NOINHERIT PASSWORD '+base.literal(passwords[2])+'; CREATE ROLE '+RUNTIME+' LOGIN NOINHERIT PASSWORD '+base.literal(passwords[1])+'; ALTER DATABASE postgres OWNER TO '+MIGRATOR+'; ALTER SCHEMA public OWNER TO '+MIGRATOR+'; REVOKE ALL ON DATABASE postgres FROM PUBLIC; GRANT CONNECT ON DATABASE postgres TO '+MIGRATOR+','+RUNTIME+'; COMMENT ON DATABASE postgres IS '+base.literal(marker)+';')
        baseline=json.loads(process([str(BASELINE)],env={**environment,'DATABASE_URL':owner_dsn,
            'COHORT_REHEARSAL_MARKER':marker}).stdout)
        require(baseline['baseline_migrations']==67,'baseline_67')
        sql("INSERT INTO global_stats_24h(id) VALUES(1) ON CONFLICT(id) DO NOTHING; UPDATE global_stats_24h SET fee_event_count_24h=4,total_fees_24h_usd=12.5,fee_event_count_7d=6,total_fees_7d_usd=NULL,fee_event_count_30d=0,total_fees_30d_usd=0,updated_at='2026-10-18T00:00:00Z' WHERE id=1;")
        profile={**environment,'DB_ADMIN_URL':dsn,'OPS_APPLICATION_DB':'postgres','INDEXER_RUNTIME_ROLE':RUNTIME,
                 'INDEXER_MIGRATOR_ROLE':MIGRATOR,'OPS_ONLINE_MODE':worker.MODE,
                 'PGOPTIONS':'-c lock_timeout=250ms -c statement_timeout=30000ms'}
        with tempfile.TemporaryDirectory(prefix='dex-online-rehearsal-') as directory, patch_env(profile), \
             patch.object(worker.urllib.request,'build_opener',side_effect=AssertionError('production_http_forbidden')):
            ready=[]
            def readiness():
                ready.append(sql('SELECT version FROM indexer_schema_version',runtime_env))
                require(int(ready[-1]) in {worker.PREVIOUS,worker.TARGET},'owned_runtime_readiness')
            job=worker.make_job(base,Path(directory),str(BINARY),readiness)
            # Reaching either legacy helper is a test failure, including on retry.
            job.assert_stopped=lambda: (_ for _ in ()).throw(RuntimeError('legacy_stop_forbidden'))
            job.transition=lambda: (_ for _ in ()).throw(RuntimeError('role_transition_forbidden'))
            job.apply=lambda: (_ for _ in ()).throw(RuntimeError('legacy_restore_workflow_forbidden'))
            history_before=job.history(); acl_before=job.existing_acl(); roles_before=job.role_fingerprint()
            # Reordering equivalent ACL entries must not masquerade as changed privileges.
            raw_acl_query="SELECT json_build_object('object',(SELECT relacl FROM pg_class WHERE oid='global_stats_24h'::regclass),'schema',(SELECT nspacl FROM pg_namespace WHERE nspname='public'),'database',(SELECT datacl FROM pg_database WHERE datname=current_database()))"
            raw_acl_before=sql(raw_acl_query)
            job.owner_sql('BEGIN; REVOKE ALL ON TABLE global_stats_24h FROM '+MIGRATOR+'; GRANT ALL ON TABLE global_stats_24h TO '+MIGRATOR+'; REVOKE ALL ON SCHEMA public FROM '+MIGRATOR+'; GRANT ALL ON SCHEMA public TO '+MIGRATOR+'; REVOKE ALL ON DATABASE postgres FROM '+MIGRATOR+'; GRANT ALL ON DATABASE postgres TO '+MIGRATOR+'; COMMIT;')
            require(raw_acl_before!=sql(raw_acl_query) and job.existing_acl()==acl_before,'canonical_acl_semantics_ignore_order')
            require(job.owner_sql("SELECT current_setting('lock_timeout')='250ms' AND current_setting('statement_timeout')='5s'")=='t','owner_low_timeouts')
            PHASE='ongoing_actual_runtime_reader_writer'
            thread=threading.Thread(target=writer,daemon=True);thread.start()
            for _ in range(60):
                if len(writes)>=2: break
                time.sleep(0.02)
            require(not errors, errors[0] if errors else 'runtime_progress_before')
            require(len(writes)>=2,'runtime_progress_before')
            PHASE='actual_low_lock_timeout_refusal'
            holder=subprocess.Popen([PG+'/psql','-X','-qAt','-v','ON_ERROR_STOP=1','-c',
                'BEGIN; LOCK TABLE protocol_fee_events IN ROW EXCLUSIVE MODE; SELECT pg_sleep(10); ROLLBACK;'],
                env={**runtime_env,'PGOPTIONS':'-c lock_timeout=250ms -c statement_timeout=15000ms'},stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            for _ in range(40):
                if sql("SELECT count(*)>0 FROM pg_locks WHERE relation='protocol_fee_events'::regclass AND mode='RowExclusiveLock' AND granted")=='t': break
                time.sleep(0.02)
            else: raise RuntimeError('owned_lock_not_ready')
            migration_times=[]; original_run=subprocess.run
            def monitored(args,*args2,**kwargs):
                begin=time.monotonic(); result=original_run(args,*args2,**kwargs)
                if args==[str(BINARY),'migrate']: migration_times.append((time.monotonic()-begin,result.returncode))
                return result
            with patch.object(worker.subprocess,'run',side_effect=monitored):
                refused=False
                try: job.apply_online()
                except RuntimeError as error:
                    refused=str(error)=='scoped_migration_cli_failed'
            require(refused and migration_times and migration_times[0][0]<2,'bounded_lock_refusal')
            require(job.history()==history_before and job.existing_acl()==acl_before and job.role_fingerprint()==roles_before,'refusal_preserves_existing_state')
            snapshot=Path(directory)/'online-before.dump'
            snapshot_before=(snapshot.stat().st_size,snapshot.stat().st_mtime_ns,worker.file_digest(snapshot))
            holder.send_signal(signal.SIGINT); holder.wait(timeout=3);holder=None
            for _ in range(40):
                if sql("SELECT count(*)=0 FROM pg_locks WHERE relation='protocol_fee_events'::regclass AND mode='RowExclusiveLock' AND granted")=='t': break
                time.sleep(0.02)
            else: raise RuntimeError('owned_conflicting_lock_not_released')
            PHASE='online_retry_while_runtime_writes'
            migration_start=time.monotonic()
            with redirect_stdout(io.StringIO()): result=job.apply_online()
            migration_end=time.monotonic()
            require(result['overall']=='PASS' and result['history_count']==68 and result['actual_permission_denials']==4,'online_migration_verified')
            PHASE='idempotent_retry_snapshot_reuse'
            with redirect_stdout(io.StringIO()): replay=job.apply_online()
            require(replay['overall']=='PASS','idempotent_online_retry')
            require(snapshot_before==(snapshot.stat().st_size,snapshot.stat().st_mtime_ns,worker.file_digest(snapshot)),'immutable_snapshot_reused')
            require(all(any(row==old for row in job.history()) for old in history_before),'original_67_complete_rows_unchanged')
            require(job.existing_acl()==acl_before and job.role_fingerprint()==roles_before,'existing_acl_roles_unchanged')
            for _ in range(40):
                if any(row[2]==worker.TARGET for row in writes): break
                time.sleep(0.02)
            require(not errors and any(migration_start<=row[0]<=migration_end for row in writes)
                    and any(row[2]==worker.PREVIOUS for row in writes) and any(row[2]==worker.TARGET for row in writes),'runtime_progress_during_after')
            stop.set();thread.join(timeout=5);thread=None
            require(not errors,'runtime_no_errors')
            require(sql('SELECT version FROM indexer_schema_version;SELECT count(*) FROM protocol_fee_attributions;',runtime_env).splitlines()==[str(worker.TARGET),'0'],'actual_runtime_new_table_login')
            rows=json.loads(sql("SELECT json_agg(json_build_object('window',\"window\",'cohort',cohort,'usd',amount_usd,'count',event_count,'clock',updated_at='2026-10-18T00:00:00Z'::timestamptz)) FROM protocol_fee_stats_by_cohort"))
            require(len(rows)==9 and all(row['clock'] for row in rows),'published_seed_clock')
            require(any(row['window']=='24h' and row['cohort']=='unknown' and row['usd']==12.5 and row['count']==4 for row in rows),'published_seed_amount')
            return {'overall':'PASS','mode':worker.MODE,'scope':'owned isolated local PostgreSQL only; no production HTTP/DB, serving stop or database restore',
                'baseline_migrations':67,'final_migrations':68,'original_67_complete_rows_unchanged':True,
                'runtime_read_write_iterations':len(writes),'runtime_errors':len(errors),'runtime_progress_during_migration':True,
                'runtime_actual_login_before_and_after':True,'existing_acl_unchanged':True,'roles_credentials_memberships_unchanged':True,
                'new_runtime_table_grants':True,'acl_semantic_ordering_verified':True,'actual_permission_denials':result['actual_permission_denials'],
                'actual_dsn_lock_refusal_seconds':round(migration_times[0][0],3),'owner_timeouts_verified':True,
                'failed_attempt_no_history_or_acl_changes':True,'idempotent_retry':True,'same_original_snapshot_reused':True,
                'snapshot_bytes':snapshot_before[0],'snapshot_digest':snapshot_before[2],'fake_readiness_calls':len(ready),
                'published_rollup_and_clock_preserved':True,'new_binary_digest':hashlib.sha256(BINARY.read_bytes()).hexdigest()}
    finally:
        stop.set()
        if thread: thread.join(timeout=5)
        if holder: holder.send_signal(signal.SIGINT);holder.wait(timeout=3)
        if owned:
            label=process(['docker','inspect','--format','{{index .Config.Labels "cl8y.cohort-online-rehearsal"}}',container]).stdout.strip()
            require(label==marker_id,'owned_cleanup_identity')
            process(['docker','rm','-fv',container])

if __name__=='__main__':
    started=time.monotonic()
    signal.signal(signal.SIGALRM,lambda *_: (_ for _ in ()).throw(TimeoutError()))
    signal.alarm(50)
    try:
        result=run();result['elapsed_seconds']=round(time.monotonic()-started,3)
        print(json.dumps(result))
    except Exception as error:
        print(json.dumps({'overall':'FAIL','failure_class':type(error).__name__,'phase':PHASE,
            'guard':str(error) if isinstance(error,RuntimeError) and str(error).replace('_','').isalnum() else None,
            'elapsed_seconds':round(time.monotonic()-started,3)}))
        raise SystemExit(1)
    finally: signal.alarm(0)
