#!/usr/bin/env bash
# Automated verification for Forgejo #1287 / #1286 — pre-push skips policy on origin/main.
#
# No LocalTerra. Regression: make test-commit-msg-hook (unit + integration skip).
#
# Refs: .githooks/pre-push, skills/AGENTS_GIT_COMMIT_HOOKS.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

echo "== verify-issue-1287: commit-msg + pre-push policy =="
make test-commit-msg-hook

echo ""
echo "OK: verify-issue-1287"
