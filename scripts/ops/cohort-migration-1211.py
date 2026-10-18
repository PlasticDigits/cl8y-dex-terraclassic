#!/usr/bin/env python3
"""Exact-main standalone cohort migration: plan/build first; explicit stop/apply later.

No command prints or persists a DB URL. The serving application never receives
migration credentials. The inherited snapshot/SQLx history engine is immutable.
"""
import argparse
import ast
import base64
from datetime import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('preflight', ROOT / 'scripts/ops/cohort-preflight-1211.py')
pre = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pre)
ROUTINE_HASH = '58227362ea3020121b9b6f1141a92a07a930f4f8090a050a4b9c1bdeab03de65'
ROLES = {'OPS_APPLICATION_DB': 'postgres', 'INDEXER_RUNTIME_ROLE': 'dex_indexer_runtime_20261006',
    'INDEXER_MIGRATOR_ROLE': 'dex_indexer_migrator_20261006', 'OPS_AUTH_MODE': 'bootstrap_owner_session'}


def git(*args):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, timeout=30)
    pre.require(result.returncode == 0, 'release_git_read_failed')
    return result.stdout


def release_manifest(revision):
    pre.require(pre.SHA.fullmatch(revision), 'release_revision')
    remote = git('ls-remote', '--exit-code', 'origin', 'refs/heads/main').decode().split()
    pre.require(len(remote) == 2 and remote[0] == revision, 'exact_remote_main_required')
    paths = git('ls-tree', '-r', '--name-only', revision, 'indexer/migrations').decode().splitlines()
    checksums = {}
    for path in paths:
        match = re.fullmatch(r'indexer/migrations/(\d{14})_[A-Za-z0-9_]+(?:\.up)?\.sql', path)
        if not match: continue
        version = int(match[1])
        pre.require(version not in checksums, 'duplicate_release_migration')
        checksums[version] = hashlib.sha384(git('show', revision + ':' + path)).hexdigest()
    pre.require(len(checksums) == 68 and max(checksums) == pre.TARGET, 'release_schema_target')
    return checksums


def replace_assignment(source, name, expression):
    source, count = re.subn(r'^' + name + r'\s*=.*$', name + ' = ' + expression, source, count=1, flags=re.M)
    pre.require(count == 1, 'routine_constant_missing')
    return source


def render(template, revision, checksums, compiler_line):
    # Reuses all existing guard bytes; only reviewed release constants change.
    runtime = pre.render_dockerfile(template, checksums)
    base_line = next(line for line in runtime.splitlines() if line.endswith(' > /ops/base.py'))
    routines = []
    for blob in re.findall(r'[A-Za-z0-9+/=]{120,}', template):
        try: source = base64.b64decode(blob, validate=True).decode()
        except (ValueError, UnicodeError): continue
        if hashlib.sha256(source.encode()).hexdigest() == ROUTINE_HASH: routines.append(source)
    pre.require(len(routines) == 1, 'routine_guard_identity')
    routine = routines[0]
    routine = replace_assignment(routine, 'REVISION', repr(revision))
    routine = replace_assignment(routine, 'NEW_VERSION', str(pre.TARGET))
    routine = replace_assignment(routine, 'ALLOWED_MIGRATIONS', repr({pre.TARGET}))
    routine = replace_assignment(routine, 'TABLES', '{' + ', '.join(repr(table) for table in sorted(pre.TABLES)) + '}')
    routine = replace_assignment(routine, 'BASE_PATH', "Path('/ops/base.py')")
    tree = ast.parse(routine, feature_version=(3, 11))
    factory = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'make_job')
    returned = factory.body[-1]
    pre.require(isinstance(returned, ast.Return) and isinstance(returned.value, ast.Call) and isinstance(returned.value.func, ast.Name) and returned.value.func.id == 'RoutineJob', 'routine_factory_shape')
    directory = returned.value.args[0]
    pre.require(isinstance(directory, ast.Call) and isinstance(directory.func, ast.Name) and directory.func.id == 'Path', 'routine_directory_shape')
    lines = routine.splitlines(keepends=True)
    begin = sum(map(len, lines[:directory.lineno-1])) + directory.col_offset
    end = sum(map(len, lines[:directory.end_lineno-1])) + directory.end_col_offset
    routine = routine[:begin] + "Path('/state/cohort-' + REVISION[:12])" + routine[end:]
    ast.parse(routine, feature_version=(3, 11))
    worker = (ROOT / 'scripts/ops/cohort-migration-worker-1211.py').read_text()
    ast.parse(worker, feature_version=(3, 11))
    pre.require(compiler_line.startswith('FROM rust:') and '@sha256:' in compiler_line, 'compiler_identity')
    # The historical job used a source archive. Fetch an exact public Git object
    # instead, then verify HEAD before compiling the migration-bearing binary.
    prefix = [compiler_line.rsplit(' AS ', 1)[0] + ' AS build', 'WORKDIR /source',
        'ENV SOURCE_DATE_EPOCH=0 CARGO_BUILD_JOBS=2',
        'RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*',
        'RUN git init . && git remote add origin https://git.cl8y.com/code/cl8y-dex-terraclassic.git && git fetch --depth=1 origin ' + revision + ' && git checkout --detach FETCH_HEAD && test "$(git rev-parse HEAD)" = ' + revision + ' && git rev-parse HEAD > /tmp/cohort-source-revision',
        'WORKDIR /source/indexer',
        'RUN cargo build --locked --release && strip target/release/cl8y-dex-indexer']
    for line in runtime.splitlines():
        if line.startswith(('ENTRYPOINT ', 'CMD ')) or re.search(r'[A-Za-z0-9+/=]{120,}', line): continue
        prefix.append(line)
    prefix.extend(['RUN mkdir -p /ops',
        'COPY --from=build /source/indexer/target/release/cl8y-dex-indexer /usr/local/bin/cl8y-dex-indexer',
        'COPY --from=build /tmp/cohort-source-revision /ops/source-revision', base_line])
    manifest = {'revision': revision, 'target': pre.TARGET,
        'migration_manifest_sha256': hashlib.sha256(json.dumps(checksums, sort_keys=True).encode()).hexdigest()}
    for source, target in [(routine, '/ops/routine.py'), (worker, '/ops/cohort-migrate.py'), (json.dumps(manifest), '/ops/release.json')]:
        prefix.append('RUN printf %s ' + base64.b64encode(source.encode()).decode() + ' | base64 -d > ' + target)
    prefix.extend(['ENTRYPOINT []', 'CMD ["python3", "/ops/cohort-migrate.py"]'])
    return '\n'.join(prefix) + '\n', manifest


def prepare(api, revision):
    checksums = release_manifest(revision)
    # Reuse validated DB/resource binding and credential retrieval, memory only.
    old_payload, credential, summary = pre.prepare(api, revision)
    template = api.call('applications/' + pre.TEMPLATE)
    compiler = next(line for line in git('show', revision + ':docker/indexer/Dockerfile').decode().splitlines() if line.startswith('FROM rust:'))
    recipe, manifest = render(template['dockerfile'], revision, checksums, compiler)
    payload = {**old_payload, 'dockerfile': base64.b64encode(recipe.encode()).decode(),
        'name': 'dex-cohort-migration-' + revision[:12] + '-' + secrets.token_hex(3),
        'description': 'Standalone cohort migration; exact release ' + revision,
        'limits_memory': '512M'}
    return payload, credential, {**manifest, 'recipe_sha256': hashlib.sha256(recipe.encode()).hexdigest(),
        'expected_migration_count': len(checksums), 'only_allowed_migration': pre.TARGET,
        'database_resource_bound': True, 'privileged_serving_environment_changed': False}


def owned(api, state):
    app = api.call('applications/' + state['job_uuid'])
    pre.require(app.get('name') == state['job_name'] and app.get('description') == 'Standalone cohort migration; exact release ' + state['revision'], 'migration_job_binding')
    pre.require(hashlib.sha256(str(app.get('dockerfile', '')).encode()).hexdigest() == state['recipe_sha256'], 'migration_recipe_binding')
    return app


def set_env(api, job, key, value, private=False):
    rows = api.call('applications/' + job + '/envs')
    matches = [row for row in rows if row.get('key') == key and not row.get('is_preview')]
    pre.require(len(matches) <= 1, 'duplicate_job_environment')
    payload = {'key': key, 'value': value, 'is_runtime': True, 'is_buildtime': False,
        'is_preview': False, 'is_literal': True, 'is_shown_once': private}
    # The existing helper only needs POST/DELETE. Native Coolify PATCH updates
    # a variable, avoiding delete/recreate races when selecting an owned action.
    return api.call('applications/' + job + '/envs', 'PATCH' if matches else 'POST', payload)


class Api(pre.Api):
    def call(self, path, method='GET', payload=None):
        if method != 'PATCH': return super().call(path, method, payload)
        pre.require(not path.startswith('/') and '..' not in path and len(path) < 256, 'api_path')
        self.writes += 1
        pre.require(self.writes <= 12, 'mutation_budget')
        request = pre.urllib.request.Request(self.reader.api_base + path, data=json.dumps(payload).encode(), method='PATCH',
            headers={'Authorization': 'Bearer ' + self.reader.token, 'Content-Type': 'application/json'})
        with self.reader.opener.open(request, timeout=15) as response:
            data = response.read(1048577)
        pre.require(len(data) <= 1048576, 'mutation_body_bound')
        return json.loads(data) if data else {}


def build(api, revision, journal):
    pre.require(not journal.exists(), 'migration_journal_exists')
    payload, _, plan = prepare(api, revision)
    result = api.call('applications/dockerfile', 'POST', payload)
    job = result.get('uuid')
    pre.require(isinstance(job, str) and pre.ID.fullmatch(job) and job not in {pre.BACKEND, pre.TEMPLATE}, 'new_migration_identity')
    state = {'job_uuid': job, 'job_name': payload['name'], 'active_action': 'waiting', 'active_run_id': 'build', **plan}
    pre.write_state(journal, state)
    owned(api, state)
    # Build starts credential-free in WAITING. No DB action can run in this mode.
    for key, value in {**ROLES, 'OPS_ROUTINE_ACTION': 'waiting', 'OPS_RUN_ID': 'build'}.items(): set_env(api, job, key, value)
    # A private named volume retains immutable snapshots and history on replacement.
    requested_name = 'cohort-' + job
    storage = api.call('applications/' + job + '/storages', 'POST', {'type': 'persistent', 'name': requested_name, 'mount_path': '/state'})
    storages = api.call('applications/' + job + '/storages')
    volumes = storages.get('persistent_storages', []) if isinstance(storages, dict) else []
    # Coolify's create_storage controller prefixes named volumes with the
    # owning application's UUID. Bind the returned UUID and the full scoped
    # name; accepting a suffix alone could select another application's volume.
    files = storages.get('file_storages', []) if isinstance(storages, dict) else []
    pre.require(isinstance(storage, dict) and pre.ID.fullmatch(str(storage.get('uuid', '')))
        and len(volumes) == 1 and volumes[0].get('uuid') == storage['uuid']
        and volumes[0].get('mount_path') == '/state'
        and volumes[0].get('name') == job + '-' + requested_name
        and not volumes[0].get('host_path') and files == [], 'private_snapshot_storage_readback')
    state['snapshot_storage_configured'] = True
    state['snapshot_storage_uuid'] = volumes[0]['uuid']
    api.call('applications/' + job + '/start', 'POST')
    state['build_started'] = True
    pre.write_state(journal, state)
    return state


def start_action(api, state, action):
    job = state['job_uuid']
    owned(api, state)
    serving = api.call('applications/' + pre.BACKEND)
    if action in {'apply', 'verify'}:
        pre.require(any(word in str(serving.get('status', '')) for word in ['stopped', 'exited']), 'serving_must_be_stopped')
        pre.require(state.get('inspection_passed') is True, 'migration_preflight_required')
    _, credential, _ = pre.prepare(api, state['revision'])
    cleanup = pre.cleanup_owned(api, job)
    pre.require(cleanup['cleanup_overall'] == 'PASS', 'prior_job_cleanup')
    try:
        set_env(api, job, 'DB_ADMIN_URL', credential, True)
        set_env(api, job, 'OPS_ROUTINE_ACTION', action)
        state['active_action'] = action
        state['active_run_id'] = secrets.token_hex(12)
        set_env(api, job, 'OPS_RUN_ID', state['active_run_id'])
        set_env(api, job, 'OPS_OLD_INDEXER_STOPPED', '1' if action in {'apply', 'verify'} else '0')
        api.call('applications/' + job + '/start', 'POST')
    except Exception:
        state.update(pre.cleanup_owned(api, job))
        raise
    state['active_action'] = action
    return {'job_uuid': job, 'action': action, 'started': True}


def receipts(logs):
    result = []
    def visit(value):
        if isinstance(value, dict):
            for child in value.values(): visit(child)
        elif isinstance(value, list):
            for child in value: visit(child)
        elif isinstance(value, str):
            for match in re.finditer(r'\{\s*"schema"\s*:', value):
                try: row, _ = json.JSONDecoder().raw_decode(value[match.start():])
                except ValueError: continue
                if row.get('schema') != 'cohort_migration_job_v1': continue
                if row.get('overall') not in {'PASS', 'WAITING', 'FAIL'}: continue
                allowed = {'schema', 'action', 'run_id', 'overall', 'revision', 'history_count', 'pending_versions', 'database_read_only_enforced',
                    'schema_marker', 'complete_history_preserved', 'runtime_table_grants', 'actual_permission_denials',
                    'runtime_login_tested', 'roles_credentials_memberships_unchanged', 'failure_class', 'database_access'}
                if row.get('action') not in {'waiting','inspect','apply','verify'} or not pre.ID.fullmatch(str(row.get('run_id',''))) or not pre.SHA.fullmatch(str(row.get('revision',''))): continue
                clean = {key: val for key, val in row.items() if key in allowed}
                if 'failure_class' in clean and clean['failure_class'] not in {'RuntimeError','ValueError','KeyError','TimeoutError','OSError','FileNotFoundError','PermissionError','JSONDecodeError','TimeoutExpired','BlockingIOError','IndexError','TypeError'}: clean['failure_class'] = 'Error'
                boolean_keys = {'database_read_only_enforced','complete_history_preserved','runtime_table_grants','runtime_login_tested','roles_credentials_memberships_unchanged','database_access'}
                if any(key in clean and type(clean[key]) is not bool for key in boolean_keys): continue
                if any(key in clean and (type(clean[key]) is not int or not 0 <= clean[key] < 100000000000000) for key in ['history_count','schema_marker','actual_permission_denials']): continue
                if 'pending_versions' in clean and clean['pending_versions'] not in [[],[pre.TARGET]]: continue
                backup = row.get('backup')
                if isinstance(backup, dict) and re.fullmatch(r'[0-9a-f]{64}', str(backup.get('sha256', ''))):
                    try:
                        clock = datetime.fromisoformat(str(backup.get('observed_at_utc','')).replace('Z','+00:00'))
                        valid_backup = clock.tzinfo is not None and type(backup.get('bytes')) is int and 0 < backup['bytes'] < 1000000000000 and type(backup.get('structurally_valid')) is bool
                    except ValueError: valid_backup = False
                    if valid_backup: clean['backup'] = {key: backup[key] for key in ['bytes', 'sha256', 'observed_at_utc', 'structurally_valid']}
                result.append(clean)
    visit(logs)
    return result[-16:]


def status(api, state, cleanup=False):
    app = owned(api, state)
    found, log_status = [], None
    if not cleanup:
        try: found = receipts(api.call('applications/' + state['job_uuid'] + '/logs?lines=60'))
        except pre.TransportError as error: log_status = error.args[0]
    result = {'job_uuid': state['job_uuid'], 'job_status': app.get('status'), 'receipts': found, 'logs_http_status': log_status}
    found = [row for row in found if row.get('action') == state.get('active_action') and row.get('run_id') == state.get('active_run_id') and row.get('revision') == state['revision']]
    pre.require(len({json.dumps(row,sort_keys=True) for row in found}) <= 1, 'conflicting_terminal_receipts')
    found = found[-1:]
    result['receipts'] = found
    if found or cleanup or any(word in str(app.get('status','')) for word in ['stopped','exited']):
        result.update(pre.cleanup_owned(api, state['job_uuid']))
        state.update({key: result[key] for key in ['credential_removed', 'stop_acknowledged', 'cleanup_overall']})
    if found and found[0].get('overall') == 'PASS':
        row = found[0]
        pre.require(row.get('revision') == state['revision'], 'receipt_revision')
        if row.get('action') == 'inspect':
            pre.require(row.get('pending_versions') in [[], [pre.TARGET]] and row.get('database_read_only_enforced') is True and row.get('history_count') == state['expected_migration_count'] - len(row.get('pending_versions', [])), 'inspection_receipt')
            state['inspection_passed'] = True
        elif row.get('action') in {'apply', 'verify'}:
            pre.require(row.get('schema_marker') == pre.TARGET and row.get('history_count') == state['expected_migration_count'] and row.get('complete_history_preserved') is True and row.get('actual_permission_denials') == 4 and row.get('runtime_table_grants') is True and row.get('roles_credentials_memberships_unchanged') is True and row.get('backup', {}).get('structurally_valid') is True, 'migration_receipt_incomplete')
            state['migration_verified'] = True
            state['verification'] = row
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['plan', 'build', 'status', 'inspect', 'stop-serving', 'apply', 'verify', 'cleanup'])
    parser.add_argument('--revision', required=True)
    parser.add_argument('--journal', type=Path, required=True)
    args = parser.parse_args()
    state = None
    try:
        api = Api(pre.load_reader())
        if args.phase == 'plan': result = prepare(api, args.revision)[2]
        elif args.phase == 'build': result = build(api, args.revision, args.journal)
        else:
            state = json.loads(args.journal.read_text())
            pre.require(state['revision'] == args.revision, 'journal_revision')
            owned(api, state)
            if args.phase == 'stop-serving':
                serving = api.call('applications/' + pre.BACKEND)
                pre.require(any(pre.urllib.parse.urlsplit(x.strip()).hostname == 'indexer.dex.cl8y.com' for x in serving.get('fqdn', '').split(',')), 'serving_identity')
                pre.require(state.get('inspection_passed'), 'preflight_before_serving_stop')
                api.call('applications/' + pre.BACKEND + '/stop', 'POST')
                result = {'serving_stop_requested': True}
            elif args.phase in {'inspect', 'apply', 'verify'}: result = start_action(api, state, args.phase)
            else: result = status(api, state, args.phase == 'cleanup')
            pre.write_state(args.journal, state)
        print(json.dumps(result))
        return int(result.get('cleanup_overall') == 'FAIL' or any(row.get('overall') == 'FAIL' for row in result.get('receipts', [])))
    except Exception as error:
        if state is not None: pre.write_state(args.journal, state)
        print(json.dumps({'overall': 'FAIL', 'phase': args.phase, 'failure_class': type(error).__name__,
            'guard': error.args[0] if isinstance(error, pre.GuardError) else None}))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
