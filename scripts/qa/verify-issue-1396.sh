#!/usr/bin/env bash
# Repeat available checks; missing capabilities are explicit skips, never invented PASS.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
pass=0 fail=0 skip=0
run() {
  local label="$1"; shift
  if "$@"; then printf 'PASS: %s\n' "$label"; pass=$((pass+1))
  else printf 'FAIL: %s\n' "$label"; fail=$((fail+1)); fi
}
record_skip() { printf 'SKIP: %s\n' "$1"; skip=$((skip+1)); }
ARTIFACTS="$(mktemp -d)"
trap 'rm -rf "$ARTIFACTS"' EXIT
suites=(seed_runtime_security)
if [[ -n "${VERIFY_ISSUE_1396_SKIP_DB_REASON:-}" ]]; then
  record_skip "isolated DB suites: $VERIFY_ISSUE_1396_SKIP_DB_REASON"
else
  suites+=(database_isolation security api_health api_candles_newest_n api_aggregator_batch)
fi
if [[ -n "${VERIFY_ISSUE_1396_SKIP_RUST_REASON:-}" ]]; then
  record_skip "Rust build/tests: $VERIFY_ISSUE_1396_SKIP_RUST_REASON"
else
  args=(--lib)
  for suite in "${suites[@]}"; do args+=(--test "$suite"); done
  if (cd indexer && cargo test --locked --jobs 2 --no-run --message-format=json "${args[@]}" > "$ARTIFACTS/build.json"); then
    python3 - "$ARTIFACTS/build.json" > "$ARTIFACTS/executables" <<'PY'
import json, sys
for line in open(sys.argv[1]):
    try:
        item = json.loads(line)
    except ValueError:
        continue
    if item.get('reason') == 'compiler-artifact' and item.get('profile', {}).get('test') and item.get('executable'):
        print(item['target']['name'] + '\t' + item['executable'])
PY
    for attempt in first retest; do
      while IFS=$'\t' read -r suite executable; do
        flags=(--test-threads=1)
        if [[ "$suite" == cl8y_dex_indexer ]]; then flags+=(--include-ignored); fi
        if [[ "$suite" == database_isolation ]]; then
          if [[ -n "${TEST_DATABASE_RUNTIME_PASSWORD:-}" ]]; then flags+=(--include-ignored)
          elif [[ "$attempt" == first ]]; then record_skip 'authenticated runtime-login fixture: TEST_DATABASE_RUNTIME_PASSWORD not injected; real ACL checks still run separately'; fi
        fi
        run "$attempt $suite (same compiled executable)" bash -c 'cd "$1/indexer" && "$2" "${@:3}"' _ "$ROOT" "$executable" "${flags[@]}"
      done < "$ARTIFACTS/executables"
    done
  else
    # Surface compiler messages without dumping build/env configuration.
    python3 - "$ARTIFACTS/build.json" <<'PY'
import json, sys
for line in open(sys.argv[1]):
    try: item = json.loads(line)
    except ValueError: continue
    if item.get('reason') == 'compiler-message': print(item['message'].get('rendered', ''), file=sys.stderr)
PY
    printf 'FAIL: Rust build\n'; fail=$((fail+1))
  fi
fi
if [[ -f /proc/sys/net/ipv6/conf/lo/disable_ipv6 && $(cat /proc/sys/net/ipv6/conf/lo/disable_ipv6) == 1 ]]; then record_skip 'host IPv6 socket query: disabled by host kernel; IPv4 and confinement probes still run'; fi
for attempt in first retest; do
  run "$attempt launcher capability isolation" python3 scripts/test-indexer-env-1396.py
  run "$attempt functional CI source tests" python3 scripts/ci/functional-test.py
  run "$attempt network rendering/socket probes" python3 -u scripts/test-postgres-network-1396.py
  if [[ -n "${VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON:-}" && "$attempt" == first ]]; then record_skip "default published-loopback query: $VERIFY_ISSUE_1396_SKIP_BRIDGE_HOST_REASON"; fi
  if [[ -n "${VERIFY_ISSUE_1396_SKIP_NETWORK_REASON:-}" && "$attempt" == first ]]; then record_skip "socket probes: $VERIFY_ISSUE_1396_SKIP_NETWORK_REASON"; fi
  if [[ -n "${VERIFY_ISSUE_1396_SKIP_ACL_REASON:-}" ]]; then
    [[ "$attempt" != first ]] || record_skip "fresh role ACL probes: $VERIFY_ISSUE_1396_SKIP_ACL_REASON"
  else
    run "$attempt fresh role ACL probes" bash scripts/test-setup-postgres-dev-databases.sh
  fi
done
run 'shell syntax' bash -n scripts/indexer-migrate.sh scripts/indexer-run.sh scripts/test-charts-integration.sh scripts/setup-postgres-dev-databases.sh \
  scripts/lib/postgres-bootstrap-role.sh scripts/lib/postgres-dev.env \
  scripts/qa/start-qa.sh scripts/e2e-start-indexer.sh scripts/setup-cloud-agent-localterra.sh
run 'diff whitespace' git diff --check
for file in AGENTS.md docs/testing.md skills/AGENTS_INDEXER_DATABASE_ISOLATION.md docs/qa/issue-1396/README.md; do
  run "cross-link $file" rg -q 'verify-issue-1396' "$file"
done
printf 'Results: %s PASS, %s FAIL, %s SKIP\n' "$pass" "$fail" "$skip"
printf 'Scope: local implementation only; production transition/reachability and hosted trusted-policy deployment require separate receipts.\n'
(( fail == 0 )) || exit 1
if (( skip > 0 )); then echo 'PASS WITH SKIPS (available checks; explicit scope above)'; else echo 'PASS (local implementation checks)'; fi
