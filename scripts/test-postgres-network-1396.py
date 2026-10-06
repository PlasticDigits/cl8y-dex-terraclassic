#!/usr/bin/env python3
"""Render and probe fresh synthetic Postgres stacks; never touch shared volumes."""
import json
import os
from pathlib import Path
import secrets
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
IMAGE = 'postgres:16-alpine@sha256:721873c34ceb9f8d8fc265984940dc982404c105f19ad51be9fdc5970a6080ea'
ENV = dict(os.environ, POSTGRES_SUPERUSER='postgres',
           POSTGRES_SUPERUSER_PASSWORD=secrets.token_hex(24), POSTGRES_DB='dex_indexer')
# Do not inherit a production or ignored automatic Compose override.
ENV.pop('COMPOSE_FILE', None)
PROJECT = f'issue1396-network-{os.getpid()}'


def run(args, *, ok=True, env=ENV):
    result = subprocess.run(args, cwd=ROOT, env=env, capture_output=True, text=True, timeout=300)
    if ok and result.returncode:
        # Full Compose output/config may contain secrets, so print only the command type.
        raise AssertionError(f'{args[0]} {args[1]} failed (exit {result.returncode}); output withheld')
    return result


def compose(*args, files=()):
    command = ['docker', 'compose', '-p', PROJECT, '-f', str(ROOT / 'docker-compose.yml')]
    for path in files:
        command.extend(['-f', str(path)])
    return run(command + list(args))


def free_port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def host_query(port, address='127.0.0.1'):
    return run(['psql', '-X', '-h', address, '-p', str(port), '-U', 'postgres',
                '-d', 'dex_indexer', '-Atc', 'SELECT 1'], ok=False,
               env=dict(ENV, PGPASSWORD=ENV['POSTGRES_SUPERUSER_PASSWORD'], PGCONNECT_TIMEOUT='2'))


def remote_query(port, network='bridge', host='host.docker.internal'):
    return run(['docker', 'run', '--rm', '--network', network,
                '--add-host=host.docker.internal:host-gateway', '-e', 'PGPASSWORD',
                '-e', 'PGCONNECT_TIMEOUT=2', IMAGE, 'psql', '-X', '-h', host,
                '-p', str(port), '-U', 'postgres', '-d', 'dex_indexer', '-Atc', 'SELECT 1'],
               ok=False, env=dict(ENV, PGPASSWORD=ENV['POSTGRES_SUPERUSER_PASSWORD']))


def wait_ready(port):
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        result = host_query(port)
        if result.returncode == 0:
            return
        time.sleep(1)
    raise AssertionError('fresh synthetic Postgres did not accept a loopback query: ' + result.stderr[:500])


def wait_internal(files):
    cid = compose('ps', '-q', 'postgres', files=files).stdout.strip()
    deadline = time.monotonic() + 600
    while time.monotonic() < deadline:
        result = run(['docker', 'exec', '-e', 'PGPASSWORD', '-e', 'PGCONNECT_TIMEOUT=2',
                      cid, 'psql', '-X', '-h', '127.0.0.1', '-U', 'postgres',
                      '-d', 'dex_indexer', '-Atc', 'SELECT 1'], ok=False,
                     env=dict(ENV, PGPASSWORD=ENV['POSTGRES_SUPERUSER_PASSWORD']))
        if result.returncode == 0:
            return
        time.sleep(1)
    raise AssertionError('fresh synthetic Postgres never became ready for internal TCP queries')


def main():
    default = json.loads(compose('config', '--format', 'json').stdout)['services']['postgres']
    assert default.get('network_mode') != 'host'
    assert default['command'] == ['postgres']
    assert all(p['host_ip'] == '127.0.0.1' for p in default['ports'])
    host_file = ROOT / 'docker-compose.postgres-host.yml'
    host = json.loads(compose('config', '--format', 'json', files=[host_file]).stdout)['services']['postgres']
    assert host['network_mode'] == 'host' and not host.get('ports')
    assert host['command'] == ['postgres', '-c', 'listen_addresses=127.0.0.1,::1']
    run(['make', '-s', '--eval', 'issue1396-compose:;@test "$$COMPOSE_FILE" = docker-compose.yml',
         'issue1396-compose'])
    # Entrypoint rejects defaults even on an existing initialized volume.
    for password in ['', 'postgres', 'cl8y_legal', 'changeme0123456789']:
        rejected = run(['sh', str(ROOT / 'docker/postgres-entrypoint.sh')], ok=False,
                       env=dict(ENV, POSTGRES_PASSWORD=password))
        assert rejected.returncode != 0
        assert not password or password not in rejected.stdout + rejected.stderr
    missing = dict(ENV)
    missing.pop('POSTGRES_SUPERUSER_PASSWORD')
    assert run(['docker', 'compose', '-f', str(ROOT / 'docker-compose.yml'), 'config', '--quiet'],
               env=missing, ok=False).returncode != 0
    print('PASS: default/opt-in Compose rendering, explicit Make selection and credential rejection')

    if os.environ.get('VERIFY_ISSUE_1396_SKIP_NETWORK_REASON'):
        print('SKIP socket probes: ' + os.environ['VERIFY_ISSUE_1396_SKIP_NETWORK_REASON'])
        return
    with tempfile.TemporaryDirectory(prefix='issue1396-network-') as temp:
        # Disposable test data uses RAM to avoid host fsync contention. Deployment
        # volume/durability settings are unchanged.
        data = ('    volumes: !override\n'
                f'      - {ROOT}/docker/postgres-entrypoint.sh:/usr/local/bin/cl8y-postgres-entrypoint.sh:ro\n'
                '    tmpfs: [/var/lib/postgresql/data]\n'
                '    environment:\n      POSTGRES_INITDB_ARGS: --no-sync\n')
        override = Path(temp) / 'ports.yml'
        port = free_port()
        override.write_text(f'services:\n  postgres:\n    ports: !override ["127.0.0.1:{port}:5432"]\n' + data)
        try:
            compose('up', '-d', 'postgres', files=[override])
            wait_internal([override])
            if ENV.get('VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON'):
                print('SKIP default published-loopback query: ' + ENV['VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON'])
            else:
                wait_ready(port)
            assert remote_query(5432, network=PROJECT + '_default', host='postgres').returncode == 0
            assert remote_query(port).returncode != 0
            print('PASS: bridge permits loopback/internal queries and denies external-namespace host access')
        finally:
            compose('down', '-v', files=[override])
        port = free_port()
        override.write_text(f'services:\n  postgres:\n    command: ["postgres", "-c", "listen_addresses=127.0.0.1,::1", "-p", "{port}"]\n' + data)
        try:
            compose('up', '-d', 'postgres', files=[host_file, override])
            wait_ready(port)
            ipv6_flag = Path('/proc/sys/net/ipv6/conf/lo/disable_ipv6')
            if ipv6_flag.exists() and ipv6_flag.read_text().strip() == '1':
                print('SKIP host IPv6 query: loopback IPv6 is disabled by the host kernel')
            else:
                assert host_query(port, '::1').returncode == 0
                print('PASS: opt-in host IPv6 loopback query')
            assert remote_query(port).returncode != 0
            print('PASS: opt-in host mode permits IPv4 loopback and denies external-namespace host access')
        finally:
            compose('down', '-v', files=[host_file, override])


if __name__ == '__main__':
    main()
