#!/usr/bin/env bash
# Verification for GitLab #585 — F6 code-id freeze on dApp + indexer route/solve.
#
# Layers (no LocalTerra / wasm deploy):
#   1. Indexer lib: freeze eval + LCD code_id parse
#   2. Indexer integration: pair code_id_frozen + route/solve exclude
#   3. Frontend: helpers, probe, humanize, Swap/Trade/Pool/Charts/Limits
#   4. Docs/skills F585 + #585 crosslinks
#
# Refs: skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md,
#       docs/frontend.md § code-id-freeze-gitlab-585,
#       indexer/src/indexer/asset_code_id_freeze.rs
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #585 — F6 code-id freeze (dApp + route/solve)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_frontend_unit() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/utils/__tests__/assetCodeIdFreeze.test.ts \
    src/services/terraclassic/__tests__/assetCodeIdFreeze.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/services/terraclassic/__tests__/pair.test.ts \
    src/services/terraclassic/__tests__/factory.test.ts \
    src/pages/SwapPage.test.tsx \
    src/pages/TradePage.test.tsx \
    src/pages/PoolPage.test.tsx \
    src/pages/ChartsPage.test.tsx \
    src/pages/LimitOrdersPage.test.tsx
}

run_docs() {
  set -euo pipefail
  rg -q 'F585-1' skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md
  rg -q 'F585-8' skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md
  rg -q 'make verify-issue-585' skills/AGENTS_FRONTEND_CODE_ID_FREEZE.md
  rg -q 'AGENTS_FRONTEND_CODE_ID_FREEZE' AGENTS.md
  rg -q 'verify-issue-585' AGENTS.md
  rg -q '#585' skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q 'code_id_frozen' docs/indexer-invariants.md
  rg -q 'code-id-freeze-gitlab-585' docs/frontend.md
  rg -q 'F585-1' docs/frontend.md
  rg -q '## Code-id freeze' docs/user-incident-faq.md
  rg -q 'verify-issue-585' docs/testing.md
  rg -q '#585' docs/contracts-terraclassic.md
  rg -q '#585' docs/contracts-security-audit.md
  rg -q '#585' docs/security-model.md
  rg -q 'route/solve excludes' docs/runbooks/cw20-code-id-ops.md
  rg -q 'code_id_frozen' docs/integrators.md
  rg -q 'frozen hops' docs/route-solver.md
  rg -q 'CODE_ID_FROZEN_CTA' frontend-dapp/src/utils/assetCodeIdFreeze.ts
  rg -q 'is_pair_code_id_frozen' indexer/src/api/route_solver.rs
  rg -q 'is_pair_code_id_frozen' indexer/src/api/route_paths.rs
  rg -q 'code_id_frozen' indexer/src/api/pairs.rs
}

echo ""
echo "── first pass ──"
run_step "indexer lib: asset_code_id_freeze + lcd code_id parse" \
  bash -c 'cd indexer && cargo test --lib asset_code_id_freeze -- --quiet && cargo test --lib get_contract_code_id_parses -- --quiet'

run_step "indexer integration: pair flag + route/solve exclude" \
  bash -c 'cd indexer && cargo test --test api_pairs pair_api_flags_code_id_frozen -- --test-threads=1 --quiet && cargo test --test api_route_solve route_solve_excludes_code_id_frozen_pair -- --test-threads=1 --quiet'

run_step "frontend unit: freeze helpers + Swap/Trade/Pool/Charts/Limits" \
  run_frontend_unit

run_step "docs: F585 + skill + crosslinks" \
  run_docs

echo ""
echo "── retest ──"
run_step "retest indexer lib" \
  bash -c 'cd indexer && cargo test --lib asset_code_id_freeze -- --quiet && cargo test --lib get_contract_code_id_parses -- --quiet'

run_step "retest indexer integration" \
  bash -c 'cd indexer && cargo test --test api_pairs pair_api_flags_code_id_frozen -- --test-threads=1 --quiet && cargo test --test api_route_solve route_solve_excludes_code_id_frozen_pair -- --test-threads=1 --quiet'

run_step "retest frontend unit" \
  run_frontend_unit

run_step "retest docs F585" \
  run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #585 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
