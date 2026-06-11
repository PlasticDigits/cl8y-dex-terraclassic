#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Golden-image setup for plasticdigits/cl8y-dex-terraclassic (Terra Classic)
# Run as root on a fresh Ubuntu 24.04 CX33 before snapshotting.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
WORKSPACE="${WORKSPACE:-/home/agent/workspace}"
AGENT_USER="${AGENT_USER:-agent}"
GCH_RUNNER_URL="${GCH_RUNNER_URL:-https://raw.githubusercontent.com/plasticdigits/gitlab-cursor-webhook/main/scripts/gch-cloud-init-runner.sh}"

echo "==> Base packages"
apt-get update
apt-get install -y \
  build-essential git curl jq sqlite3 postgresql postgresql-contrib \
  docker.io xvfb chromium-browser unzip ca-certificates \
  libnss3 libnspr4 libdbus-1-3 libatk1.0-0 libatk-bridge2.0-0 libcups2 \
  libdrm2 libxkbcommon0 libxcomposite1 libxdamage1 libxfixes3 libxrandr2 \
  libgbm1 libasound2t64 libpango-1.0-0 libcairo2 libatspi2.0-0 fonts-liberation

echo "==> Agent user"
if ! id "${AGENT_USER}" &>/dev/null; then
  useradd -m -s /bin/bash "${AGENT_USER}" 2>/dev/null || useradd -s /bin/bash "${AGENT_USER}"
fi
mkdir -p "/home/${AGENT_USER}"
chown -R "${AGENT_USER}:${AGENT_USER}" "/home/${AGENT_USER}"
passwd -l "${AGENT_USER}"
echo "${AGENT_USER} ALL=(ALL) NOPASSWD:ALL" >/etc/sudoers.d/"${AGENT_USER}"
chmod 440 /etc/sudoers.d/"${AGENT_USER}"

echo "==> Docker"
systemctl enable --now docker
usermod -aG docker "${AGENT_USER}"

echo "==> Swap (8G)"
if [[ ! -f /swapfile ]]; then
  fallocate -l 8G /swapfile
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  grep -q swapfile /etc/fstab || echo '/swapfile none swap sw 0 0' >>/etc/fstab
fi

echo "==> Rust"
sudo -u "${AGENT_USER}" bash -lc 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'

echo "==> LocalTerra / Terra tooling"
# Admin: install localterra or Terrad per project docs
sudo -u "${AGENT_USER}" bash -lc 'mkdir -p ~/.local/bin'

echo "==> Node + Playwright"
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt-get install -y nodejs
sudo -u "${AGENT_USER}" bash -lc '
  export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64
  mkdir -p ~/.gch/playwright
  cd ~/.gch/playwright
  npm init -y
  npm install @playwright/test
  npx playwright install chromium
'
if ! grep -q PLAYWRIGHT_HOST_PLATFORM_OVERRIDE "/home/${AGENT_USER}/.bashrc"; then
  echo 'export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64' >>"/home/${AGENT_USER}/.bashrc"
fi

echo "==> glab"
curl -fsSL https://gitlab.com/gitlab-org/cli/-/releases/v1.58.0/downloads/glab_1.58.0_linux_amd64.deb -o /tmp/glab.deb
dpkg -i /tmp/glab.deb || apt-get install -f -y

echo "==> Cursor CLI"
sudo -u "${AGENT_USER}" bash -lc 'curl https://cursor.com/install -fsS | bash'
echo 'export PATH="$HOME/.cursor/bin:$PATH"' >>"/home/${AGENT_USER}/.bashrc"

echo "==> Browser profile + Keplr (Terra)"
sudo -u "${AGENT_USER}" mkdir -p "/home/${AGENT_USER}/.gch/browser-profile"
# Admin: download Keplr unpacked extension into /home/agent/.gch/extensions/keplr

echo "==> Shared cloud-init runner"
curl -fsSL "${GCH_RUNNER_URL}" -o "/home/${AGENT_USER}/gch-cloud-init-runner.sh"
chmod +x "/home/${AGENT_USER}/gch-cloud-init-runner.sh"
chown "${AGENT_USER}:${AGENT_USER}" "/home/${AGENT_USER}/gch-cloud-init-runner.sh"

echo "==> Workspace"
mkdir -p "${WORKSPACE}"
chown -R "${AGENT_USER}:${AGENT_USER}" "${WORKSPACE}"

echo "Setup complete. Clone repo into ${WORKSPACE}, install Keplr extension, configure localterra, then run pre-snapshot cleanup."
