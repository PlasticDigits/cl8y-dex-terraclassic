#!/usr/bin/env python3
"""Finite read-only schema discovery in a separate Coolify maintenance job.

Credentials stay in memory and a runtime-only, shown-once job variable. This
helper cannot migrate, stop the serving app, change a role or rewrite history.
"""
import argparse
import ast
import base64
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import urllib.parse
import urllib.request
import urllib.error

ROOT = Path(__file__).resolve().parents[2]
TEMPLATE = 'y6yipoxl1ckfzprcfmscosar'
TEMPLATE_HASH = '65a5e41f2188e6c81c6ef6f01883ecdd6420b59e50469211cd6087174d8e94b1'
BASE_HASH = '1d973d38a6a31628af02ede7add0011bcd06b4c6c46ac68815caf97edab2b09b'
DATABASE = 'haxovngt9di3lloakkpuj6dj'
BACKEND = 's14i3w45r5g6ulyu6nsxzm0n'
TABLES = {'protocol_fee_attributions', 'fee_registry_events', 'fee_registry_snapshots', 'protocol_fee_stats_by_cohort'}
TARGET = 20261018110000
ID = re.compile(r'[A-Za-z0-9_-]{1,128}')
SHA = re.compile(r'[0-9a-f]{40}')


class GuardError(ValueError):
    """Only fixed local labels, never received provider data."""


def require(condition, label):
    if not condition:
        raise GuardError(label)


def load_reader():
    spec = importlib.util.spec_from_file_location('cohort_attest', ROOT / 'scripts/qa/attest-trading-quotes-1473.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.Reader()


class TransportError(RuntimeError):
    """HTTP status and field names only, never raw response bodies."""


class Api:
    def __init__(self, reader):
        self.reader, self.writes = reader, 0

    def call(self, path, method='GET', payload=None):
        require(not path.startswith('/') and '..' not in path and len(path) < 256, 'api_path')
        if method == 'GET':
            try:
                return self.reader.api(path)
            except urllib.error.HTTPError as error:
                raise TransportError(error.code, []) from None
        require(method in {'POST', 'DELETE'}, 'mutation_method')
        self.writes += 1
        require(self.writes <= 12, 'mutation_budget')
        body = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(self.reader.api_base + path, data=body, method=method,
            headers={'Authorization': 'Bearer ' + self.reader.token, 'Content-Type': 'application/json'})
        try:
            with self.reader.opener.open(request, timeout=15) as response:
                data = response.read(1048577)
        except urllib.error.HTTPError as error:
            fields = []
            try:
                failure = json.loads(error.read(65537))
                errors = failure.get('errors') or {}
                fields = [key for key in errors if re.fullmatch(r'[A-Za-z0-9_.-]{1,80}', key)] if isinstance(errors, dict) else []
            except (ValueError, AttributeError):
                pass
            raise TransportError(error.code, fields) from None
        require(len(data) <= 1048576, 'mutation_body_bound')
        return json.loads(data) if data else {}


def manifest():
    result = {}
    for path in sorted((ROOT / 'indexer/migrations').glob('*.sql')):
        if path.name.endswith('.down.sql'):
            continue
        match = re.fullmatch(r'(\d{14})_[A-Za-z0-9_]+(?:\.up)?\.sql', path.name)
        require(match is not None, 'migration_filename')
        version = int(match[1])
        require(version not in result, 'duplicate_migration_version')
        result[version] = hashlib.sha384(path.read_bytes()).hexdigest()
    require(result and max(result) == TARGET, 'unexpected_schema_target')
    return result


def render_dockerfile(template, checksums):
    require(hashlib.sha256(template.encode()).hexdigest() == TEMPLATE_HASH, 'maintenance_template_changed')
    blobs = re.findall(r'[A-Za-z0-9+/=]{120,}', template)
    candidates = []
    for blob in blobs:
        try:
            source = base64.b64decode(blob, validate=True).decode()
        except (ValueError, UnicodeError):
            continue
        if hashlib.sha256(source.encode()).hexdigest() == BASE_HASH:
            candidates.append(source)
    require(len(candidates) == 1, 'maintenance_guard_identity')
    tree = ast.parse(candidates[0])
    require(any(isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'EXPECTED_CHECKSUMS' for t in node.targets) for node in tree.body), 'maintenance_checksum_guard_missing')
    # Preserve the reviewed runtime-compatible source byte-for-byte. Python3.12
    # ast.unparse may emit syntax unavailable in the pinned Debian3.11 runtime.
    guard = candidates[0] + '\nEXPECTED_CHECKSUMS = ' + repr(dict(sorted(checksums.items()))) + '\nTABLES.update(' + repr(sorted(TABLES)) + ')\n'
    ast.parse(guard, feature_version=(3, 11))
    worker = (ROOT / 'scripts/ops/cohort-preflight-worker-1211.py').read_text()
    require('READ_ONLY_PREFLIGHT_V1' in worker and 'subprocess.run' not in worker, 'worker_identity')
    ast.parse(worker, feature_version=(3, 11))
    lines = template.splitlines()
    final = max(i for i, line in enumerate(lines) if line.startswith('FROM '))
    runtime = []
    for line in lines[final:]:
        if line.startswith(('COPY ', 'CMD ', 'ENTRYPOINT ')) or re.search(r'[A-Za-z0-9+/=]{120,}', line):
            continue
        require(not line.startswith(('ENV ', 'ARG ', 'ADD ')), 'unexpected_runtime_recipe')
        runtime.append(line)
    require(runtime and ('postgres:' in runtime[0] or any('postgresql-client' in line for line in runtime)), 'maintenance_runtime_shape')
    for source, target in [(guard, '/ops/base.py'), (worker, '/ops/preflight.py')]:
        encoded = base64.b64encode(source.encode()).decode()
        runtime.append('RUN mkdir -p /ops && printf %s ' + encoded + ' | base64 -d > ' + target)
    runtime.append('ENTRYPOINT []')
    runtime.append('CMD ["python3", "/ops/preflight.py"]')
    return '\n'.join(runtime) + '\n'


def prepare(api, revision):
    require(SHA.fullmatch(revision), 'revision_shape')
    template = api.call('applications/' + TEMPLATE)
    backend = api.call('applications/' + BACKEND)
    database = api.call('databases/' + DATABASE)
    require(template.get('environment_id') == backend.get('environment_id') == database.get('environment_id'), 'resource_environment_mismatch')
    server = template['destination']['server']['uuid']
    require(server == backend['destination']['server']['uuid'] == database['destination']['server']['uuid'], 'resource_server_mismatch')
    require(any(urllib.parse.urlsplit(x.strip()).hostname == 'indexer.dex.cl8y.com' for x in backend.get('fqdn', '').split(',')), 'backend_domain')
    dsn = database.get('internal_db_url')
    require(isinstance(dsn, str), 'database_credential_unavailable')
    parsed = urllib.parse.urlsplit(dsn)
    require(parsed.scheme in {'postgres', 'postgresql'} and parsed.hostname == DATABASE and parsed.username and parsed.password and not parsed.query and not parsed.fragment, 'database_identity')
    require(urllib.parse.unquote(parsed.path).strip('/') == database.get('postgres_db') == 'postgres', 'database_name_mismatch')
    require(urllib.parse.unquote(parsed.username) == database.get('postgres_user') == 'postgres', 'database_admin_identity')
    require(urllib.parse.unquote(parsed.password) == database.get('postgres_password'), 'database_credential_mismatch')
    envs = api.call('applications/' + BACKEND + '/envs')
    runtime = [e for e in envs if e.get('key') == 'DATABASE_URL' and not e.get('is_preview')]
    require(len(runtime) == 1 and runtime[0].get('is_runtime') and not runtime[0].get('is_buildtime'), 'runtime_credential_scope')
    if runtime[0].get('value') is not None:
        serving = urllib.parse.urlsplit(runtime[0]['value'])
        require(serving.hostname == parsed.hostname and serving.path == parsed.path and serving.username == 'dex_indexer_runtime_20261006', 'serving_database_mismatch')
    checksums = manifest()
    dockerfile = render_dockerfile(template['dockerfile'], checksums)
    payload = {'project_uuid': template['environment']['project']['uuid'], 'server_uuid': server,
        'environment_uuid': template['environment']['uuid'], 'destination_uuid': template['destination']['uuid'],
        'dockerfile': base64.b64encode(dockerfile.encode()).decode(), 'ports_exposes': '80', 'name': 'dex-cohort-preflight-' + revision[:12] + '-' + secrets.token_hex(3),
        'description': 'READ ONLY cohort schema preflight; never migrates; revision ' + revision,
        'build_pack': 'dockerfile', 'instant_deploy': False, 'autogenerate_domain': False,
        'health_check_enabled': False, 'custom_docker_run_options': '--restart=no',
        'limits_memory': '256M', 'limits_cpus': '1', 'max_restart_count': 0}
    summary = {'revision': revision, 'mode': 'read_only', 'target_schema': TARGET,
        'expected_migration_count': len(checksums), 'manifest_sha256': hashlib.sha256(json.dumps(checksums, sort_keys=True).encode()).hexdigest(),
        'template_sha256': TEMPLATE_HASH, 'worker_recipe_sha256': hashlib.sha256(dockerfile.encode()).hexdigest(),
        'same_environment_and_server': True, 'runtime_only_credentials': True,
        'runtime_dsn_api_visible': runtime[0].get('value') is not None,
        'runtime_connection_identity_requires_job_probe': True,
        'manifest_source': 'worktree; read-only preflight only'}
    return payload, dsn, summary


def write_state(path, state):
    path.parent.mkdir(parents=True, exist_ok=True)
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), 'w') as stream:
        os.chmod(path, 0o600)
        json.dump(state, stream, indent=2)
        stream.write('\n')


def remove_credential(api, job):
    rows = api.call('applications/' + job + '/envs')
    for row in rows:
        if row.get('key') == 'DB_ADMIN_URL':
            require(ID.fullmatch(row.get('uuid', '')), 'credential_identity')
            api.call('applications/' + job + '/envs/' + row['uuid'], 'DELETE')
    require(not any(row.get('key') == 'DB_ADMIN_URL' for row in api.call('applications/' + job + '/envs')), 'credential_removal_readback')


def cleanup_owned(api, job):
    result = {'stop_acknowledged': False, 'credential_removed': False}
    try:
        api.call('applications/' + job + '/stop', 'POST')
        result['stop_acknowledged'] = True
    except Exception:
        pass
    finally:
        try:
            remove_credential(api, job)
            result['credential_removed'] = True
        except Exception:
            pass
    result['cleanup_overall'] = 'PASS' if all([result['stop_acknowledged'], result['credential_removed']]) else 'FAIL'
    return result


def bind_owned(app, state):
    require(app.get('name') == state['job_name'] and app.get('description') == 'READ ONLY cohort schema preflight; never migrates; revision ' + state['revision'], 'owned_job_binding')
    require(isinstance(app.get('dockerfile'), str) and hashlib.sha256(app['dockerfile'].encode()).hexdigest() == state['worker_recipe_sha256'], 'owned_job_recipe')


def create(api, revision, journal):
    require(not journal.exists(), 'journal_already_exists')
    payload, credential, summary = prepare(api, revision)
    result = api.call('applications/dockerfile', 'POST', payload)
    job = result.get('uuid')
    require(isinstance(job, str) and ID.fullmatch(job) and job not in {BACKEND, TEMPLATE}, 'created_job_identity')
    state = {'job_uuid': job, 'job_name': payload['name'], **summary}
    write_state(journal, state)
    bind_owned(api.call('applications/' + job), state)
    try:
        api.call('applications/' + job + '/envs', 'POST', {'key': 'DB_ADMIN_URL', 'value': credential,
            'is_runtime': True, 'is_buildtime': False, 'is_preview': False, 'is_literal': True, 'is_shown_once': True})
        for key, value in [('OPS_APPLICATION_DB', 'postgres'), ('INDEXER_RUNTIME_ROLE', 'dex_indexer_runtime_20261006'), ('INDEXER_MIGRATOR_ROLE', 'dex_indexer_migrator_20261006')]:
            api.call('applications/' + job + '/envs', 'POST', {'key': key, 'value': value, 'is_runtime': True, 'is_buildtime': False, 'is_preview': False, 'is_literal': True})
        response = api.call('applications/' + job + '/start', 'POST')
        state['started'] = True
        state['start_acknowledged'] = isinstance(response, dict)
        write_state(journal, state)
    except Exception:
        state.update(cleanup_owned(api, job))
        write_state(journal, state)
        raise
    return state


def status(api, journal, cleanup=False):
    state = json.loads(journal.read_text())
    job = state['job_uuid']
    require(ID.fullmatch(job) and job not in {BACKEND, TEMPLATE}, 'journal_job_identity')
    app = api.call('applications/' + job)
    bind_owned(app, state)
    logs = {}
    logs_http_status = None
    if not cleanup:
        try:
            logs = api.call('applications/' + job + '/logs?lines=40')
        except TransportError as error:
            logs_http_status = error.args[0]
    text = json.dumps(logs)
    # Only an explicitly allowlisted worker receipt is exported. Never raw logs.
    receipts = []
    for value in re.findall(r'\{[^{}]{1,8192}\}', text.replace('\\"', '"')):
        try:
            receipt = json.loads(value)
        except ValueError:
            continue
        if receipt.get('schema') == 'cohort_read_only_preflight_v1':
            allowed = {'schema', 'overall', 'history_count', 'pending_versions', 'schema_marker', 'runtime_session_seen', 'runtime_acl_safe', 'history_checksums_match', 'resource_schema_unique', 'failure_class', 'failure_guard', 'database_read_only_enforced', 'complete_history_prefix'}
            receipts.append({k: v for k, v in receipt.items() if k in allowed})
    result = {'job_uuid': job, 'job_status': app.get('status'), 'receipts': receipts[-1:], 'read_only': True, 'logs_http_status': logs_http_status}
    if receipts or cleanup or any(s in str(app.get('status','')) for s in ['exited', 'stopped']):
        result.update(cleanup_owned(api, job))
        state.update({key: result[key] for key in ['credential_removed', 'stop_acknowledged', 'cleanup_overall']})
        write_state(journal, state)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['plan', 'create', 'status', 'cleanup'])
    parser.add_argument('--revision', required=True)
    parser.add_argument('--journal', type=Path, required=True)
    args = parser.parse_args()
    try:
        api = Api(load_reader())
        if args.phase == 'plan':
            result = prepare(api, args.revision)[2]
        elif args.phase == 'create':
            result = create(api, args.revision, args.journal)
        else:
            result = status(api, args.journal, args.phase == 'cleanup')
        print(json.dumps(result))
    except Exception as error:
        print(json.dumps({'overall': 'FAIL', 'failure_class': type(error).__name__, 'operation': args.phase,
            'guard': error.args[0] if isinstance(error, GuardError) else None,
            'http_status': error.args[0] if isinstance(error, TransportError) else None,
            'validation_fields': error.args[1] if isinstance(error, TransportError) else []}))
        return 1
    return int(result.get('cleanup_overall') == 'FAIL' or any(row.get('overall') == 'FAIL' for row in result.get('receipts', [])))


if __name__ == '__main__':
    raise SystemExit(main())
