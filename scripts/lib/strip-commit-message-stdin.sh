#!/usr/bin/env bash
# Read a full commit message on stdin, strip policy violations from the body, print result.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=.githooks/lib/validate-commit-message.sh
source "${REPO_ROOT}/.githooks/lib/validate-commit-message.sh"

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cat >"$tmp"
strip_commit_message_file "$tmp"
cat "$tmp"
