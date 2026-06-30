#!/usr/bin/env bash
# Automated verification for GitLab #439 — incident template timeline (SEC-G06).
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
echo "  GitLab #439 — incident template timeline (SEC-G06)"
echo "════════════════════════════════════════════════════════════════"

run_step "incident template doc invariant" \
  make check-incident-template-docs

run_step "agent skill present" \
  test -f skills/AGENTS_INCIDENT_TEMPLATE.md

run_step "timeline section in incident template" \
  grep -q '## Incident timeline' docs/templates/incident-dex-indexer.md

run_step "blacklist rollback links incident timeline" \
  grep -q 'incident-dex-indexer.md#incident-timeline' docs/runbooks/blacklist-decision.md

run_step "emergency commands link incident timeline" \
  grep -q 'incident-dex-indexer.md#incident-timeline' docs/runbooks/emergency-commands.md

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
