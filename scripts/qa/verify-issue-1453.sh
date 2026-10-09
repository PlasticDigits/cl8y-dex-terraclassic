#!/usr/bin/env bash
# Source/mock/disposable-DB verification. Executed failures cannot become skips.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
ROOT="$PWD"
PASS=0 FAIL=0 SKIP=0
declare -a RESULTS=()
printf 'Issue #1453 verification\nRevision: %s\nUTC start: %s\n' \
  "$(git rev-parse HEAD)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
if [[ -n "$(git status --porcelain)" ]]; then
  echo 'Dirty worktree: link this receipt to the reviewed patch and final commit.'
fi
run() {
  local label="$1"; shift
  printf '\n[%s]\nCommand:' "$label"
  printf ' %q' "$@"
  printf '\n'
  if "$@"; then PASS=$((PASS+1)); RESULTS+=("PASS $label")
  else FAIL=$((FAIL+1)); RESULTS+=("FAIL $label"); fi
}
skip() { SKIP=$((SKIP+1)); RESULTS+=("SKIP $1: $2"); }
selected() {
  local label="$1" reason_var="$2"; shift 2
  if [[ -n "${!reason_var:-}" ]]; then skip "$label" "${!reason_var}"
  else run "$label" "$@"; fi
}
cargo_indexer() { (cd "$ROOT/indexer" && cargo "$1" --locked "${@:2}"); }
format_changed() {
  local edition
  edition="$(python3 -c 'import tomllib; print(tomllib.load(open("indexer/Cargo.toml", "rb"))["package"]["edition"])')"
  local files=(indexer/src/indexer/book_snapshot.rs indexer/src/db/queries/pair_reserves.rs
    indexer/src/lcd/mod.rs indexer/src/lcd/chain_identity.rs indexer/src/config.rs indexer/src/main.rs indexer/src/api/db_orderbook_sim.rs indexer/tests/book_snapshot_loop.rs
    indexer/tests/api_route_solve_db_hybrid.rs indexer/tests/indexer_protocol_liquidity.rs
    indexer/tests/common/lcd_mock.rs indexer/tests/common/mod.rs indexer/src/startup.rs
    indexer/src/api/listing_spread.rs)
  local candidate
  for candidate in indexer/src/lcd/provider_pool.rs indexer/src/db/queries/hybrid_pair_reserves.rs; do
    [[ ! -f "$candidate" ]] || files+=("$candidate")
  done
  rustfmt --edition "$edition" --config skip_children=true --check "${files[@]}"
}
library() {
  cargo_indexer test --lib lcd:: &&
  cargo_indexer test --lib config::tests::provider_policy_settings_are_bounded_and_defaulted &&
  cargo_indexer test --lib book_snapshot &&
  cargo_indexer test --lib protocol_tvl &&
  cargo_indexer test --lib listing_spread &&
  cargo_indexer test --lib pair_discovery
}
postgres() {
  local failed=0 suite
  # Cargo stops at the first failed binary when several are selected together.
  # Run each independently so a regression cannot conceal the remaining checks.
  for suite in book_snapshot_loop indexer_protocol_liquidity indexer_protocol_liquidity_series \
    api_overview indexer_pair_discovery api_route_solve_db_hybrid; do
    cargo_indexer test --test "$suite" -- --test-threads=1 || failed=1
  done
  return "$failed"
}
frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/pages/ProtocolPage.test.tsx src/utils/__tests__/protocolVolumeGrain.test.ts \
    src/utils/__tests__/formatProtocolStats.test.ts src/utils/__tests__/protocolLiquidityQuality.test.ts
}
browser() {
  env PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT=30453 \
    PLAYWRIGHT_BASE_URL=http://127.0.0.1:30453 bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=1 \
    --output=/tmp/issue1453-quality-verifier-artifacts e2e/protocol-liquidity-quality.spec.ts
}
run 'verifier shell syntax' bash -n scripts/qa/verify-issue-1453.sh
run 'operational probe privacy and budget controls' python3 scripts/qa/test-probe-issue-1453.py
run 'operational probe help' python3 scripts/qa/probe-issue-1453.py --help
run 'documentation and evidence structure' python3 scripts/qa/check-issue-1453-docs.py
selected 'changed Rust formatting' VERIFY_ISSUE_1453_SKIP_FORMAT_REASON format_changed
for attempt in first retest; do
  selected "$attempt provider/book/valuation/discovery library" VERIFY_ISSUE_1453_SKIP_CARGO_REASON library
  if [[ -n "${VERIFY_ISSUE_1453_SKIP_CARGO_REASON:-}" ]]; then
    skip "$attempt disposable Postgres" "$VERIFY_ISSUE_1453_SKIP_CARGO_REASON"
  else
    selected "$attempt disposable Postgres" VERIFY_ISSUE_1453_SKIP_POSTGRES_REASON postgres
  fi
  selected "$attempt retail retention/chart regressions" VERIFY_ISSUE_1453_SKIP_FRONTEND_REASON frontend
done
selected 'desktop/phone mocked outage and recovery' VERIFY_ISSUE_1453_SKIP_BROWSER_REASON browser
skip 'active production providers, lag/history and deployed failover acceptance' \
  "${VERIFY_ISSUE_1453_SKIP_LIVE_REASON:-focused verifier is local; inspect the redacted operational receipt and independently verify its required live scope}"
skip 'exact historical missing contribution/chain reconstruction acceptance' \
  "${VERIFY_ISSUE_1453_SKIP_HISTORY_REASON:-focused verifier cannot reconstruct historical chain evidence; inspect historical reconciliation receipt and retain unknown scope}"
printf '\n%s\n' "${RESULTS[@]}"
printf '\nUTC end: %s\nPASS=%s FAIL=%s SKIP=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$PASS" "$FAIL" "$SKIP"
if (( FAIL )); then echo 'Overall FAIL: executed checks failed.'; exit 1; fi
if (( SKIP )); then echo 'Overall PASS WITH SKIPS; skipped criteria remain unverified.'
else echo 'Overall PASS: all selected checks passed.'; fi
echo 'Evidence scope: source, mock providers and disposable databases; no production rollout or #1452 zero quote-RPC claim.'
