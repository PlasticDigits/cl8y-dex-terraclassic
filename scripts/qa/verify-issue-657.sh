#!/usr/bin/env bash
# Automated verification for GitLab #657 — Trader page global leaderboard.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Shared TraderLeaderboard on /trader + Charts; not on /portfolio.
#   2. Volume tab is total_volume_usd limit 20; no formatNum(raw total_volume).
#   3. Board is last TraderPage sibling (outside profile gates).
#   4. Docs/skills TL-1–TL-10 + #553 / #653 crosslinks.
#   5. Optional: e2e/trader-page.spec.ts (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md,
#       frontend-dapp/src/components/trader/TraderLeaderboard.tsx,
#       docs/frontend.md § Trader profile
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

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #657 — Trader page global leaderboard"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: TraderLeaderboard + TraderPage + Charts #553 + Portfolio no-board" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/trader/TraderLeaderboard.test.tsx \
    src/pages/TraderPage.test.tsx \
    src/pages/ChartsPage.test.tsx \
    src/pages/PortfolioPage.test.tsx \
    src/utils/__tests__/chartsOverviewStats.test.ts'

run_step "code: shared component last on TraderPage; Charts wrapper; no portfolio board" \
  bash -c '
    set -euo pipefail
    grep -q "TraderLeaderboard" frontend-dapp/src/pages/TraderPage.tsx
    grep -q "highlightAddress" frontend-dapp/src/pages/TraderPage.tsx
    grep -q "<TraderLeaderboard" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "TraderLeaderboard" frontend-dapp/src/pages/PortfolioPage.tsx
    grep -q "getLeaderboard(sort, TRADER_LEADERBOARD_LIMIT)" frontend-dapp/src/components/trader/useTraderLeaderboardQuery.ts
    grep -q "total_volume_usd" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "formatIndexedVolumeUsd" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "charts-leaderboard-volume" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    grep -q "isValidTerraAddress" frontend-dapp/src/components/trader/traderLeaderboard.ts
    grep -q "to={\`/trader/\${address}\`}" frontend-dapp/src/components/trader/TraderIdentity.tsx
    grep -q "queryKey:" frontend-dapp/src/components/trader/useTraderLeaderboardQuery.ts
    grep -q "pairQueryKey" frontend-dapp/src/components/trader/useTraderLeaderboardQuery.ts
    ! grep -qE "formatNum\(.*total_volume[^_]" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    ! grep -q "dangerouslySetInnerHTML" frontend-dapp/src/components/trader/TraderLeaderboard.tsx
    ! grep -q "formatNum(trader.total_volume)" frontend-dapp/src/pages/TraderPage.tsx
  '

run_step "code: board is last TraderPage sibling (outside trader &&)" \
  python3 - <<'PY'
from pathlib import Path
src = Path("frontend-dapp/src/pages/TraderPage.tsx").read_text()
# Last non-closing content child of the page root must be TraderLeaderboard.
idx = src.rfind("<TraderLeaderboard")
if idx < 0:
    raise SystemExit("TraderLeaderboard missing")
after_profile = src.rfind("{trader &&")
if after_profile > idx:
    raise SystemExit("TraderLeaderboard is not after the profile gate")
tail = src[idx:]
if "Trade History" in tail:
    raise SystemExit("Trade History appears after TraderLeaderboard")
print("TraderLeaderboard is last content mount")
PY

run_step "code: unscoped leaderboard remains; pair ranks are #666" \
  bash -c '
    set -euo pipefail
    grep -q "get_leaderboard_for_pair" indexer/src/db/queries/traders.rs
    grep -q "PAIR_SCOPED_SORTS" indexer/src/api/traders.rs
    grep -qE "fn get_leaderboard|async fn get_leaderboard" indexer/src/db/queries/traders.rs
  '

run_step "docs: frontend.md TL-1–TL-10 + Charts crosslink" \
  grep -qE '\*\*TL-1\*\*' docs/frontend.md && \
  grep -qE '\*\*TL-10\*\*' docs/frontend.md && \
  grep -qE 'trader-leaderboard|#657' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD' docs/frontend.md && \
  grep -qE 'TraderLeaderboard|#657' docs/frontend.md

run_step "docs: testing.md + indexer-invariants #657" \
  grep -qE 'verify-issue-657' docs/testing.md && \
  grep -qE 'TL-1' docs/testing.md && \
  grep -qE '#657' docs/indexer-invariants.md

run_step "skill: AGENTS_FRONTEND_TRADER_LEADERBOARD + volume USD / chrome / portfolio" \
  grep -qE '\*\*TL-1' skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md && \
  grep -qE 'make verify-issue-657' skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md && \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD|#657' skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md && \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD|#657' skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD|#657' skills/AGENTS_FRONTEND_PORTFOLIO.md && \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD|#657' skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md

run_step "AGENTS.md playbook + verify-issue-657" \
  grep -qE 'AGENTS_FRONTEND_TRADER_LEADERBOARD' AGENTS.md && \
  grep -qE 'verify-issue-657' AGENTS.md && \
  grep -qE 'verify-issue-657' Makefile

run_step "chrome nesting static guard" \
  python3 scripts/check_chrome_nesting.py

if make -s has-localterra >/dev/null 2>&1 && [[ -f frontend-dapp/.env.local ]]; then
  run_step "playwright: trader-page board above footer (5 workers)" \
    bash -c 'PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}" \
      PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-3173}" \
      bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 e2e/trader-page.spec.ts'
else
  echo ""
  echo "[playwright: trader-page board above footer (5 workers)]"
  echo "  SKIP (LocalTerra/.env.local not available in this checkout — unit + docs still required)"
  ok "playwright: skipped (no LocalTerra env)"
fi

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
