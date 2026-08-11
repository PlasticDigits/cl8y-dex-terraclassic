#!/usr/bin/env bash
# Automated verification for GitLab #517 — CL8Y Legal clickwrap (TermsGate).
#
# Proves (unit + docs; no Legal portal / chain required):
#   1. Property defaults to dex.cl8y.com; redirect allowlist rejects evil origins.
#   2. ConnectedTermsGate fail-closed / TerraClassic status / signed path.
#   3. Production CSP includes Legal hosts without blanket https:.
#   4. Skills/docs/invariants C1–C10 crosslinked; AGENTS playbook present.
#   5. SDK dependency + GitLab npm registry .npmrc present.
#
# Refs: skills/AGENTS_FRONTEND_CLICKWRAP.md,
#       frontend-dapp/src/utils/legalClickwrap.ts,
#       frontend-dapp/src/components/legal/ConnectedTermsGate.tsx
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
echo "  GitLab #517 — CL8Y Legal clickwrap (TermsGate)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: legalClickwrap + ConnectedTermsGate + viteCsp" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/legalClickwrap.test.ts \
    src/components/legal/__tests__/ConnectedTermsGate.test.tsx \
    src/utils/__tests__/viteCsp.test.ts'

run_step "dependency: @plasticdigits/cl8y-clickwrap in package.json" \
  grep -qE '"@plasticdigits/cl8y-clickwrap"' frontend-dapp/package.json

run_step "npmrc: @plasticdigits GitLab registry" \
  grep -qE 'packages/npm' frontend-dapp/.npmrc && \
  grep -qE '@plasticdigits:registry' frontend-dapp/.npmrc

run_step "code: ConnectedTermsGate uses TerraClassic + Layout Outlet wrap" \
  grep -qE 'network="TerraClassic"' frontend-dapp/src/components/legal/ConnectedTermsGate.tsx && \
  grep -qE 'ConnectedTermsGate' frontend-dapp/src/components/common/Layout.tsx

run_step "code: no custom wallet verify helpers in legal gate" \
  bash -c '! grep -qE "signArbitrary|submitWallet|experimentalSuggestChain" frontend-dapp/src/utils/legalClickwrap.ts frontend-dapp/src/components/legal/ConnectedTermsGate.tsx'

run_step "skill: AGENTS_FRONTEND_CLICKWRAP C1–C10" \
  grep -qE '\*\*C1' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE '\*\*C10' skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE 'dex\.cl8y\.com' skills/AGENTS_FRONTEND_CLICKWRAP.md

run_step "docs: frontend.md legal-clickwrap section" \
  grep -qE 'legal-clickwrap|#517' docs/frontend.md && \
  grep -qE '\*\*C1\*\*' docs/frontend.md

run_step "docs: soft-launch runbook #517" \
  grep -qE '#517|clickwrap' docs/runbooks/mainnet-soft-launch.md

run_step "AGENTS.md playbook link #517" \
  grep -qE 'AGENTS_FRONTEND_CLICKWRAP|#517' AGENTS.md

run_step "risk skill crosslink #517" \
  grep -qE 'AGENTS_FRONTEND_CLICKWRAP|#517' skills/AGENTS_FRONTEND_RISK_DISCLAIMERS.md

run_step "env example Legal vars" \
  grep -qE 'VITE_LEGAL_PROPERTY|VITE_LEGAL_API_BASE_URL' frontend-dapp/.env.example

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

echo ""
echo "Optional Playwright smoke (needs frontend deps + browsers):"
echo "  bash scripts/with-node.sh --cwd frontend-dapp -- npx playwright test e2e/legal-clickwrap-517.spec.ts --project=e2e-smoke"
exit 0
