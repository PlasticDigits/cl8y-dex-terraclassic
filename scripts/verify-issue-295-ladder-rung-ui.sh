#!/usr/bin/env bash
# Browser QA for GitLab #295 — limit ladder rung count input (Playwright, no on-chain tx).
# Requires: frontend on :5173, .env.local from deploy-local.
#
# Usage:
#   ./scripts/verify-issue-295-ladder-rung-ui.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "[verify-295] ERROR: missing $ENV_LOCAL — run ./scripts/setup-cloud-agent-localterra.sh" >&2
  exit 1
fi

if ! curl -sf "http://127.0.0.1:5173/" >/dev/null 2>&1; then
  echo "[verify-295] ERROR: frontend not on http://127.0.0.1:5173 — run: make dev" >&2
  exit 1
fi

export PATH="$HOME/.nvm/versions/node/$(tr -d '[:space:]' <"$REPO_ROOT/.nvmrc")/bin:${PATH}"

export PLAYWRIGHT_BASE_URL="${PLAYWRIGHT_BASE_URL:-http://127.0.0.1:5173}"

# Cloud Agent VMs may not have browsers cached yet (~200MB download first run).
if [[ ! -d "$HOME/.cache/ms-playwright/chromium-"* ]] 2>/dev/null; then
  echo "[verify-295] installing Playwright Chromium (one-time; may take several minutes)…"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npx playwright install chromium
fi

exec bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
  npx playwright test e2e/verify-issue-295-ladder-rung-ui.spec.ts \
  --config=playwright.local-dev.config.ts \
  --project=e2e-smoke \
  "$@"
