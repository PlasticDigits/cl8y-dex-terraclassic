#!/usr/bin/env bash
# Existing supported route presentation and campaign recovery only.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
timeout 55s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/components/swap/SwapQuoteSummary.test.tsx src/utils/__tests__/swapQueryParams.test.ts
timeout 55s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/pages/SwapPage.queryParams.test.tsx -t 'campaign parameters'
printf '%s\n' 'PASS: supported-route presentation and campaign retry; external adapters and live evidence remain open.'
