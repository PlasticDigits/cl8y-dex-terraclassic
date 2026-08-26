#!/usr/bin/env bash
# Automated verification for GitLab #672 — Connect Wallet / Modal dismiss overlay.
#
# Proves (unit + docs; Playwright when LocalTerra / Vite is up):
#   D1  Labeled Close control on dismissible modals.
#   D2  Portal root / backdrop click dismisses.
#   D3  Escape closes when dismissible; focus trap while open.
#   D4  Panel / wallet row / Install / pairing Open-Copy do not dismiss.
#   D5  Header Connect Wallet toggles closed.
#   D6  Dismiss while connecting cancels (closeWalletModal → cancelConnection).
#   D7  Risk acknowledgement stays dismissible={false}.
#   D8  Pairing z-[10001] above Connect z-[9999].
#   D9  Vitest + skill/docs crosslinks.
#
# Optional: VERIFY_ISSUE_672_CHAIN=1 runs e2e-smoke navigation Connect dismiss
# (5 workers).
#
# Refs: skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md,
#       frontend-dapp/src/components/ui/Modal.tsx,
#       docs/frontend.md#connect-modal-dismiss
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
echo "  GitLab #672 — Connect Wallet / Modal dismiss overlay"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: Modal + WalletModal + WalletButton + useWallet + risk + expert + pairing" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/ui/__tests__/Modal.test.tsx \
    src/components/wallet/__tests__/WalletModal.test.tsx \
    src/components/wallet/__tests__/WalletButton.test.tsx \
    src/hooks/__tests__/useWallet.test.ts \
    src/components/legal/__tests__/RiskAcknowledgementModal.test.tsx \
    src/components/swap/__tests__/ExpertModeModal.test.tsx \
    src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx'

run_step "code: portal root dismiss + panel stopPropagation" \
  grep -qE 'onClick=\{dismissible \? handleDismiss' frontend-dapp/src/components/ui/Modal.tsx && \
  grep -qE 'stopPropagation' frontend-dapp/src/components/ui/Modal.tsx && \
  grep -qE 'app-modal-body' frontend-dapp/src/components/ui/Modal.tsx && \
  grep -qE 'app-modal-close' frontend-dapp/src/components/ui/Modal.tsx

run_step "code: labeled Close (not Close modal / btn-muted-only)" \
  grep -qE 'closeAriaLabel' frontend-dapp/src/components/ui/Modal.tsx && \
  grep -qE 'Close connect wallet' frontend-dapp/src/components/wallet/WalletModal.tsx && \
  bash -c '! grep -qE "aria-label=\"Close modal\"" frontend-dapp/src/components/ui/Modal.tsx' && \
  bash -c '! grep -qE "btn-muted.*Close|aria-label=\"Close modal\"" frontend-dapp/src/components/ui/Modal.tsx'

run_step "code: header Connect toggles via closeWalletModal" \
  grep -qE 'walletModalOpen \|\| isConnecting' frontend-dapp/src/components/wallet/WalletButton.tsx && \
  grep -qE 'aria-expanded=\{walletModalOpen\}' frontend-dapp/src/components/wallet/WalletButton.tsx && \
  grep -qE 'aria-haspopup="dialog"' frontend-dapp/src/components/wallet/WalletButton.tsx

run_step "code: risk ack stays dismissible false" \
  grep -qE 'dismissible=\{false\}' frontend-dapp/src/components/legal/RiskAcknowledgementModal.tsx && \
  bash -c '! grep -qE "dangerouslySetInnerHTML" frontend-dapp/src/components/ui/Modal.tsx'

run_step "css: pinned header + scroll body + visible close" \
  grep -qE '\.app-modal-body' frontend-dapp/src/index.css && \
  grep -qE '\.app-modal-close' frontend-dapp/src/index.css && \
  grep -qE 'max-height: calc\(100dvh' frontend-dapp/src/index.css && \
  grep -qE 'flex-shrink: 0' frontend-dapp/src/index.css

run_step "skill: AGENTS_FRONTEND_WALLET_CONNECT_MODAL D1–D9" \
  grep -qE '\*\*D1' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md && \
  grep -qE '\*\*D9' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md && \
  grep -qE '#672' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

run_step "docs: frontend.md connect-modal-dismiss D1–D9" \
  grep -qE 'connect-modal-dismiss' docs/frontend.md && \
  grep -qE '\*\*D1\*\*' docs/frontend.md && \
  grep -qE '\*\*D9\*\*' docs/frontend.md && \
  grep -qE '#672' docs/frontend.md

run_step "AGENTS.md playbook link #672" \
  grep -qE 'AGENTS_FRONTEND_WALLET_CONNECT_MODAL|#672' AGENTS.md

run_step "crosslinks: WC-M9 + risk + clickwrap" \
  grep -qE '#672' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '#672' skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md && \
  grep -qE '#672' skills/AGENTS_FRONTEND_CLICKWRAP.md

run_step "QA template 1.8 dismiss rows" \
  grep -qE '1.8.2' QA_TEMPLATE.md && \
  grep -qE '#672' QA_TEMPLATE.md && \
  grep -qE '11.1.8' QA_TEMPLATE.md

if [[ "${VERIFY_ISSUE_672_CHAIN:-}" == "1" ]]; then
  run_step "playwright e2e-smoke: Connect dismiss (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test --project=e2e-smoke \
      e2e/navigation.spec.ts -g "wallet modal"'
else
  echo ""
  echo "[skip] Playwright chain specs (set VERIFY_ISSUE_672_CHAIN=1 when Vite/LocalTerra are up)"
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
