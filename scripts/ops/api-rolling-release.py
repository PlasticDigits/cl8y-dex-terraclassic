#!/usr/bin/env python3
"""Exact-main rolling release; no API stop, restart, cancellation or DB operation.

Plan/profile/pin/deploy/status are separate, bounded invocations. The public
profile is the only writable environment key. Preserve the exact release pin.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('eligibility', ROOT / 'scripts/ops/api-rolling-eligibility.py')
e = importlib.util.module_from_spec(spec)
spec.loader.exec_module(e)
PROFILE_CHECKS = {'explicit_profile_valid', 'profile_runtime_only_public', 'profile_matches_reviewed_input'}


class Writer:
    def __init__(self, reader):
        self.reader, self.writes = reader, 0

    def call(self, path, method, payload=None):
        # There is deliberately no general-purpose mutation capability.
        pin = path == 'applications/' + e.APP and method == 'PATCH' and isinstance(payload, dict) and set(payload) == {'git_commit_sha'} and e.SHA.fullmatch(str(payload['git_commit_sha']))
        profile = path == 'applications/' + e.APP + '/envs' and method in {'POST', 'PATCH'} and isinstance(payload, dict) and payload.get('key') == 'API_READY_SCENARIOS' and set(payload) == {'key','value','is_runtime','is_buildtime','is_preview','is_literal','is_shown_once'} and payload['is_runtime'] is True and payload['is_buildtime'] is False and payload['is_preview'] is False and payload['is_literal'] is True and payload['is_shown_once'] is False
        deploy = path == 'deploy?uuid=' + e.APP + '&force=false' and method == 'POST' and payload is None
        e.require(pin or profile or deploy, 'release_mutation_scope')
        if profile: e.public_profile(payload['value'])
        self.writes += 1
        e.require(self.writes <= 1, 'one_release_mutation_per_phase')
        request = urllib.request.Request(self.reader.api_base + path, method=method,
            data=None if payload is None else e.canonical(payload),
            headers={'Authorization':'Bearer ' + self.reader.token, 'Content-Type':'application/json'})
        with self.reader.opener.open(request, timeout=15) as response:
            data = response.read(1048577)
        e.require(len(data) <= 1048576, 'release_response_bound')
        return json.loads(data) if data else {}


def proof_inputs(revision, profile_path, proof_path, *, exact_main=True):
    manifest, files = e.revision_manifest(revision, exact_main=exact_main)
    e.proof_ready(e.read_json(proof_path, private=True), manifest)
    e.startup_healthcheck(files)
    profile = e.public_profile(json.dumps(e.read_json(profile_path)))
    return manifest, profile


def state_for(path, revision, manifest, profile):
    state = e.read_json(path, private=True)
    e.require(state.get('schema') == 'api_rolling_release_v1' and state.get('application_uuid') == e.APP
        and state.get('revision') == revision and state.get('source_manifest_sha256') == manifest['sha256']
        and state.get('profile_sha256') == hashlib.sha256(e.canonical(profile)).hexdigest(), 'release_journal_binding')
    return state


def checked_inventory(reader, state, profile, permit_missing_profile=False):
    observed = e.inventory(reader, profile)
    required = set(observed['checks']) - (PROFILE_CHECKS if permit_missing_profile else set())
    e.require(all(observed['checks'][key] for key in required), 'rolling_eligibility_required')
    if state:
        e.require(observed['old_commit'] == state['old_commit'], 'serving_revision_changed_since_plan')
        e.require(observed['prior_git_commit_sha'] in {state['prior_git_commit_sha'],state['revision']}, 'release_pin_changed_since_plan')
    return observed


def plan(reader, revision, manifest, profile, journal):
    e.require(not journal.exists(), 'release_journal_exists')
    observed = checked_inventory(reader, None, profile, True)
    state = {'schema':'api_rolling_release_v1', 'application_uuid':e.APP, 'revision':revision,
        'source_manifest_sha256':manifest['sha256'], 'profile_sha256':hashlib.sha256(e.canonical(profile)).hexdigest(),
        'old_commit':observed['old_commit'], 'prior_git_commit_sha':observed['prior_git_commit_sha'],
        'prepared_at_utc':datetime.now(timezone.utc).isoformat()}
    e.write(journal, state)
    return {'overall':'PREPARED', 'remote_changed':False}


def configure_profile(reader, writer, state, profile):
    checked_inventory(reader, state, profile, True)
    rows = reader.api('applications/' + e.APP + '/envs')
    matches = [row for row in rows if row.get('key') == 'API_READY_SCENARIOS' and not row.get('is_preview')]
    e.require(len(matches) <= 1, 'duplicate_readiness_profile')
    payload = {'key':'API_READY_SCENARIOS','value':e.canonical(profile).decode(),
        'is_runtime':True,'is_buildtime':False,'is_preview':False,'is_literal':True,'is_shown_once':False}
    writer.call('applications/' + e.APP + '/envs', 'PATCH' if matches else 'POST', payload)
    rows = reader.api('applications/' + e.APP + '/envs')
    matches = [row for row in rows if row.get('key') == 'API_READY_SCENARIOS' and not row.get('is_preview')]
    e.require(len(matches) == 1 and all(matches[0].get(key) == value for key,value in payload.items()), 'profile_readback')
    state['profile_configured'] = True
    return {'overall':'PROFILE_CONFIGURED', 'restart_requested':False}


def pin(reader, writer, state, profile):
    observed = checked_inventory(reader, state, profile)
    if observed['prior_git_commit_sha'] != state['revision']:
        writer.call('applications/' + e.APP, 'PATCH', {'git_commit_sha':state['revision']})
    current = reader.api('applications/' + e.APP)
    e.require(current.get('uuid') == e.APP and current.get('git_commit_sha') == state['revision'], 'release_pin_readback')
    state['pinned'] = True
    return {'overall':'PINNED', 'restart_requested':False}


def deploy(reader, writer, state, profile, journal):
    if state.get('deployment_uuid'):
        return {'overall':'ALREADY_REQUESTED','deployment_uuid':state['deployment_uuid']}
    e.require(not state.get('deploy_requested'), 'ambiguous_deployment_requires_status')
    observed = checked_inventory(reader, state, profile)
    e.require(state.get('pinned') is True and observed['prior_git_commit_sha'] == state['revision'], 'exact_release_pin_required')
    rows = reader.api('deployments/applications/' + e.APP + '?skip=0&take=3')['deployments']
    e.require(all(e.ID.fullmatch(str(row.get('deployment_uuid',''))) for row in rows), 'prior_deployment_identity')
    state['previous_deployment_uuids'] = [row['deployment_uuid'] for row in rows]
    state['deploy_requested'] = True
    # Persist BEFORE dispatch: an ambiguous acknowledgement must never cause a
    # second deploy, cancellation or fallback to an incompatible old image.
    e.write(journal, state)
    response = writer.call('deploy?uuid=' + e.APP + '&force=false', 'POST')
    rows = response.get('deployments')
    e.require(isinstance(rows,list) and len(rows) == 1 and e.ID.fullmatch(str(rows[0].get('deployment_uuid',''))), 'single_deployment_acknowledgement')
    resource = rows[0].get('resource_uuid') or rows[0].get('application_uuid')
    e.require(resource in {None,e.APP}, 'deployment_resource_binding')
    state['deployment_uuid'] = rows[0]['deployment_uuid']
    return {'overall':'DEPLOY_REQUESTED','deployment_uuid':state['deployment_uuid']}


def status(reader, state):
    rows = reader.api('deployments/applications/' + e.APP + '?skip=0&take=3')['deployments']
    e.require(isinstance(rows,list) and len(rows) <= 3, 'deployment_history_bound')
    identity = state.get('deployment_uuid')
    if not identity:
        e.require(state.get('deploy_requested') is True, 'deployment_not_requested')
        matches = [row for row in rows if row.get('deployment_uuid') not in state.get('previous_deployment_uuids', [])
            and row.get('commit') == state['revision']]
        e.require(len(matches) <= 1, 'ambiguous_release_history')
        if not matches: return {'overall':'PENDING','acknowledgement_unresolved':True}
        identity = matches[0].get('deployment_uuid')
        e.require(e.ID.fullmatch(str(identity)), 'deployment_identity')
        state['deployment_uuid'] = identity
    matches = [row for row in rows if row.get('deployment_uuid') == identity]
    e.require(len(matches) == 1, 'deployment_scoped_history_binding')
    detail = reader.api('deployments/' + identity)
    embedded = detail.get('application', {})
    e.require(detail.get('deployment_uuid') == identity and embedded.get('uuid') == e.APP and embedded.get('name') == e.NAME
        and e.numeric(detail.get('application_id')) == e.numeric(matches[0].get('application_id')), 'deployment_application_binding')
    phase = detail.get('status')
    e.require(phase in {'queued','pending','in_progress','finished','failed','cancelled','cancelled-by-user','canceled'}, 'deployment_status')
    result = {'overall':'FAIL' if phase in {'failed','cancelled','cancelled-by-user','canceled'} else 'PENDING',
        'deployment_uuid':identity,'deployment_status':phase}
    if result['overall'] == 'FAIL':
        health = json.loads(reader.read('https://' + e.DOMAIN + '/health', 8192))
        result['old_api_still_serving'] = health.get('status') == 'ok' and health.get('git_sha') == state['old_commit']
    if phase == 'finished':
        e.require(detail.get('commit') == state['revision'], 'deployed_release_revision')
        app = reader.api('applications/' + e.APP)
        health = json.loads(reader.read('https://' + e.DOMAIN + '/health', 8192))
        ready = json.loads(reader.read('https://' + e.DOMAIN + '/ready-api', 8192))
        e.require(app.get('status') == 'running:healthy' and health.get('git_sha') == state['revision'] and health.get('status') == 'ok'
            and ready.get('ready') is True, 'live_release_usefulness')
        result['overall'] = 'PASS'
    # Publish counts only; full deployment output may contain private data.
    logs = detail.get('logs')
    if isinstance(logs,str):
        try: logs = json.loads(logs)
        except ValueError: logs = None
    available = isinstance(logs,list) and bool(logs) and all(isinstance(row,dict) and isinstance(row.get('output'),str) for row in logs)
    text = '\n'.join(row['output'] for row in logs) if available else ''
    available = available and bool(text.strip())
    result['deployment_logs_available'] = available
    result['log_failure_pattern_count'] = len(re.findall(r'(?im)(?:error:|failed to solve|unhealthy|out of memory|schema version differs)',text)) if available else None
    if phase == 'finished' and not available:
        result.update(overall='FAIL', reason='deployment_logs_unavailable')
    elif phase == 'finished' and result['log_failure_pattern_count']:
        result.update(overall='FAIL', reason='deployment_log_review_required')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase', choices=['plan','profile','pin','deploy','status'])
    parser.add_argument('--revision',required=True)
    parser.add_argument('--profile',type=Path,required=True)
    parser.add_argument('--proof',type=Path,required=True)
    parser.add_argument('--journal',type=Path,required=True)
    args = parser.parse_args()
    state = None
    try:
        manifest, profile = proof_inputs(args.revision,args.profile,args.proof, exact_main=args.phase != 'status')
        reader = e.load_reader(); writer = Writer(reader)
        if args.phase == 'plan': result = plan(reader,args.revision,manifest,profile,args.journal)
        else:
            state = state_for(args.journal,args.revision,manifest,profile)
            if args.phase == 'profile': result = configure_profile(reader,writer,state,profile)
            elif args.phase == 'pin': result = pin(reader,writer,state,profile)
            elif args.phase == 'deploy': result = deploy(reader,writer,state,profile,args.journal)
            else: result = status(reader,state)
            e.write(args.journal,state)
        print(json.dumps({'revision':args.revision,**result}))
        return int(result.get('overall') == 'FAIL')
    except Exception as error:
        if state is not None: e.write(args.journal,state)
        print(json.dumps({'overall':'FAIL','phase':args.phase,'failure_class':type(error).__name__,
            'guard':str(error) if isinstance(error,e.Refusal) else None}))
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
