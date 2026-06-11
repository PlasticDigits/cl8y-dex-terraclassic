#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Golden-image setup for plasticdigits/cl8y-dex-terraclassic (Terra Classic)
# Run as root on a fresh Ubuntu 24.04 CX33 before snapshotting.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
WORKSPACE="${WORKSPACE:-/home/agent/workspace}"
AGENT_USER="${AGENT_USER:-agent}"
GCH_RUNNER_URL="${GCH_RUNNER_URL:-https://gitlab.com/plasticdigits/gitlab-cursor-webhook/-/raw/main/scripts/gch-cloud-init-runner.sh}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENT_HOME="/home/${AGENT_USER}"

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
mkdir -p "${AGENT_HOME}"
if [[ ! -f "${AGENT_HOME}/.bashrc" ]]; then
  cp -a /etc/skel/. "${AGENT_HOME}/"
fi
chown -R "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}"
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
echo "==> glab"
curl -fsSL https://gitlab.com/gitlab-org/cli/-/releases/v1.58.0/downloads/glab_1.58.0_linux_amd64.deb -o /tmp/glab.deb
dpkg -i /tmp/glab.deb || apt-get install -f -y

echo "==> Cursor CLI"
sudo -u "${AGENT_USER}" bash -lc 'curl https://cursor.com/install -fsS | bash'
if [[ ! -x "${AGENT_HOME}/.local/bin/agent" ]]; then
  echo "ERROR: Cursor CLI not found at ${AGENT_HOME}/.local/bin/agent" >&2
  exit 1
fi
ln -sf "${AGENT_HOME}/.local/bin/agent" /usr/local/bin/agent

echo "==> Agent shell env"
touch "${AGENT_HOME}/.bashrc"
if ! grep -q PLAYWRIGHT_HOST_PLATFORM_OVERRIDE "${AGENT_HOME}/.bashrc"; then
  echo 'export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64' >>"${AGENT_HOME}/.bashrc"
fi
if ! grep -q '\.local/bin' "${AGENT_HOME}/.bashrc"; then
  echo 'export PATH="$HOME/.local/bin:$PATH"' >>"${AGENT_HOME}/.bashrc"
fi
chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/.bashrc"

echo "==> Browser profile + Keplr (Terra)"
sudo -u "${AGENT_USER}" mkdir -p "${AGENT_HOME}/.gch/browser-profile"
# Admin: download Keplr unpacked extension into /home/agent/.gch/extensions/keplr

echo "==> Shared cloud-init runner"
RUNNER_DST="${AGENT_HOME}/gch-cloud-init-runner.sh"
if [[ -f "${SCRIPT_DIR}/gch-cloud-init-runner.sh" ]]; then
  install -m 755 -o "${AGENT_USER}" -g "${AGENT_USER}" \
    "${SCRIPT_DIR}/gch-cloud-init-runner.sh" "${RUNNER_DST}"
elif curl -fsSL "${GCH_RUNNER_URL}" -o "${RUNNER_DST}"; then
  chown "${AGENT_USER}:${AGENT_USER}" "${RUNNER_DST}"
  chmod 755 "${RUNNER_DST}"
else
  echo "ERROR: add gch-cloud-init-runner.sh next to gch-cloud-setup.sh in the project repo." >&2
  exit 1
fi

echo "==> Workspace"
mkdir -p "${WORKSPACE}"
chown -R "${AGENT_USER}:${AGENT_USER}" "${WORKSPACE}"

echo "Setup complete. Clone repo into ${WORKSPACE}, install Keplr extension, configure localterra, then run pre-snapshot cleanup."
