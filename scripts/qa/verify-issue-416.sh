#!/usr/bin/env bash
# Automated verification for GitLab #416 — design token alignment.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #416 — design token alignment"
echo "════════════════════════════════════════════════════════════════"

run_step "design token doc invariant" \
  python3 scripts/check_design_tokens.py

run_step "design-system Tailwind section" \
  grep -q '## Tailwind color aliases' docs/design-system.md

run_step "agent design skill token rules" \
  grep -q 'trade-bootstrap.css' skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md

run_step "QA_TEMPLATE header theme toggle" \
  grep -q 'Header theme toggle' QA_TEMPLATE.md

run_step "QA_TEMPLATE trade bootstrap row" \
  grep -q 'Trade bootstrap continuity' QA_TEMPLATE.md

run_step "QA_PASS footer toggle historical note" \
  grep -q 'Historical note' QA_PASS_2026-03-13.md

run_step "visual QA pass artifact" \
  test -f QA_PASS_2026-06-25.md

run_step "frontend unit tests (designTokens)" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm run test -- --run src/designTokens.test.ts

run_step "frontend lint" \
  make lint-frontend

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
