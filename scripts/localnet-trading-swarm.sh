#!/usr/bin/env bash
# Entrypoint for the localnet trading swarm (GitLab #119). LocalTerra + deploy only.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export SWARM_REPO_ROOT="${SWARM_REPO_ROOT:-$REPO_ROOT}"
PKG="$REPO_ROOT/packages/localnet-trading-swarm"
if [[ ! -d "$PKG" ]]; then
  echo "localnet-trading-swarm: missing package at $PKG" >&2
  exit 1
fi
cd "$PKG"
if [[ ! -d node_modules ]]; then
  echo "localnet-trading-swarm: running npm ci in $PKG …" >&2
  npm ci
fi
exec npm run start -- "$@"
