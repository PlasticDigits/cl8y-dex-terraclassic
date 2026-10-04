#!/usr/bin/env bash
# #1261: offline directory pack checks; never submits an upstream PR.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
python3 scripts/qa/terra_classic_io_validate.py
bash scripts/qa/verify-issue-639.sh
