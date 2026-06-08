#!/usr/bin/env bash
# Cloud Agent VM toolchain bootstrap (idempotent).
#
# Installs system deps agents need every session:
#   - Docker CE (fuse-overlayfs or vfs fallback) + dockerd when systemd cannot start it
#   - Node from .nvmrc via nvm
#   - apt: libssl-dev, pkg-config (indexer build)
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

log() {
  echo "[cloud-agent-toolchain] $*"
}

need_sudo() {
  [[ "$(id -u)" -eq 0 ]] || command -v sudo >/dev/null 2>&1
}

run_as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

apt_packages_present() {
  dpkg-query -W -f='${Status}' "$1" 2>/dev/null | grep -q "install ok installed"
}

ensure_apt_packages() {
  local pkgs=()
  for pkg in libssl-dev pkg-config curl; do
    apt_packages_present "$pkg" || pkgs+=("$pkg")
  done
  if [[ ${#pkgs[@]} -gt 0 ]] && need_sudo; then
    log "apt packages: ${pkgs[*]}"
    run_as_root apt-get update -qq
    run_as_root DEBIAN_FRONTEND=noninteractive apt-get install -y "${pkgs[@]}"
  fi
}

ensure_node() {
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ ! -s "$NVM_DIR/nvm.sh" ]]; then
    log "installing nvm…"
    curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
  fi
  # shellcheck source=/dev/null
  . "$NVM_DIR/nvm.sh"
  local want
  want="$(tr -d '[:space:]' <"$REPO_ROOT/.nvmrc")"
  log "Node v${want} (nvm)…"
  nvm install "$want"
  nvm use --silent "$want"
  local node_bin
  node_bin="$(dirname "$(nvm which current)")"
  export PATH="${node_bin}:${PATH}"
}

log "repo: ${REPO_ROOT}"
ensure_apt_packages
cloud_agent_install_docker || true
ensure_node
log "OK"
