#!/usr/bin/env bash
# Automated verification for GitLab #671 — connected wallet dropdown alignment.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. .wallet-menu-item is a horizontal flex row (icon left, label right).
#   2. Wallet header AddressRow is nowrap/truncated; showFull still exists for other pages.
#   3. CopyButton menuLabel is menuitem; icon-only / buttonLabel are not.
#   4. Explorer javascript:/data: hrefs are omitted; trader path rejects non-bech32.
#   5. Docs/skills W671-1–W671-8 + AGENTS.md / testing.md crosslinks.
#   6. Optional: e2e/navigation.spec.ts wallet geometry (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_WALLET_CHIP.md,
#       frontend-dapp/src/index.css,
#       docs/frontend.md § Connected wallet dropdown
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
echo "  GitLab #671 — connected wallet dropdown alignment"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: WalletButton + dropdown + CopyButton + AddressRow + routes" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/wallet/__tests__/WalletButton.test.tsx \
    src/components/wallet/__tests__/WalletDropdownMenuItems.test.tsx \
    src/components/ui/__tests__/CopyButton.test.tsx \
    src/components/ui/__tests__/AddressRow.test.tsx \
    src/components/ui/__tests__/AddressRow.explorerSafety.test.tsx \
    src/utils/__tests__/walletMenuRoutes.test.ts \
    src/utils/__tests__/terraExplorer.test.ts'

run_step "code: .wallet-menu-item owns horizontal flex (no per-row Tailwind)" \
  grep -qE '\.wallet-menu-item \{' frontend-dapp/src/index.css && \
  grep -qE 'display: inline-flex' frontend-dapp/src/index.css && \
  grep -qE 'flex-wrap: nowrap' frontend-dapp/src/index.css && \
  grep -qE 'align-items: center' frontend-dapp/src/index.css && \
  grep -nE 'GitLab #671' frontend-dapp/src/index.css >/dev/null && \
  bash -c '! grep -nE "inline-flex items-center gap-2" frontend-dapp/src/components/ui/CopyButton.tsx' && \
  grep -qE 'className=\{`wallet-menu-item' frontend-dapp/src/components/ui/CopyButton.tsx && \
  grep -qE 'nowrap' frontend-dapp/src/components/wallet/WalletButton.tsx && \
  bash -c '! grep -nE "showFull" frontend-dapp/src/components/wallet/WalletButton.tsx' && \
  grep -qE 'isSafeExplorerHref' frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx && \
  grep -qE 'traderProfilePath' frontend-dapp/src/components/wallet/WalletButton.tsx && \
  bash -c '! grep -nE -- "-neo" frontend-dapp/src/components/wallet/WalletButton.tsx' && \
  bash -c '! grep -nE -- "-neo" frontend-dapp/src/components/wallet/WalletDropdownMenuItems.tsx'

run_step "docs: frontend.md W671-1–W671-8 + AddressRow wallet header" \
  grep -qE '\*\*W671-1' docs/frontend.md && \
  grep -qE '\*\*W671-8' docs/frontend.md && \
  grep -qE 'connected-wallet-dropdown|#671' docs/frontend.md && \
  grep -qE 'nowrap' docs/frontend.md && \
  bash -c '! grep -nE "showFull.*wallet dropdown menu" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_WALLET_CHIP W671 + verify target" \
  grep -qE '\*\*W671-1' skills/AGENTS_FRONTEND_WALLET_CHIP.md && \
  grep -qE '\*\*W671-8' skills/AGENTS_FRONTEND_WALLET_CHIP.md && \
  grep -qE 'make verify-issue-671' skills/AGENTS_FRONTEND_WALLET_CHIP.md && \
  grep -qE 'Forbid.*stacking icons above labels|stacking icons above labels' skills/AGENTS_FRONTEND_WALLET_CHIP.md

run_step "skill: AddressRow + CopyButton + explorer + copy-load crosslinks #671" \
  grep -qE 'AGENTS_FRONTEND_WALLET_CHIP|#671' skills/AGENTS_FRONTEND_ADDRESS_ROW.md && \
  grep -qE 'nowrap' skills/AGENTS_FRONTEND_ADDRESS_ROW.md && \
  grep -qE 'AGENTS_FRONTEND_WALLET_CHIP|#671' skills/AGENTS_FRONTEND_COPY_BUTTON.md && \
  grep -qE 'isSafeExplorerHref|#671' skills/AGENTS_FRONTEND_TERRA_EXPLORER.md && \
  grep -qE 'AGENTS_FRONTEND_COPY_COGNITIVE_LOAD' skills/AGENTS_FRONTEND_WALLET_CHIP.md

run_step "AGENTS.md playbook link #671" \
  grep -qE 'AGENTS_FRONTEND_WALLET_CHIP|#671' AGENTS.md && \
  grep -qE 'verify-issue-671' AGENTS.md

run_step "docs: testing.md + QA_TEMPLATE verify-issue-671" \
  grep -qE 'verify-issue-671' docs/testing.md && \
  grep -qE 'W671-1' docs/testing.md && \
  grep -qE '#671|icon-left' QA_TEMPLATE.md

if make -s has-localterra >/dev/null 2>&1; then
  run_step "playwright: connected dropdown geometry (5 workers)" \
    bash -c 'PLAYWRIGHT_WEB_PORT=3100 bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/navigation.spec.ts -g "671|#185|disconnects wallet|connected wallet dropdown"'
else
  echo ""
  echo "[playwright: connected dropdown geometry (5 workers)]"
  echo "  SKIP (LocalTerra not up — unit + docs still required)"
  ok "playwright: skipped (no LocalTerra)"
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
