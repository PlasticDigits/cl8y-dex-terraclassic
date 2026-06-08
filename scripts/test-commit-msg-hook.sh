#!/usr/bin/env bash
# Unit checks for .githooks/commit-msg body policy.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=.githooks/lib/validate-commit-message.sh
source "${REPO_ROOT}/.githooks/lib/validate-commit-message.sh"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

write_msg() {
  local file=$1
  shift
  printf '%s\n' "$@" >"$file"
}

assert_ok() {
  local name=$1 file=$2
  COMMIT_MSG_POLICY=reject validate_commit_message "$file" || {
    echo "FAIL: expected accept: $name" >&2
    exit 1
  }
  echo "OK accept: $name"
}

assert_reject() {
  local name=$1 file=$2
  if COMMIT_MSG_POLICY=reject validate_commit_message "$file"; then
    echo "FAIL: expected reject: $name" >&2
    exit 1
  fi
  echo "OK reject: $name"
}

assert_strip() {
  local name=$1 file=$2 expected=$3
  COMMIT_MSG_POLICY=strip validate_commit_message "$file" || {
    echo "FAIL: strip failed: $name" >&2
    exit 1
  }
  if ! diff -u <(printf '%s' "$expected") <(cat "$file"); then
    echo "FAIL: strip output mismatch: $name" >&2
    exit 1
  fi
  echo "OK strip: $name"
}

f="$tmpdir/subject-only"
write_msg "$f" "fix: something simple"
assert_ok "subject only" "$f"

f="$tmpdir/body-clean"
write_msg "$f" "feat: add widget" "" "Adds a widget for the pool page."
assert_ok "clean body" "$f"

f="$tmpdir/subject-email"
write_msg "$f" "fix: contact contact@example.com"
assert_ok "email in subject allowed" "$f"

f="$tmpdir/body-email"
write_msg "$f" "fix: thing" "" "See contact@example.com for details."
assert_reject "email in body" "$f"

f="$tmpdir/co-authored"
write_msg "$f" "fix: thing" "" "Co-authored-by: Someone <contact@example.com>"
assert_reject "Co-authored-by trailer" "$f"

f="$tmpdir/author-word"
write_msg "$f" "docs: update" "" "Original author notes preserved elsewhere."
assert_reject "author word in body" "$f"

f="$tmpdir/authoritative"
write_msg "$f" "docs: update" "" "This is the authoritative reference."
assert_ok "authoritative does not match author keyword" "$f"

f="$tmpdir/strip"
write_msg "$f" "fix: thing" "" "Good line." "Co-authored-by: x <x@y.com>" "Also good."
assert_strip "strip offending lines" "$f" $'fix: thing\n\nGood line.\nAlso good.\n'

echo "All commit-msg hook checks passed."
