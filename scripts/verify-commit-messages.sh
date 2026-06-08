#!/usr/bin/env bash
# Verify commit message bodies in a revision range (default: all reachable commits).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=.githooks/lib/validate-commit-message.sh
source "${REPO_ROOT}/.githooks/lib/validate-commit-message.sh"

range="${1:-HEAD}"
failed=0

while read -r hash; do
  [[ -z "$hash" ]] && continue
  if ! validate_commit_message_hash "$hash" >/dev/null 2>&1; then
    echo "FAIL: $hash $(git log -1 --format='%s' "$hash")" >&2
    validate_commit_message_hash "$hash" >&2 || true
    failed=1
  fi
done < <(git rev-list "$range")

if [[ "$failed" -ne 0 ]]; then
  echo "verify-commit-messages: ${failed} commit(s) violate message body policy." >&2
  exit 1
fi

echo "OK: all commits in ${range} pass message body policy"
