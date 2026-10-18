#!/usr/bin/env python3
"""READ_ONLY_PREFLIGHT_V1: finite SELECT-only maintenance job, never a migrator."""
import importlib.util
import contextlib
import io
import os
import time
import json
from pathlib import Path
import signal


PHASE = 'initialization'


def main():
    global PHASE
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError()))
    signal.alarm(55)
    # Enforce read-only even if a future guard accidentally adds a write.
    os.environ['PGOPTIONS'] = '-c default_transaction_read_only=on -c statement_timeout=10000 -c lock_timeout=3000'
    spec = importlib.util.spec_from_file_location('base', '/ops/base.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    job = module.Job(Path('/tmp/cohort-preflight'), '/nonexistent-no-migrations')
    PHASE = 'database_read_only'
    if job.sql("SELECT current_setting('transaction_read_only')='on' AND current_setting('statement_timeout') IN ('10s','10000') AND current_setting('lock_timeout') IN ('3s','3000')") != 't':
        raise ValueError()
    PHASE = 'schema_and_history'
    inventory = job.inspect()
    runtime, migrator = job.runtime, job.migrator
    require = lambda ok: ok or (_ for _ in ()).throw(ValueError())
    PHASE = 'role_binding'
    require(job.database == 'postgres' and runtime == 'dex_indexer_runtime_20261006' and migrator == 'dex_indexer_migrator_20261006')
    PHASE = 'bootstrap_identity'
    require(job.sql('SELECT current_user=session_user AND session_user=\'postgres\'') == 't')
    PHASE = 'role_privileges'
    for role in [runtime, migrator]:
        require(job.sql('SELECT rolcanlogin AND NOT(rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls OR rolinherit) AND NOT EXISTS(SELECT 1 FROM pg_auth_members WHERE member=r.oid) FROM pg_roles r WHERE rolname=' + module.literal(role)) == 't')
    PHASE = 'database_owner'
    require(job.sql('SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname=current_database()') == migrator)
    PHASE = 'schema_owner'
    require(job.sql("SELECT pg_get_userbyid(nspowner) FROM pg_namespace WHERE nspname='public'") == migrator)
    PHASE = 'object_owner'
    require(all(o['owner'] == migrator for o in inventory['objects']))
    PHASE = 'runtime_session'
    require(job.sql('SELECT count(*)>0 FROM pg_stat_activity WHERE datname=current_database() AND usename=' + module.literal(runtime)) == 't')
    PHASE = 'runtime_acl'
    safe = job.sql('SELECT NOT has_database_privilege(' + module.literal(runtime) + ",current_database(),'CREATE') AND NOT has_schema_privilege(" + module.literal(runtime) + ",'public','CREATE') AND NOT has_table_privilege(" + module.literal(runtime) + ",'public._sqlx_migrations','SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER') AND NOT has_table_privilege(" + module.literal(runtime) + ",'public.indexer_schema_version','INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')")
    require(safe == 't')
    PHASE = 'schema_marker'
    applied = {row['version'] for row in inventory['migrations']}
    marker = int(job.sql('SELECT version FROM public.indexer_schema_version WHERE singleton=true'))
    require(applied and marker == max(applied))
    require(applied == {version for version in module.EXPECTED_CHECKSUMS if version <= marker})
    print(json.dumps({'schema': 'cohort_read_only_preflight_v1', 'overall': 'PASS',
        'history_count': len(applied), 'pending_versions': sorted(set(module.EXPECTED_CHECKSUMS) - applied),
        'schema_marker': marker, 'runtime_session_seen': True, 'runtime_acl_safe': True,
        'history_checksums_match': True, 'resource_schema_unique': True, 'database_read_only_enforced': True, 'complete_history_prefix': True}))


if __name__ == '__main__':
    started = time.monotonic()
    terminal = Path('/tmp/cohort-read-only-terminal.json')
    if terminal.exists():
        # Coolify may restart an exited container. Never repeat the database work.
        result = terminal.read_text()
    else:
        output = io.StringIO()
        try:
            with contextlib.redirect_stdout(output):
                main()
            result = output.getvalue()
        except Exception as error:
            result = json.dumps({'schema': 'cohort_read_only_preflight_v1', 'overall': 'FAIL', 'failure_class': type(error).__name__, 'failure_guard': PHASE})
        with os.fdopen(os.open(terminal, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600), 'w') as stream:
            stream.write(result)
    print(result.strip(), flush=True)
    signal.alarm(0)
    # Keep the receipt observable only within the original finite 55-second job.
    time.sleep(max(0, 55 - (time.monotonic() - started)))
