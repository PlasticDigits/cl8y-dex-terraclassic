#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/pages/TradePage.test.tsx \
  src/utils/__tests__/tradePairRoute.test.ts \
  src/utils/__tests__/tradeQueryResolve.test.ts \
  src/utils/__tests__/pairCatalogRank.test.ts \
  src/utils/__tests__/pairCatalogRank.issue562.test.ts \
  src/components/common/__tests__/AppShellNavLink.test.tsx \
  src/components/common/__tests__/LazyRoute.test.tsx \
  src/utils/__tests__/chunkLoadError.test.ts
bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/tsc -b --pretty false
bash scripts/with-node.sh -- node --check scripts/qa/browser-trade-entry-1497.cjs
git diff --check
printf '%s\n' 'PASS: Trade entry source checks. Deployed browser/wallet/outage scopes require separate receipts.'
