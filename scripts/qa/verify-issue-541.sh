#!/usr/bin/env bash
# Automated verification for GitLab #541 — compact token identity on Pool / Trade / Charts.
#
# Proves (unit + docs; Playwright P4 always; P1–P3 when chain is up):
#   1. tokenIdentityTarget CW20 / native / adversarial / look-alike / invert-stable.
#   2. TokenIdentity + PairTokenLinks SEC-E10 + native-no-link + invert payloads.
#   3. Pool / Trade / Charts page mounts; #176 hides the row; LP AddressRow kept.
#   4. Docs/skills/invariants T541-1–T541-8 crosslinked; AGENTS playbook present.
#   5. No /token route, no picker-option identity icons, no factory/router clone.
#
# Refs: skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md,
#       frontend-dapp/src/utils/tokenIdentity.ts,
#       docs/frontend.md § Token identity
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
echo "  GitLab #541 — compact token identity (Pool / Trade / Charts)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: tokenIdentity helper T1–T5 / A1 / A4 / A6 / A7" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenIdentity.test.ts'

run_step "frontend: TokenIdentity + PairTokenLinks + explorer safety" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/ui/__tests__/TokenIdentity.test.tsx \
    src/components/ui/__tests__/TokenIdentity.explorerSafety.test.tsx \
    src/components/ui/__tests__/PairTokenLinks.test.tsx \
    src/components/ui/__tests__/AddressRow.explorerSafety.test.tsx \
    src/utils/__tests__/terraExplorer.test.ts'

run_step "frontend: Pool / Trade / Charts #541 page mounts" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx \
    src/pages/TradePage.test.tsx \
    src/pages/ChartsPage.test.tsx \
    -t "541"'

run_step "code: pages use PairTokenLinks (no hand-rolled explorer href)" \
  grep -qE 'PairTokenLinks' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'PairTokenLinks' frontend-dapp/src/pages/TradePage.tsx && \
  grep -qE 'PairTokenLinks' frontend-dapp/src/pages/ChartsPage.tsx && \
  bash -c '! grep -nE "finder\\.terraclassic|coingecko|coinmarketcap|/token/" \
    frontend-dapp/src/pages/PoolPage.tsx \
    frontend-dapp/src/pages/TradePage.tsx \
    frontend-dapp/src/pages/ChartsPage.tsx \
    frontend-dapp/src/components/ui/TokenIdentity.tsx \
    frontend-dapp/src/components/ui/PairTokenLinks.tsx'

run_step "code: identity not inside pair pickers; no factory/router clone" \
  bash -c '! grep -qE "TokenIdentity|PairTokenLinks" \
    frontend-dapp/src/components/trade/PairSearchSelect.tsx \
    frontend-dapp/src/components/trade/TokenSearchSelect.tsx' && \
  bash -c '! grep -qE "wallet-menu-address-row|protocol-factory|FACTORY_CONTRACT_ADDRESS" \
    frontend-dapp/src/components/ui/PairTokenLinks.tsx' && \
  grep -qE 'pool-lp-token-address-row' frontend-dapp/src/pages/PoolPage.tsx && \
  grep -qE 'trade-pair-select-panel' frontend-dapp/src/pages/TradePage.tsx

run_step "code: no /token route" \
  bash -c '! grep -qE "path=[\"'\'']/token" frontend-dapp/src/App.tsx frontend-dapp/src/main.tsx 2>/dev/null; \
    ! grep -qE "path: [\"'\'']/token" frontend-dapp/src/**/*.{ts,tsx} 2>/dev/null; true' && \
  python3 - <<'PY'
from pathlib import Path
hits = []
for p in Path("frontend-dapp/src").rglob("*"):
    if p.suffix not in {".ts", ".tsx"}:
        continue
    text = p.read_text(errors="ignore")
    if 'path: "/token' in text or "path: '/token" in text or 'path="/token' in text:
        hits.append(str(p))
if hits:
    raise SystemExit("unexpected /token route in " + ", ".join(hits))
PY

run_step "docs: frontend.md T541-1–T541-8" \
  grep -qE 'token-identity' docs/frontend.md && \
  grep -qE '\*\*T541-1\*\*' docs/frontend.md && \
  grep -qE '\*\*T541-8\*\*' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_TOKEN_IDENTITY" \
  grep -qE '\*\*T541-1' skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md && \
  grep -qE 'tokenIdentityTarget' skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md && \
  grep -qE 'make verify-issue-541' skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md

run_step "skill: address-row leftover + explorer + invert + copy + trust crosslinks #541" \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_ADDRESS_ROW.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_TERRA_EXPLORER.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_TRADE_PAIR_INVERT.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' skills/AGENTS_FRONTEND_COPY_BUTTON.md

run_step "AGENTS.md playbook link #541" \
  grep -qE 'AGENTS_FRONTEND_TOKEN_IDENTITY|#541' AGENTS.md && \
  grep -qE 'verify-issue-541' AGENTS.md

run_step "Playwright smoke P1–P4 (5 workers, no e2e-tx / no globalSetup seed)" \
  bash -c 'PLAYWRIGHT_SKIP_CHAIN=1 bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke \
    e2e/token-identity-541.spec.ts'

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
