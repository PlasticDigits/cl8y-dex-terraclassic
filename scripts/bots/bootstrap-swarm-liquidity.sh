#!/usr/bin/env bash
# One-shot LP top-up before swarm workers (GitLab #293). See bootstrap-swarm-liquidity.py.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
chmod +x scripts/bots/bootstrap-swarm-liquidity.py
exec python3 scripts/bots/bootstrap-swarm-liquidity.py "$@"
