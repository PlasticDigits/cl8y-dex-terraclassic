#!/usr/bin/env python3
"""Bounded frontend-only Coolify release for #1560/#1561; never prints raw logs.

Run through bao-exec with COOLIFY_URL,COOLIFY_WIRE_TOKEN. Inspect is read-only;
deploy explicitly pins the reviewed main revision and starts its rolling build.
The existing API, indexer and logging applications are outside this tool's scope.
"""
import argparse
from datetime import datetime, timezone
import importlib.util
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('monitor_ops', ROOT / 'scripts/ops/dex-monitoring-1544.py')
OPS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(OPS)
APP = 'afs50d2ect995yzc9jkp7u2u'
SHA = re.compile(r'[0-9a-f]{40}')
ID = re.compile(r'[a-z0-9]{20,32}')
STATES = {'queued', 'in_progress', 'finished', 'failed', 'cancelled', 'cancelled-by-user', 'canceled'}


def owned(api):
    app = api.api('applications/' + APP)
    repo = urlsplit(app.get('git_repository', ''))
    OPS.require(app.get('uuid') == APP and app.get('git_branch') == 'main'
                and repo.hostname == 'git.cl8y.com'
                and repo.path.rstrip('/') == '/code/cl8y-dex-terraclassic'
                and app.get('build_pack') == 'dockerfile'
                and app.get('dockerfile_location') == '/docker/frontend/Dockerfile'
                and any(urlsplit(s.strip()).hostname == 'dex.cl8y.com'
                        for s in app.get('fqdn', '').split(',')), 'frontend_binding')
    OPS.require(SHA.fullmatch(app.get('git_commit_sha', '')), 'pinned_revision')
    return app


def deployment(api, identifier, revision):
    OPS.require(ID.fullmatch(identifier or ''), 'deployment_id')
    OPS.require(SHA.fullmatch(revision or ''), 'reviewed_revision')
    detail = api.api('deployments/' + identifier)
    app = owned(api)
    OPS.require(detail.get('deployment_uuid') == identifier and detail.get('status') in STATES
                and detail.get('application_id') == app.get('id')
                and app.get('git_commit_sha') == revision
                and detail.get('commit') == revision, 'deployment_binding')
    logs = detail.get('logs')
    logs = json.loads(logs) if isinstance(logs, str) else logs
    OPS.require(logs is None or isinstance(logs, list) and len(logs) <= 20000, 'log_bound')
    errors = cleanup = 0
    for row in logs or []:
        OPS.require(isinstance(row, dict), 'log_shape')
        text = row.get('output', '')
        OPS.require(isinstance(text, str), 'log_shape')
        if re.search(r'(?i)(error|panic|fatal|failed)', text):
            if re.fullmatch(r'(?:Error(?: response from daemon)?: )?No such container: [A-Za-z0-9_.-]+', text.strip(), re.IGNORECASE):
                cleanup += 1
            else:
                errors += 1
    state = detail['status']
    overall = ('PENDING' if state in {'queued', 'in_progress'} else
               'PASS' if state == 'finished' and errors == 0 else 'FAIL')
    return {'overall': overall, 'deployment_uuid': identifier, 'status': state, 'revision': detail['commit'],
            'log_entries_inspected': len(logs or []), 'error_pattern_entries': errors,
            'missing_cleanup_container_entries': cleanup}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['inspect', 'deploy', 'deployment'])
    parser.add_argument('--revision')
    parser.add_argument('--deployment')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    receipt = {'issues': [1560, 1561], 'utc': datetime.now(timezone.utc).isoformat(),
               'application_uuid': APP, 'operation': args.operation}
    try:
        api = OPS.Coolify()
        app = owned(api)
        receipt['prior_pinned_revision'] = app['git_commit_sha']
        if args.operation == 'inspect':
            receipt['status'] = app['status'] if app.get('status') in {'running:healthy', 'running:unknown', 'running:unhealthy'} else 'other'
        elif args.operation == 'deploy':
            OPS.require(SHA.fullmatch(args.revision or ''), 'reviewed_revision')
            api.api('applications/' + APP, 'PATCH', {'git_commit_sha': args.revision})
            OPS.require(owned(api)['git_commit_sha'] == args.revision, 'revision_readback')
            result = api.api('deploy', 'POST', {'uuid': APP, 'force': False})
            rows = result.get('deployments', [])
            OPS.require(len(rows) == 1 and rows[0].get('resource_uuid') == APP
                        and ID.fullmatch(str(rows[0].get('deployment_uuid', ''))), 'queued_binding')
            receipt.update(overall='PENDING', revision=args.revision, deployment_uuid=rows[0]['deployment_uuid'], status='queued')
        else:
            receipt.update(deployment(api, args.deployment, args.revision))
        receipt.setdefault('overall', 'PASS')
    except Exception as error:
        receipt.update(overall='FAIL', failure_class=type(error).__name__)
    args.output.write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))
    return 0 if receipt['overall'] in {'PASS', 'PENDING'} else 1


if __name__ == '__main__':
    raise SystemExit(main())
