#!/usr/bin/env bash
# Automated verification for GitLab #516 — wrap-mapper fee_wrap_bps / fee_unwrap_bps.
#
# Proves (unit + docs; no chain required):
#   1. Config parse: split fees; transitional fee_bps; fail closed on partial.
#   2. Wrap quote uses wrap fee only (10_000 @ 200 → 9_800).
#   3. Unwrap quote uses unwrap fee then tax (10_000 @ 51 + 1.5% → 9_800).
#   4. Fee notes stay honest; exchange-deposit warning retained.
#   5. Skills/docs/ops/health understand split fees + retune; no gross-up as 2% fix.
#   6. Quote helpers do not hardcode 200/51.
#
# Refs: skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md
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
echo "  GitLab #516 — wrap-mapper split fees (W12–W15)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: wrapMapper + router + pool + WrapPage + SwapPage dual-fee tests" \
  bash -c '
    ROOT="'"$REPO_ROOT"'"
    SIBLING="$(dirname "$ROOT")/cl8y-dex-terraclassic/frontend-dapp/node_modules"
    if [[ ! -x "$ROOT/frontend-dapp/node_modules/.bin/vitest" ]]; then
      if [[ -x "$SIBLING/.bin/vitest" ]]; then
        ln -sfn "$SIBLING" "$ROOT/frontend-dapp/node_modules"
      else
        bash "$ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm ci --silent
      fi
    fi
    bash "$ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm test -- --run \
      src/services/terraclassic/__tests__/wrapMapper.test.ts \
      src/services/terraclassic/router.test.ts \
      src/utils/__tests__/poolProvideCounterpart.test.ts \
      src/pages/WrapPage.test.tsx \
      src/pages/SwapPage.test.tsx
  '

run_step "skill: AGENTS_WRAP_MAPPER_SPLIT_FEES W12–W15 + retune" \
  grep -qE '\*\*W12\*\*' skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md && \
  grep -qE '\*\*W15\*\*' skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md && \
  grep -qE 'fee_unwrap_bps = round' skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md

run_step "skill: #512 playbook points at #516 not gross-up" \
  grep -qE 'AGENTS_WRAP_MAPPER_SPLIT_FEES' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md && \
  ! grep -qE 'On-chain \*\*gross-up\*\* InstantWithdraw' skills/AGENTS_WRAP_UNWRAP_BURN_TAX.md

run_step "skill: enablement W4 split fees" \
  grep -qE 'fee_wrap_bps' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md && \
  grep -qE 'AGENTS_WRAP_MAPPER_SPLIT_FEES' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md

run_step "docs: NATIVE_TOKEN_WRAPPING #516 + retune" \
  grep -qE 'fee_unwrap_bps' NATIVE_TOKEN_WRAPPING.md && \
  grep -qE '9800 / \(1' NATIVE_TOKEN_WRAPPING.md

run_step "docs: QA wrap-unwrap #516 9800 not 9653 post-migrate" \
  grep -qE '#516' docs/qa-templates/wrap-unwrap-test-pass.md && \
  grep -qE '9 800' docs/qa-templates/wrap-unwrap-test-pass.md

run_step "ops: REGISTRY + runbook split fees + retune" \
  grep -qE 'fee_unwrap_bps' deployments/mainnet-ust1-wrap/REGISTRY.md && \
  grep -qE 'fee_unwrap_bps' docs/runbooks/ust1-wrap-production-ops.md && \
  grep -qE 'round\(10000' docs/runbooks/ust1-wrap-production-ops.md

run_step "ops: health script understands split fees" \
  grep -qE 'fee_wrap_bps' scripts/check-ust1-wrap-ops-health.sh && \
  grep -qE 'fee_unwrap_bps' scripts/check-ust1-wrap-ops-health.sh && \
  grep -qE 'ustr-cmm#9 migrate pending' scripts/check-ust1-wrap-ops-health.sh

run_step "AGENTS.md playbook link #516" \
  grep -qE 'AGENTS_WRAP_MAPPER_SPLIT_FEES|#516' AGENTS.md

run_step "code: queryWrapMapperFeeBps requires kind" \
  grep -qE 'export async function queryWrapMapperFeeBps\(kind: WrapMapperFeeKind\)' \
    frontend-dapp/src/services/terraclassic/wrapMapper.ts

run_step "code: netCw20AfterNativeWrap uses wrap fee" \
  grep -qE "queryWrapMapperFeeBps\('wrap'\)" \
    frontend-dapp/src/services/terraclassic/router.ts

run_step "code: netNativeAfterUnwrap uses unwrap fee" \
  grep -qE "queryWrapMapperFeeBps\('unwrap'\)" \
    frontend-dapp/src/services/terraclassic/router.ts

run_step "code: quote helpers do not hardcode 200/51 assignments" \
  bash -c '! grep -nE "(fee_wrap_bps|fee_unwrap_bps|feeBps)\s*[:=]\s*(51|200)\b" \
    frontend-dapp/src/services/terraclassic/wrapMapper.ts \
    frontend-dapp/src/services/terraclassic/router.ts'

run_step "code: cache TTL documented as 30s" \
  grep -qE 'WRAP_MAPPER_CONFIG_CACHE_MS = 30_000' \
    frontend-dapp/src/services/terraclassic/wrapMapper.ts

run_step "code: ust1-window fee_bps left unchanged" \
  grep -qE 'fee_bps: number' frontend-dapp/src/services/terraclassic/ust1Window.ts

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
exit 0
