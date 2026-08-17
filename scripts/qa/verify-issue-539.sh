#!/usr/bin/env bash
# Automated verification for GitLab #539 — LocalTerra wrap-mapper split-fee
# instantiate + #533 e2e-tx P4–P8 recording.
#
# Proves (script + docs; Playwright P4–P8 when LocalTerra is up):
#   1. deploy-dex-local.sh instantiates wrap-mapper with fee_wrap_bps / fee_unwrap_bps.
#   2. Dual-read: legacy fee_bps fallback remains for older wasm.
#   3. #533 tx spec P4–P8 exists; verify-issue-533 runs it when the chain is up.
#
# Optional chain: make has-localterra + frontend-dapp/.env.local → Playwright
#   e2e/pool-one-sided-533-tx.spec.ts (e2e-tx, 1 worker). Skip with
#   VERIFY_ISSUE_539_SKIP_E2E=1.
#
# Refs: skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md,
#       skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md,
#       scripts/deploy-dex-local.sh
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
echo "  GitLab #539 — LocalTerra wrap-mapper split-fee instantiate"
echo "════════════════════════════════════════════════════════════════"

run_step "deploy: wrap-mapper instantiate prefers fee_wrap_bps / fee_unwrap_bps" \
  bash -c 'grep -qE "WRAP_MAPPER_INIT_SPLIT=.*fee_wrap_bps.*fee_unwrap_bps" scripts/deploy-dex-local.sh &&
    grep -qE "instantiate_wrap_mapper" scripts/deploy-dex-local.sh &&
    grep -qE "fee_wrap_bps/fee_unwrap_bps" scripts/deploy-dex-local.sh'

run_step "deploy: legacy fee_bps fallback for older wrap_mapper.wasm" \
  bash -c 'grep -qE "WRAP_MAPPER_INIT_LEGACY=.*fee_bps" scripts/deploy-dex-local.sh &&
    grep -qE "legacy fee_bps" scripts/deploy-dex-local.sh &&
    grep -qE "Instantiated with split fees \(GitLab #539\)" scripts/deploy-dex-local.sh'

run_step "deploy: wrap-mapper init no longer sends fee_bps as the only msg" \
  bash -c '! grep -nE "WRAP_MAPPER_INIT_MSG=.*fee_bps" scripts/deploy-dex-local.sh'

run_step "shell: deploy-dex-local.sh syntax" \
  bash -n scripts/deploy-dex-local.sh

run_step "deploy: rebuild wrap_mapper.wasm when missing (not only treasury.wasm)" \
  grep -qE 'treasury.wasm" \] \|\| \[ ! -f "\$ARTIFACTS_DIR/wrap_mapper.wasm"' scripts/deploy-dex-local.sh

run_step "e2e: pool-one-sided-533-tx.spec.ts P4–P8 present" \
  bash -c 'test -f frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    grep -qE "P4 one-sided add" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    grep -qE "P5 one-sided add with native LUNC" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    grep -qE "P6 / P7 withdraw" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts &&
    grep -qE "P8 empty pool" frontend-dapp/e2e/pool-one-sided-533-tx.spec.ts'

run_step "verify-issue-533: runs P4–P8 tx spec when LocalTerra is up" \
  bash -c 'grep -qE "pool-one-sided-533-tx.spec.ts" scripts/qa/verify-issue-533.sh &&
    grep -qE "P4–P8" scripts/qa/verify-issue-533.sh'

run_step "skill + Makefile: #539 LocalTerra wrap instantiate + verify-issue-539" \
  bash -c 'grep -qE "#539" skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md &&
    grep -qE "make verify-issue-539" skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md &&
    grep -qE "#539" skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md &&
    grep -qE "verify-issue-539" Makefile &&
    grep -qE "verify-issue-539" AGENTS.md'

if [[ "${VERIFY_ISSUE_539_SKIP_E2E:-}" == "1" ]]; then
  echo ""
  echo "[playwright P4–P8] skipped (VERIFY_ISSUE_539_SKIP_E2E=1)"
elif make has-localterra >/dev/null 2>&1 && [ -f frontend-dapp/.env.local ]; then
  run_step "playwright: P4–P8 one-sided tx (e2e-tx, 1 worker)" \
    bash -c 'CI=1 bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test e2e/pool-one-sided-533-tx.spec.ts --project=e2e-tx'
else
  echo ""
  echo "[playwright P4–P8] skipped — LocalTerra or frontend-dapp/.env.local not ready"
  echo "  Provision: make setup-cloud-localterra"
  echo "  Probe: make has-localterra"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for line in "${RESULTS[@]}"; do
  echo "  $line"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
echo "==> GitLab #539 verification passed"
