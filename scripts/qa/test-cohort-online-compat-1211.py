#!/usr/bin/env python3
"""Run a precompiled compatibility test in its own disposable loopback PostgreSQL.

No production endpoint, credential, migration history or volume is used. Build
separately; the complete provision/test/cleanup invocation owns a48-second alarm.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import signal
import subprocess
import time
from urllib.parse import quote


def run(binary):
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError('precompiled_test_binary_required')
    label = secrets.token_hex(12)
    name = 'dex-cohort-compat-' + label
    password = secrets.token_urlsafe(32)
    environment = {key: os.environ[key] for key in ('PATH', 'HOME') if key in os.environ}
    docker_environment = {**environment, 'POSTGRES_PASSWORD': password}
    owned = False
    def command(args, *, input=None, env=environment, seconds=8, check=True):
        result = subprocess.run(args, input=input, text=True, capture_output=True,
                                env=env, timeout=seconds)
        if check and result.returncode:
            raise RuntimeError('owned_fixture_command_failed')
        return result
    def deadline(*_):
        raise TimeoutError('owned_fixture_deadline')
    signal.signal(signal.SIGALRM, deadline)
    signal.signal(signal.SIGTERM, deadline)
    signal.alarm(48)
    started = time.monotonic()
    try:
        command(['docker', 'run', '--detach', '--rm', '--name', name,
                 '--label', 'cl8y.qa.cohort-compat=' + label,
                 '--memory', '384m', '--cpus', '2', '-p', '127.0.0.1::5432',
                 '-e', 'POSTGRES_PASSWORD', 'postgres:16-alpine'], env=docker_environment)
        owned = True
        ready = False
        for _ in range(30):
            if command(['docker', 'exec', name, 'pg_isready', '-U', 'postgres'], check=False).returncode == 0:
                ready = True
                break
            time.sleep(0.15)
        if not ready:
            raise RuntimeError('owned_fixture_not_ready')
        mapping = json.loads(command(['docker', 'inspect', '--format', '{{json .NetworkSettings.Ports}}', name]).stdout)
        bindings = mapping.get('5432/tcp', [])
        if len(bindings) != 1 or bindings[0].get('HostIp') != '127.0.0.1':
            raise ValueError('owned_loopback_binding_required')
        marker = 'cl8y-dex:cohort-compat:' + label
        sql = f"""CREATE ROLE cohort_compat_migrator LOGIN NOINHERIT PASSWORD '{password}';
CREATE ROLE cohort_compat_runtime LOGIN NOINHERIT PASSWORD '{password}';
CREATE DATABASE cohort_compat OWNER cohort_compat_migrator;
REVOKE ALL ON DATABASE postgres,template1,cohort_compat FROM PUBLIC;
GRANT CONNECT ON DATABASE cohort_compat TO cohort_compat_runtime,cohort_compat_migrator;
\\connect cohort_compat
ALTER SCHEMA public OWNER TO cohort_compat_migrator;
REVOKE ALL ON SCHEMA public FROM PUBLIC;
COMMENT ON DATABASE cohort_compat IS '{marker}';
"""
        command(['docker', 'exec', '-i', name, 'psql', '-U', 'postgres', '-XqAt',
                 '-v', 'ON_ERROR_STOP=1'], input=sql)
        url = 'postgresql://postgres:' + quote(password, safe='') + '@127.0.0.1:' + bindings[0]['HostPort'] + '/cohort_compat'
        result = command([str(binary), '--exact', 'real67_to68_keeps_runtime_api_ingestion_and_joint_publication_safe', '--nocapture'],
                         env={**environment, 'COHORT_COMPAT_DATABASE_URL': url, 'COHORT_COMPAT_DATABASE_MARKER': marker}, seconds=35, check=False)
        # Child diagnostics stay bounded and cannot accidentally reveal the
        # generated local capability if a future library includes it in errors.
        output = (result.stdout + result.stderr).replace(password, '[redacted]')
        print(output[-12000:], end='')
        if result.returncode:
            raise RuntimeError('compatibility_test_failed')
        print(json.dumps({'overall': 'PASS', 'scope': 'owned_local_only',
                          'elapsed_seconds': round(time.monotonic() - started, 3)}))
    finally:
        signal.alarm(0)
        if owned:
            result = command(['docker', 'rm', '--force', '--volumes', name], seconds=7, check=False)
            if result.returncode:
                raise RuntimeError('owned_fixture_cleanup_failed')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test-binary', type=Path, required=True)
    args = parser.parse_args()
    try:
        run(args.test_binary.resolve())
    except Exception as error:
        print(json.dumps({'overall': 'FAIL', 'failure_class': type(error).__name__}))
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
