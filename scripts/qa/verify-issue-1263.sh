#!/usr/bin/env bash
# Automated verification for Forgejo #1263 — /protocol top-5 30d pair volume + TVL + vol/LP.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env) for indexer tests.
#
# Refs: skills/AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md,
#       skills/AGENTS_FRONTEND_PROTOCOL_STATS.md,
#       docs/frontend.md Protocol (P1263),
#       docs/indexer-invariants.md Protocol top pairs 30d
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

docs_crosslinks() {
  grep -qE "P1263-1" docs/frontend.md
  grep -qE "P1263-8" docs/frontend.md
  grep -qE "Protocol top pairs 30d" docs/indexer-invariants.md
  grep -qE "volume_usd_30d" docs/indexer-invariants.md
  grep -qE "make verify-issue-1263" skills/AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md
  grep -qE "I1263-1" skills/AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md
  grep -qE "P1263-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  grep -qE "AGENTS_INDEXER_PROTOCOL_TOP_PAIRS" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  grep -qE "AGENTS_INDEXER_PROTOCOL_TOP_PAIRS" AGENTS.md
  grep -qE "verify-issue-1263" AGENTS.md
  grep -qE "verify-issue-1263" docs/testing.md
  test -f indexer/migrations/20260921120001_pair_volume_30d.sql
}

source_guards() {
  grep -qE "protocol/top-pairs" indexer/src/api/mod.rs
  grep -qE "refresh_pair_volumes_30d" indexer/src/db/queries/volume.rs
  grep -qE "refresh_pair_volumes_30d" indexer/src/indexer/volume_aggregator.rs
  grep -qE "COLUMBUS5_GEM_ADDRESSES|gem_addresses_lowercased" indexer/src/api/protocol_top_pairs.rs indexer/src/db/queries/protocol_top_pairs.rs
  grep -qE "pair_volume_30d" indexer/src/db/queries/protocol_top_pairs.rs
  grep -qE "pair_liquidity_usd" indexer/src/db/queries/protocol_top_pairs.rs
  grep -qE "protocol-top-pairs" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx
  grep -qE "chartsPairHref" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx
  grep -qE "formatVolumePerTvl" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx
  grep -qE "getProtocolTopPairs" frontend-dapp/src/services/indexer/client.ts
  if grep -nE "volume_usd_24h" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx 2>/dev/null; then
    echo "Top pairs must not reuse 24h list volume as 30d" >&2
    return 1
  fi
  if grep -nE "getPairs\(|sort=volume_usd_24h" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx \
       frontend-dapp/src/pages/ProtocolPage.tsx 2>/dev/null; then
    echo "Protocol top pairs must not rank via GET /pairs" >&2
    return 1
  fi
  if grep -nE "innerHTML" frontend-dapp/src/components/protocol/ProtocolTopPairs.tsx 2>/dev/null; then
    echo "Top pairs must not use innerHTML" >&2
    return 1
  fi
  python3 scripts/check_chrome_nesting.py >/dev/null
}

indexer_integration() {
  (cd indexer && cargo test --lib volume_per_tvl_tests -- --test-threads=1 --quiet)
  (cd indexer && cargo test --test indexer_protocol_top_pairs -- --test-threads=1 --quiet)
}

frontend_vitest() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx \
    src/utils/__tests__/formatProtocolStats.test.ts \
    src/utils/__tests__/trailingWindowCopy.test.ts \
    src/utils/__tests__/chartsPairRoute.test.ts
}

echo "================================================================"
echo "  Forgejo #1263 — /protocol top-5 30d pair volume + TVL + vol/LP"
echo "================================================================"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres"
  make setup-indexer-postgres
fi

if [ ! -d "$REPO_ROOT/frontend-dapp/node_modules" ]; then
  COMMON="$(git rev-parse --git-common-dir)"
  MAIN_ROOT="$(cd "$COMMON/.." && pwd)"
  if [ -d "$MAIN_ROOT/frontend-dapp/node_modules" ] && [ "$MAIN_ROOT" != "$REPO_ROOT" ]; then
    ln -s "$MAIN_ROOT/frontend-dapp/node_modules" "$REPO_ROOT/frontend-dapp/node_modules"
    echo ""
    echo "[bootstrap] linked frontend-dapp/node_modules from primary checkout"
  fi
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skills + AGENTS crosslinks" docs_crosslinks
run_step "source: rollup GET; gem list reuse; no 24h /pairs rank" source_guards
run_step "indexer: 30d stamp + GET 400/cap/gems/EXPLAIN" indexer_integration
run_step "frontend: ProtocolPage top-pairs + ratio format" frontend_vitest

if [[ "${VERIFY_ISSUE_1263_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 550/569/655/692/653/562] skipped (VERIFY_ISSUE_1263_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
else
  run_step "related: verify-issue-653" make verify-issue-653
  run_step "related: verify-issue-562 (no nested e2e)" \
    env VERIFY_ISSUE_562_SKIP_E2E=1 make verify-issue-562
  run_step "related: verify-issue-655 (no nested e2e/related)" \
    env VERIFY_ISSUE_655_SKIP_E2E=1 VERIFY_ISSUE_655_SKIP_RELATED=1 make verify-issue-655
  run_step "related: verify-issue-692 (no nested e2e/related)" \
    env VERIFY_ISSUE_692_SKIP_E2E=1 VERIFY_ISSUE_692_SKIP_RELATED=1 make verify-issue-692
fi

echo ""
echo "================================================================"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "================================================================"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
