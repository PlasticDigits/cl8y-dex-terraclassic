#!/usr/bin/env bash
# Automated verification for GitLab #669 — Create Token desktop density.
#
# Proves (unit + docs; Playwright when LocalTerra is up):
#   1. Configured page is w-full (not max-w-[520px]); md:grid desktop contract.
#   2. Phone DOM order; SKU panels toggle; query payee/manager/treasury ignored.
#   3. Combined 20+20 still errors; helpers + free/pay paths untouched.
#   4. Docs/skills C669-1–C669-8 + QA desktop/phone row.
#   5. Optional: e2e/create-token-602.spec.ts (5 workers, no e2e-tx).
#
# Refs: skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md,
#       frontend-dapp/src/pages/CreateTokenPage.tsx,
#       docs/frontend.md § Create Token
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
echo "  GitLab #669 — Create Token desktop density"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: CreateTokenPage layout + SKU / identity contract" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/CreateTokenPage.test.tsx \
    src/utils/communityTaxCreateForm.test.ts'

run_step "code: desktop grid + no 520 chimney on configured page; no nested chrome" \
  grep -q 'create-token-desktop-grid' frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q 'CREATE_TOKEN_PAGE_CLASS' frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q 'md:grid-cols-2' frontend-dapp/src/utils/createTokenLayout.ts && \
  grep -q "CREATE_TOKEN_PAGE_CLASS = 'w-full'" frontend-dapp/src/utils/createTokenLayout.ts && \
  bash -c '! grep -nE "max-w-\[520px\]" frontend-dapp/src/pages/CreateTokenPage.tsx' && \
  grep -q 'CREATE_TOKEN_UNAVAILABLE_CLASS' frontend-dapp/src/pages/CreateTokenPage.tsx && \
  bash -c '! grep -nE -- "-neo" frontend-dapp/src/pages/CreateTokenPage.tsx' && \
  bash -c '! grep -qE "dangerouslySetInnerHTML" frontend-dapp/src/pages/CreateTokenPage.tsx' && \
  bash -c '! grep -qE "card-glass" frontend-dapp/src/pages/CreateTokenPage.tsx'

run_step "code: layout-only (PayWithAnyToken + identity parsers still wired)" \
  grep -q "PayWithAnyToken" frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q "buildValidatedCreateArgs" frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q "walletOwnershipHelper" frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q "create-token-ack" frontend-dapp/src/pages/CreateTokenPage.tsx && \
  grep -q "create-token-free-cta" frontend-dapp/src/pages/CreateTokenPage.tsx && \
  bash -c '! grep -qE "executeMultiHopSwap|quoteCw20ViaRouteSolve" frontend-dapp/src/pages/CreateTokenPage.tsx'

run_step "docs: frontend.md C669-1–C669-8 + QA Create Token row" \
  grep -qE '\*\*C669-1\*\*' docs/frontend.md && \
  grep -qE '\*\*C669-8\*\*' docs/frontend.md && \
  grep -qE 'create-token-desktop|#669' docs/frontend.md && \
  grep -qE '11.1.10' QA_TEMPLATE.md && \
  grep -qE '#669' QA_TEMPLATE.md

run_step "skill: AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT" \
  grep -qE '\*\*C669-1' skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md && \
  grep -qE 'make verify-issue-669' skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md && \
  grep -qE 'create-token-desktop-grid' skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md

run_step "skill: create-token + chrome + design + copy crosslinks" \
  grep -qE 'AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT|#669' skills/AGENTS_FRONTEND_CREATE_TOKEN.md && \
  grep -qE 'AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT|#669' skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE 'AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT|#669' skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE 'AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT|#669' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md

run_step "AGENTS.md playbook link #669" \
  grep -qE 'AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT|#669' AGENTS.md && \
  grep -qE 'verify-issue-669' AGENTS.md

run_step "docs: testing.md verify-issue-669 row" \
  grep -qE 'verify-issue-669' docs/testing.md && \
  grep -qE 'C669-1' docs/testing.md

run_step "chrome nesting static (no new card-glass nest)" \
  python3 scripts/check_chrome_nesting.py

if make -s has-localterra >/dev/null 2>&1; then
  run_step "playwright: create-token desktop + phone (5 workers)" \
    bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/create-token-602.spec.ts'
else
  echo ""
  echo "[playwright: create-token desktop + phone (5 workers)]"
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
echo "==> GitLab #669 verification passed"
