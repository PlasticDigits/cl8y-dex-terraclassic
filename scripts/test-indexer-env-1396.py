#!/usr/bin/env python3
"""Verify actual launcher subprocesses get only their scoped DB capability."""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FORBIDDEN = ['MIGRATION_DATABASE_URL', 'MIGRATOR_DATABASE_URL', 'DATABASE_ADMIN_URL',
             'TEST_DATABASE_URL', 'TEST_DATABASE_ADMIN_URL', 'TEST_DATABASE_RUNTIME_PASSWORD',
             'POSTGRES_PASSWORD', 'POSTGRES_SUPERUSER_PASSWORD',
             'POSTGRES_MIGRATOR_PASSWORD', 'POSTGRES_QA_PASSWORD']
with tempfile.TemporaryDirectory(prefix='dex1396-env-') as temp:
    directory = Path(temp)
    fake = directory / 'indexer'
    fake.write_text('#!/usr/bin/python3\nimport os,sys\n' +
                    'for key in ' + repr(FORBIDDEN) + ':\n    assert key not in os.environ, key\n' +
                    "assert os.environ['DATABASE_URL'] == 'postgres://scoped:synthetic@localhost/fixture'\n" +
                    "if len(sys.argv)>1: assert sys.argv[1] == 'migrate'\n")
    fake.chmod(0o700)
    private = directory / 'private.env'
    private.write_text("DATABASE_URL='postgres://scoped:synthetic@localhost/fixture'\n")
    private.chmod(0o600)
    environment = dict(os.environ, **{key: 'synthetic-inherited-secret' for key in FORBIDDEN})
    environment.update(INDEXER_RUNTIME_ENV=str(private), INDEXER_MIGRATION_ENV=str(private),
                       INDEXER_MIGRATION_BIN=str(fake))
    for script, args in [('indexer-run.sh', [str(fake)]), ('indexer-migrate.sh', [])]:
        result = subprocess.run(['bash', str(ROOT / 'scripts' / script)] + args,
                                cwd=ROOT, env=environment, capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, script + ': capability-scoping assertion failed'
        assert 'synthetic-inherited-secret' not in result.stdout + result.stderr
        print('PASS: ' + script + ' strips inherited capabilities in the actual subprocess')
