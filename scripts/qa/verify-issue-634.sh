#!/usr/bin/env bash
# Automated verification for GitLab #634 — migrate pair inventory +
# post-adopt CL8Y register tool (no Refresh / no register-external).
#
# Refs: skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md (M634)
#
# LocalTerra (when chain is up, or VERIFY634_REQUIRE_CHAIN=1): adopt a
# factory-listed mintable that already has a CL8Y pair → F6 freeze →
# governance Refresh → register only that factory pair. Columbus-5
# Open/ALPHA LCD walkthrough is leftover #636 — this script never
# queries columbus-5 except the documented Terraport factory addr, which
# LocalTerra must not invent pairs for.
# VERIFY634_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra is missing.
# VERIFY634_SKIP_CHAIN=1 — skip LocalTerra even if the chain is up.
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
echo "  GitLab #634 — migrate pair inventory + CL8Y register tool"
echo "════════════════════════════════════════════════════════════════"

run_frontend() {
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
  fi
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxMigratePairs.test.ts \
    src/components/community/MigratePairInventory.test.tsx \
    src/pages/MigrateTokenPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q "M634-1" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "M634-8" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "verify-issue-634" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "verify-issue-634" AGENTS.md
  rg -q "verify-issue-634" docs/testing.md
  rg -q "M634" docs/frontend.md
  rg -q "M634" docs/contracts-terraclassic.md
  rg -q "M634" docs/contracts-security-audit.md
  rg -q "#634" docs/runbooks/cw20-code-id-ops.md
  rg -q "#634" skills/AGENTS_COMMUNITY_TAX_AUTOREGISTER.md
  rg -q "loadMigratePairInventory" frontend-dapp/src/utils/communityTaxMigratePairs.ts
  rg -q "migrate-venue-inventory" frontend-dapp/src/components/community/MigratePairInventory.tsx
  rg -q "terra12u7khzrzn05a73xkpq6a5zrcazz2xmqn7lvupmqmca06pgcyt5qsa9e7p6" frontend-dapp/src/utils/communityTaxMigratePairs.ts
  rg -q "This CL8Y market pauses until CL8Y governance refreshes" frontend-dapp/src/utils/communityTaxMigratePairs.ts
  ! rg -n "refresh_pair_asset_code_ids|set_pair_paused|add_whitelisted_code_id" \
    frontend-dapp/src/pages/MigrateTokenPage.tsx \
    frontend-dapp/src/components/community/MigratePairInventory.tsx \
    frontend-dapp/src/utils/communityTaxMigratePairs.ts
  rg -q "localterra-634-migrate-inventory" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "localterra-634-migrate-inventory" docs/testing.md
  bash -n scripts/qa/localterra-634-migrate-inventory.sh
}

run_live() {
  set -euo pipefail
  ./scripts/qa/localterra-634-migrate-inventory.sh
}

echo ""
echo "── first pass ──"
run_step "frontend: inventory helper + card + migrate page" run_frontend
run_step "docs: M634 + no Refresh execute on migrate" run_docs

if [[ "${VERIFY634_SKIP_CHAIN:-}" == "1" ]]; then
  echo "  [SKIP] LocalTerra (VERIFY634_SKIP_CHAIN=1)"
  RESULTS+=("SKIP  LocalTerra live (VERIFY634_SKIP_CHAIN=1)")
elif timeout 20 make -s has-localterra >/dev/null 2>&1 \
  && grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' frontend-dapp/.env.local 2>/dev/null; then
  run_step "LocalTerra: mintable adopt + Refresh + factory-only register" run_live
elif [[ "${VERIFY634_REQUIRE_CHAIN:-}" == "1" ]]; then
  bad "LocalTerra required (VERIFY634_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
else
  echo "  [SKIP] LocalTerra (make has-localterra + tax pins). Cloud Agent: make setup-cloud-localterra"
  RESULTS+=("SKIP  LocalTerra live (chain/pins down)")
fi

echo ""
echo "── retest ──"
run_step "retest frontend: inventory helper + card + migrate page" run_frontend
run_step "retest docs: M634 + no Refresh execute on migrate" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #634 verification passed"
