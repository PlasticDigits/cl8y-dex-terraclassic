#!/usr/bin/env bash
# Run gitleaks on the repo (Docker fallback when binary missing). GitLab #380.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GITLEAKS_IMAGE="${GITLEAKS_IMAGE:-ghcr.io/gitleaks/gitleaks:v8.24.2}"
CONFIG="${ROOT}/.gitleaks.toml"

if command -v gitleaks >/dev/null 2>&1; then
  exec gitleaks detect --source "$ROOT" --no-git --config "$CONFIG" --redact "$@"
fi

exec docker run --rm -v "${ROOT}:${ROOT}" -w "$ROOT" --entrypoint gitleaks \
  "$GITLEAKS_IMAGE" detect --source "$ROOT" --no-git --config "$CONFIG" --redact "$@"
