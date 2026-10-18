#!/usr/bin/env python3
"""Online, append-only cohort migration. Never stops or restores the serving DB.

The compiled serving revision must support both reviewed schema markers before
this worker can mutate anything. Privileged credentials stay in this job only.
"""
import contextlib
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time
import urllib.request
from urllib.parse import quote, unquote, urlunsplit

PREVIOUS = 20261016120000
TARGET = 20261018110000
MODE = 'online_additive_cohorts_v1'
TABLES = ['protocol_fee_attributions', 'fee_registry_events', 'fee_registry_snapshots', 'protocol_fee_stats_by_cohort']


def require(ok, guard):
    if not ok:
        raise RuntimeError(guard)


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def file_digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs):
        return None


def serving_ready(revision, opener=None):
    require(re.fullmatch(r'[0-9a-f]{40}', revision), 'serving_revision_shape')
    transport = opener or urllib.request.build_opener(NoRedirect())
    for path in ['/health', '/ready-api']:
        request = urllib.request.Request('https://indexer.dex.cl8y.com' + path,
            headers={'Cache-Control': 'no-cache', 'Accept': 'application/json'})
        with transport.open(request, timeout=5) as response:
            data = response.read(65537)
            require(response.status == 200 and len(data) <= 65536, 'serving_readiness')
        value = json.loads(data)
        if path == '/health':
            require(value.get('status') == 'ok' and value.get('git_sha') == revision, 'serving_revision_binding')
        else:
            require(value.get('ready') is True, 'serving_useful_readiness')


def make_job(base, directory, binary, readiness):
    class OnlineJob(base.Job):
        readonly = False

        def owner_session_url(self):
            options = '-c role=' + self.migrator + ' -c lock_timeout=250ms -c statement_timeout=5000ms'
            return urlunsplit((self.admin.scheme, self.admin.netloc, '/' + self.database,
                'options=' + quote(options, safe=''), ''))

        def owner_sql(self, query):
            env = self.connection()
            env['PGOPTIONS'] = '-c role=' + self.migrator + ' -c lock_timeout=250ms -c statement_timeout=5000ms' + (
                ' -c default_transaction_read_only=on' if self.readonly else '')
            result = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], input=query,
                text=True, capture_output=True, env=env, timeout=10)
            require(result.returncode == 0, 'owner_query_failed')
            return result.stdout.strip()

        def role_fingerprint(self):
            # Only an aggregate digest is retained privately, never password
            # verifiers or their individual hashes. Nothing here authenticates.
            roles = self.sql("SELECT coalesce(json_agg(row_to_json(r) ORDER BY oid),'[]') FROM pg_authid r")
            members = self.sql("SELECT coalesce(json_agg(row_to_json(m) ORDER BY roleid,member,grantor),'[]') FROM pg_auth_members m")
            return hashlib.sha256((roles + '\n' + members).encode()).hexdigest()

        def history(self):
            return self.json_sql("SELECT coalesce(json_agg(row_to_json(m) ORDER BY version),'[]') FROM public._sqlx_migrations m")

        def existing_acl(self):
            # Canonical privilege entries avoid differences in ACL array order.
            excluded = ','.join(base.literal(name) for name in TABLES)
            return self.json_sql("""WITH privileges AS (
              SELECT 'object' AS kind,c.relname AS name,c.relowner AS owner,
                     a.grantee,a.privilege_type,a.grantor,a.is_grantable
              FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
              LEFT JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault(
                CASE WHEN c.relkind='S' THEN 's'::\"char\" ELSE 'r'::\"char\" END,c.relowner))) a ON true
              WHERE n.nspname='public' AND c.relkind IN ('r','p','S')
                AND c.relname NOT IN (""" + excluded + """)
              UNION ALL
              SELECT 'schema',n.nspname,n.nspowner,a.grantee,a.privilege_type,a.grantor,a.is_grantable
              FROM pg_namespace n
              LEFT JOIN LATERAL aclexplode(coalesce(n.nspacl,acldefault('n',n.nspowner))) a ON true
              WHERE n.nspname='public'
              UNION ALL
              SELECT 'database',d.datname,d.datdba,a.grantee,a.privilege_type,a.grantor,a.is_grantable
              FROM pg_database d
              LEFT JOIN LATERAL aclexplode(coalesce(d.datacl,acldefault('d',d.datdba))) a ON true
              WHERE d.datname=current_database()
            ) SELECT coalesce(json_agg(row_to_json(p) ORDER BY kind,name,grantee,
              privilege_type,grantor,is_grantable),'[]') FROM privileges p""")

        def inspect(self):
            require(os.environ.get('OPS_ONLINE_MODE') == MODE, 'explicit_online_mode_required')
            require(os.environ.get('OPS_OLD_INDEXER_STOPPED', '0') == '0', 'stopped_serving_flag_forbidden')
            require(self.database == 'postgres' and self.runtime == 'dex_indexer_runtime_20261006'
                and self.migrator == 'dex_indexer_migrator_20261006' and unquote(self.admin.username) == 'postgres', 'existing_identity_binding')
            require(self.sql("SELECT current_user=session_user AND session_user='postgres' AND current_database()='postgres'") == 't', 'bootstrap_session_binding')
            for role in [self.runtime, self.migrator]:
                require(self.sql("SELECT rolcanlogin AND NOT(rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls OR rolinherit) AND NOT EXISTS(SELECT 1 FROM pg_auth_members WHERE member=r.oid) FROM pg_roles r WHERE rolname=" + base.literal(role)) == 't', 'role_boundary')
            require(self.owner_sql('SELECT current_user=' + base.literal(self.migrator)) == 't', 'effective_owner_binding')
            inventory = super().inspect()
            require(all(obj['owner'] == self.migrator for obj in inventory['objects']), 'application_owner_binding')
            require(self.sql("SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname=current_database()") == self.migrator
                and self.sql("SELECT pg_get_userbyid(nspowner) FROM pg_namespace WHERE nspname='public'") == self.migrator, 'database_schema_owner')
            applied = {row['version'] for row in inventory['migrations']}
            require(max(base.EXPECTED_CHECKSUMS) == TARGET and len(base.EXPECTED_CHECKSUMS) == 68
                and applied in [set(base.EXPECTED_CHECKSUMS), set(base.EXPECTED_CHECKSUMS) - {TARGET}], 'exact_history_prefix')
            marker = int(self.sql('SELECT version FROM public.indexer_schema_version WHERE singleton=true'))
            require(marker in {PREVIOUS, TARGET} and (marker == max(applied)
                or marker == PREVIOUS and max(applied) == TARGET), 'reviewed_schema_or_forward_pending')
            return inventory

        def backup(self):
            receipt_path = self.directory / 'online-backup.json'
            target = self.directory / 'online-before.dump'
            if receipt_path.exists():
                receipt = json.loads(receipt_path.read_text())
                require(target.is_file() and file_digest(target) == receipt['sha256'], 'immutable_backup_identity')
                return receipt
            require(not target.exists(), 'partial_snapshot_requires_new_owned_operation')
            baseline = self.history()
            require(max(row['version'] for row in baseline) == PREVIOUS, 'snapshot_before_first_target_only')
            self.write('online-baseline.json', {'history': baseline, 'acl': self.existing_acl(), 'roles_digest': self.role_fingerprint()})
            env = self.connection()
            env['PGOPTIONS'] = '-c lock_timeout=250ms -c statement_timeout=30000ms -c default_transaction_read_only=on'
            with target.open('xb') as stream:
                target.chmod(0o600)
                result = subprocess.run(['pg_dump', '--format=custom', '--no-password', '--lock-wait-timeout=250ms'],
                    stdout=stream, stderr=subprocess.PIPE, env=env, timeout=900)
            require(result.returncode == 0, 'concurrent_snapshot_failed')
            listing = subprocess.run(['pg_restore', '--list', str(target)], capture_output=True, timeout=30)
            require(listing.returncode == 0 and target.stat().st_size >= 1024, 'snapshot_structure')
            receipt = {'bytes': target.stat().st_size, 'sha256': file_digest(target),
                'observed_at_utc': datetime.now(timezone.utc).isoformat(), 'structurally_valid': True,
                'mode': 'concurrent_mvcc', 'automatic_database_restore_allowed': False}
            self.write('online-backup.json', receipt)
            return receipt

        def verify(self):
            inventory = self.inspect()
            baseline = json.loads((self.directory / 'online-baseline.json').read_text())
            before = {row['version']: row for row in baseline['history']}
            after = {row['version']: row for row in self.history()}
            require(set(after) == set(base.EXPECTED_CHECKSUMS)
                and all(after.get(version) == row for version, row in before.items()), 'complete_original_history_preserved')
            require(self.existing_acl() == baseline['acl'], 'existing_acl_changed')
            require(self.role_fingerprint() == baseline['roles_digest'], 'roles_credentials_memberships_changed')
            require(int(self.sql('SELECT version FROM public.indexer_schema_version WHERE singleton=true')) == TARGET, 'schema_marker')
            names = ','.join(base.literal('public.' + table) for table in TABLES)
            require(self.sql('BEGIN; SET LOCAL ROLE ' + base.identifier(self.runtime) + "; SELECT bool_and(has_table_privilege(current_user,t,p)) FROM unnest(ARRAY[" + names + "]) t CROSS JOIN unnest(ARRAY['SELECT','INSERT','UPDATE','DELETE']) p; ROLLBACK;") == 't', 'new_table_runtime_grants')
            denied = 0
            for statement in ['SELECT * FROM public._sqlx_migrations LIMIT 0',
                'UPDATE public.indexer_schema_version SET version=version WHERE false',
                'CREATE TABLE public.online_runtime_denial_probe(id integer)', 'CREATE ROLE online_runtime_denial_probe NOLOGIN']:
                result = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
                    input='BEGIN; SET LOCAL ROLE ' + base.identifier(self.runtime) + '; ' + statement + '; ROLLBACK;',
                    text=True, capture_output=True, env=self.connection(), timeout=10)
                require(result.returncode != 0 and 'permission denied' in result.stderr.lower(), 'permission_denial_missing')
                denied += 1
            readiness()
            return {'overall': 'PASS', 'mode': MODE, 'history_count': len(inventory['migrations']), 'schema_marker': TARGET,
                'complete_history_preserved': True, 'existing_acl_unchanged': True, 'runtime_table_grants': True,
                'actual_permission_denials': denied, 'roles_credentials_memberships_unchanged': True,
                'serving_compatibility_verified': True, 'runtime_login_tested': False,
                'backup': self.backup()}

        def apply_online(self):
            self.inspect()
            readiness()
            self.backup()
            # Snapshotting is concurrent and may be slow. Revalidate ownership
            # and useful compatible serving immediately before the first DDL.
            self.inspect()
            readiness()
            env = {'PATH': os.environ.get('PATH', '/usr/bin:/bin'), 'DATABASE_URL': self.owner_session_url(),
                'INDEXER_RUNTIME_ROLE': self.runtime}
            result = subprocess.run([self.binary, 'migrate'], env=env, capture_output=True, timeout=900)
            # Preserve diagnostic class, never publish database messages/DSNs.
            require(result.returncode == 0, 'scoped_migration_cli_failed')
            return self.verify()

    return OnlineJob(directory, binary)


def run(action, release, ops=Path('/ops'), state=Path('/state'), readiness=None):
    require(action in {'waiting', 'inspect', 'apply', 'verify'}, 'action')
    require((ops / 'source-revision').read_text().strip() == release['revision'], 'compiled_source_revision')
    if action == 'waiting':
        return {'overall': 'WAITING', 'database_access': False}
    base = load('online_base', ops / 'base.py')
    checker = readiness or (lambda: serving_ready(release['revision']))
    os.environ['PGOPTIONS'] = '-c lock_timeout=250ms -c statement_timeout=30000ms' + (
        ' -c default_transaction_read_only=on' if action == 'inspect' else '')
    job = make_job(base, state / ('online-' + release['revision'][:12]), '/usr/local/bin/cl8y-dex-indexer', checker)
    job.readonly = action == 'inspect'
    if action == 'inspect':
        require(job.sql("SELECT current_setting('transaction_read_only')='on'") == 't', 'read_only_enforcement')
        inventory = job.inspect()
        checker()
        return {'overall': 'PASS', 'history_count': len(inventory['migrations']),
            'pending_versions': sorted(set(base.EXPECTED_CHECKSUMS) - {row['version'] for row in inventory['migrations']}),
            'database_read_only_enforced': True, 'serving_compatibility_verified': True}
    return job.apply_online() if action == 'apply' else job.verify()


def main():
    release = json.loads(Path('/ops/release.json').read_text())
    action = os.environ.get('OPS_ROUTINE_ACTION', 'waiting')
    run_id = os.environ.get('OPS_RUN_ID', 'build')
    require(re.fullmatch(r'[0-9a-f]{40}', release['revision']) and re.fullmatch(r'[A-Za-z0-9_-]{1,64}', run_id), 'run_binding')
    directory = Path('/state') / ('online-' + release['revision'][:12])
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    directory.chmod(0o700)
    lock = (directory / 'operation.lock').open('a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    terminal = directory / ('terminal-' + action + '-' + run_id + '.json')
    started = time.monotonic()
    if terminal.exists():
        receipt = json.loads(terminal.read_text())
    else:
        signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
        signal.alarm(900 if action in {'apply', 'verify'} else 55)
        try:
            with contextlib.redirect_stdout(io.StringIO()):
                result = run(action, release)
            receipt = {'schema': 'cohort_migration_job_v1', 'revision': release['revision'], 'mode': MODE,
                'action': action, 'run_id': run_id, **result}
        except Exception as error:
            guard = str(error) if isinstance(error, RuntimeError) and re.fullmatch(r'[a-z_]{1,80}', str(error)) else None
            receipt = {'schema': 'cohort_migration_job_v1', 'revision': release['revision'], 'mode': MODE,
                'action': action, 'run_id': run_id, 'overall': 'FAIL', 'failure_class': type(error).__name__, 'failure_guard': guard}
        signal.alarm(0)
        with os.fdopen(os.open(terminal, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'w') as stream:
            json.dump(receipt, stream)
    fcntl.flock(lock, fcntl.LOCK_UN)
    lock.close()
    print(json.dumps(receipt), flush=True)
    time.sleep(max(0, 55 - (time.monotonic() - started)))


if __name__ == '__main__':
    main()
