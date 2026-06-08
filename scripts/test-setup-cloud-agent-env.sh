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
  'setup-glab-cloud-agent.sh' \
  'setup-cloud-agent-env.sh must invoke setup-glab-cloud-agent.sh'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'PlasticDigits' \
  'cloud-agent-env lib must pin PlasticDigits git user.name'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'plasticdigits@protonmail.com' \
  'cloud-agent-env lib must pin plasticdigits@protonmail.com git email'

_assert_file_contains "$REPO_ROOT/scripts/lib/cloud-agent-env.sh" \
  'cloud_agent_is_bot_git_identity' \
  'cloud-agent-env lib must detect bot git identities'

_assert_file_contains "$REPO_ROOT/scripts/cloud-agent-shell-init.sh" \
  'cloud-agent-env.sh' \
  'shell init must source cloud-agent-env lib'

_assert_file_contains "$REPO_ROOT/scripts/setup-cloud-agent-env.sh" \
  'cloud-agent-shell-init.sh' \
  'setup-cloud-agent-env.sh must install shell init hook'

_assert_file_contains "$REPO_ROOT/AGENTS.md" \
  'setup-cloud-agent-env.sh' \
  'AGENTS.md must document setup-cloud-agent-env.sh'

_assert_file_contains "$REPO_ROOT/AGENTS.md" \
  'plasticdigits@protonmail.com' \
  'AGENTS.md must document PlasticDigits git email default'

# shellcheck source=scripts/lib/cloud-agent-env.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-env.sh"

if cloud_agent_is_bot_git_identity 'project_80162261_bot_cd5edbc1' 'project_80162261_bot_cd5edbc1@noreply.gitlab.com'; then
  :
else
  _fail 'must detect GitLab project bot identity'
fi

if cloud_agent_is_bot_git_identity 'PlasticDigits' 'plasticdigits@protonmail.com'; then
  _fail 'must not flag PlasticDigits identity as bot'
fi

echo "OK: setup-cloud-agent-env static checks"
