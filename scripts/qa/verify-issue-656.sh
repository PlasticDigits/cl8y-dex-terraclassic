#!/usr/bin/env bash
# Automated verification for GitLab #656 — trader 4/6 address + blockie PFP.
#
# Proves (unit + docs; no LocalTerra / Postgres):
#   1. shortenTraderAddress is 4/6; shortenAddress defaults stay 8/6.
#   2. TraderBlockie lowercase seed; invalid strings paint nothing.
#   3. TraderIdentity leaderboard Link is /trader/{full}; A1 collision; A3/A5 junk.
#   4. Charts leaderboard + TraderSummaryStats identity + AddressRow 4/6 / copy-full.
#   5. Docs/skills T-ID-1–T-ID-10 crosslinked; no second identicon package.
#
# Refs: skills/AGENTS_FRONTEND_TRADER_IDENTITY.md,
#       frontend-dapp/src/components/trader/TraderIdentity.tsx,
#       docs/frontend.md § Trader identity
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
echo "  GitLab #656 — trader 4/6 + blockie identity"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: shortenTraderAddress + TraderBlockie + TraderIdentity" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenDisplay.test.ts \
    src/components/trader/TraderBlockie.test.tsx \
    src/components/trader/TraderIdentity.test.tsx'

run_step "frontend: Charts leaderboard + TraderSummaryStats + AddressRow" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ChartsPage.test.tsx \
    src/components/trader/TraderSummaryStats.test.tsx \
    src/components/ui/__tests__/AddressRow.test.tsx \
    src/components/ui/__tests__/AddressRow.explorerSafety.test.tsx'

run_step "frontend: TokenLogo allowlist + wallet chip unchanged (T-ID-8)" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenLogoAllowlist.test.ts \
    src/components/wallet/__tests__/WalletButton.test.tsx'

run_step "code: shared primitive; no TokenLogo; no second identicon pkg" \
  bash -c '
    set -euo pipefail
    grep -q "resolveAllowedTokenLogoUri" frontend-dapp/src/components/ui/TokenLogo.tsx
    grep -q "TraderIdentity" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "TraderIdentity" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "TRADER_ADDR_START_CHARS" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    grep -q "shortenTraderAddress" frontend-dapp/src/utils/tokenDisplay.ts
    grep -q "react-blockies" frontend-dapp/src/components/trader/TraderBlockie.tsx
    ! grep -qE "TokenLogo" frontend-dapp/src/components/trader/TraderBlockie.tsx
    ! grep -qE "TokenLogo" frontend-dapp/src/components/trader/TraderIdentity.tsx
    ! grep -qE "shortenAddress\\(trader.address, 10, 6\\)" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -qE "startChars=\\{12\\}" frontend-dapp/src/components/trader/TraderSummaryStats.tsx
    python3 - <<'"'"'PY'"'"'
from pathlib import Path
pkg = Path("frontend-dapp/package.json").read_text()
if "react-blockies" not in pkg:
    raise SystemExit("react-blockies missing from package.json")
for needle in ("jdenticon", "@download/blockies", "ethereum-blockies"):
    if needle in pkg:
        raise SystemExit("unexpected identicon package: " + needle)
for rel in (
    "frontend-dapp/src/components/trader/TraderIdentity.tsx",
    "frontend-dapp/src/components/trader/TraderBlockie.tsx",
):
    text = Path(rel).read_text()
    if "logo_url" in text or "avatar_url" in text:
        raise SystemExit("trader identity must ignore remote avatars in " + rel)
    if "dangerouslySetInnerHTML" in text:
        raise SystemExit("no dangerouslySetInnerHTML in " + rel)
PY
  '

run_step "docs: frontend.md T-ID-1–T-ID-10" \
  bash -c '
    grep -qE "\*\*T-ID-1\*\*" docs/frontend.md && \
    grep -qE "\*\*T-ID-10\*\*" docs/frontend.md && \
    grep -qE "trader-identity" docs/frontend.md && \
    grep -qE "verify-issue-656" docs/frontend.md
  '

run_step "skill: AGENTS_FRONTEND_TRADER_IDENTITY + AddressRow + AGENTS.md" \
  bash -c '
    grep -qE "\*\*T-ID-1" skills/AGENTS_FRONTEND_TRADER_IDENTITY.md && \
    grep -qE "make verify-issue-656" skills/AGENTS_FRONTEND_TRADER_IDENTITY.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_ADDRESS_ROW.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_TRADER_VOLUME_USD.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_WALLET_CHIP.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_COPY_BUTTON.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_TERRA_EXPLORER.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" skills/AGENTS_FRONTEND_PORTFOLIO.md && \
    grep -qE "AGENTS_FRONTEND_TRADER_IDENTITY|#656" AGENTS.md && \
    grep -qE "verify-issue-656" AGENTS.md && \
    grep -qE "Trader identity \(#656\)" docs/indexer-invariants.md && \
    grep -qE "verify-issue-656" Makefile
  '

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
