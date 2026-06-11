#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Golden-image setup for plasticdigits/cl8y-dex-terraclassic (Terra Classic)
# Run as root on a fresh Ubuntu 24.04 CPX32 (fsn1) before snapshotting.
set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
WORKSPACE="${WORKSPACE:-/home/agent/workspace}"
AGENT_USER="${AGENT_USER:-agent}"
GCH_RUNNER_URL="${GCH_RUNNER_URL:-https://gitlab.com/plasticdigits/gitlab-cursor-webhook/-/raw/main/scripts/gch-cloud-init-runner.sh}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENT_HOME="/home/${AGENT_USER}"
GCH_GOLDEN_IMAGE_MODEL="${GCH_GOLDEN_IMAGE_MODEL:-composer-2.5}"
FINALIZE_PROMPT="${SCRIPT_DIR}/gch-golden-image-finalize.md"

# Ubuntu bash -lc is non-interactive; .profile skips .bashrc, so nvm is not on PATH.
_agent_nvm_sh() {
  sudo -u "${AGENT_USER}" bash -lc "
    export NVM_DIR=\"\${HOME}/.nvm\"
    [ -s \"\${NVM_DIR}/nvm.sh\" ] && . \"\${NVM_DIR}/nvm.sh\"
    ${1}
  "
}

echo "==> Base packages"
apt-get update
apt-get upgrade -y
apt-get install -y \
  build-essential git curl jq sqlite3 ripgrep file \
  xvfb chromium-browser unzip ca-certificates \
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

echo "==> Docker (CE + compose plugin)"
install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
chmod a+r /etc/apt/keyrings/docker.asc
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "${VERSION_CODENAME}") stable" \
  >/etc/apt/sources.list.d/docker.list
apt-get update
apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
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

echo "==> glab"
GLAB_VERSION="${GLAB_VERSION:-1.102.0}"
curl -fsSL "https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/glab_${GLAB_VERSION}_linux_amd64.deb" \
  -o /tmp/glab.deb
dpkg -i /tmp/glab.deb || apt-get install -f -y

echo "==> Cursor CLI"
sudo -u "${AGENT_USER}" bash -lc 'curl https://cursor.com/install -fsS | bash'
if [[ ! -x "${AGENT_HOME}/.local/bin/agent" ]]; then
  echo "ERROR: Cursor CLI not found at ${AGENT_HOME}/.local/bin/agent" >&2
  exit 1
fi
ln -sf "${AGENT_HOME}/.local/bin/agent" /usr/local/bin/agent

echo "==> Cursor CLI config"
mkdir -p "${AGENT_HOME}/.cursor"
cat >"${AGENT_HOME}/.cursor/cli-config.json" <<'EOF'
{
  "version": 1,
  "editor": { "vimMode": false },
  "permissions": { "allow": [], "deny": [] },
  "approvalMode": "unrestricted",
  "attribution": {
    "attributeCommitsToAgent": false,
    "attributePRsToAgent": false
  }
}
EOF
chown -R "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/.cursor"
chmod 600 "${AGENT_HOME}/.cursor/cli-config.json"

echo "==> GCH agent env (system + agent shell)"
cat >/etc/profile.d/gch-agent.sh <<'EOF'
# Playwright: no ubuntu26.04-x64 build yet; use 24.04 userspace on golden images.
export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64
EOF
chmod 644 /etc/profile.d/gch-agent.sh

touch "${AGENT_HOME}/.bashrc"
if ! grep -q 'GCH job secrets' "${AGENT_HOME}/.bashrc"; then
  cat >>"${AGENT_HOME}/.bashrc" <<'EOF'

# GCH job secrets (cloud-init writes /etc/gch/job.env per VM)
if [[ -r /etc/gch/job.env ]]; then
  set -a
  # shellcheck source=/dev/null
  source /etc/gch/job.env
  set +a
  export GLAB_TOKEN="${GITLAB_TOKEN:-}"
fi
EOF
fi
if ! grep -q '\.local/bin' "${AGENT_HOME}/.bashrc"; then
  echo 'export PATH="$HOME/.local/bin:$PATH"' >>"${AGENT_HOME}/.bashrc"
fi
chown "${AGENT_USER}:${AGENT_USER}" "${AGENT_HOME}/.bashrc"

echo "==> GCH agent directories"
sudo -u "${AGENT_USER}" mkdir -p \
  "${AGENT_HOME}/.gch/browser-profile" \
  "${AGENT_HOME}/.gch/extensions"

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
if [[ -d "${SCRIPT_DIR}/.git" ]]; then
  rsync -a "${SCRIPT_DIR}/" "${WORKSPACE}/"
fi
chown -R "${AGENT_USER}:${AGENT_USER}" "${WORKSPACE}"

if [[ -f "${WORKSPACE}/scripts/setup-cloud-agent-toolchain.sh" ]]; then
  echo "==> Project toolchain (nvm, Node per .nvmrc)"
  sudo -u "${AGENT_USER}" bash -lc "bash '${WORKSPACE}/scripts/setup-cloud-agent-toolchain.sh'"
fi

if [[ -d "${WORKSPACE}/frontend-dapp" ]]; then
  echo "==> Frontend dependencies"
  _agent_nvm_sh "cd '${WORKSPACE}/frontend-dapp' && npm ci"

  # Browsers land in ~/.cache/ms-playwright (default). Install from frontend-dapp so
  # versions match package-lock (@playwright/test). Includes chromium_headless_shell.
  echo "==> Frontend Playwright browsers"
  if [[ -f "${WORKSPACE}/scripts/with-node.sh" ]]; then
    _agent_nvm_sh "
      export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64
      bash '${WORKSPACE}/scripts/with-node.sh' --cwd frontend-dapp -- npx playwright install
    "
  else
    _agent_nvm_sh "
      export PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64
      cd '${WORKSPACE}/frontend-dapp' && npx playwright install
    "
  fi
fi

echo "==> Golden image finalize (Cursor agent)"
if [[ ! -f "${FINALIZE_PROMPT}" ]]; then
  echo "ERROR: missing ${FINALIZE_PROMPT} in the project repo." >&2
  exit 1
fi
if [[ -z "${CURSOR_API_KEY:-}" ]]; then
  echo "ERROR: export CURSOR_API_KEY before running setup (needed for golden-image finalize agent)." >&2
  exit 1
fi
if ! pgrep -x Xvfb >/dev/null 2>&1; then
  Xvfb :99 -screen 0 1920x1080x24 &
  sleep 1
fi
sudo -u "${AGENT_USER}" env \
  CURSOR_API_KEY="${CURSOR_API_KEY}" \
  DISPLAY=:99 \
  PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64 \
  bash -lc "
    set -euo pipefail
    cd '${WORKSPACE}'
    agent -p \"\$(cat '${FINALIZE_PROMPT}')\" \
      --model '${GCH_GOLDEN_IMAGE_MODEL}' \
      --force --trust \
      --workspace '${WORKSPACE}' \
      --output-format text
  "

echo "Setup complete. Review ${AGENT_HOME}/.gch/golden-image-verify.log, then run pre-snapshot cleanup."
