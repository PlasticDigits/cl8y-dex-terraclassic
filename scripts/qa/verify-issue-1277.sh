#!/usr/bin/env bash
# Verification for GitLab #1277: NUMERIC(38, 0) trader rolling + lifetime raw volume.
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
echo "  GitLab #1277 — trader rolling raw volume NUMERIC(38, 0)"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS + sibling crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md
    grep -q "Trader rolling raw volume (#1277)" docs/indexer-invariants.md
    grep -q "R1277-1" docs/indexer-invariants.md
    grep -q "R1277-8" skills/AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md
    grep -q "AGENTS_INDEXER_TRADER_ROLLING_NUMERIC" AGENTS.md
    grep -q "verify-issue-1277" AGENTS.md
    grep -q "verify-issue-1277" Makefile
    grep -q "NUMERIC(38, 0)" skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md
    grep -q "AGENTS_INDEXER_TRADER_ROLLING_NUMERIC" skills/AGENTS_INDEXER_VOLUME_WINDOW_DECAY.md
    grep -q "AGENTS_INDEXER_TRADER_ROLLING_NUMERIC" skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md
    grep -q "20260921120000_traders_rolling_volume_numeric_38_0" skills/AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md
  '

run_step "code: widen + integer LEAST cap; USD cap untouched" \
  bash -c '
    set -euo pipefail
    grep -q "ALTER COLUMN volume_24h TYPE NUMERIC(38, 0)" indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql
    grep -q "ALTER COLUMN total_volume TYPE NUMERIC(38, 0)" indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql
    grep -q "total_volume_usd stays P522-Q" indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql
    grep -q "LEAST(\$2::numeric, POWER(10::numeric, 38) - 1)" indexer/src/db/queries/traders.rs
    grep -q "LEAST(traders.total_volume + \$2, POWER(10::numeric, 38) - 1)" indexer/src/db/queries/traders.rs
    grep -q "POWER(10::numeric, 20) - POWER(10::numeric, -18)" indexer/src/db/queries/traders.rs
    grep -q "volume_24h: bd_plain_string" indexer/src/api/traders.rs
    grep -q "Failed to refresh {label}" indexer/src/indexer/volume_aggregator.rs
    grep -q "heal_trader_lifetime_from_swaps_if_needed" indexer/src/indexer/poller.rs
    test -f indexer/migrations/20260921130000_traders_lifetime_heal_from_swaps.sql
    grep -q "heal_trader_lifetime_from_swaps" indexer/src/db/queries/traders.rs
    # Forbidden: clamp rolling raw to USD 10^20 cap (A3).
    ! grep -n "vol_24h" indexer/src/db/queries/traders.rs | grep -q "POWER(10::numeric, 20)"
  '

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

# Isolate sqlx migrate from sibling-worktree checksums on shared dex_indexer_test
# (VersionMismatch). Integration tests load indexer/.env via from_filename_override.
# shellcheck source=scripts/lib/upsert-dotenv.sh
source "$REPO_ROOT/scripts/lib/upsert-dotenv.sh"

TEST_DB_1277="${TEST_DATABASE_URL_1277:-postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test_1277}"
export TEST_DB_LOCK_FILE="${TEST_DB_LOCK_FILE:-/tmp/cl8y-dex-indexer-test-1277.seed.lock}"
if command -v psql >/dev/null 2>&1; then
  PGPASSWORD="${PGPASSWORD:-cl8y_legal}" timeout 5s psql -h 127.0.0.1 -p 5432 -U cl8y_legal -d postgres \
    -tc "SELECT 1 FROM pg_database WHERE datname='dex_indexer_test_1277'" 2>/dev/null | grep -q 1 || \
  PGPASSWORD="${PGPASSWORD:-cl8y_legal}" timeout 5s psql -h 127.0.0.1 -p 5432 -U cl8y_legal -d postgres \
    -c "CREATE DATABASE dex_indexer_test_1277 OWNER cl8y_legal" >/dev/null 2>&1 || true
fi

ENV_BACKUP="$(mktemp)"
cp "$REPO_ROOT/indexer/.env" "$ENV_BACKUP"
restore_indexer_env() { cp "$ENV_BACKUP" "$REPO_ROOT/indexer/.env"; rm -f "$ENV_BACKUP"; }
trap restore_indexer_env EXIT
upsert_dotenv_var "$REPO_ROOT/indexer/.env" TEST_DATABASE_URL "$TEST_DB_1277"
upsert_dotenv_var "$REPO_ROOT/indexer/.env" TEST_DB_LOCK_FILE "$TEST_DB_LOCK_FILE"
export TEST_DATABASE_URL="$TEST_DB_1277"

run_step "indexer integration: 10^21 rolling + upsert cap + leaderboard" \
  bash -c 'cd indexer && cargo test -j 1 --test indexer_trader_rolling_numeric -- --test-threads=1 --quiet'

run_step "indexer integration: #577 decay still green" \
  bash -c 'cd indexer && cargo test -j 1 --test indexer_volume_window_decay -- --test-threads=1 --quiet'

run_step "indexer integration: #548 catalog overflow" \
  bash -c 'cd indexer && cargo test -j 1 --test volume_usd_catalog -- --test-threads=1 --quiet'

run_step "indexer integration: traders API" \
  bash -c 'cd indexer && cargo test -j 1 --test api_traders -- --test-threads=1 --quiet'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
