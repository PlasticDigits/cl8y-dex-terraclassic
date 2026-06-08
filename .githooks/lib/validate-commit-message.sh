#!/usr/bin/env bash
# Validate commit message body: no email addresses, no "author" keyword.
# Sourced by .githooks/commit-msg and scripts/test-commit-msg-hook.sh
#
# Policy (body only — subject line is not checked):
#   - Reject lines containing an email address
#   - Reject lines containing the word "author" (matches Co-authored-by, Author:, etc.)
#
# COMMIT_MSG_POLICY=strip  — remove offending body lines and allow the commit (stderr warning)
# COMMIT_MSG_POLICY=reject — fail the commit (default)

validate_commit_message() {
  local msgfile=$1
  local policy="${COMMIT_MSG_POLICY:-reject}"
  local email_re='[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'
  local author_re='(^|[^[:alnum:]_-])author([^[:alnum:]_-]|$)'

  if [[ ! -f "$msgfile" ]]; then
    echo "validate_commit_message: missing message file: $msgfile" >&2
    return 1
  fi

  local subject body
  subject=$(head -n 1 "$msgfile" || true)
  body=$(awk 'NR==1 { next } found { print } /^$/ { found=1 }' "$msgfile")

  if [[ -z "${body//[$'\t ']/}" ]]; then
    return 0
  fi

  local -a violations=()
  local -a clean_lines=()
  local line n=0

  while IFS= read -r line || [[ -n "$line" ]]; do
    n=$((n + 1))
    local bad=false
    local reason=""

    if echo "$line" | grep -qE "$email_re"; then
      bad=true
      reason="email address"
    elif echo "$line" | grep -qiE "$author_re"; then
      bad=true
      reason="keyword \"author\""
    fi

    if [[ "$bad" == true ]]; then
      violations+=("body line $n ($reason): $line")
    else
      clean_lines+=("$line")
    fi
  done <<<"$body"

  if [[ ${#violations[@]} -eq 0 ]]; then
    return 0
  fi

  if [[ "$policy" == "strip" ]]; then
    {
      printf '%s\n' "$subject"
      if [[ ${#clean_lines[@]} -gt 0 ]]; then
        printf '\n'
        printf '%s\n' "${clean_lines[@]}"
      fi
    } >"$msgfile"
    echo "commit-msg: stripped ${#violations[@]} body line(s) (email/author policy):" >&2
    printf '  - %s\n' "${violations[@]}" >&2
    return 0
  fi

  echo "commit-msg: REJECTED — commit message body must not contain email addresses or the word \"author\"." >&2
  echo "  (Co-authored-by / Author trailers are not allowed.)" >&2
  printf '  - %s\n' "${violations[@]}" >&2
  echo "  Fix the message, or set COMMIT_MSG_POLICY=strip to auto-remove offending body lines." >&2
  return 1
}
