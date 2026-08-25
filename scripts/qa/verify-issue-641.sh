#!/usr/bin/env bash
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
PASS=0
FAIL=0
ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
run() {
  echo ""; echo "[$1]"
  if "${@:2}"; then ok "$1"; else bad "$1"; fi
}
echo "GitLab #641 — Hexxagon CW20 pack"
run "pack: schema + Keplr lockstep" python3 scripts/qa/hexxagon_cw20_validate.py
run "docs: H641-1–H641-8" bash -c '
  grep -qE "\*\*H641-1\*\*" docs/listings/hexxagon/README.md &&
  grep -qE "\*\*H641-8\*\*" docs/listings/hexxagon/README.md &&
  grep -qE "already listed|already live" docs/listings/hexxagon/README.md &&
  grep -qE "make verify-issue-641" docs/listings/hexxagon/README.md
'
run "docs: skill + AGENTS.md" bash -c '
  grep -qE "\*\*H641-1" skills/AGENTS_HEXXAGON.md &&
  grep -qE "\*\*H641-8" skills/AGENTS_HEXXAGON.md &&
  grep -qE "AGENTS_HEXXAGON" AGENTS.md &&
  grep -qE "verify-issue-641" AGENTS.md &&
  grep -qE "hexxagon-cw20-gitlab-641" docs/integrators.md &&
  grep -qE "verify-issue-641" docs/testing.md
'
echo "  $PASS passed, $FAIL failed"
if [[ "$FAIL" -gt 0 ]]; then exit 1; fi
