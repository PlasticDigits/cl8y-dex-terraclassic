#!/usr/bin/env bash
# Forgejo #1397: focused evidence schema/privacy, query budgets and recovery.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
ARTIFACT_DIR="$(mktemp -d)"
trap 'rm -rf "$ARTIFACT_DIR"' EXIT
SKIPS=0
if [[ -n "${INDEXER_CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$INDEXER_CARGO_TARGET_DIR"
fi
if [[ -n "${INDEXER_CARGO_HOME:-}" ]]; then
  export CARGO_HOME="$INDEXER_CARGO_HOME"
fi

skip() {
  printf 'SKIP %s: %s\n' "$1" "$2"
  SKIPS=$((SKIPS + 1))
}

# Reuse the exact compiled test executable for the retest. A build/test failure
# exits nonzero; skip variables only represent already-established unavailability.
build_tests() {
  local output="$1"
  shift
  if (cd indexer && cargo test --no-run --message-format=json "$@" > "$output"); then
    return 0
  else
    local status=$?
    python3 - "$output" <<'PY'
import json
import sys
for line in open(sys.argv[1]):
    try:
        item = json.loads(line)
    except ValueError:
        continue
    if item.get("reason") == "compiler-message":
        print(item["message"].get("rendered", ""), file=sys.stderr)
PY
    return "$status"
  fi
}

test_executable() {
  python3 - "$1" "$2" <<'PY'
import json
import sys
for line in open(sys.argv[1]):
    try:
        item = json.loads(line)
    except ValueError:
        continue
    if (item.get("reason") == "compiler-artifact"
            and item.get("target", {}).get("name") == sys.argv[2]
            and item.get("profile", {}).get("test")
            and item.get("executable")):
        print(item["executable"])
        break
else:
    raise SystemExit("Missing compiled test executable: " + sys.argv[2])
PY
}

if [[ -n "${VERIFY_ISSUE_1397_SKIP_LIB_REASON:-}" ]]; then
  skip 'evidence library tests' "$VERIFY_ISSUE_1397_SKIP_LIB_REASON"
else
  build_tests "$ARTIFACT_DIR/lib.json" --lib evidence
  LIB_TEST="$(test_executable "$ARTIFACT_DIR/lib.json" cl8y_dex_indexer)"
  # Refuse a silent zero-test PASS if evidence tests move or disappear.
  (cd indexer && "$LIB_TEST" evidence --list) > "$ARTIFACT_DIR/lib-list.txt"
  rg -q ': test$' "$ARTIFACT_DIR/lib-list.txt"
  for attempt in 1 2; do
    printf 'Evidence library pass %s\n' "$attempt"
    (cd indexer && "$LIB_TEST" evidence --test-threads=1)
  done
fi

if [[ -n "${VERIFY_ISSUE_1397_SKIP_DB_REASON:-}" ]]; then
  skip 'synthetic Postgres/socket integration' "$VERIFY_ISSUE_1397_SKIP_DB_REASON"
else
  build_tests "$ARTIFACT_DIR/db.json" -j 1 \
    --test api_evidence_daily --test evidence_daily_budget
  for suite in api_evidence_daily evidence_daily_budget; do
    TEST_BINARY="$(test_executable "$ARTIFACT_DIR/db.json" "$suite")"
    for attempt in 1 2; do
      printf '%s pass %s\n' "$suite" "$attempt"
      (cd indexer && "$TEST_BINARY" --test-threads=1)
    done
  done
fi

if [[ -n "${VERIFY_ISSUE_1397_SKIP_DOCS_REASON:-}" ]]; then
  skip 'shell/documentation crosslinks' "$VERIFY_ISSUE_1397_SKIP_DOCS_REASON"
else
  bash -n scripts/qa/verify-issue-1397.sh scripts/qa/verify-issue-1205.sh
  for file in AGENTS.md Makefile docs/testing.md \
    skills/AGENTS_EVIDENCE_EXPORT_PRIVACY.md docs/qa/issue-1397/README.md; do
    rg -q 'verify-issue-1397' "$file"
  done
  for file in skills/AGENTS_EVIDENCE_EXPORT_PRIVACY.md docs/qa/issue-1397/README.md \
    docs/indexer-http.md docs/indexer-invariants.md; do
    rg -q 'public-chain-linked-no-actor-identifiers' "$file"
  done
  git diff --check
fi
if (( SKIPS > 0 )); then
  printf 'PASS WITH SKIPS: issue 1397 runnable checks passed (%s explicit unavailable groups).\n' "$SKIPS"
else
  echo 'PASS: issue 1397 focused verification and retest.'
fi
