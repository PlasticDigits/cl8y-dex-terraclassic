#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
ARTIFACT_DIR="$(mktemp -d)"
trap 'rm -rf "$ARTIFACT_DIR"' EXIT

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

# Build each selected executable once, then retest the exact same compiled code.
# Repeated Cargo invocations can invalidate shared dependency caches on busy hosts.
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

run_test_binary() {
  local executable="$1"
  printf 'Running %s\n' "$(basename "$executable")"
  (cd indexer && "$executable" --test-threads=1)
}

if [[ -n "${VERIFY_ISSUE_1395_SKIP_LIB_REASON:-}" ]]; then
  printf 'SKIP library: %s\n' "$VERIFY_ISSUE_1395_SKIP_LIB_REASON"
else
  build_tests "$ARTIFACT_DIR/lib.json" --lib
  LIB_TEST="$(test_executable "$ARTIFACT_DIR/lib.json" cl8y_dex_indexer)"
  for attempt in 1 2; do
    printf 'Library test pass %s\n' "$attempt"
    run_test_binary "$LIB_TEST"
  done
fi

if [[ -n "${VERIFY_ISSUE_1395_SKIP_DB_REASON:-}" ]]; then
  printf 'SKIP isolated Postgres: %s\n' "$VERIFY_ISSUE_1395_SKIP_DB_REASON"
else
  build_tests "$ARTIFACT_DIR/db.json" -j 1 \
    --test indexer_event_amount_integrity \
    --test indexer_ingestion_hardening \
    --test indexer_protocol_fees \
    --test api_evidence_daily \
    --test indexer_protocol_volume \
    --test indexer_pair_volume_usd \
    --test position_tracker_18dec \
    --test indexer_trader_rolling_numeric \
    --test limit_fill_swap_linkage \
    --test limit_order_parked_lifecycle
  INTEGRITY_TEST="$(test_executable "$ARTIFACT_DIR/db.json" indexer_event_amount_integrity)"
  for attempt in 1 2; do
    printf 'Event integrity database pass %s\n' "$attempt"
    run_test_binary "$INTEGRITY_TEST"
  done
  for suite in indexer_ingestion_hardening indexer_protocol_fees api_evidence_daily \
    indexer_protocol_volume indexer_pair_volume_usd position_tracker_18dec \
    indexer_trader_rolling_numeric limit_fill_swap_linkage limit_order_parked_lifecycle; do
    run_test_binary "$(test_executable "$ARTIFACT_DIR/db.json" "$suite")"
  done
fi

bash -n scripts/indexer-reorg-recover.sh
for file in AGENTS.md docs/testing.md Makefile \
  skills/AGENTS_INDEXER_EVENT_AMOUNTS.md docs/qa/issue-1395/README.md; do
  rg -q 'verify-issue-1395' "$file"
done
git diff --check
echo 'PASS: issue 1395 runnable verification (see any explicit SKIP scope above)'
