#!/usr/bin/env python3
"""Recovery retains the supplied disposable DB capability and suppresses its secret."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="reorg-1437-") as directory:
    temp = Path(directory)
    capture = temp / "calls.jsonl"
    psql = temp / "psql"
    psql.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['REORG_TEST_CAPTURE'], 'a') as output:
    output.write(json.dumps(sys.argv[1:]) + '\\n')
print('0')
""")
    psql.chmod(0o700)
    supplied = "postgres://dex_indexer_qa:fixture-secret@127.0.0.1:15432/dex_indexer_test_1437"
    result = subprocess.run(
        ["bash", str(ROOT / "scripts/indexer-reorg-recover.sh"), "--height", "100", "--cleanup-derived", "--apply"],
        env={**os.environ, "PATH": str(temp) + os.pathsep + os.environ["PATH"], "DATABASE_URL": supplied,
             "POSTGRES_PSQL_MODE": "compose", "REORG_TEST_CAPTURE": str(capture)},
        capture_output=True, text=True,
    )
    assert result.returncode == 0, result.stderr
    calls = [json.loads(line) for line in capture.read_text().splitlines()]
    assert calls and all(supplied in call for call in calls), "recovery redirected the selected database"
    assert "fixture-secret" not in result.stdout + result.stderr, "recovery exposed DSN credentials"
    assert any("DELETE FROM listing_wrap_principal" in argument for call in calls for argument in call)
    assert any("DELETE FROM listing_mapper_custody" in argument for call in calls for argument in call)
print("PASS: recovery targets only the explicit disposable DSN and omits credentials")
