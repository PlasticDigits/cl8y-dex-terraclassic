#!/usr/bin/env bash
# Automated verification for GitLab #566 — Station + Cosmostation WalletConnect
# (ustr-cmm parity, no Leap).
#
# Proves (unit + docs; no wallet app / chain required):
#   1. Mobile + no Station injection → Station WalletConnect, not Install-only.
#   2. Mobile + no Cosmostation injection → Cosmostation WalletConnect.
#   3. Injected Station / Cosmostation stay Extension (WC-M7).
#   4. Desktop Station / Cosmostation stay Extension (WC-M2).
#   5. Leap is absent from the Connect list (GitLab #159).
#   6. Pairing allowlist includes terrastation.page.link + cosmostation: (WC-M5).
#   7. Skills/docs/QA/AGENTS.md crosslinked with #566.
#
# Refs: skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md,
#       frontend-dapp/src/components/wallet/connectWalletOptions.ts,
#       frontend-dapp/src/utils/walletConnectPairing.ts
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
echo "  GitLab #566 — Station + Cosmostation WalletConnect"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: options + modal + pairing + Leap purge" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/wallet/__tests__/connectWalletOptions.test.ts \
    src/components/wallet/__tests__/WalletModal.test.tsx \
    src/utils/__tests__/walletConnectPairing.test.ts \
    src/hooks/__tests__/useWallet.test.ts \
    src/services/terraclassic/__tests__/walletExtensionInstall.test.ts'

run_step "skill: AGENTS_FRONTEND_WALLETCONNECT_MOBILE WC-M10 + #566" \
  grep -qE '\*\*WC-M10' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '#566' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE 'cosmostation:' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE 'Open Station' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE 'Open Cosmostation' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md

run_step "docs: frontend.md WC-M5 cosmostation + WC-M10 Station/Cosmostation" \
  grep -qE 'cosmostation:' docs/frontend.md && \
  grep -qE '\*\*WC-M10\*\*' docs/frontend.md && \
  grep -qE '#566' docs/frontend.md && \
  grep -qE 'verify-issue-566' docs/frontend.md

run_step "AGENTS.md playbook link #566" \
  grep -qE 'verify-issue-566|#566' AGENTS.md

run_step "connect-modal skill crosslink #566" \
  grep -qE '#566' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

run_step "code: Station/Cosmostation WC helpers + cosmostation allowlist" \
  grep -qE 'shouldOfferStationWalletConnect' frontend-dapp/src/components/wallet/connectWalletOptions.ts && \
  grep -qE 'shouldOfferCosmostationWalletConnect' frontend-dapp/src/components/wallet/connectWalletOptions.ts && \
  grep -qE 'stationInjected' frontend-dapp/src/components/wallet/WalletModal.tsx && \
  grep -qE 'cosmostationInjected' frontend-dapp/src/components/wallet/WalletModal.tsx && \
  grep -qE 'cosmostation:' frontend-dapp/src/utils/walletConnectPairing.ts

run_step "code: Leap stays out of Connect list and install URLs" \
  bash -c '! grep -qE "WalletName\.LEAP" frontend-dapp/src/components/wallet/connectWalletOptions.ts' && \
  bash -c '! grep -qE "WalletName\.LEAP" frontend-dapp/src/services/terraclassic/walletExtensionInstall.ts'

run_step "code: no async auto-redirect in pairing helpers" \
  bash -c '! grep -qE "location\.(href|assign)" frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx frontend-dapp/src/components/wallet/connectWalletOptions.ts'

run_step "QA template Station/Cosmostation WC cases" \
  grep -qE '1.1.13' QA_TEMPLATE.md && \
  grep -qE '1.4.11' QA_TEMPLATE.md && \
  grep -qE '#566' QA_TEMPLATE.md

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
