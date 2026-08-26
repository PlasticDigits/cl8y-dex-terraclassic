#!/usr/bin/env bash
# Automated verification for GitLab #658 — Legal terms hint is not Keplr-only.
#
# Proves (unit + docs; no Legal portal / wallet app / chain required):
#   1. Unsigned + no signer injector → hint visible; copy is not Keplr-only.
#   2. Hint names DEX wallets (or the connected wallet); Leap is absent.
#   3. Hide on window.keplr / station.keplr / cosmostation.providers.keplr.
#   4. Hide when signed_latest or status null; Station without shim still shows.
#   5. WC-M12 / C1 / L658 docs + skills match the new hide rule.
#   6. make verify-issue-554 assertions stay compatible (no Keplr-only sentence).
#
# Refs: skills/AGENTS_FRONTEND_CLICKWRAP.md,
#       skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md,
#       frontend-dapp/src/utils/legalKeplrInAppHint.ts
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
echo "  GitLab #658 — Legal terms hint is not Keplr-only"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: hint visibility + copy + ConnectedTermsGate + clickwrap" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/legalKeplrInAppHint.test.ts \
    src/components/legal/__tests__/ConnectedTermsGate.test.tsx \
    src/utils/__tests__/legalClickwrap.test.ts'

run_step "skill: AGENTS_FRONTEND_CLICKWRAP L658-1–L658-8 + not Keplr-only" \
  grep -qE '\*\*L658-1' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE '\*\*L658-8' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE '#658' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE 'not Keplr-only' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE 'ADR-036' skills/AGENTS_FRONTEND_CLICKWRAP.md

run_step "skill: AGENTS_FRONTEND_WALLETCONNECT_MOBILE WC-M12 not Keplr-only" \
  grep -qE '\*\*WC-M12' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE 'not Keplr-only' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE '#658' skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md

run_step "docs: frontend.md WC-M12 + C1 + #658" \
  grep -qE '\*\*WC-M12\*\*' docs/frontend.md && \
  grep -qE 'not Keplr-only' docs/frontend.md && \
  grep -qE '#658' docs/frontend.md && \
  grep -qE '\*\*C1\*\*' docs/frontend.md && \
  grep -qE 'verify-issue-658' docs/frontend.md

run_step "AGENTS.md playbook link #658" \
  grep -qE 'verify-issue-658|#658' AGENTS.md && \
  grep -qE 'L658' AGENTS.md

run_step "code: hasLegalSignerInjector reuses getKeplrLikeExtension; DEX wallet list; no Leap" \
  grep -qE 'hasLegalSignerInjector' frontend-dapp/src/utils/legalKeplrInAppHint.ts && \
  grep -qE 'getKeplrLikeExtension' frontend-dapp/src/utils/legalKeplrInAppHint.ts && \
  grep -qE 'LEGAL_TERMS_WALLET_HINT' frontend-dapp/src/utils/legalKeplrInAppHint.ts && \
  grep -qE 'Lunc Dash' frontend-dapp/src/utils/legalKeplrInAppHint.ts && \
  grep -qE 'Galaxy Station' frontend-dapp/src/utils/legalKeplrInAppHint.ts && \
  bash -c '! grep -E "LEGAL_TERMS_WALLET_HINT|LEGAL_TERMS_DEX_WALLET_NAMES" frontend-dapp/src/utils/legalKeplrInAppHint.ts | grep -qi Leap' && \
  bash -c '! grep -qE "signArbitrary" frontend-dapp/src/utils/legalKeplrInAppHint.ts frontend-dapp/src/components/legal/LegalKeplrInAppHint.tsx'

run_step "code: old Keplr-only sentence is gone from hint + gate" \
  bash -c '! grep -qF "Open this site in the Keplr browser to accept terms." \
    frontend-dapp/src/utils/legalKeplrInAppHint.ts \
    frontend-dapp/src/components/legal/LegalKeplrInAppHint.tsx \
    frontend-dapp/src/components/legal/ConnectedTermsGate.tsx \
    frontend-dapp/src/components/legal/__tests__/ConnectedTermsGate.test.tsx'

run_step "code: CSS class renamed off Keplr-only" \
  grep -qE 'app-connected-terms-wallet-hint' frontend-dapp/src/index.css \
    frontend-dapp/src/components/legal/LegalKeplrInAppHint.tsx && \
  bash -c '! grep -qE "app-connected-terms-keplr-hint" frontend-dapp/src/index.css frontend-dapp/src/components/legal/LegalKeplrInAppHint.tsx'

run_step "QA template multi-wallet terms-hint rows (#658)" \
  grep -qE '1\.2\.14' QA_TEMPLATE.md && \
  grep -qE '1\.1\.16' QA_TEMPLATE.md && \
  grep -qE '#658' QA_TEMPLATE.md

run_step "connect-modal skill crosslink #658" \
  grep -qE '#658' skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

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
