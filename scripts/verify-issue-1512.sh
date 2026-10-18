#!/usr/bin/env bash
# #1512: focused display contract + actual page regressions; each test run <60s.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
timeout 59s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/utils/__tests__/pairPriceUsd.test.ts \
  src/utils/__tests__/pairPriceUsd1512.test.ts \
  src/components/charts/__tests__/chartHeadlinePrice.test.ts \
  src/components/charts/__tests__/PriceChart.test.tsx
timeout 59s bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/pages/TradePage.test.tsx src/pages/ChartsPage.test.tsx -t '#1512'
