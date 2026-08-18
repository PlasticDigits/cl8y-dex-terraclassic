#!/usr/bin/env bash
# Automated verification for GitLab #554 — Android Chrome Connect Wallet
# (pairing foreground, cancel/timeout, Keplr WC, Galaxy intent).
#
# Proves (unit + docs; no wallet app / chain required):
#   1. Android Galaxy Station href is intent:// (not https+#Intent).
#   2. Pairing portal z-[10001]; Connect list hides while pairing is open.
#   3. Cancel / timeout clears isConnecting; late session does not attach.
#   4. Mobile + no window.keplr offers Keplr WalletConnect, not Install-only.
#   5. Skills/docs invariants WC-M8–WC-M12 + Legal Keplr-browser hint (C1).
#
# Refs: skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md,
#       frontend-dapp/src/utils/walletConnectPairing.ts,
#       frontend-dapp/src/hooks/useWallet.ts
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
echo "  GitLab #554 — Android Chrome Connect Wallet"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: pairing + session + modal + options + useWallet + legal hint" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/walletConnectPairing.test.ts \
    src/utils/__tests__/walletConnectSession.test.ts \
    src/utils/__tests__/detectWalletInAppBrowser.test.ts \
    src/utils/__tests__/legalKeplrInAppHint.test.ts \
    src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts \
    src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx \
    src/components/wallet/__tests__/WalletModal.test.tsx \
    src/components/wallet/__tests__/connectWalletOptions.test.ts \
    src/components/wallet/__tests__/WalletButton.test.tsx \
    src/components/ui/__tests__/Modal.test.tsx \
    src/hooks/__tests__/useWallet.test.ts \
    src/components/legal/__tests__/ConnectedTermsGate.test.tsx \
    src/services/terraclassic/__tests__/cosmesPatch127.test.ts \
    src/components/ui/__tests__/CopyButton.test.tsx'

run_step "skill: AGENTS_FRONTEND_WALLETCONNECT_MOBILE WC-M8–WC-M12 + #554" \
  grep -qE '\*\*WC-M8' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '\*\*WC-M12' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '#554' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md

run_step "docs: frontend.md WC-M8–WC-M12" \
  grep -qE '\*\*WC-M8\*\*' docs/frontend.md && \
  grep -qE '\*\*WC-M12\*\*' docs/frontend.md && \
  grep -qE '#554' docs/frontend.md

run_step "AGENTS.md playbook link #554" \
  grep -qE 'verify-issue-554|#554' AGENTS.md

run_step "connect-modal skill crosslink #554" \
  grep -qE '#554' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

run_step "clickwrap skill WC-M12 / no ADR-036" \
  grep -qE 'WC-M12|#554' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE 'ADR-036' skills/AGENTS_FRONTEND_CLICKWRAP.md

run_step "code: pairing portal z-[10001] and Layout mounts pairing last" \
  grep -qE 'z-\[10001\]' frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx && \
  grep -qE 'WalletConnectPairingModal' frontend-dapp/src/components/common/Layout.tsx && \
  bash -c 'python3 - <<'"'"'PY'"'"'
from pathlib import Path
text = Path("frontend-dapp/src/components/common/Layout.tsx").read_text()
pairing = text.rfind("WalletConnectPairingModal")
wallet = text.find("<WalletButton")
assert pairing > wallet, "pairing modal must mount after WalletButton"
print("ok")
PY'

run_step "code: Android intent normalize + Keplr WC options + cancelConnection" \
  grep -qE 'toAndroidIntentUri' frontend-dapp/src/utils/walletConnectPairing.ts && \
  grep -qE 'shouldOfferKeplrWalletConnect' frontend-dapp/src/components/wallet/connectWalletOptions.ts && \
  grep -qE 'cancelConnection' frontend-dapp/src/hooks/useWallet.ts && \
  grep -qE 'abortPendingTerraWalletConnect' frontend-dapp/src/services/terraclassic/wallet.ts

run_step "code: no async auto-redirect in pairing helpers" \
  bash -c '! grep -qE "location\.(href|assign)" frontend-dapp/src/components/wallet/WalletConnectPairingModal.tsx frontend-dapp/src/components/wallet/connectWalletOptions.ts'

run_step "QA template Android Chrome #554 cases" \
  grep -qE '1.5.1b' QA_TEMPLATE.md && \
  grep -qE '1.6.1b' QA_TEMPLATE.md && \
  grep -qE '1.2.11' QA_TEMPLATE.md

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
