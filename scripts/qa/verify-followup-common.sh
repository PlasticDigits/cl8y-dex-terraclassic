#!/usr/bin/env bash
# Shared stable-source first/retest accounting for #1479/#1480.
set -euo pipefail
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
selected() {
  local label="$1" group="$2"; shift 2
  local reason="VERIFY_ISSUE_${ISSUE}_SKIP_${group}_REASON"
  if [[ -n "${!reason:-}" ]]; then
    SKIP=$((SKIP+1)); RESULTS+=("SKIP $label: ${!reason}")
  else run "$label" "$@"; fi
}
skip() { SKIP=$((SKIP+1)); RESULTS+=("SKIP $1"); }
fingerprint() {
  python3 - <<'PY'
import hashlib
from pathlib import Path
h=hashlib.sha256()
for root in ['indexer/src','indexer/tests','frontend-dapp/src','smartcontracts/contracts','smartcontracts/packages']:
    for p in sorted(Path(root).rglob('*')):
        if p.is_file(): h.update(str(p).encode()+b'\0'+p.read_bytes()+b'\0')
print(h.hexdigest())
PY
}
start() {
  printf 'Issue #%s verification\nRevision: %s\nUTC: %s\n' "$ISSUE" "$(git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  SOURCE_FINGERPRINT="$(fingerprint)"
  printf 'Source fingerprint: %s\n' "$SOURCE_FINGERPRINT"
}
documentation() {
  python3 - "$ISSUE" <<'PYDOC'
import sys
from pathlib import Path
issue=int(sys.argv[1])
skill='AGENTS_SMALL_MARKET_REFERENCE.md' if issue == 1479 else 'AGENTS_OFFLINE_SWAP_BUDGET.md'
prefix='SM1479' if issue == 1479 else 'OB1480'
requirements={
 'AGENTS.md':[skill, f'verify-issue-{issue}'],
 'Makefile':[f'scripts/verify-issue-{issue}.sh'],
 f'skills/{skill}':[*(f'{prefix}-{i}' for i in range(1,11)), f'issue-{issue}/README.md'],
 f'docs/qa/issue-{issue}/README.md':['PASS WITH SKIPS','Executed failures remain FAIL','RESULTS.md',*(f'AC{i}' for i in range(1,9 if issue == 1479 else 8))],
}
for name, terms in requirements.items():
    body=Path(name).read_text()
    for term in terms: assert term in body, f'{name}: missing {term}'
print('Policy, issue acceptance and verifier crosslinks PASS')
PYDOC
}
finish() {
  run 'diff whitespace' git diff --check
  if [[ "$(fingerprint)" == "$SOURCE_FINGERPRINT" ]]; then
    PASS=$((PASS+1)); RESULTS+=("PASS stable source across first/retest")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL source changed across first/retest"); fi
  printf '\n%s\n' "${RESULTS[@]}"
  local overall=PASS
  if (( FAIL > 0 )); then overall=FAIL; elif (( SKIP > 0 )); then overall='PASS WITH SKIPS'; fi
  printf '\nOverall: %s; passed=%s failed=%s skipped=%s\n' "$overall" "$PASS" "$FAIL" "$SKIP"
  (( FAIL == 0 ))
}
