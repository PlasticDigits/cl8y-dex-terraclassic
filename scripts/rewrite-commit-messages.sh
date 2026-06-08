#!/usr/bin/env bash
# Rewrite all commit messages: strip email/author violations from bodies (history cleanup).
# Requires git-filter-repo. Re-adds origin remote after rewrite.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if ! command -v git-filter-repo >/dev/null 2>&1; then
  echo "rewrite-commit-messages: git-filter-repo is required" >&2
  exit 1
fi

origin_url=$(git remote get-url origin 2>/dev/null || true)
if [[ -z "$origin_url" ]]; then
  echo "rewrite-commit-messages: no origin remote" >&2
  exit 1
fi

export FILTER_REPO_ROOT="$REPO_ROOT"

echo "[rewrite-commit-messages] stripping policy violations from all commit messages…"
git filter-repo --force \
  --message-callback "
import os, subprocess
root = os.environ['FILTER_REPO_ROOT']
script = os.path.join(root, 'scripts/lib/strip-commit-message-stdin.sh')
proc = subprocess.run(['bash', script], input=message, capture_output=True)
if proc.returncode != 0:
    raise RuntimeError(proc.stderr.decode())
return proc.stdout
"

git remote add origin "$origin_url" 2>/dev/null || git remote set-url origin "$origin_url"

echo "[rewrite-commit-messages] done — verify with: ./scripts/verify-commit-messages.sh HEAD"
