#!/usr/bin/env bash
# Verification for GitLab #577: token/trader/pair/global trailing-window decay + stale overview.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #577 — trailing-window volume decay"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS + runbook crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md
    grep -q "Trailing window decay (#577)" docs/indexer-invariants.md
    grep -q "AGENTS_INDEXER_VOLUME_WINDOW_DECAY" AGENTS.md
    grep -q "verify-issue-577" Makefile
    grep -q "D1" skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md
    grep -q "D6" skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md
    grep -q "Rollup freshness (GitLab #577" docs/runbooks/overview-global-stats-brin.md
    grep -q "AGENTS_INDEXER_VOLUME_WINDOW_DECAY" skills/AGENTS_INDEXER_VOLUME_PAGINATION.md
    grep -q "AGENTS_INDEXER_VOLUME_WINDOW_DECAY" skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "refresh_all_volume_windows" indexer/src/indexer/poller.rs
    grep -q "refresh_token_volumes" indexer/src/indexer/volume_aggregator.rs
    grep -q "refresh_rolling_volumes" indexer/src/indexer/volume_aggregator.rs
  '

run_step "code: decay SQL is parameterized; no live scan on stale GET" \
  bash -c '
    set -euo pipefail
    grep -q "WHERE tvs.\"window\" = \$1" indexer/src/db/queries/volume.rs
    grep -q "is_global_stats_stale" indexer/src/db/queries/volume.rs
    grep -q "no live 30d swap_events scan" indexer/src/db/queries/volume.rs
    grep -q "volume_24h = 0" indexer/src/db/queries/traders.rs
    grep -q "total_volume" indexer/src/db/queries/traders.rs
    # Zero-out must not concatenate window strings into SQL.
    ! grep -E "token_volume_stats.*window.*format!" indexer/src/db/queries/volume.rs
    grep -q "GROUP BY offer_asset_id" indexer/src/db/queries/volume.rs
  '

if [ -f "$REPO_ROOT/indexer/.env" ]; then
  :
else
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "indexer lib: stale-bound unit tests" \
  bash -c 'cd indexer && cargo test --lib db::queries::volume::tests -- --quiet'

run_step "indexer integration: decay + overview + pair + traders + tokens" \
  bash -c 'cd indexer && cargo test --test indexer_volume_window_decay --test indexer_overview_global_stats --test indexer_pair_volume_pagination --test api_traders --test api_tokens -- --test-threads=1 --quiet'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
