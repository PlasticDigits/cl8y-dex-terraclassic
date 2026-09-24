#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

run_step() {
  local label="$1"
  shift
  printf '\n== %s ==\n' "$label"
  "$@"
}

run_step "Frontend gas, credit, error, URL, and docs regressions" \
  env VITE_CL8Y_TOKEN_ADDRESS=terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3 \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
    src/services/terraclassic/__tests__/transactions.test.ts \
    src/services/terraclassic/__tests__/terraGas.retailShapes.test.ts \
    src/utils/__tests__/limitOrderNativeGasBalanceGate.test.ts \
    src/utils/__tests__/humanizeTerraTxError.test.ts \
    src/utils/__tests__/swapQueryParams.test.ts \
    src/utils/__tests__/limitOrderBatchGasSummary.test.ts \
    src/utils/__tests__/maxSpendableAmount.test.ts

run_step "Contract rollback and existing limit-order safeguards" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests hybrid_book_fill_reverts_when_pool_cannot_take_remainder -- --test-threads=1 && cargo test -p cl8y-dex-tests hybrid_split_mismatch_rejected -- --test-threads=1 && cargo test -p cl8y-dex-tests limit_batch_partial_success_skips_book_walk_failures -- --test-threads=1 && cargo test -p cl8y-dex-tests cancel_limit_order_refunds_escrow -- --test-threads=1'

run_step "Documentation cross-links" \
  bash -c 'rg -q "verify-issue-1329" AGENTS.md docs/testing.md Makefile skills/AGENTS_HYBRID_LIMIT_GAS.md && rg -q "max_adjust_steps" docs/limit-orders.md docs/frontend.md skills/AGENTS_HYBRID_LIMIT_GAS.md && rg -q "gas.*credit|credit.*gas" frontend-dapp/src/utils/__tests__/swapQueryParams.test.ts'

if [[ "${VERIFY_ISSUE_1329_CHAIN:-0}" == "1" ]]; then
  [[ -f frontend-dapp/.env.local ]] || { echo "Missing frontend-dapp/.env.local; run make setup-cloud-localterra first." >&2; exit 1; }
  [[ -f frontend-dapp/.env.development ]] || { echo "Missing frontend-dapp/.env.development with the local simulated wallet; provision LocalTerra first." >&2; exit 1; }
  [[ -f indexer/.env ]] || { echo "Missing indexer/.env; run make setup-cloud-localterra first." >&2; exit 1; }
  make has-localterra

  if ! curl -fsS --max-time 3 http://127.0.0.1:3001/api/v1/overview >/dev/null; then
    if [[ ! -x indexer/target/release/cl8y-dex-indexer ]]; then
      run_step "Build indexer on host for LocalTerra E2E" bash -c 'cd indexer && cargo build --release'
    fi
    PLAYWRIGHT_WEB_PORT=3173 bash scripts/e2e-start-indexer.sh
  fi

  run_step "LocalTerra limit place/cancel + hybrid fill gas_used checks" \
    env PLAYWRIGHT_WEB_PORT=3173 bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-tx \
      --grep 'hybrid swap emits wasm limit_order_fill|place limit shows success with tx hash|cancel limit via my open limits panel' \
      e2e/limit-orders-tx.spec.ts e2e/hybrid-swap.spec.ts
fi

echo "OK: verify-issue-1329"
