#!/usr/bin/env python3
"""Run the pinned maintenance engine against one exact release and migration."""
import contextlib
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

TABLES = ['protocol_fee_attributions', 'fee_registry_events', 'fee_registry_snapshots', 'protocol_fee_stats_by_cohort']


def require(condition, label):
    if not condition:
        raise RuntimeError(label)


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def run(action, release, load_module=None, ops_root=Path('/ops'), state_root=Path('/state')):
    require(action in {'waiting', 'inspect', 'apply', 'verify'}, 'action')
    require((ops_root / 'source-revision').read_text().strip() == release['revision'], 'compiled_source_revision')
    if action == 'waiting':
        return {'overall': 'WAITING', 'revision': release['revision'], 'database_access': False}
    readonly = action == 'inspect'
    os.environ['PGOPTIONS'] = '-c statement_timeout=30000 -c lock_timeout=5000' + (' -c default_transaction_read_only=on' if readonly else '')
    loader = load_module or module
    base, routine = loader('base', str(ops_root / 'base.py')), loader('routine', str(ops_root / 'routine.py'))
    require(routine.REVISION == release['revision'] and routine.NEW_VERSION == release['target'] and routine.ALLOWED_MIGRATIONS == {release['target']}, 'release_binding')
    old = routine.make_job(base)

    class BoundJob(old.__class__):
        def inspect(self):
            inventory = super().inspect()
            applied = {row['version'] for row in inventory['migrations']}
            marker = int(self.sql('SELECT version FROM public.indexer_schema_version WHERE singleton=true'))
            require(applied and applied == {v for v in base.EXPECTED_CHECKSUMS if v <= marker} and marker == max(applied), 'complete_history_prefix')
            return inventory

        def owner_sql(self, query):
            env = self.connection()
            env.update(PGOPTIONS='-c role=' + self.migrator + ' -c statement_timeout=30000 -c lock_timeout=5000' + (' -c default_transaction_read_only=on' if readonly else ''))
            result = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], input=query,
                text=True, capture_output=True, env=env, timeout=35)
            require(result.returncode == 0, 'owner_query_failed')
            return result.stdout.strip()

        def verify(self):
            inventory = self.inspect()
            require({row['version'] for row in inventory['migrations']} == set(base.EXPECTED_CHECKSUMS), 'pending_migration_remains')
            security = self.role_security_snapshot()
            before = {m['version']: m for m in json.loads((self.directory / 'routine-baseline-history.json').read_text())}
            current = {m['version']: m for m in self.full_history()}
            require(set(before).issubset(current) and all(current[v] == row for v, row in before.items()), 'original_history_changed')
            require(int(self.effective_runtime_sql('SELECT version FROM public.indexer_schema_version WHERE singleton=true')) == release['target'], 'runtime_schema_marker')
            names = ','.join(base.literal('public.' + table) for table in TABLES)
            require(self.effective_runtime_sql("SELECT bool_and(has_table_privilege(current_user,t,p)) FROM unnest(ARRAY[" + names + "]) t CROSS JOIN unnest(ARRAY['SELECT','INSERT','UPDATE','DELETE']) p") == 't', 'runtime_new_table_acl')
            # Permission denials are actual operations in rolled-back transactions,
            # with SQLx/marker/DDL authority tested under the effective runtime role.
            denied = 0
            for sql in ['SELECT * FROM public._sqlx_migrations LIMIT 0',
                        'UPDATE public.indexer_schema_version SET version=version WHERE false',
                        'CREATE TABLE public.cohort_runtime_denial_probe(id integer)',
                        'CREATE ROLE cohort_runtime_denial_probe NOLOGIN']:
                probe = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
                    input='BEGIN; SET LOCAL ROLE ' + base.identifier(self.runtime) + '; ' + sql + '; ROLLBACK;',
                    text=True, capture_output=True, env=self.connection(), timeout=35)
                require(probe.returncode != 0 and 'permission denied' in probe.stderr.lower(), 'runtime_permission_denial_missing')
                denied += 1
            require(self.role_security_snapshot() == security, 'role_security_changed')
            backup = self.backup_identity()
            result = {'backup': {key: backup[key] for key in ['bytes', 'sha256', 'observed_at_utc', 'structurally_valid']}, 'overall': 'PASS', 'revision': release['revision'], 'schema_marker': release['target'],
                'history_count': len(current), 'complete_history_preserved': True, 'runtime_table_grants': True,
                'actual_permission_denials': denied, 'runtime_login_tested': False, 'roles_credentials_memberships_unchanged': True}
            self.write('cohort-verification.json', result)
            return result

    job = BoundJob(state_root / ('cohort-' + release['revision'][:12]), old.binary)
    if readonly:
        require(job.sql("SELECT current_setting('transaction_read_only')='on'") == 't', 'read_only_enforcement')
        inventory = job.inspect()
        return {'overall': 'PASS', 'revision': release['revision'], 'history_count': len(inventory['migrations']),
            'pending_versions': sorted(set(base.EXPECTED_CHECKSUMS) - {m['version'] for m in inventory['migrations']}),
            'database_read_only_enforced': True}
    require(os.environ.get('OPS_OLD_INDEXER_STOPPED') == '1', 'serving_stop_required')
    if action == 'apply':
        result = job.apply()  # Existing snapshot -> exact binary migrate -> this verifier.
        return result['verify']
    return job.verify()


def main():
    started = time.monotonic()
    release = json.loads(Path('/ops/release.json').read_text())
    require(re.fullmatch(r'[0-9a-f]{40}', release['revision']), 'release_revision')
    action = os.environ.get('OPS_ROUTINE_ACTION', 'waiting')
    require(action in {'waiting', 'inspect', 'apply', 'verify'}, 'action')
    run_id = os.environ.get('OPS_RUN_ID', 'waiting')
    require(re.fullmatch(r'[A-Za-z0-9_-]{1,64}', run_id), 'run_identity')
    terminal = Path('/state/cohort-' + release['revision'][:12]) / ('terminal-' + action + '-' + run_id + '.json')
    terminal.parent.mkdir(parents=True, exist_ok=True)
    terminal.parent.chmod(0o700)
    operation = (terminal.parent / 'operation.lock').open('a')
    fcntl.flock(operation, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if terminal.exists():
        receipt = json.loads(terminal.read_text())
    else:
        signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
        signal.alarm(900 if action in {'apply', 'verify'} else 55)
        try:
            # Historical engine output remains private; only our bounded receipt
            # is emitted. Its snapshot/history files stay on the private volume.
            with contextlib.redirect_stdout(io.StringIO()):
                result = run(action, release)
            receipt = {'schema': 'cohort_migration_job_v1', 'action': action, 'run_id': run_id, **result}
        except Exception as error:
            receipt = {'schema': 'cohort_migration_job_v1', 'action': action, 'run_id': run_id, 'revision': release['revision'], 'overall': 'FAIL', 'failure_class': type(error).__name__}
        signal.alarm(0)
        with os.fdopen(os.open(terminal, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'w') as stream:
            json.dump(receipt, stream)
    fcntl.flock(operation, fcntl.LOCK_UN)
    operation.close()
    print(json.dumps(receipt), flush=True)
    # Receipt adoption is finite; restart re-emits this terminal result without
    # rerunning any database operation. An explicit new run ID owns any retry.
    time.sleep(max(0, 55 - (time.monotonic() - started)))


if __name__ == '__main__':
    main()
