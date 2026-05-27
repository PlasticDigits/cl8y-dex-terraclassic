#!/usr/bin/env bash
# Unit checks for QA fresh-volumes toggle (no Docker required).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/qa/lib/qa-env.sh
source "$REPO_ROOT/scripts/qa/lib/qa-env.sh"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

_assert_fresh() {
  local val="$1"
  QA_FRESH_VOLUMES="$val"
  qa_is_fresh_volumes || _fail "expected fresh for QA_FRESH_VOLUMES=$val"
  [ "$(qa_compose_down_volume_args)" = "-v" ] || _fail "expected -v for QA_FRESH_VOLUMES=$val"
}

_assert_not_fresh() {
  local val="${1-}"
  QA_FRESH_VOLUMES="$val"
  if qa_is_fresh_volumes; then
    _fail "expected not fresh for QA_FRESH_VOLUMES=${val:-<unset>}"
  fi
  [ -z "$(qa_compose_down_volume_args)" ] || _fail "expected no -v for QA_FRESH_VOLUMES=${val:-<unset>}"
}

_assert_fresh 1
_assert_fresh true
_assert_fresh yes
_assert_fresh TRUE

_assert_not_fresh ""
_assert_not_fresh 0
_assert_not_fresh false
_assert_not_fresh no

grep -q 'QA_FRESH_VOLUMES=1' "$REPO_ROOT/scripts/qa/reset-qa.sh" \
  || _fail 'reset-qa.sh must set QA_FRESH_VOLUMES=1'

grep -q 'qa_compose_down_volume_args' "$REPO_ROOT/scripts/qa/stop-qa.sh" \
  || _fail 'stop-qa.sh must use qa_compose_down_volume_args'

grep -q 'reset-qa' "$REPO_ROOT/Makefile" || _fail 'Makefile must define reset-qa'
grep -q 'QA_FRESH_VOLUMES' "$REPO_ROOT/scripts/qa/README.md" \
  || _fail 'scripts/qa/README.md must document QA_FRESH_VOLUMES'

echo "OK: qa fresh-volumes helpers"
