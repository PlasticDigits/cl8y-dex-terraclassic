#!/usr/bin/env bash
# Verification for GitLab #398 — admin-key custody and signer roster (SEC-B10).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); SKIP=$((SKIP + 1)); echo "  [SKIP] $1"; }

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #398 — admin-key custody and signer roster (SEC-B10)"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Doc invariants (runbook, skill, cross-links, Makefile targets)..."
if python3 scripts/check_key_custody_docs.py; then
  ok "check-key-custody-docs"
else
  bad "check-key-custody-docs"
fi

echo ""
echo "[2] Custody runbook docs/runbooks/key-custody.md present..."
if [[ -f docs/runbooks/key-custody.md ]]; then
  ok "key-custody.md present"
else
  bad "key-custody.md missing"
fi

echo ""
echo "[3] Agent skill AGENTS_KEY_CUSTODY.md..."
if [[ -f skills/AGENTS_KEY_CUSTODY.md ]]; then
  ok "agent skill present"
else
  bad "agent skill missing"
fi

echo ""
echo "[4] Phase 0 custody gate wired into launch checklist..."
LAUNCH="docs/runbooks/launch-checklist.md"
if grep -q "key-custody.md" "$LAUNCH" && grep -q "verify-issue-398" "$LAUNCH"; then
  ok "launch-checklist Phase 0 links key-custody.md + verify-issue-398"
else
  bad "launch-checklist missing key-custody.md or verify-issue-398 reference"
fi

echo ""
echo "[5] Checklist coverage (multisig threshold, roster, backup/escalation, rotation, no-EOA)..."
RB="docs/runbooks/key-custody.md"
if grep -q "## 1. Multisig type and threshold" "$RB" \
  && grep -q "## 2. Signer roster" "$RB" \
  && grep -q "## 3. Backup signer and escalation" "$RB" \
  && grep -q "## 4. Key rotation" "$RB" \
  && grep -q "single EOA" "$RB"; then
  ok "runbook covers all six SEC-B10 checklist items"
else
  bad "runbook missing one or more SEC-B10 checklist sections"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL   SKIP: $SKIP"
echo "════════════════════════════════════════════════════════════════"

[[ "$FAIL" -eq 0 ]]
