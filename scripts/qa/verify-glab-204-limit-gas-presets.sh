#!/usr/bin/env bash
# GitLab #204 — verify placement gas presets wire max_adjust_steps in CW20 batch hook on LocalTerra.
# Requires: localterra up, frontend-dapp/.env.local, dev wallet provisioned (e2e-provision-dev-wallet.sh).
set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$REPO_ROOT"

echo "==> Provision dev wallet CW20 balances (idempotent)"
bash scripts/e2e-provision-dev-wallet.sh

echo "==> Playwright: limit-orders-gas-presets-tx.spec.ts (5 workers)"
bash scripts/with-node.sh --cwd frontend-dapp -- \
  npx playwright test e2e/limit-orders-gas-presets-tx.spec.ts --project=e2e-tx

echo "==> GitLab #204 preset hook verification passed"
