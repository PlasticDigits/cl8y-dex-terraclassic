#!/usr/bin/env bash
# Shared Cloud Agent GitLab / git identity helpers.
# Sourced by setup-cloud-agent-env.sh and cloud-agent-shell-init.sh.
# Callers that need strict mode (e.g. setup-cloud-agent-env.sh) set -euo pipefail themselves.

CLOUD_AGENT_GIT_USER_NAME=""
CLOUD_AGENT_GIT_USER_EMAIL=""
CLOUD_AGENT_GIT_ENV_FILE="${CLOUD_AGENT_GIT_ENV_FILE:-.env.git}"

cloud_agent_is_bot_git_identity() {
  local name="${1:-}"
  local email="${2:-}"
  case "$name" in
    project_*_bot_* | cursoragent | *Cursor* | *Codex* | *Copilot* | *Claude*)
      return 0
      ;;
  esac
  case "$email" in
    *@noreply.gitlab.com | *cursor* | *noreply*agent*)
      return 0
      ;;
  esac
  return 1
}

cloud_agent_resolve_git_identity() {
  local name="${GIT_USERNAME:-}"
  local email="${GIT_EMAIL:-}"

  if [[ -z "$name" ]]; then
    echo "[cloud-agent-env] GIT_USERNAME is not set" >&2
    return 1
  fi
  if [[ -z "$email" ]]; then
    echo "[cloud-agent-env] GIT_EMAIL is not set" >&2
    return 1
  fi
  if cloud_agent_is_bot_git_identity "$name" "$email"; then
    echo "[cloud-agent-env] GIT_USERNAME/GIT_EMAIL look like a bot or service account" >&2
    return 1
  fi

  CLOUD_AGENT_GIT_USER_NAME="$name"
  CLOUD_AGENT_GIT_USER_EMAIL="$email"
  return 0
}

cloud_agent_require_git_identity() {
  if ! cloud_agent_resolve_git_identity; then
    echo "[cloud-agent-env] Configure GIT_USERNAME and GIT_EMAIL in Cursor Cloud Agent secrets." >&2
    exit 1
  fi
}

cloud_agent_write_git_env_file() {
  local repo_root="${1:-}"
  local env_file="${repo_root}/${CLOUD_AGENT_GIT_ENV_FILE}"
  cloud_agent_resolve_git_identity || return 1
  mkdir -p "$(dirname "$env_file")"
  # shellcheck disable=SC2016
  printf 'export GIT_USERNAME=%q\nexport GIT_EMAIL=%q\n' \
    "$CLOUD_AGENT_GIT_USER_NAME" "$CLOUD_AGENT_GIT_USER_EMAIL" >"$env_file"
}

cloud_agent_configure_git_identity() {
  cloud_agent_resolve_git_identity || return 1
  git config --global user.name "$CLOUD_AGENT_GIT_USER_NAME"
  git config --global user.email "$CLOUD_AGENT_GIT_USER_EMAIL"
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git config user.name "$CLOUD_AGENT_GIT_USER_NAME"
    git config user.email "$CLOUD_AGENT_GIT_USER_EMAIL"
  fi
}

cloud_agent_verify_git_identity() {
  local name email global_name global_email
  cloud_agent_resolve_git_identity || return 1
  name="$(git config user.name 2>/dev/null || true)"
  email="$(git config user.email 2>/dev/null || true)"
  global_name="$(git config --global user.name 2>/dev/null || true)"
  global_email="$(git config --global user.email 2>/dev/null || true)"

  if [[ "$name" != "$CLOUD_AGENT_GIT_USER_NAME" || "$email" != "$CLOUD_AGENT_GIT_USER_EMAIL" ]]; then
    echo "[cloud-agent-env] git identity mismatch (local): name='${name}' email='${email}'" >&2
    return 1
  fi
  if [[ "$global_name" != "$CLOUD_AGENT_GIT_USER_NAME" || "$global_email" != "$CLOUD_AGENT_GIT_USER_EMAIL" ]]; then
    echo "[cloud-agent-env] git identity mismatch (global): name='${global_name}' email='${global_email}'" >&2
    return 1
  fi
  if cloud_agent_is_bot_git_identity "$name" "$email"; then
    echo "[cloud-agent-env] git identity still looks like a bot/service account" >&2
    return 1
  fi
  return 0
}

cloud_agent_verify_glab() {
  if ! command -v glab >/dev/null 2>&1; then
    echo "[cloud-agent-env] glab not installed" >&2
    return 1
  fi
  if [[ -z "${GITLAB_TOKEN:-}" ]]; then
    echo "[cloud-agent-env] GITLAB_TOKEN is not set" >&2
    return 1
  fi
  if [[ -z "${GITLAB_REPO:-}" ]]; then
    echo "[cloud-agent-env] GITLAB_REPO is not set (source .env.glab)" >&2
    return 1
  fi
  if ! glab api "projects/$(printf '%s' "$GITLAB_REPO" | sed 's|/|%2F|g')" >/dev/null 2>&1; then
    echo "[cloud-agent-env] glab API check failed for ${GITLAB_REPO}" >&2
    return 1
  fi
  return 0
}
