#!/usr/bin/env bash
# Incremental gitleaks for GitLab CI (GitLab #380 / M-13).
# Full history may flag pre-#118 dev mnemonic commits; CI scans only the push range.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CONFIG="${GITLEAKS_CONFIG:-.gitleaks.toml}"
LOG_OPTS="${GITLEAKS_LOG_OPTS:-}"

if [[ -z "$LOG_OPTS" ]]; then
  if [[ -n "${CI_COMMIT_BEFORE_SHA:-}" && -n "${CI_COMMIT_SHA:-}" \
    && "${CI_COMMIT_BEFORE_SHA}" != "0000000000000000000000000000000000000000" ]]; then
    LOG_OPTS="${CI_COMMIT_BEFORE_SHA}..${CI_COMMIT_SHA}"
  else
    LOG_OPTS="-1"
  fi
fi

echo ">> gitleaks detect --source . -c ${CONFIG} --log-opts=${LOG_OPTS}"
gitleaks detect --source . -c "$CONFIG" --verbose --log-opts="$LOG_OPTS"
