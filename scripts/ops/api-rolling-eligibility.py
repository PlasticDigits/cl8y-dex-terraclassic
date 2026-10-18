#!/usr/bin/env python3
"""Read-only rolling eligibility. Never changes an app, environment or deployment.

`inventory` is explicitly not deployment permission. `inspect` binds exact main,
compiled-source inputs and an owned offline proof before checking live metadata.
"""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tarfile
import time
from datetime import datetime, timezone
import urllib.parse

ROOT = Path(__file__).resolve().parents[2]
APP = 's14i3w45r5g6ulyu6nsxzm0n'
NAME = 'CL8Y Dex TerraClassic Indexer'
DOMAIN = 'indexer.dex.cl8y.com'
REPOSITORY = 'https://git.cl8y.com/code/cl8y-dex-terraclassic'
SHA = re.compile(r'[0-9a-f]{40}')
DIGEST = re.compile(r'[0-9a-f]{64}')
ID = re.compile(r'[A-Za-z0-9_-]{1,128}')
SOURCE_DIRS = ('indexer/src', 'indexer/migrations')
SOURCE_REQUIRED = ('indexer/Cargo.toml', 'indexer/Cargo.lock', 'docker/indexer/Dockerfile')
SOURCE_OPTIONAL = ('indexer/build.rs', 'indexer/.cargo/config', 'indexer/.cargo/config.toml',
    '.cargo/config', '.cargo/config.toml', 'rust-toolchain', 'rust-toolchain.toml')
SOURCE_CRITICAL = ('indexer/src/main.rs','indexer/src/database_security.rs','indexer/src/api/mod.rs',
    'indexer/src/api/rolling_readiness.rs','indexer/migrations/20261018110000_protocol_fee_cohorts.sql')
SOURCE_LIMIT = 32 * 1024 * 1024
PRIVILEGED_KEYS = frozenset(('MIGRATION_DATABASE_URL','MIGRATOR_DATABASE_URL','DATABASE_ADMIN_URL',
    'DB_ADMIN_URL','TEST_DATABASE_ADMIN_URL','TEST_DATABASE_URL','POSTGRES_SUPERUSER_PASSWORD',
    'POSTGRES_MIGRATOR_PASSWORD','POSTGRES_QA_PASSWORD'))
PROOF_CONTROLS = frozenset(('serving_schema_67_and_68','strict_migrator_schema_68',
    'cohort_queries_gated_on_67','cohort_publication_clock_fail_closed',
    'privilege_boundaries_preserved','history_preserved','independent_instances_and_lcd_namespaces',
    'standby_native_quotes_without_leadership','actual_block_handoff_after_lock_transfer',
    'continuous_proxy_native_success','cold_candidate_preserves_old',
    'serving_latch_survives_quote_loss','dynamic_quotes_fail_closed_after_loss',
    'fresh_process_gate_resets','request_lcd_calls_zero'))

class Refusal(ValueError):
    """Fixed labels only; raw API/configuration/error data stays private."""

def require(value, label):
    if not value: raise Refusal(label)

def unique_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, "duplicate_json_key"); value[key] = item
    return value

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode()

def source_path(path):
    return path in SOURCE_REQUIRED + SOURCE_OPTIONAL or any(path.startswith(root + '/') for root in SOURCE_DIRS)

def manifest_from_files(files):
    """Pure shared receipt contract: relative path -> bytes, no network or Git."""
    require(all(path in files for path in SOURCE_REQUIRED + SOURCE_CRITICAL), 'required_compiled_inputs')
    require(0 < len(files) <= 4096 and all(source_path(path) for path in files), 'compiled_input_scope')
    require(sum(map(len, files.values())) <= SOURCE_LIMIT, 'compiled_input_bound')
    rows = [{'path': path, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
        for path, data in sorted(files.items())]
    return {'schema': 'indexer_compiled_inputs_v1', 'sha256': hashlib.sha256(canonical(rows)).hexdigest(),
        'file_count': len(rows), 'total_bytes': sum(row['bytes'] for row in rows), 'files': rows}

def application_manifest(root=ROOT):
    """Pure filesystem manifest for a frozen owned build, including uncommitted inputs.

    Covers every file under src/migrations, Cargo manifests, Dockerfile and any
    per-project compiler config/build script. Excludes env files, test data,
    target caches, external Cargo registry state and host credentials.
    """
    root = Path(root)
    selected = [root / path for path in SOURCE_REQUIRED + SOURCE_OPTIONAL if (root / path).exists()]
    for directory in SOURCE_DIRS:
        for path in (root / directory).rglob('*'):
            require(not path.is_symlink(), 'compiled_input_regular_file')
            if path.is_file(): selected.append(path)
    files = {}
    for path in selected:
        require(not path.is_symlink() and path.is_file(), 'compiled_input_regular_file')
        require(path.stat().st_size <= SOURCE_LIMIT, 'compiled_input_file_bound')
        files[path.relative_to(root).as_posix()] = path.read_bytes()
    return manifest_from_files(files)

def git(*args):
    result = subprocess.run(['git', *args], cwd=ROOT, capture_output=True, timeout=25)
    require(result.returncode == 0, 'git_read_failed')
    return result.stdout

def revision_manifest(revision, *, exact_main=True):
    require(SHA.fullmatch(revision), 'revision_shape')
    if exact_main:
        remote = git('ls-remote', '--exit-code', 'origin', 'refs/heads/main').decode().split()
        require(len(remote) == 2 and remote[0] == revision, 'exact_remote_main_required')
    inventory = git('ls-tree', '-r', '-l', '-z', revision, '--', *SOURCE_DIRS, *SOURCE_REQUIRED, *SOURCE_OPTIONAL)
    paths, total = set(), 0
    for row in inventory.split(b'\0'):
        if not row: continue
        metadata, path = row.split(b'\t', 1)
        mode, kind, _, size = metadata.decode().split()
        path = path.decode()
        require(mode in {'100644','100755'} and kind == 'blob' and source_path(path), 'revision_input_regular_file')
        total += int(size); paths.add(path)
    require(total <= SOURCE_LIMIT and len(paths) <= 4096, 'revision_input_bound')
    selected = list(SOURCE_DIRS) + [path for path in SOURCE_REQUIRED + SOURCE_OPTIONAL if path in paths]
    archive = git('archive', '--format=tar', revision, '--', *selected)
    require(len(archive) <= SOURCE_LIMIT + 4 * 1024 * 1024, 'source_archive_bound')
    files = {}
    with tarfile.open(fileobj=io.BytesIO(archive), mode='r:') as stream:
        for member in stream:
            if member.isdir(): continue
            require(member.isfile() and member.name in paths and member.size <= SOURCE_LIMIT, 'source_archive_identity')
            files[member.name] = stream.extractfile(member).read()
    require(set(files) == paths, 'source_archive_complete')
    return manifest_from_files(files), files

def startup_healthcheck(files):
    """Require the reviewed image gate as an executable instruction, never a comment."""
    lines, pending = [], ''
    for line in files['docker/indexer/Dockerfile'].decode().splitlines():
        line = line.strip()
        if not line or line.startswith('#'): continue
        pending += line[:-1].strip() + ' ' if line.endswith('\\') else line
        if not line.endswith('\\'):
            lines.append(pending); pending = ''
    require(not pending, 'source_docker_instruction')
    health = [line for line in lines if line.split()[0].upper() == 'HEALTHCHECK']
    expected = 'HEALTHCHECK --interval=10s --timeout=5s --start-period=180s --retries=12 CMD curl -fsS "http://127.0.0.1:${API_PORT}/health/serving" >/dev/null || exit 1'
    require(health == [expected], 'source_startup_health_endpoint')
    require([line for line in lines if line.split()[0].upper() == 'CMD'] == ['CMD ["cl8y-dex-indexer"]']
        and not any(line.split()[0].upper() == 'ENTRYPOINT' for line in lines), 'source_runtime_command')

def read_json(path, private=False):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(descriptor) as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
            and not stat.S_IMODE(info.st_mode) & 0o022 and info.st_size <= 2 * 1024 * 1024, 'owned_receipt_file')
        if private: require(stat.S_IMODE(info.st_mode) == 0o600, 'private_proof_permissions')
        return json.load(stream, object_pairs_hook=unique_object)

def public_profile(raw):
    require(isinstance(raw, str) and len(raw.encode()) <= 2048, 'profile_bound')
    try: rows = json.loads(raw, object_pairs_hook=unique_object)
    except (ValueError, TypeError): raise Refusal('profile_json') from None
    require(isinstance(rows, list) and 1 <= len(rows) <= 2, 'profile_scenario_count')
    result, seen = [], set()
    for row in rows:
        require(isinstance(row, dict) and set(row) <= {'token_in','token_out','amount_in','max_spread'}
            and {'token_in','token_out','amount_in'} <= set(row), 'profile_fields')
        a, b, amount, spread = row['token_in'], row['token_out'], row['amount_in'], row.get('max_spread','0.01')
        require(all(isinstance(value, str) for value in (a,b,amount,spread)) and a != b, 'profile_identity')
        require(a in {'uluna','uusd'} or b in {'uluna','uusd'}, 'profile_native_required')
        require(all(value in {'uluna','uusd'} or terra_address(value) for value in (a,b)), 'profile_checksum_identity')
        require(re.fullmatch(r'[1-9][0-9]{0,38}', amount) and int(amount) < 2**128, 'profile_raw_amount')
        require(re.fullmatch(r'0\.[0-9]{1,6}', spread) and 0 < int(spread[2:]) * 10**(6-len(spread[2:])) <= 300000, 'profile_spread')
        key = (a,b,amount); require(key not in seen, 'profile_duplicate_scenario'); seen.add(key)
        result.append({'token_in':a,'token_out':b,'amount_in':amount,'max_spread':spread})
    return result

def terra_address(value):
    # Bech32 checksum/padding per BIP173; Terra payloads have no witness version.
    # https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
    if not 14 <= len(value) <= 90 or value != value.lower() or not value.startswith('terra1'): return False
    chars = 'qpzry9x8gf2tvdw0s3jn54khce6mua7l'
    try: data = [chars.index(char) for char in value[6:]]
    except ValueError: return False
    if len(data) < 6: return False
    checksum = 1
    for item in [ord(c)>>5 for c in 'terra'] + [0] + [ord(c)&31 for c in 'terra'] + data:
        top = checksum >> 25; checksum = ((checksum & 0x1ffffff) << 5) ^ item
        for index, generator in enumerate((0x3b6a57b2,0x26508e6d,0x1ea119fa,0x3d4233dd,0x2a1462b3)):
            if (top >> index) & 1: checksum ^= generator
    if checksum != 1: return False
    accumulator = bits = length = 0
    for item in data[:-6]:
        accumulator = ((accumulator << 5) | item) & 8191; bits += 5
        while bits >= 8: bits -= 8; length += 1
    return length in {20,32} and bits < 5 and not ((accumulator << (8-bits)) & 255)

def proof_ready(proof, manifest):
    require(proof.get('schema') == 'api_rolling_eligibility_proof_v1' and proof.get('overall') == 'PASS', 'owned_rolling_proof_required')
    require(proof.get('source_manifest') == manifest, 'owned_source_manifest_binding')
    controls = proof.get('controls', {})
    require(isinstance(controls, dict) and all(controls.get(name) is True for name in PROOF_CONTROLS), 'owned_controls_incomplete')
    measurements = proof.get('measurements', {})
    def number(key):
        value = measurements.get(key); require(type(value) is int and value >= 0, 'owned_measurement_shape'); return value
    require(number('proxy_requests') >= 10 and number('native_quote_successes') >= 4
        and number('http_failures') == 0 and number('request_lcd_calls') == 0
        and number('parsed_blocks_after_handoff') >= 1, 'owned_measurements_incomplete')
    require(proof.get('scope') == 'owned independent HTTP/state, production block indexing and proxy forwarding; not OS-process or actual Coolify deployment', 'owned_scope_truth')


def numeric(value):
    require(type(value) in {str,int} and re.fullmatch(r'[1-9][0-9]{0,18}', str(value)), 'application_numeric_identity')
    return int(value)


class ReadBudget:
    """Wrap the shared GET-only Reader; never exposes mutation methods."""
    def __init__(self, reader):
        self.reader = reader
        self.api_base = reader.api_base
        self.deadline = time.monotonic() + 45
    @property
    def reads(self): return self.reader.reads
    def before(self):
        require(time.monotonic() < self.deadline and self.reads < 8, 'inspection_read_deadline')
    def api(self, path):
        self.before(); value = self.reader.api(path)
        require(time.monotonic() < self.deadline, 'inspection_read_deadline')
        return value
    def read(self, *args):
        self.before(); value = self.reader.read(*args)
        require(time.monotonic() < self.deadline, 'inspection_read_deadline')
        return value

def inventory(reader, expected_profile=None):
    # Only fixed/scoped GET paths and a single fixed public health read.
    reader = ReadBudget(reader)
    version = reader.read(reader.api_base + 'version', 64, True).decode().strip().strip('"')
    app = reader.api('applications/' + APP)
    hosts = {urllib.parse.urlsplit(value.strip()).hostname for value in str(app.get('fqdn','')).split(',')}
    require(app.get('uuid') == APP and app.get('name') == NAME and DOMAIN in hosts
        and str(app.get('git_repository','')).rstrip('/').removesuffix('.git') == REPOSITORY
        and app.get('git_branch') == 'main', 'application_identity')
    history = reader.api('deployments/applications/' + APP + '?skip=0&take=3').get('deployments')
    require(isinstance(history,list) and 1 <= len(history) <= 3, 'application_history_bound')
    identities = {numeric(row.get('application_id')) for row in history}
    require(len(identities) == 1, 'application_history_consistency')
    prior = next((row for row in history if row.get('status') == 'finished' and SHA.fullmatch(str(row.get('commit','')))), None)
    require(prior is not None and ID.fullmatch(str(prior.get('deployment_uuid',''))), 'prior_serving_provenance')
    detail = reader.api('deployments/' + prior['deployment_uuid'])
    embedded = detail.get('application', {})
    require(detail.get('deployment_uuid') == prior['deployment_uuid'] and numeric(detail.get('application_id')) in identities
        and embedded.get('uuid') == APP and embedded.get('name') == NAME and detail.get('commit') == prior['commit'], 'deployment_application_binding')
    health = json.loads(reader.read('https://' + DOMAIN + '/health', 8192))
    rows = reader.api('applications/' + APP + '/envs')
    require(isinstance(rows,list) and len(rows) <= 256, 'environment_inventory_bound')
    rows = [row for row in rows if not row.get('is_preview')]
    configured = [row for row in rows if row.get('key') == 'API_READY_SCENARIOS']
    profile = None
    if len(configured) == 1:
        try: profile = public_profile(configured[0].get('value'))
        except Refusal: pass
    active = reader.api('deployments')
    if isinstance(active, dict) and all(str(key).isdigit() for key in active): active = list(active.values())
    require(isinstance(active,list) and len(active) <= 128
        and all(isinstance(row,dict) and row.get('status') in {'queued','in_progress'} for row in active), 'active_deployment_inventory')
    active_for_app = any(numeric(row.get('application_id')) in identities for row in active)
    settings = app.get('settings', {})
    proxy = app.get('destination', {}).get('server', {}).get('proxy', {})
    if isinstance(proxy,str):
        try: proxy = json.loads(proxy)
        except ValueError: proxy = {}
    checks = {
        'reviewed_coolify_version': version == '4.4.6',
        'standalone_docker': app.get('destination_type') == 'App\\Models\\StandaloneDocker',
        'no_host_port_mappings': not app.get('ports_mappings'),
        'no_fixed_container_name': settings.get('is_consistent_container_name_enabled') is False and not settings.get('custom_internal_name'),
        'no_custom_docker_overrides': not app.get('custom_docker_run_options'),
        'uses_image_healthcheck': app.get('health_check_enabled') is False and app.get('custom_healthcheck_found') is True and not app.get('health_check_command'),
        'image_build_secrets_enabled': settings.get('use_build_secrets') is True,
        'no_unreviewed_compiler_environment': not any(row.get('is_buildtime') and re.match(r'^(?:CARGO_|RUST|CC$|CXX$|AR$|LD$|CFLAGS$|CPPFLAGS$|LDFLAGS$|SOURCE_DATE_EPOCH$)',str(row.get('key',''))) for row in rows),
        'auto_deploy_disabled': settings.get('is_auto_deploy_enabled') is False,
        'no_active_deployment': not active_for_app and all(row.get('status') in {'finished','failed','cancelled','cancelled-by-user','canceled'} for row in history),
        'old_application_healthy': app.get('status') == 'running:healthy',
        'public_health_matches_prior': health.get('status') == 'ok' and health.get('git_sha') == prior['commit'],
        'traefik_proxy_running': isinstance(proxy,dict) and proxy.get('type') == 'TRAEFIK' and proxy.get('status') == 'running',
        'no_privileged_environment': not any(row.get('key') in PRIVILEGED_KEYS for row in rows),
        'explicit_profile_valid': profile is not None,
        'profile_runtime_only_public': len(configured) == 1 and configured[0].get('is_runtime') is True
            and configured[0].get('is_buildtime') is False and configured[0].get('is_shown_once') is False
            and configured[0].get('is_literal') is True,
        'profile_matches_reviewed_input': profile is not None and expected_profile is not None and profile == expected_profile,
    }
    return {'application_uuid':APP, 'old_commit':prior['commit'],'prior_git_commit_sha':app.get('git_commit_sha'),
        'checks':checks,'profile_sha256':hashlib.sha256(canonical(profile)).hexdigest() if profile else None,
        'scenario_count':len(profile) if profile else 0,'reads':reader.reads}


def load_reader():
    spec=importlib.util.spec_from_file_location('attest',ROOT/'scripts/qa/attest-trading-quotes-1473.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module.Reader()


def write(path, value):
    descriptor=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC|os.O_NOFOLLOW,0o600)
    with os.fdopen(descriptor,'w') as stream:
        os.fchmod(stream.fileno(),0o600);json.dump(value,stream,indent=2);stream.write('\n')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('phase',choices=['manifest','inventory','inspect'])
    parser.add_argument('--revision');parser.add_argument('--profile',type=Path);parser.add_argument('--proof',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    try:
        if args.phase=='manifest': result=application_manifest()
        else:
            profile=None
            if args.profile: profile=public_profile(json.dumps(read_json(args.profile)))
            if args.phase=='inspect':
                require(args.revision and args.profile and args.proof,'inspection_inputs_required')
                manifest,files=revision_manifest(args.revision)
                proof_ready(read_json(args.proof,private=True),manifest)
                startup_healthcheck(files)
            result=inventory(load_reader(),profile)
            eligible=all(result['checks'].values())
            result.update(overall=('ELIGIBLE' if eligible else 'INELIGIBLE') if args.phase=='inspect' else 'INVENTORY_ONLY',
                revision=args.revision if args.phase=='inspect' else None,remote_changed=False,
                observed_at_utc=datetime.now(timezone.utc).isoformat())
            if args.phase=='inspect': result['source_manifest_sha256']=manifest['sha256']
        write(args.output,result)
        print(json.dumps({key:value for key,value in result.items() if key!='files'}))
        return int(result.get('overall')=='INELIGIBLE')
    except Exception as error:
        print(json.dumps({'overall':'FAIL','failure_class':type(error).__name__,'reason':str(error) if isinstance(error,Refusal) else 'read_only_inspection_failed','remote_changed':False}))
        return 1
if __name__=='__main__':raise SystemExit(main())
