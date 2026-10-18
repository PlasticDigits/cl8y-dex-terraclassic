#!/usr/bin/env python3
"""Real owned-local 67->68 rehearsal; builds are separate from the <59s run.

Creates/removes only its own labeled PostgreSQL container and anonymous volume.
No production DB credential is read. SQLx creates all 67 baseline history rows.
"""
import ast
import base64
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import secrets
import signal
import socket
import shutil
import subprocess
import tempfile
import time
import urllib.parse

ROOT = Path(__file__).resolve().parents[2]
PG = '/usr/lib/postgresql/16/bin'
MIGRATOR = 'dex_indexer_migrator_20261006'
RUNTIME = 'dex_indexer_runtime_20261006'
TARGET_DIR = Path(os.environ.get('CARGO_TARGET_DIR', '/home/answorld/repos/cl8y-dex-terraclassic/indexer/target'))
BINARY = TARGET_DIR / 'debug/cl8y-dex-indexer'
BASELINE = TARGET_DIR / 'debug/examples/cohort_baseline_1211'
PHASE = 'initialization'
KNOWN_GUARDS = set()


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def require(ok, label):
    if not ok: raise RuntimeError(label)


def process(args, env=None, input=None, ok=True, seconds=20):
    done = subprocess.run(args, input=input, text=True, capture_output=True, env=env, timeout=seconds)
    if ok and done.returncode != 0:
        text = done.stdout + done.stderr
        kind = next((label for fragment, label in [('Connection refused','connection_refused'),('password authentication failed','authentication'),('wrong fixture marker','fixture_marker'),('DSN options must','effective_owner'),('does not exist','schema_or_role_missing')] if fragment in text), 'command_exit')
        raise RuntimeError('local_' + kind)
    return done


def run():
    global PHASE
    require(BINARY.is_file() and BASELINE.is_file(), 'build_new_binary_and_baseline_first')
    ops = load('migration_ops', ROOT / 'scripts/ops/cohort-migration-1211.py')
    # Exactly one read of the pinned, credential-free maintenance recipe.
    PHASE = 'template_render'
    template = ops.Api(ops.pre.load_reader()).call('applications/' + ops.pre.TEMPLATE)['dockerfile']
    revision = process(['git', 'rev-parse', 'HEAD']).stdout.strip()
    compiler = next(line for line in (ROOT / 'docker/indexer/Dockerfile').read_text().splitlines() if line.startswith('FROM rust:'))
    recipe, release = ops.render(template, revision, ops.pre.manifest(), compiler)
    # Do not inherit unrelated connector credentials into PostgreSQL subprocesses.
    for key in list(os.environ):
        if key.startswith('COOLIFY_'): os.environ.pop(key)
    label = secrets.token_hex(12)
    container = 'dex-cohort-rehearsal-' + label
    marker = 'cl8y-dex:cohort-rehearsal:' + label
    password, runtime_password, migrator_password = (secrets.token_urlsafe(32) for _ in range(3))
    with socket.socket() as reserved:
        reserved.bind(('127.0.0.1', 0))
        port = reserved.getsockname()[1]
    environment = {'PATH': PG + ':' + os.environ['PATH'], 'HOME': os.environ['HOME']}
    docker_env = {**environment, 'POSTGRES_PASSWORD': password}
    pg_env = {**environment, 'PGHOST': '127.0.0.1', 'PGPORT': str(port), 'PGUSER': 'postgres', 'PGDATABASE': 'postgres', 'PGPASSWORD': password, 'PGCONNECT_TIMEOUT': '3'}
    dsn = 'postgresql://postgres:' + urllib.parse.quote(password, safe='') + '@127.0.0.1:' + str(port) + '/postgres'
    owner_dsn = dsn + '?' + urllib.parse.urlencode({'options': '-c role=' + MIGRATOR})
    owned = False
    try:
        PHASE = 'owned_postgres_provision'
        process(['docker', 'run', '-d', '--name', container, '--label', 'cl8y.cohort-rehearsal=' + label,
            '--memory', '256m', '--cpus', '1', '-p', '127.0.0.1:' + str(port) + ':5432', '-e', 'POSTGRES_PASSWORD', 'postgres:16-alpine'], env=docker_env)
        owned = True
        ready = False
        for _ in range(35):
            probe_env = {**pg_env, 'PGCONNECT_TIMEOUT': '1'}
            if process([PG + '/psql', '-X', '-qAt', '-c', 'SELECT 1'], env=probe_env, ok=False).stdout.strip() == '1':
                ready = True; break
            time.sleep(0.2)
        require(ready, 'owned_postgres_readiness')
        def sql(query, env=pg_env):
            return process([PG + '/psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], env=env, input=query).stdout.strip()
        def history():
            return sql("SELECT coalesce(json_agg(row_to_json(m) ORDER BY version)::text,'[]') FROM public._sqlx_migrations m")
        PHASE = 'owned_roles_setup'
        sql("CREATE ROLE " + MIGRATOR + " LOGIN NOINHERIT PASSWORD '" + migrator_password + "'; CREATE ROLE " + RUNTIME + " LOGIN NOINHERIT PASSWORD '" + runtime_password + "'; ALTER DATABASE postgres OWNER TO " + MIGRATOR + "; ALTER SCHEMA public OWNER TO " + MIGRATOR + "; REVOKE ALL ON DATABASE postgres FROM PUBLIC; GRANT CONNECT ON DATABASE postgres TO " + MIGRATOR + ',' + RUNTIME + "; COMMENT ON DATABASE postgres IS '" + marker + "';")
        original_security = sql("SELECT json_agg(json_build_object('role',rolname,'password',rolpassword,'super',rolsuper,'createdb',rolcreatedb,'createrole',rolcreaterole,'inherit',rolinherit) ORDER BY rolname) FROM pg_authid WHERE rolname IN ('" + MIGRATOR + "','" + RUNTIME + "')")
        PHASE = 'sqlx_identity'
        identity_env = {**environment, 'DATABASE_URL': dsn, 'PGOPTIONS': '-c role=' + MIGRATOR,
            'COHORT_REHEARSAL_MARKER': marker, 'COHORT_IDENTITY_ONLY': '1'}
        env_identity = json.loads(process([str(BASELINE)], env=identity_env).stdout)
        identity_env['DATABASE_URL'] = owner_dsn
        dsn_identity = json.loads(process([str(BASELINE)], env=identity_env).stdout)
        require(dsn_identity['effective_migrator'] and dsn_identity['bootstrap_session'], 'sqlx_dsn_effective_role')
        baseline_env = {key: value for key, value in identity_env.items() if key != 'COHORT_IDENTITY_ONLY'}
        PHASE = 'real_sqlx_baseline_67'
        baseline = json.loads(process([str(BASELINE)], env=baseline_env).stdout)
        require(baseline['baseline_migrations'] == 67, 'real_sqlx_baseline')
        PHASE = 'published_rollup_fixture'
        sql("INSERT INTO global_stats_24h(id) VALUES(1) ON CONFLICT(id) DO NOTHING; UPDATE global_stats_24h SET fee_event_count_24h=4,total_fees_24h_usd=12.5,fee_event_count_7d=6,total_fees_7d_usd=NULL,fee_event_count_30d=0,total_fees_30d_usd=0,updated_at='2026-10-18T00:00:00Z' WHERE id=1;")
        before = history()
        # Wrong effective owner must fail before SQLx history or marker changes.
        PHASE = 'wrong_owner_refusal'
        bad = process([str(BINARY), 'migrate'], env={**environment, 'DATABASE_URL': dsn, 'INDEXER_RUNTIME_ROLE': RUNTIME}, ok=False)
        require(bad.returncode != 0 and history() == before and sql('SELECT version FROM indexer_schema_version') == '20261016120000', 'wrong_owner_refusal')
        with tempfile.TemporaryDirectory(prefix='cohort-engine-1211-') as work:
            work = Path(work); work.chmod(0o700)
            source = work / 'ops'; source.mkdir(mode=0o700)
            for line in recipe.splitlines():
                match = re.fullmatch(r'RUN (?:mkdir -p /ops && )?printf %s ([A-Za-z0-9+/=]+) \| base64 -d > /ops/([A-Za-z0-9_.-]+)', line)
                if match:
                    content = base64.b64decode(match[1]); (source / match[2]).write_bytes(content)
                    if match[2].endswith('.py'):
                        tree = ast.parse(content)
                        KNOWN_GUARDS.update(node.args[0].value for node in ast.walk(tree) if isinstance(node,ast.Call) and isinstance(node.func,ast.Name) and node.func.id=='RuntimeError' and node.args and isinstance(node.args[0],ast.Constant) and isinstance(node.args[0].value,str))
            (source / 'source-revision').write_text(revision + '\n')
            state = work / 'state'; state.mkdir(mode=0o700)
            worker = load('migration_worker', ROOT / 'scripts/ops/cohort-migration-worker-1211.py')
            tamper = [False]
            def loader(name, path):
                actual = load(name + '_' + secrets.token_hex(4), path)
                if name == 'base' and tamper[0]:
                    actual.EXPECTED_CHECKSUMS[min(actual.EXPECTED_CHECKSUMS)] = '0' * 96
                if name == 'routine':
                    real_path, make_job = actual.Path, actual.make_job
                    actual.Path = lambda p: state / real_path(p).name if str(p).startswith('/state/') else real_path(p)
                    def make(module):
                        job = make_job(module); job.binary = str(BINARY); return job
                    actual.make_job = make
                return actual
            profile = {**environment, **ops.ROLES, 'DB_ADMIN_URL': dsn, 'OPS_OLD_INDEXER_STOPPED': '1'}
            with contextlib.redirect_stdout(io.StringIO()), patch_env(profile):
                PHASE = 'expected_checksum_refusal'
                tamper[0] = True
                rejected = False
                try: worker.run('inspect', release, loader, source, state)
                except RuntimeError: rejected = True
                require(rejected and history() == before, 'changed_expected_checksum_refusal')
                tamper[0] = False
                PHASE = 'guard_inspection'
                inspected = worker.run('inspect', release, loader, source, state)
                require(inspected['pending_versions'] == [ops.pre.TARGET], 'pending_exactly_cohort')
                # An actual serving runtime session blocks snapshot/migration.
                PHASE = 'active_runtime_refusal'
                active_env = {**pg_env, 'PGUSER': RUNTIME, 'PGPASSWORD': runtime_password}
                active = subprocess.Popen([PG + '/psql', '-X', '-qAt', '-c', 'SELECT pg_sleep(15)'], env=active_env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                try:
                    seen = False
                    for _ in range(20):
                        if sql("SELECT count(*)>0 FROM pg_stat_activity WHERE usename='" + RUNTIME + "'") == 't': seen = True; break
                        time.sleep(0.05)
                    require(seen, 'actual_runtime_session_ready')
                    rejected = False
                    try: worker.run('apply', release, loader, source, state)
                    except RuntimeError: rejected = True
                    require(rejected and history() == before, 'active_runtime_refusal')
                finally:
                    active.send_signal(signal.SIGINT); active.wait(timeout=3)
                    closed = False
                    for _ in range(30):
                        if sql("SELECT count(*)=0 FROM pg_stat_activity WHERE usename='" + RUNTIME + "'") == 't': closed = True; break
                        time.sleep(0.05)
                    require(closed, 'owned_runtime_session_closed')
                PHASE = 'snapshot_and_new_binary_migration'
                result = worker.run('apply', release, loader, source, state)
            require(result['overall'] == 'PASS' and result['schema_marker'] == ops.pre.TARGET and result['history_count'] == 68 and result['actual_permission_denials'] == 4, 'new_binary_migration_verification')
            PHASE = 'seed_and_original_clock'
            cohort = json.loads(sql("SELECT json_agg(json_build_object('window',\"window\",'cohort',cohort,'usd',amount_usd,'count',event_count,'share',share_pct,'clock',updated_at='2026-10-18T00:00:00Z'::timestamptz) ORDER BY \"window\",cohort) FROM protocol_fee_stats_by_cohort"))
            require(len(cohort) == 9 and all(row['clock'] for row in cohort), 'nine_seed_rows_original_clock')
            for row in cohort:
                if row['cohort'] != 'unknown':
                    require(row['usd'] == 0 and row['count'] == 0 and row['share'] == (0 if row['window']=='24h' else None), 'retail_mm_idle_seed')
                elif row['window']=='24h': require(row['usd']==12.5 and row['count']==4 and row['share']==100, 'priced_unknown_seed')
                elif row['window']=='7d': require(row['usd'] is None and row['count']==6 and row['share'] is None, 'unpriced_unknown_seed')
                else: require(row['usd']==0 and row['count']==0 and row['share'] is None, 'idle_unknown_seed')
            PHASE = 'actual_binary_replay'
            replay_before = history()
            with contextlib.redirect_stdout(io.StringIO()), patch_env(profile):
                replay = worker.run('apply', release, loader, source, state)
            require(replay['overall']=='PASS' and history()==replay_before and sql('SELECT count(*) FROM protocol_fee_stats_by_cohort')=='9', 'idempotent_actual_binary_replay')
            current = json.loads(history()); prior = json.loads(before)
            require(all(any(row == old for row in current) for old in prior), 'original_67_complete_rows_preserved')
            require(original_security == sql("SELECT json_agg(json_build_object('role',rolname,'password',rolpassword,'super',rolsuper,'createdb',rolcreatedb,'createrole',rolcreaterole,'inherit',rolinherit) ORDER BY rolname) FROM pg_authid WHERE rolname IN ('" + MIGRATOR + "','" + RUNTIME + "')"), 'roles_and_credentials_unchanged')
            PHASE = 'actual_runtime_login'
            runtime_env = {**pg_env, 'PGUSER': RUNTIME, 'PGPASSWORD': runtime_password}
            require(sql("SELECT version FROM indexer_schema_version; SELECT count(*) FROM protocol_fee_attributions", runtime_env).splitlines() == [str(ops.pre.TARGET), '0'], 'actual_runtime_login_new_schema')
            # Restore the real custom snapshot into another disposable DB.
            PHASE = 'actual_snapshot_restore'
            backups = list(state.rglob('*.dump'))
            require(len(backups) == 1, 'single_private_snapshot')
            sql('CREATE DATABASE cohort_restore OWNER ' + MIGRATOR + ' TEMPLATE template0; REVOKE ALL ON DATABASE cohort_restore FROM PUBLIC;')
            restore_env = {**pg_env, 'PGDATABASE': 'cohort_restore'}
            sql('ALTER SCHEMA public OWNER TO ' + MIGRATOR + ';', restore_env)
            process([PG + '/pg_restore', '--exit-on-error', '--clean', '--if-exists', '--no-owner', '--role=' + MIGRATOR, '--dbname=cohort_restore', str(backups[0])], env=restore_env)
            restored = sql("SELECT coalesce(json_agg(row_to_json(m) ORDER BY version)::text,'[]') FROM public._sqlx_migrations m", restore_env)
            require(restored == before, 'restored_original_history')
            return {'overall': 'PASS', 'checkout_head': revision, 'worktree_dirty': True, 'compile_scope': 'current worktree; seed/depositor fixes not contained in checkout_head', 'release_image_attested': False, 'migration_sha384': sql("SELECT encode(checksum,'hex') FROM public._sqlx_migrations WHERE version=" + str(ops.pre.TARGET)), 'new_binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
                'baseline_migrations': 67, 'final_migrations': 68, 'published_priced_unpriced_idle_seed_preserved': True, 'seed_source_clock_preserved': True, 'actual_binary_replay_idempotent': True, 'schema_marker': ops.pre.TARGET,
                'sqlx_pgoptions_env_effective_migrator': env_identity['effective_migrator'],
                'sqlx_dsn_options_effective_migrator': True, 'wrong_owner_refused_before_writes': True,
                'changed_expected_checksum_refused': True, 'active_runtime_refused': True,
                'original_67_rows_preserved': True, 'snapshot_restore_history_matches': True,
                'actual_runtime_login_tested': True, 'actual_permission_denials': 4, 'roles_credentials_unchanged': True,
                'snapshot_bytes': result['backup']['bytes'], 'snapshot_sha256': result['backup']['sha256'],
                'scope': 'owned local PostgreSQL/new debug binary; not production migration or release-image attestation'}
    finally:
        if owned:
            actual = process(['docker', 'inspect', '--format', '{{index .Config.Labels "cl8y.cohort-rehearsal"}}', container]).stdout.strip()
            require(actual == label, 'owned_cleanup_identity')
            process(['docker', 'rm', '-fv', container])


@contextlib.contextmanager
def patch_env(values):
    saved = dict(os.environ)
    os.environ.clear(); os.environ.update(values)
    try: yield
    finally: os.environ.clear(); os.environ.update(saved)


if __name__ == '__main__':
    started = time.monotonic()
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
    signal.alarm(50)
    try:
        original_binary = BINARY
        with tempfile.TemporaryDirectory(prefix='cohort-binary-pin-') as pinned:
            pin = Path(pinned) / 'new-indexer'
            try: os.link(BINARY, pin)
            except OSError: shutil.copy2(BINARY, pin)
            BINARY = pin
            result = run()
        BINARY = original_binary
        result['elapsed_seconds'] = round(time.monotonic() - started, 3)
        print(json.dumps(result))
    except Exception as error:
        print(json.dumps({'overall': 'FAIL', 'failure_class': type(error).__name__, 'phase': PHASE, 'elapsed_seconds': round(time.monotonic() - started, 3),
            'guard': str(error) if isinstance(error, RuntimeError) and re.fullmatch(r'[a-z0-9_]+', str(error)) else None,
            'engine_guard': re.sub('[^a-z0-9]+','_',str(error).lower()).strip('_')[:120] if str(error) in KNOWN_GUARDS else None}))
        raise SystemExit(1)
