#!/usr/bin/env bash
# Automated verification for GitLab #661 — /pool Manage provide name/symbol + wrap default on.
#
# Proves (unit + docs; no chain required):
#   1. formatPoolAssetFieldLabel / usablePoolAssetName / poolProvideAmountAriaLabel.
#   2. provideWrapDefaultOn only when getNativeEquivalent is set.
#   3. useTokenDisplayInfo registry name vs HTML indexer spoof.
#   4. PoolPage: no Asset A/B; wrap checkbox default on for cLUNC; uncheck flips label.
#   5. PoolAdvancedManage.tsx has no Asset A / Asset B strings.
#   6. Docs/skills P661-1–P661-12 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md,
#       frontend-dapp/src/components/pool/PoolAdvancedManage.tsx,
#       docs/frontend.md § Pool page — provide liquidity
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
echo "  GitLab #661 — Manage provide labels + wrap default on"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: label helper + wrap default + display name" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenDisplay.test.ts \
    src/utils/__tests__/poolProvideWrapDefault.test.ts \
    src/hooks/__tests__/useTokenDisplayInfo.test.tsx'

run_step "frontend: PoolPage labels + wrap default + #480 counterpart" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/PoolPage.test.tsx'

run_step "code: PoolAdvancedManage has no Asset A/B strings" \
  bash -c '! grep -qE "Asset A|Asset B|Asset A amount|Asset B amount" \
    frontend-dapp/src/components/pool/PoolAdvancedManage.tsx'

run_step "code: wrap default helper + selected-input labels" \
  grep -qE 'provideWrapDefaultOn' frontend-dapp/src/utils/poolProvideWrapDefault.ts && \
  grep -qE 'provideWrapDefaultOn' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  grep -qE 'formatPoolAssetFieldLabel' frontend-dapp/src/utils/tokenDisplay.ts && \
  grep -qE 'formatPoolAssetFieldLabel' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  grep -qE 'poolProvideAmountAriaLabel' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  grep -qE 'normal-case' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  grep -qE 'pool-provide-auto-wrap-a' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  bash -c '! grep -qE "dangerouslySetInnerHTML" frontend-dapp/src/components/pool/PoolAdvancedManage.tsx'

run_step "code: wrap default is useState init, not useEffect" \
  grep -qE 'useState\(\(\) => provideWrapDefaultOn' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx && \
  bash -c '! grep -nE "setUseNativeA\(true\)|setUseNativeB\(true\)" \
    frontend-dapp/src/components/pool/PoolAdvancedManage.tsx'

run_step "docs: frontend.md P661-1–P661-12" \
  grep -qE 'P661-1' docs/frontend.md && \
  grep -qE 'P661-12' docs/frontend.md && \
  grep -qE '#661' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_POOL_PROVIDE_LABELS" \
  grep -qE '\*\*P661-1' skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md && \
  grep -qE '\*\*P661-12' skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md && \
  grep -qE 'make verify-issue-661' skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md && \
  grep -qE 'formatPoolAssetFieldLabel' skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md

run_step "skill: preview + one-sided + native tickers + copy crosslinks #661" \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' skills/AGENTS_FRONTEND_POOL_PROVIDE_WITHDRAW_PREVIEW.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' skills/AGENTS_FRONTEND_POOL_ONE_SIDED.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' skills/AGENTS_FRONTEND_NATIVE_TICKERS.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' skills/AGENTS_FRONTEND_POOL_TABLE.md

run_step "AGENTS.md playbook link #661" \
  grep -qE 'AGENTS_FRONTEND_POOL_PROVIDE_LABELS|#661' AGENTS.md && \
  grep -qE 'verify-issue-661' AGENTS.md

run_step "design-system + wrap QA: wrap defaults on" \
  grep -qE 'Defaults on|#661' docs/design-system.md && \
  grep -qE 'checked on first paint|#661' docs/qa-templates/wrap-unwrap-test-pass.md

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
