#!/usr/bin/env bash
# Frontend-only scroll invariants: deterministic transport; no chain or broadcast.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
  src/components/ui/__tests__/portalListboxPosition.test.ts \
  src/lib/__tests__/portalListboxViewport.test.ts \
  src/components/charts/__tests__/priceChartLightweightSeriesSync.test.ts \
  src/components/charts/__tests__/PriceChartLightweightCanvas.test.tsx

export PLAYWRIGHT_SKIP_CHAIN=1
export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-5136}"
# A dedicated server ensures evidence comes from this checkout. All market/LCD
# requests in these specs are intercepted, including the synthetic factory.
unset PLAYWRIGHT_BASE_URL
export VITE_FACTORY_ADDRESS=terra1scrollfactory00000000000000000000000000
export VITE_TERRA_LCD_URL=http://127.0.0.1:1317
export VITE_INDEXER_URL=http://127.0.0.1:3001
bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/playwright test \
  --project=e2e-smoke --workers=5 --reporter=line \
  e2e/portal-scroll-invariants.spec.ts \
  e2e/layout-scroll-invariants.spec.ts \
  e2e/chart-scroll-refresh.spec.ts
