#!/usr/bin/env bash
# Automated verification for GitLab #438 — incident communications templates (SEC-G05).
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
echo "  GitLab #438 — incident communications templates (SEC-G05)"
echo "════════════════════════════════════════════════════════════════"

run_step "incident comms doc invariant" \
  make check-incident-comms-templates-docs

run_step "agent skill present" \
  test -f skills/AGENTS_INCIDENT_COMMS_TEMPLATES.md

run_step "pair paused template" \
  grep -q '### 1. Pair paused' docs/templates/incident-dex-indexer.md

run_step "blacklist applied template" \
  grep -q '### 2. Blacklist applied' docs/templates/incident-dex-indexer.md

run_step "exploit interim template" \
  grep -q '### 3. Exploit under investigation' docs/templates/incident-dex-indexer.md

run_step "false alarm retraction template" \
  grep -q '### 4. False alarm retraction' docs/templates/incident-dex-indexer.md

run_step "postmortem template" \
  grep -q '### 5. Postmortem summary' docs/templates/incident-dex-indexer.md

run_step "Communications section links appendix" \
  grep -q 'Appendix: Communications templates' docs/templates/incident-dex-indexer.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
