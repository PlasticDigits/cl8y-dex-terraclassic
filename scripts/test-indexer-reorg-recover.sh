#!/usr/bin/env bash
# Smoke test for indexer-reorg-recover.sh dry-run (GitLab #362).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SCRIPT="$REPO_ROOT/scripts/indexer-reorg-recover.sh"
_fail() { echo "FAIL: $*" >&2; exit 1; }

[ -x "$SCRIPT" ] || chmod +x "$SCRIPT"

out="$("$SCRIPT" --height 100 2>&1)" || _fail "dry-run exited non-zero"
echo "$out" | grep -q "DRY RUN" || _fail "expected DRY RUN banner"
echo "$out" | grep -q "Row impact preview" || _fail "expected row impact preview"
echo "$out" | grep -q "last_indexed_height" || _fail "expected cursor SQL preview"
echo "$out" | grep -q "swap_events" || _fail "expected swap_events in preview"

out2="$("$SCRIPT" --height 100 --cleanup-derived 2>&1)"
echo "$out2" | grep -q "Derived cleanup SQL" || _fail "expected derived cleanup SQL section"

# Must not mutate without --apply
if echo "$out" | grep -q "Applying cursor reset"; then
  _fail "dry-run must not apply SQL"
fi

echo "PASS: indexer-reorg-recover dry-run smoke"
