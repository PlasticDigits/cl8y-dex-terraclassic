#!/usr/bin/env bash
# Automated verification for GitLab #695 — production VITE_DEV_MODE reject (FE-01).
#
# Proves (unit + docs + greps; no LocalTerra / no wallet):
#   1. assertBuildEnvGuards throws on production + VITE_DEV_MODE=true.
#   2. viteConfig.build.test.ts covers reject / load / serve / local-only escape.
#   3. Docs + skill invariants D695-1–D695-8 + AGENTS.md playbook.
#   4. Vitest / Playwright keep VITE_DEV_MODE=true for local.
#
# Refs: skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md,
#       docs/frontend.md § Simulated (dev) wallet,
#       frontend-dapp/vite.config.ts
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
echo "  GitLab #695 — production VITE_DEV_MODE reject (FE-01)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: viteConfig.build.test.ts production VITE_DEV_MODE cases" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/viteConfig.build.test.ts

run_step "code: production reject names VITE_DEV_MODE and Coolify / .env.production" \
  grep -qE "mode === 'production' && env.VITE_DEV_MODE === 'true'" frontend-dapp/vite.config.ts && \
  grep -qE 'VITE_DEV_MODE=true is not allowed for production vite builds' frontend-dapp/vite.config.ts && \
  grep -qE 'Unset VITE_DEV_MODE in Coolify / \.env\.production' frontend-dapp/vite.config.ts && \
  grep -qE 'GitLab #695' frontend-dapp/vite.config.ts

run_step "code: test names the production reject (DEV_MODE throw does not mention mnemonic)" \
  grep -qE "rejects production build when VITE_DEV_MODE is true \(GitLab #695\)" \
    frontend-dapp/src/viteConfig.build.test.ts && \
  grep -qE 'still rejects production VITE_DEV_MODE when local-only mnemonic escape is set' \
    frontend-dapp/src/viteConfig.build.test.ts && \
  grep -qE "process.env.VITE_DEV_MODE = ''" frontend-dapp/src/viteConfig.build.test.ts && \
  grep -qE 'lets dotenv re-inject' frontend-dapp/src/viteConfig.build.test.ts && \
  bash -c '! grep -A2 "VITE_DEV_MODE=true is not allowed" frontend-dapp/vite.config.ts | grep -qiE "mnemonic|seed|BIP39"'

run_step "code: LocalTerra / Vitest / Playwright keep VITE_DEV_MODE=true" \
  grep -qE "VITE_DEV_MODE: 'true'" frontend-dapp/vitest.config.ts && \
  grep -qE 'VITE_DEV_MODE=true' frontend-dapp/playwright.config.ts && \
  grep -qE '^VITE_DEV_MODE=true$' frontend-dapp/.env.example

run_step "docs: frontend.md production reject + verify target" \
  grep -qE 'Production `VITE_DEV_MODE` reject' docs/frontend.md && \
  grep -qE 'make verify-issue-695' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_DEV_MODE_GUARD' docs/frontend.md

run_step "docs: security-model + operator-secrets + launch runbooks" \
  grep -qE 'VITE_DEV_MODE=true' docs/security-model.md && \
  grep -qE '#695' docs/security-model.md && \
  grep -qE '`VITE_DEV_MODE`' docs/operator-secrets.md && \
  grep -qE 'VITE_DEV_MODE' docs/runbooks/launch-checklist.md && \
  grep -qE 'VITE_DEV_MODE=true' docs/runbooks/mainnet-soft-launch.md

run_step "docs: skill + AGENTS.md playbook #695" \
  grep -qE '\*\*D695-1' skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md && \
  grep -qE '\*\*D695-8' skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md && \
  grep -qE 'make verify-issue-695' skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md && \
  grep -qE 'AGENTS_FRONTEND_DEV_MODE_GUARD' AGENTS.md && \
  grep -qE 'verify-issue-695' AGENTS.md && \
  grep -qE 'AGENTS_FRONTEND_DEV_MODE_GUARD' skills/AGENTS_FRONTEND_TRUST_BOUNDARIES.md && \
  grep -qE 'AGENTS_FRONTEND_DEV_MODE_GUARD' skills/AGENTS_BUNDLE_DEV_WALLET.md && \
  grep -qE 'AGENTS_FRONTEND_DEV_MODE_GUARD' skills/AGENTS_FRONTEND_PRODUCTION_BUILD.md

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
