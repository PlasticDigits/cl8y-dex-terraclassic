#!/usr/bin/env python3
"""Separate online cohort job; the production API must remain serving.

Deploy and verify the compatible 67/68 server first. This helper never stops,
restarts or changes the environment of that server and never restores its DB.
"""
import argparse
import ast
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import secrets

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


migration = load('online_lifecycle', ROOT / 'scripts/ops/cohort-migration-1211.py')
worker = load('online_worker', ROOT / 'scripts/ops/cohort-online-worker-1211.py')
pre = migration.pre


def render(template, revision, checksums, compiler):
    recipe, manifest = migration.render(template, revision, checksums, compiler)
    # The worker, like its migration-bearing binary, comes from the exact
    # reviewed release object rather than mutable operator working-tree bytes.
    source = migration.git('show', revision + ':scripts/ops/cohort-online-worker-1211.py').decode()
    ast.parse(source, feature_version=(3, 11))
    lines = []
    replacements = 0
    for line in recipe.splitlines():
        if line.endswith(' > /ops/routine.py'):
            continue
        if line.endswith(' > /ops/cohort-migrate.py'):
            line = 'RUN printf %s ' + base64.b64encode(source.encode()).decode() + ' | base64 -d > /ops/cohort-migrate.py'
            replacements += 1
        lines.append(line)
    pre.require(replacements == 1, 'online_worker_recipe_binding')
    return '\n'.join(lines) + '\n', {**manifest, 'mode': worker.MODE}


def prepare(api, revision):
    checksums = migration.release_manifest(revision)
    old_payload, credential, _ = pre.prepare(api, revision)
    template = api.call('applications/' + pre.TEMPLATE)
    compiler = next(line for line in migration.git('show', revision + ':docker/indexer/Dockerfile').decode().splitlines()
        if line.startswith('FROM rust:'))
    recipe, manifest = render(template['dockerfile'], revision, checksums, compiler)
    payload = {**old_payload, 'dockerfile': base64.b64encode(recipe.encode()).decode(),
        'name': 'dex-cohort-online-' + revision[:12] + '-' + secrets.token_hex(3),
        'description': 'Online additive cohort migration; exact release ' + revision,
        'limits_memory': '512M'}
    return payload, credential, {**manifest,
        'recipe_sha256': hashlib.sha256(recipe.encode()).hexdigest(),
        'expected_migration_count': len(checksums), 'only_allowed_migration': pre.TARGET,
        'database_resource_bound': True, 'privileged_serving_environment_changed': False}


def build(api, revision, journal):
    pre.require(not journal.exists(), 'migration_journal_exists')
    payload, _, plan = prepare(api, revision)
    return migration.create_prepared_job(api, payload, plan, journal, {'OPS_ONLINE_MODE': worker.MODE})


def start_action(api, state, action):
    pre.require(state.get('mode') == worker.MODE and action in {'inspect', 'apply', 'verify'}, 'online_action_mode')
    # A second command must not implicitly abort a still-owned snapshot or DDL
    # operation. Only status terminal handling or explicit cleanup resolves it.
    pre.require(state.get('cleanup_overall') == 'PASS' and state.get('credential_removed') is True
        and state.get('stop_acknowledged') is True, 'prior_online_operation_unresolved')
    migration.owned(api, state)
    serving = api.call('applications/' + pre.BACKEND)
    pre.require(serving.get('uuid') == pre.BACKEND and serving.get('status') == 'running:healthy', 'serving_must_remain_healthy')
    worker.serving_ready(state['revision'])
    if action in {'apply', 'verify'}:
        pre.require(state.get('inspection_passed') is True, 'online_preflight_required')
    _, credential, _ = pre.prepare(api, state['revision'])
    job = state['job_uuid']
    previous = migration.deployment_rows(api, state)
    dispatch_attempted = False
    try:
        migration.set_env(api, job, 'DB_ADMIN_URL', credential, True)
        migration.set_env(api, job, 'OPS_ROUTINE_ACTION', action)
        migration.set_env(api, job, 'OPS_ONLINE_MODE', worker.MODE)
        state['active_action'] = action
        state['active_run_id'] = secrets.token_hex(12)
        migration.set_env(api, job, 'OPS_RUN_ID', state['active_run_id'])
        state.update(cleanup_overall='PENDING', credential_removed=False, stop_acknowledged=False)
        previous_ids = [row['deployment_uuid'] for row in previous]
        migration.remember_start(state, {}, previous_ids)
        dispatch_attempted = True
        started = api.call('applications/' + job + '/start', 'POST')
        migration.remember_start(state, started, previous_ids)
    except Exception:
        # A lost acknowledgement can still own a real running migration. Keep
        # its nonce/history exclusions and reconcile with status; never stop it
        # just because the HTTP request failed after dispatch.
        if not dispatch_attempted:
            state.update(pre.cleanup_owned(api, job))
        raise
    return {'job_uuid': job, 'action': action, 'started': True, 'mode': worker.MODE}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['plan', 'build', 'status', 'inspect', 'apply', 'verify', 'cleanup'])
    parser.add_argument('--revision', required=True)
    parser.add_argument('--journal', type=Path, required=True)
    args = parser.parse_args()
    state = None
    try:
        api = migration.Api(pre.load_reader())
        if args.phase == 'plan':
            result = prepare(api, args.revision)[2]
        elif args.phase == 'build':
            result = build(api, args.revision, args.journal)
        else:
            state = json.loads(args.journal.read_text())
            pre.require(state.get('revision') == args.revision and state.get('mode') == worker.MODE, 'online_journal_binding')
            migration.owned(api, state)
            result = start_action(api, state, args.phase) if args.phase in {'inspect', 'apply', 'verify'} else migration.status(api, state, args.phase == 'cleanup')
            pre.write_state(args.journal, state)
        print(json.dumps(result))
        return int(result.get('overall') == 'FAIL' or result.get('cleanup_overall') == 'FAIL'
            or any(row.get('overall') == 'FAIL' for row in result.get('receipts', [])))
    except Exception as error:
        if state is not None:
            pre.write_state(args.journal, state)
        print(json.dumps({'overall': 'FAIL', 'phase': args.phase, 'failure_class': type(error).__name__,
            'guard': error.args[0] if isinstance(error, pre.GuardError) else None}))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
