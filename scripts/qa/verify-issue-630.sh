#!/usr/bin/env bash
# Automated verification for GitLab #630 — LUNC/USTC (not uluna/uusd) in token pickers.
#
# Proves (unit + docs; no chain required):
#   1. registryProductSymbol + getTokenDisplaySymbol map uluna/uusd → LUNC/USTC.
#   2. useTokenDisplayInfo registry-first vs indexer spoof / down / wrap #507.
#   3. TokenSearchSelect visible text LUNC/USTC; data-testid stays denom.
#   4. Search haystack keeps both ticker and denom; Create Pair still excludes natives.
#   5. #541 copy payload stays uluna/uusd.
#   6. Indexer native_retail_meta + repair SQL; unknown denoms fail closed.
#   7. Docs/skills N630-1–N630-8 crosslinked.
#
# Refs: skills/AGENTS_FRONTEND_NATIVE_TICKERS.md,
#       frontend-dapp/src/hooks/useTokenDisplayInfo.ts,
#       docs/frontend.md § Token search
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
echo "  GitLab #630 — native LUNC / USTC picker labels"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: registry + display + search haystack" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenDisplay.test.ts \
    src/utils/__tests__/tokenRegistry.test.ts \
    src/utils/__tests__/tokenSearchQuery.test.ts \
    src/utils/__tests__/createPairTokenCatalog.test.ts'

run_step "frontend: useTokenDisplayInfo precedence + TokenSearchSelect labels" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/hooks/__tests__/useTokenDisplayInfo.test.tsx \
    src/components/trade/__tests__/TokenSearchSelect.test.tsx \
    src/components/trade/__tests__/TokenSearchSelect.issue630.test.tsx \
    src/components/ui/__tests__/TokenSelect.keyboard.test.tsx'

run_step "frontend: #541 copy payload stays denom" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/tokenIdentity.test.ts \
    src/components/ui/__tests__/TokenIdentity.test.tsx'

run_step "indexer: native retail labels (lib)" \
  bash -c 'cd indexer && cargo test --lib asset_resolver -- --nocapture'

run_step "code: hook uses registryProductSymbol before indexer symbol" \
  grep -qE 'registryProductSymbol' frontend-dapp/src/hooks/useTokenDisplayInfo.ts && \
  grep -qE 'registryProductSymbol' frontend-dapp/src/utils/tokenDisplay.ts && \
  grep -qE 'registryProductSymbol' frontend-dapp/src/utils/tokenRegistry.ts && \
  bash -c '! grep -qE "wrapProductSymbol \|\| indexerMeta" frontend-dapp/src/hooks/useTokenDisplayInfo.ts'

run_step "code: indexer native upsert is not denom/denom for known banks" \
  grep -qE 'native_retail_meta' indexer/src/indexer/asset_resolver.rs && \
  grep -qE 'native_insert_labels' indexer/src/indexer/asset_resolver.rs && \
  grep -qE 'repair_native_bank_tickers|GitLab #630' indexer/migrations/20260825140000_repair_native_bank_tickers.sql && \
  grep -qE "lower\(symbol\) = 'uluna'" indexer/migrations/20260825140000_repair_native_bank_tickers.sql && \
  bash -c '! grep -nE "upsert_asset\(pool, None, Some\(denom\), false, denom, denom" \
    indexer/src/indexer/asset_resolver.rs'

run_step "docs: frontend.md N630-1–N630-8" \
  grep -qE 'native-lunc-ustc-labels|#630' docs/frontend.md && \
  grep -qE '\*\*N630-1\*\*' docs/frontend.md && \
  grep -qE '\*\*N630-8\*\*' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_NATIVE_TICKERS" \
  grep -qE '\*\*N630-1' skills/AGENTS_FRONTEND_NATIVE_TICKERS.md && \
  grep -qE 'registryProductSymbol' skills/AGENTS_FRONTEND_NATIVE_TICKERS.md && \
  grep -qE 'make verify-issue-630' skills/AGENTS_FRONTEND_NATIVE_TICKERS.md

run_step "skill: token search + identity + wrap + create-pair + copy crosslinks #630" \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' skills/AGENTS_FRONTEND_TOKEN_SEARCH.md && \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' skills/AGENTS_FRONTEND_TOKEN_IDENTITY.md && \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' skills/AGENTS_MAINNET_WRAP_ENABLEMENT.md && \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' skills/AGENTS_FRONTEND_CREATE_PAIR_PICKER.md && \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md

run_step "AGENTS.md playbook link #630" \
  grep -qE 'AGENTS_FRONTEND_NATIVE_TICKERS|#630' AGENTS.md && \
  grep -qE 'verify-issue-630' AGENTS.md

run_step "wrap QA template + E11 name LUNC not uluna" \
  grep -qE 'never `uluna`|never uluna|#630' docs/qa-templates/wrap-unwrap-test-pass.md && \
  grep -qE 'LUNC and cLUNC|#630' NATIVE_TOKEN_WRAPPING.md

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
