#!/usr/bin/env bash
# Synthetic discovery checks; failures remain failures, unavailable checks stay explicit.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
ROOT="$PWD"
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
REPEATS="${VERIFY_ISSUE_1394_REPEATS:-8}"
THREADS="${VERIFY_ISSUE_1394_THREADS:-32}"
if [[ ! "$REPEATS" =~ ^[1-9][0-9]*$ || ! "$THREADS" =~ ^[1-9][0-9]*$ ]]; then
  echo 'VERIFY_ISSUE_1394_REPEATS and VERIFY_ISSUE_1394_THREADS must be positive integers.' >&2
  exit 2
fi
printf 'Issue #1394 verification\nRevision: %s\nUTC start: %s\n' \
  "$(git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
if [[ -n "$(git status --porcelain)" ]]; then
  echo 'Working tree contains changes: record the final commit with this receipt.'
fi
run() {
  local label="$1"; shift
  printf '\n[%s]\nCommand:' "$label"
  if [[ "$1" == cargo_indexer ]]; then
    printf ' (cd %q && cargo' "$ROOT/indexer"
    printf ' %q' "$2" --locked "${@:3}"
    printf ')'
  else
    printf ' %q' "$@"
  fi
  printf '\n'
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
skip() { SKIP=$((SKIP+1)); RESULTS+=("SKIP $1: $2"); }
cargo_indexer() { (cd "$ROOT/indexer" && cargo "$1" --locked "${@:2}"); }
format_changed() {
  local files=(indexer/src/indexer/pair_discovery.rs indexer/src/lcd/mod.rs indexer/src/indexer/poller.rs indexer/src/config.rs)
  # Check the implementation leaves without recursively rewriting unrelated modules.
  local candidate
  for candidate in indexer/src/lcd/pair_discovery_state.rs indexer/tests/indexer_pair_discovery.rs; do
    [[ ! -f "$candidate" ]] || files+=("$candidate")
  done
  printf 'Rustfmt command:'
  printf ' %q' rustfmt --edition 2021 --config skip_children=true --check "${files[@]}"
  printf '\n'
  rustfmt --edition 2021 --config skip_children=true --check "${files[@]}"
}
run 'changed Rust formatting' format_changed
run 'discovery cache and admission tests' cargo_indexer test --lib pair_discovery
for ((round=1; round<=REPEATS; round++)); do
  run "focused default parallel $round/$REPEATS" cargo_indexer test --lib discover_new_pair
  run "focused high parallel $round/$REPEATS" cargo_indexer test --lib discover_new_pair -- --test-threads="$THREADS"
done
run 'full default-parallel library' cargo_indexer test --lib
run 'hot lookup and capacity churn benchmark' cargo_indexer test --lib pair_discovery_cache_benchmark -- --ignored --nocapture
run 'discovery cache and admission retest' cargo_indexer test --lib pair_discovery -- --test-threads="$THREADS"
if [[ -n "${VERIFY_ISSUE_1394_SKIP_POSTGRES_REASON:-}" ]]; then
  skip 'Postgres discovery test and retest' "$VERIFY_ISSUE_1394_SKIP_POSTGRES_REASON"
else
  run 'Postgres discovery' cargo_indexer test --test indexer_pair_discovery -- --test-threads=1
  run 'Postgres discovery retest' cargo_indexer test --test indexer_pair_discovery -- --test-threads=1
fi
printf '\n%s\n' "${RESULTS[@]}"
printf '\nUTC end: %s\nPASS=%s FAIL=%s SKIP=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS with explicit skips; skipped criteria remain unverified.'
else echo 'Overall PASS: all implementation checks passed.'; fi
echo 'Evidence scope: synthetic source/library/Postgres tests, not production deployment or flood acceptance.'
