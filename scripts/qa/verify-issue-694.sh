#!/usr/bin/env bash
# Verification for GitLab #694 — API4 per-request caps (RE-01/02/03).
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env).
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
echo "  GitLab #694 — API4 per-request caps (RE-01 / RE-02 / RE-03)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "caps + lcd_heavy_router membership + no pair_reserves GET" \
  bash -c '
    grep -qE "MAX_GT_EVENT_ROWS" indexer/src/api/gt.rs && \
    grep -qE "event count exceeds 5000" indexer/src/api/gt.rs && \
    grep -qE "MAX_BLACKLIST_TOKENS" indexer/src/api/compliance.rs && \
    grep -qE "MAX_BLACKLIST_PAIRS" indexer/src/api/compliance.rs && \
    grep -qE "DISCOUNT_BPS_CACHE_TTL" indexer/src/api/route_solver.rs && \
    ! grep -qE "FROM pair_reserves|JOIN pair_reserves" indexer/src/api/gt.rs && \
    python3 - <<'"'"'PY'"'"'
from pathlib import Path
text = Path("indexer/src/api/mod.rs").read_text()
start = text.find("let lcd_heavy_router = Router::new()")
end = text.find("apply_rate_limit_layer(lcd_heavy_router")
block = text[start:end]
assert "/api/v1/route/solve/progress" in block, "progress not on lcd_heavy_router"
assert "/api/v1/compliance/blacklist-check" in block, "blacklist-check not on lcd_heavy_router"
api = text[text.find("let api_router = Router::new()"):text.find(".merge(lcd_heavy_router)")]
assert "blacklist-check" not in api, "blacklist-check still on global router"
after = text[text.find(".merge(lcd_heavy_router)"):]
assert after.find("/api/v1/route/solve/progress") == -1, "progress still registered after merge"
print("router membership ok")
PY
  '

run_step "docs + skills #694 crosslinks" \
  bash -c '
    grep -qE "MAX_GT_EVENT_ROWS|#694" docs/indexer-invariants.md && \
    grep -qE "route/solve/progress" docs/indexer-invariants.md && \
    grep -qE "blacklist-check" docs/indexer-invariants.md && \
    grep -qE "lcd_heavy" docs/route-solver.md && \
    grep -qE "A694-1" skills/AGENTS_INDEXER_API4_PER_REQUEST.md && \
    grep -qE "verify-issue-694" AGENTS.md && \
    grep -qE "MAX_GT_EVENT_ROWS|5000" scripts/geckoterminal/README.md
  '

run_step "indexer lib: gt + discount cache + progress" \
  bash -c 'cd indexer && cargo test --lib -- --quiet event_row_cap_is_5000 discount_bps_cache parse_progress_discount begin_update_complete list_caps_are_16'

run_step "indexer integration: security 400/429 + GT row cap + progress cache" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --test security -- --test-threads=1 --quiet route_solve_progress_lcd_heavy
    cargo test --test security -- --test-threads=1 --quiet route_solve_best_lcd_heavy
    cargo test --test security -- --test-threads=1 --quiet blacklist_check
    cargo test --test api_gt -- --test-threads=1 --quiet row_cap
    cargo test --test api_route_solve -- --test-threads=1 --quiet discount_bps_cache
  '

run_step "frontend progress omit-trader + backoff (#694)" \
  bash -c './scripts/with-node.sh --cwd frontend-dapp -- npm exec -- vitest run \
    src/utils/routeSolveProgress.test.ts \
    src/services/indexer/__tests__/client.test.ts --reporter=dot'

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
