#!/usr/bin/env bash
# Automated verification for GitLab #519 — WalletConnect same-device mobile pairing.
#
# Proves (unit + docs; no wallet app / chain required):
#   1. Mobile detect, wc: validation, Lunc Dash / Galaxy deep links, scheme allowlist.
#   2. Hook intercepts mobile and leaves desktop QR to cosmes.
#   3. Pairing modal renders Open + Copy without a QR canvas.
#   4. Cosmes patch delegates to __CL8Y_WC_PAIRING_MODAL__ and does not auto-redirect.
#   5. Skills/docs/invariants WC-M1–WC-M7 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md,
#       frontend-dapp/src/utils/walletConnectPairing.ts,
#       frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx
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
echo "  GitLab #519 — WalletConnect same-device mobile pairing"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: pairing helpers + hook + modal + cosmes patch + CopyButton" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/walletConnectPairing.test.ts \
    src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts \
    src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx \
    src/services/terraclassic/__tests__/cosmesPatch127.test.ts \
    src/components/ui/__tests__/CopyButton.test.tsx'

run_step "skill: AGENTS_FRONTEND_WALLETCONNECT_MOBILE WC-M1–WC-M7" \
  grep -qE '\*\*WC-M1' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '\*\*WC-M7' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '#519' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md

run_step "docs: frontend.md walletconnect-same-device-mobile section" \
  grep -qE 'walletconnect-same-device-mobile|#519' docs/frontend.md && \
  grep -qE '\*\*WC-M1\*\*' docs/frontend.md && \
  grep -qE '\*\*WC-M7\*\*' docs/frontend.md

run_step "AGENTS.md playbook link #519" \
  grep -qE 'AGENTS_FRONTEND_WALLETCONNECT_MOBILE|#519' AGENTS.md

run_step "connect-modal skill crosslink #519" \
  grep -qE 'AGENTS_FRONTEND_WALLETCONNECT_MOBILE|#519' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

run_step "copy-button skill buttonLabel #519" \
  grep -qE 'buttonLabel|#519' skills/AGENTS_FRONTEND_COPY_BUTTON.md

run_step "code: hook installed before createRoot" \
  grep -qE 'installWalletConnectPairingHook' frontend-dapp/src/main.tsx && \
  grep -qE 'WalletConnectPairingModal' frontend-dapp/src/components/common/Layout.tsx

run_step "code: no async auto-redirect in pairing helpers" \
  bash -c '! grep -qE "location\.(href|assign)" frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx'

run_step "QA template same-device mobile cases" \
  grep -qE '1.5.1a' QA_TEMPLATE.md && \
  grep -qE '1.6.1a' QA_TEMPLATE.md

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
