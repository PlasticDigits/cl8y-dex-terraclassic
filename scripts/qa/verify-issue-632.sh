#!/usr/bin/env bash
# Automated verification for GitLab #632 — Keplr in-app / visualViewport token picker.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. readPortalListboxViewport + computePortalListboxStyle clearance / flip / clamp.
#   2. TokenSearchSelect coarse/narrow browse-without-IME + factory gate / #350 / #498.
#   3. detectWalletInAppBrowser: Keplr = in-app; Android Chrome ≠ in-app.
#   4. Mint TokenSelect stays a button listbox.
#   5. Skills/docs/invariants V632-1–V632-8 + QA 1.2.13 crosslinked.
#   6. usePortalListbox reads visualViewport (not innerHeight alone).
#
# Optional chain: VERIFY_ISSUE_632_CHAIN=1 runs e2e-smoke clearance + #498 CLS
# (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md,
#       frontend-dapp/src/lib/portalListboxViewport.ts,
#       frontend-dapp/src/components/ui/PortalListbox.tsx
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
echo "  GitLab #632 — Keplr in-app / visualViewport token picker"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: portal viewport + position + coarse/narrow + tap stability" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/lib/__tests__/portalListboxViewport.test.ts \
    src/lib/__tests__/coarseNarrowViewport.test.ts \
    src/lib/__tests__/optionTapStability.test.ts \
    src/components/ui/__tests__/portalListboxPosition.test.ts'

run_step "frontend: TokenSearchSelect #481/#350/#498/#632 + Mint TokenSelect" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/trade/__tests__/TokenSearchSelect.test.tsx \
    src/components/ui/__tests__/TokenSelect.keyboard.test.tsx \
    src/utils/__tests__/detectWalletInAppBrowser.test.ts'

run_step "code: visualViewport in usePortalListbox (not innerHeight-only)" \
  grep -qE 'visualViewport' frontend-dapp/src/components/ui/PortalListbox.tsx && \
  grep -qE 'getPortalListboxViewport' frontend-dapp/src/components/ui/PortalListbox.tsx && \
  bash -c '! grep -qE "height: window\.innerHeight" frontend-dapp/src/components/ui/PortalListbox.tsx'

run_step "code: coarse/narrow browse-without-IME (B2)" \
  grep -qE 'useCoarseNarrowViewport' frontend-dapp/src/components/trade/TokenSearchSelect.tsx && \
  grep -qE 'useCoarseNarrowViewport' frontend-dapp/src/components/trade/PairSearchSelect.tsx && \
  grep -qE 'SearchSelectMenuSearch' frontend-dapp/src/components/trade/TokenSearchSelect.tsx && \
  grep -qE 'TOKEN_SEARCH_MAX_QUERY_LENGTH' frontend-dapp/src/components/ui/SearchSelectMenuSearch.tsx

run_step "code: Mint TokenSelect stays a button listbox" \
  grep -qE 'triggerRef = useRef<HTMLButtonElement>' frontend-dapp/src/components/ui/TokenSelect.tsx && \
  bash -c '! grep -qE "useCoarseNarrowViewport" frontend-dapp/src/components/ui/TokenSelect.tsx'

run_step "skill: AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT V632-1–V632-8" \
  grep -qE '\*\*V632-1' skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md && \
  grep -qE '\*\*V632-8' skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md && \
  grep -qE '#632' skills/AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT.md

run_step "docs: frontend.md visualViewport + token search #632" \
  grep -qE 'V632-1' docs/frontend.md && \
  grep -qE 'visualViewport' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT' docs/frontend.md

run_step "AGENTS.md playbook link #632" \
  grep -qE 'AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT|#632' AGENTS.md

run_step "crosslinks: CLS + token-search + WC-M7 playbooks" \
  grep -qE 'AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT|#632' skills/AGENTS_FRONTEND_PORTAL_LISTBOX_CLS.md && \
  grep -qE 'AGENTS_FRONTEND_PORTAL_LISTBOX_VIEWPORT|#632' skills/AGENTS_FRONTEND_TOKEN_SEARCH.md && \
  grep -qE '#632' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md

run_step "QA template Keplr in-app picker row 1.2.13" \
  grep -qE '1.2.13' QA_TEMPLATE.md && \
  grep -qE '#632' QA_TEMPLATE.md

if [[ "${VERIFY_ISSUE_632_CHAIN:-}" == "1" ]]; then
  run_step "playwright e2e-smoke: #498 CLS + #632 clearance (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test --project=e2e-smoke \
      e2e/swap-token-select-cls.spec.ts \
      e2e/swap-token-select-viewport.spec.ts \
      e2e/trade-pair-select-cls.spec.ts'
else
  echo ""
  echo "[skip] Playwright chain specs (set VERIFY_ISSUE_632_CHAIN=1 when LocalTerra + deploy env are up)"
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
echo "OK"
