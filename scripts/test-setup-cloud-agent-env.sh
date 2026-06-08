#!/usr/bin/env bash
# Static checks for Cloud Agent startup / GitLab env setup.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

_assert_file_contains() {
  local file=$1 pattern=$2 msg=$3
  grep -q "$pattern" "$file" || _fail "$msg"
}

_assert_file_contains "$REPO_ROOT/.cursor/environment.json" \
  'setup-cloud-agent-env.sh' \
  'environment.json must run setup-cloud-agent-env.sh on install'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'GITLAB_TOKEN is not set' \
  'setup-cloud-agent-env.sh must require GITLAB_TOKEN'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'cloud_agent_require_git_identity' \
  'setup-cloud-agent-env.sh must require GIT_USERNAME/GIT_EMAIL'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'setup-glab-cloud-agent.sh' \
  'setup-cloud-agent-env.sh must invoke setup-glab-cloud-agent.sh'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'GIT_USERNAME' \
  'cloud-agent-env lib must read GIT_USERNAME'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'GIT_EMAIL' \
  'cloud-agent-env lib must read GIT_EMAIL'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'cloud_agent_is_bot_git_identity' \
  'cloud-agent-env lib must detect bot git identities'

_assert_file_contains "$REPO_ROOT/scripts/cloud-agent-shell-init.sh" \
  '.env.git' \
  'shell init must source .env.git'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'cloud-agent-shell-init.sh' \
  'setup-cloud-agent-env.sh must install shell init hook'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'cloud_agent_ensure_vm_toolchain' \
  'setup-cloud-agent-env.sh must provision VM toolchain'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'cloud_agent_ensure_dockerd' \
  'setup-cloud-agent-env.sh must ensure dockerd'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-toolchain.sh" \
  'cloud_agent_strip_exec_daemon_from_path' \
  'toolchain lib must strip /exec-daemon from PATH'

_assert_file_contains "$REPO_ROOT/scripts/with-node.sh" \
  'cloud-agent-toolchain.sh' \
  'with-node.sh must use cloud-agent-toolchain helpers'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-docker.sh" \
  'cloud_agent_install_docker' \
  'docker lib must install Docker CE when missing'

_assert_file_contains "$REPO_ROOT/scripts/cloud-agent-shell-init.sh" \
  'cloud-agent-toolchain.sh' \
  'shell init must prefer nvm node over /exec-daemon'

_assert_file_contains "$REPO_ROOT/AGENTS.md" \
  'GIT_USERNAME' \
  'AGENTS.md must document GIT_USERNAME'

_assert_file_contains "$REPO_ROOT/AGENTS.md" \
  'GIT_EMAIL' \
  'AGENTS.md must document GIT_EMAIL'

# shellcheck source=scripts/lib/cloud-agent-env.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-env.sh"

if cloud_agent_is_bot_git_identity 'project_80162261_bot_cd5edbc1' 'project_80162261_bot_cd5edbc1@noreply.gitlab.com'; then
  :
else
  _fail 'must detect GitLab project bot identity'
fi

export GIT_USERNAME='PlasticDigits'
export GIT_EMAIL='plasticdigits@protonmail.com'
if cloud_agent_is_bot_git_identity "$GIT_USERNAME" "$GIT_EMAIL"; then
  _fail 'must not flag valid GIT_USERNAME/GIT_EMAIL as bot'
fi
if ! cloud_agent_resolve_git_identity; then
  _fail 'must resolve identity when GIT_USERNAME/GIT_EMAIL are set'
fi
unset GIT_USERNAME GIT_EMAIL
if cloud_agent_resolve_git_identity 2>/dev/null; then
  _fail 'must reject missing GIT_USERNAME/GIT_EMAIL'
fi

echo "OK: setup-cloud-agent-env static checks"
