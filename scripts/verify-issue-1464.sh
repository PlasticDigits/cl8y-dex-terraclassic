#!/usr/bin/env bash
# Focused source/socket recovery checks; unavailable scope stays explicit.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
printf 'Issue #1464 verification\nRevision: %s\nUTC start: %s\n' \
  "$(git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
run() {
  local label="$1"; shift
  printf '\n[%s]\nCommand:' "$label"; printf ' %q' "$@"; printf '\n'
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
skip() { SKIP=$((SKIP+1)); RESULTS+=("SKIP $1: $2"); }
selected() {
  local label="$1" reason_var="$2"; shift 2
  if [[ -n "${!reason_var:-}" ]]; then skip "$label" "${!reason_var}"
  else run "$label" "$@"; fi
}
cargo_indexer() { cargo "$1" --locked --manifest-path indexer/Cargo.toml "${@:2}"; }
library() {
  local failed=0 filter
  for filter in lcd:: recovery1464 api::local_quote; do
    cargo_indexer test --lib "$filter" -- --test-threads=1 || failed=1
  done
  return "$failed"
}
socket_tests() {
  local failed=0 suite
  if [[ -f indexer/.env.qa ]]; then
    set -a
    # Dedicated QA capabilities only; runtime/migration env is never sourced.
    source indexer/.env.qa
    set +a
  fi
  if [[ -z "${TEST_DATABASE_ADMIN_URL:-${TEST_DATABASE_URL:-}}" ]]; then
    echo 'Missing isolated QA control credentials; provision .env.qa or record unavailable DB reason.' >&2
    return 1
  fi
  for suite in indexer_ingestion_hardening api_health book_snapshot_loop route_local_quotes_1452; do
    cargo_indexer test --test "$suite" -- --test-threads=1 --nocapture || failed=1
  done
  return "$failed"
}
format_changed() {
  local files=() file
  while IFS= read -r file; do [[ "$file" != indexer/*.rs || ! -f "$file" ]] || files+=("$file"); done \
    < <(git diff --name-only; git ls-files --others --exclude-standard)
  if (( ${#files[@]} )); then rustfmt --edition 2021 --config skip_children=true --check "${files[@]}"; fi
}
fingerprint() {
  python3 - <<'PY'
import hashlib
from pathlib import Path
h = hashlib.sha256()
for root in ['indexer/src', 'indexer/tests']:
    for p in sorted(Path(root).rglob('*.rs')):
        h.update(str(p).encode() + b'\0' + p.read_bytes() + b'\0')
print(h.hexdigest())
PY
}
docs() {
  python3 - <<'PY'
from pathlib import Path
required = {
 'skills/AGENTS_PROVIDER_RECOVERY.md': [*(f'PR1464-{i}' for i in range(1, 11)), 'verify-issue-1464'],
 'docs/qa/issue-1464/README.md': [*(f'AC{i}' for i in range(1, 10)), 'PASS WITH SKIPS', 'Executed failures remain FAIL'],
 'docs/runbooks/indexer-provider-recovery.md': ['/ready', '/health', 'halted', '#1456'],
 'docs/indexer-invariants.md': ['#1464', 'AGENTS_PROVIDER_RECOVERY.md'],
 'docs/route-solver.md': ['#1464', 'AGENTS_PROVIDER_RECOVERY.md'],
 'AGENTS.md': ['AGENTS_PROVIDER_RECOVERY.md', 'verify-issue-1464'],
 'Makefile': ['scripts/verify-issue-1464.sh'],
}
for file, terms in required.items():
    body = Path(file).read_text()
    for term in terms:
        assert term in body, f'{file}: missing {term}'
print('Recovery invariant/checklist/runbook crosslinks PASS')
PY
}
SOURCE_FINGERPRINT="$(fingerprint)"
printf 'Source fingerprint: %s\n' "$SOURCE_FINGERPRINT"
run 'shell syntax' bash -n scripts/verify-issue-1464.sh
run 'corpus executable-shape and evidence controls' python3 scripts/qa/test-probe-issue-1464.py
run 'bounded corpus CLI' python3 scripts/qa/probe-issue-1464.py --help
run 'invariant and checklist crosslinks' docs
selected 'changed Rust formatting' VERIFY_ISSUE_1464_SKIP_FORMAT_REASON format_changed
for attempt in first retest; do
  selected "$attempt provider/checkpoint/admission regressions" VERIFY_ISSUE_1464_SKIP_RUST_REASON library
  if [[ -n "${VERIFY_ISSUE_1464_SKIP_RUST_REASON:-}" ]]; then
    skip "$attempt disposable socket/book/health regressions" "$VERIFY_ISSUE_1464_SKIP_RUST_REASON"
  else
    selected "$attempt disposable socket/book/health regressions" VERIFY_ISSUE_1464_SKIP_DB_REASON socket_tests
  fi
done
run 'diff whitespace' git diff --check
if [[ "$(fingerprint)" == "$SOURCE_FINGERPRINT" ]]; then
  PASS=$((PASS+1)); RESULTS+=("PASS unchanged source during test/retest")
else
  FAIL=$((FAIL+1)); RESULTS+=("FAIL source changed during verification; repeat on stable final source")
fi
skip 'deployed corpus/browser/region/sustained outage acceptance' \
  "${VERIFY_ISSUE_1464_SKIP_LIVE_REASON:-focused verifier is source/socket scope; run bounded public corpus and inspect independent deployed evidence}"
skip 'funded quote/settlement parity' \
  "${VERIFY_ISSUE_1464_SKIP_PARITY_REASON:-focused verifier does not broadcast; inspect funded LocalTerra parity receipt separately}"
printf '\n%s\n' "${RESULTS[@]}"
printf 'UTC end: %s\nPASS=%s FAIL=%s SKIP=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS WITH SKIPS; skipped criteria remain unverified.'
else echo 'Overall PASS: selected source checks passed.'; fi
echo 'Also run verify-issue-1452/1453/1394/1392/1396; liveness/source PASS does not establish deployed trading acceptance.'
