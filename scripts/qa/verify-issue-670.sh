#!/usr/bin/env bash
# Automated verification for GitLab #670 — /token/migrate why-copy (Unlock {X}).
#
# Copy only. No wasm / indexer / e2e-tx. Playwright smoke uses 5 workers.
#
# Refs: skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md **M670-1–M670-8**,
#       frontend-dapp/src/utils/communityTaxMigrateCopy.ts
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
echo "  GitLab #670 — Migrate Token why-copy"
echo "════════════════════════════════════════════════════════════════"

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/MigrateTokenPage.test.tsx \
    src/utils/communityTaxMigrate.test.ts \
    src/utils/communityTaxMigrateCopy.test.ts \
    src/utils/communityTaxSku.test.ts \
    src/pages/CreateTokenPage.test.tsx
}

run_code() {
  grep -q 'migrate-token-why' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  grep -q 'migrate-token-why-examples' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  grep -q 'migrateUnlockFeatureCount' frontend-dapp/src/utils/communityTaxMigrateCopy.ts && \
  grep -q 'MIGRATE_WHY_HEADLINE' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  ! grep -nE -- '-neo' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  ! grep -q 'dangerouslySetInnerHTML' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  ! grep -q 'PayWithAnyToken' frontend-dapp/src/pages/MigrateTokenPage.tsx && \
  ! grep -qE 'features:\s*\[' frontend-dapp/src/utils/communityTaxMigrate.ts
}

run_docs() {
  grep -qE '\*\*M670-1' skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md && \
  grep -qE 'M670-8' skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md && \
  grep -q 'communityTaxMigrateCopy' skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md && \
  grep -qE 'Unlock \{X\}|#670' docs/frontend.md && \
  grep -qE 'M670' docs/frontend.md && \
  grep -qE 'two short why-paragraphs|#670' docs/frontend.md && \
  grep -qE 'Documented exception \(#670' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'communityTaxMigrateCopy' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE '21.7|#670' QA_TEMPLATE.md && \
  grep -qE 'verify-issue-670' docs/testing.md && \
  grep -qE 'M670-1' docs/testing.md && \
  grep -qE 'AGENTS_FRONTEND_TOKEN_MIGRATE|#670' AGENTS.md && \
  grep -qE 'verify-issue-670' AGENTS.md && \
  grep -qE 'M670' skills/AGENTS_FRONTEND_CREATE_TOKEN.md
}

# Copy-only smoke: skip chain globalSetup (.env.local). Playwright config keeps 5 workers.
run_playwright() {
  PLAYWRIGHT_SKIP_CHAIN=1 PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-4173}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/token-create-migrate-copy.spec.ts
}

echo ""
echo "── first pass ──"
run_step "frontend: why copy + SKU count + create Migrate here" run_frontend
run_step "code: testids, derived helper, no invoice/neo/features payload" run_code
run_step "docs: M670 + copy exception + QA 21.7" run_docs
run_step "playwright: migrate why-copy smoke (5 workers, skip chain)" run_playwright

echo ""
echo "── retest ──"
run_step "retest frontend: why copy + SKU count + create Migrate here" run_frontend
run_step "retest code: testids, derived helper, no invoice/neo/features payload" run_code
run_step "retest docs: M670 + copy exception + QA 21.7" run_docs
run_step "retest playwright: migrate why-copy smoke (5 workers, skip chain)" run_playwright

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #670 verification passed"
