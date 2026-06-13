#!/usr/bin/env bash
# Run gitleaks on git-tracked files (Docker fallback when binary missing). GitLab #380.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec "${ROOT}/scripts/ci/gitleaks-scan-tracked.sh" "$@"
