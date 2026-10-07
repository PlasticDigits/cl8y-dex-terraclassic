#!/usr/bin/env bash
# Build once and retest the exact executables; unavailable prerequisites are explicit skips.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
PASS=0 FAIL=0 SKIP=0
run() {
  local label="$1"; shift
  if "$@"; then printf 'PASS: %s\n' "$label"; PASS=$((PASS+1))
  else printf 'FAIL: %s\n' "$label"; FAIL=$((FAIL+1)); fi
}
record_skip() { printf 'SKIP: %s\n' "$1"; SKIP=$((SKIP+1)); }
ARTIFACTS="$(mktemp -d)"
trap 'rm -rf "$ARTIFACTS"' EXIT
source_fingerprint() {
  python3 - <<'PYTHON'
from pathlib import Path
import hashlib
root = Path("indexer")
files = sorted([*root.joinpath("src").rglob("*.rs"), *root.joinpath("tests").rglob("*.rs"), *root.joinpath("migrations").glob("*.sql"), root / "Cargo.toml", root / "Cargo.lock", Path("scripts/indexer-reorg-recover.sh"), Path("scripts/verify-issue-1394.sh"), Path("scripts/lib/postgres-dev.env"), Path("scripts/qa/test-issue-1437-reorg.py"), Path("scripts/qa/check-issue-1437.py"), Path("scripts/qa/verify-issue-1437.sh")])
hash_ = hashlib.sha256()
for path in files:
    hash_.update(str(path).encode() + b"\0" + path.read_bytes() + b"\0")
print(hash_.hexdigest())
PYTHON
}
SOURCE_FINGERPRINT="$(source_fingerprint)"
printf 'Tested source fingerprint: %s\n' "$SOURCE_FINGERPRINT"
run 'source boundaries and documentation' python3 scripts/qa/check-issue-1437.py
run 'shell syntax' bash -n scripts/qa/verify-issue-1437.sh scripts/indexer-reorg-recover.sh
run 'diff whitespace' git diff --check
run 'recovery DSN isolation and output privacy' python3 scripts/qa/test-issue-1437-reorg.py
run 'recovery retest' python3 scripts/qa/test-issue-1437-reorg.py
if [[ -n "${VERIFY_ISSUE_1437_SKIP_RUST_REASON:-}" ]]; then
  record_skip "Rust build/test: $VERIFY_ISSUE_1437_SKIP_RUST_REASON"
else
  run 'Rust formatting' cargo fmt --manifest-path indexer/Cargo.toml -- --check
  args=(--lib)
  if [[ -n "${VERIFY_ISSUE_1437_SKIP_DB_REASON:-}" ]]; then
    record_skip "isolated Postgres: $VERIFY_ISSUE_1437_SKIP_DB_REASON"
  else
    args+=(--test api_virtual_listing --test listing_principal_ingest)
  fi
  if (cd indexer && cargo test --locked --jobs 2 --no-run --message-format=json "${args[@]}" > "$ARTIFACTS/build.json"); then
    python3 - "$ARTIFACTS/build.json" > "$ARTIFACTS/executables" <<'PY'
import json, sys
executables = {}
for line in open(sys.argv[1]):
    try: item = json.loads(line)
    except ValueError: continue
    if item.get('reason') == 'compiler-artifact' and item.get('profile', {}).get('test') and item.get('executable'):
        executables[item['target']['name']] = item['executable']
assert 'cl8y_dex_indexer' in executables, 'missing library test executable'
for suite, executable in sorted(executables.items()):
    print(suite + '\t' + executable)
PY
    for attempt in first retest; do
      while IFS=$'\t' read -r suite executable; do
        run "$attempt $suite (same executable)" bash -c 'cd "$1/indexer" && "$2" --test-threads=1' _ "$ROOT" "$executable"
      done < "$ARTIFACTS/executables"
    done
  else
    python3 - "$ARTIFACTS/build.json" <<'PY'
import json, sys
for line in open(sys.argv[1]):
    try: item = json.loads(line)
    except ValueError: continue
    if item.get('reason') == 'compiler-message': print(item['message'].get('rendered', ''), file=sys.stderr)
PY
    printf 'FAIL: Rust build\n'; FAIL=$((FAIL+1))
  fi
fi
if [[ "$(source_fingerprint)" == "$SOURCE_FINGERPRINT" ]]; then
  printf 'PASS: source unchanged during build/test/retest\n'; PASS=$((PASS+1))
else
  printf 'FAIL: source changed during verification; rebuild and rerun\n'; FAIL=$((FAIL+1))
fi
printf 'Results: %s PASS, %s FAIL, %s SKIP\n'  "$PASS" "$FAIL" "$SKIP"
(( FAIL == 0 )) || exit 1
if (( SKIP > 0 )); then echo 'PASS WITH SKIPS (local implementation; unavailable criteria unverified)'
else echo 'PASS (local implementation; deployment and form submission outside scope)'; fi
