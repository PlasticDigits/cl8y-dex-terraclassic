#!/usr/bin/env bash
# Regression: pre-push skips policy violations already on origin/main (#1286 / #1287).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=.githooks/lib/validate-commit-message.sh
source "${REPO_ROOT}/.githooks/lib/validate-commit-message.sh"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

count_push_violations() {
  local git_dir=$1
  local integration=$2
  local range=$3
  local hash violations=0

  while read -r hash; do
    [[ -z "$hash" ]] && continue
    if (
      cd "$git_dir"
      commit_message_policy_on_integration_branch "$hash" "$integration"
    ); then
      continue
    fi
    if ! (
      cd "$git_dir"
      validate_commit_message_hash "$hash"
    ) >/dev/null 2>&1; then
      violations=$((violations + 1))
    fi
  done < <(git -C "$git_dir" rev-list "$range" 2>/dev/null || true)
  printf '%s' "$violations"
}

if ! integration_ref=$(resolve_integration_main_ref origin 2>/dev/null); then
  echo "SKIP: no origin/main ref for pre-push integration test"
  exit 0
fi

# Known policy violation on main (ADR 0006 / #1282); must stay on main without rewrite.
bad_on_main=ad2d249358556cfd833416df366a198de1a6884c
if ! git merge-base --is-ancestor "$bad_on_main" "$integration_ref" 2>/dev/null; then
  echo "SKIP: $bad_on_main not on $integration_ref (fork or shallow clone)"
  exit 0
fi

if validate_commit_message_hash "$bad_on_main" >/dev/null 2>&1; then
  _fail "expected $bad_on_main to fail validate_commit_message_hash"
fi

if ! commit_message_policy_on_integration_branch "$bad_on_main" "$integration_ref"; then
  _fail "expected $bad_on_main to be skipped as on integration branch"
fi

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT
repo="$tmpdir/repo"

git init -q "$repo"
git -C "$repo" config user.email 'agent-test@example.com'
git -C "$repo" config user.name 'Agent Test'
git -C "$repo" config core.hooksPath /dev/null

echo init >"$repo/f"
git -C "$repo" add f
git -C "$repo" commit -q -m "chore: init"
git -C "$repo" branch -M main

git -C "$repo" checkout -q -b feature
git -C "$repo" commit -q --allow-empty -m "feat: feature-only"
feature_remote_tip=$(git -C "$repo" rev-parse HEAD)

git -C "$repo" checkout -q main
echo bad >>"$repo/f"
git -C "$repo" add f
git -C "$repo" commit -q -m "docs: bad on main" -m "Co-authored-by: Cursor <cursoragent@cursor.com>"

git -C "$repo" checkout -q feature
git -C "$repo" merge -q main
feature_local_tip=$(git -C "$repo" rev-parse HEAD)
range="${feature_remote_tip}..${feature_local_tip}"

violations=$(count_push_violations "$repo" main "$range")
if [[ "$violations" -ne 0 ]]; then
  _fail "after merge main, push range should skip main violations (got $violations)"
fi

git -C "$repo" commit -q --allow-empty -m "feat: bad feature" -m "Co-authored-by: Cursor <cursoragent@cursor.com>"
feature_local_tip=$(git -C "$repo" rev-parse HEAD)
range="${feature_remote_tip}..${feature_local_tip}"

violations=$(count_push_violations "$repo" main "$range")
if [[ "$violations" -eq 0 ]]; then
  _fail "new feature commit with Co-authored-by should still fail"
fi

echo "OK: pre-push integration-branch skip (#1286 / #1287)"
