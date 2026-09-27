#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

# Pin token identities so an ignored LocalTerra .env.local cannot rewrite fixtures.
VITE_CL8Y_TOKEN_ADDRESS=terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3 \
VITE_UST1_TOKEN_ADDRESS=terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72 \
VITE_VFDUSD_TOKEN_ADDRESS=terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3 \
VITE_LUNC_C_TOKEN_ADDRESS=terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg \
VITE_USTC_C_TOKEN_ADDRESS=terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch \
  bash scripts/with-node.sh --cwd frontend-dapp -- ./node_modules/.bin/vitest run \
  src/services/terraclassic/__tests__/swapAutoGas.test.ts \
  src/services/terraclassic/__tests__/terraSimulateTx.test.ts \
  src/services/terraclassic/__tests__/terraBroadcast.autoGas.test.ts \
  src/services/terraclassic/__tests__/swapAutoGasFeeHint.test.tsx \
  src/services/terraclassic/__tests__/terraGas.issue1328.test.ts \
  src/utils/__tests__/humanizeTerraTxError.test.ts \
  src/utils/swapPayAcquireGuidance.test.ts

if [[ -n "${VERIFY1360_SIMULATE_ADDRESS:-}" ]]; then
  bash scripts/with-node.sh --cwd frontend-dapp -- node scripts/measureIssue1360Columbus.mjs
else
  echo "SKIP Columbus-5 CL8Y → KENA simulation (set VERIFY1360_SIMULATE_ADDRESS to a public account that can simulate a CL8Y send)"
fi

echo "NOTE: a signed Keplr swap (gas_used < gas_wanted, Network fee matches signed gas) remains a later wallet check."
