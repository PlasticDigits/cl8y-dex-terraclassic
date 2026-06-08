#!/usr/bin/env bash
# Cloud Agent VM toolchain bootstrap (idempotent).
#
# Installs system deps agents need every session:
#   - Docker CE (fuse-overlayfs or vfs fallback) + dockerd when systemd cannot start it
#   - Node from .nvmrc via nvm (strips /exec-daemon/node v22 from PATH)
#   - Rust stable 1.96+ and apt: libssl-dev, pkg-config (indexer build)
#
# Does NOT run npm ci, Playwright, browser, or LocalTerra deploy — those run from
# setup-cloud-agent-env.sh after this script.
#
# Usage (repo root):
#   ./scripts/setup-cloud-agent-toolchain.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/cloud-agent-docker.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-docker.sh"
# shellcheck source=scripts/lib/cloud-agent-toolchain.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-toolchain.sh"

log() {
  echo "[cloud-agent-toolchain] $*"
}

log "repo: ${REPO_ROOT}"

if ! cloud_agent_install_docker; then
  log "Docker setup incomplete" >&2
  exit 1
fi

if ! cloud_agent_ensure_rust; then
  log "Rust setup failed" >&2
  exit 1
fi

if ! cloud_agent_ensure_node "$REPO_ROOT"; then
  log "Node setup failed" >&2
  exit 1
fi

log "OK"
