#!/usr/bin/env bash
# Verification for GitLab #396 (SEC-B06): wrap-mapper pause/unpause on LocalTerra.
#
# Acceptance:
#   - LocalTerra smoke: wrap rejected under pause
#   - LocalTerra smoke: unwrap rejected under pause
#   - LocalTerra smoke: wrap + unwrap succeed after unpause
#   - Frontend unit coverage for pause CTA (SEC-A02) already in SwapPage.test.tsx
#
# Refs: docs/testing.md § Wrap-mapper pause smoke (SEC-B06),
#       skills/AGENTS_FRONTEND_SWAP_SAFETY_CTA.md,
#       docs/qa-templates/wrap-unwrap-test-pass.md § Paused State.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

echo "== GitLab #396 / SEC-B06 verification =="

echo ""
echo "-- Frontend unit (SEC-A02 pause CTA, no chain) --"
if bash scripts/with-node.sh --cwd frontend-dapp -- npm run test:run -- \
  src/pages/SwapPage.test.tsx -t "SEC-A02" >/tmp/verify-396-vitest.log 2>&1; then
  ok "Vitest SwapPage SEC-A02 pause + rate-limit CTA"
else
  bad "Vitest SwapPage SEC-A02 (see /tmp/verify-396-vitest.log)"
fi

echo ""
echo "-- LocalTerra wrap-mapper pause smoke --"
if chmod +x scripts/smoke-wrap-mapper-pause.sh scripts/lib/smoke-wrap-env.sh \
  scripts/lib/lcd-smart-query.sh scripts/lib/e2e-terrad-tx.sh scripts/lib/terrad-wait-tx.sh \
  && ./scripts/smoke-wrap-mapper-pause.sh >/tmp/verify-396-smoke.log 2>&1; then
  ok "smoke-wrap-mapper-pause: wrap rejected while paused"
  ok "smoke-wrap-mapper-pause: unwrap rejected while paused"
  ok "smoke-wrap-mapper-pause: wrap + unwrap restored after unpause"
else
  bad "smoke-wrap-mapper-pause failed (see /tmp/verify-396-smoke.log)"
  tail -30 /tmp/verify-396-smoke.log >&2 || true
fi

echo ""
echo "== Summary =="
printf '%s\n' "${RESULTS[@]}"
echo "PASS=$PASS FAIL=$FAIL"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi

echo "OK: GitLab #396 acceptance criteria satisfied."
