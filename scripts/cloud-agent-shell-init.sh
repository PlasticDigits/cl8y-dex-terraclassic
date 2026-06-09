#!/usr/bin/env bash
# Lightweight per-shell Cloud Agent init (sourced from ~/.bashrc).
# Full provisioning runs via scripts/setup-cloud-agent-env.sh on VM startup.
# Do not enable errexit here — this file is dotted into interactive shells.

_REPO_ROOT="${CLOUD_AGENT_REPO_ROOT:-/workspace}"
_ENV_GLIB="${_REPO_ROOT}/.env.glab"
_ENV_GIT="${_REPO_ROOT}/.env.git"
_LIB="${_REPO_ROOT}/scripts/lib/cloud-agent-env.sh"
_TOOLCHAIN="${_REPO_ROOT}/scripts/lib/cloud-agent-toolchain.sh"

if [[ -f "$_ENV_GIT" ]]; then
  # shellcheck disable=SC1090
  . "$_ENV_GIT"
fi

if [[ -f "$_ENV_GLIB" ]]; then
  # shellcheck disable=SC1090
  . "$_ENV_GLIB"
fi

if [[ -d "$_REPO_ROOT/.githooks" ]]; then
  git -C "$_REPO_ROOT" config core.hooksPath .githooks 2>/dev/null || true
fi

if [[ -f "$_LIB" ]]; then
  # shellcheck disable=SC1090
  . "$_LIB"
  cloud_agent_configure_git_identity 2>/dev/null || true
fi

# Prefer nvm Node from .nvmrc over Cloud VM /exec-daemon/node (v22).
if [[ -f "$_TOOLCHAIN" && -f "${_REPO_ROOT}/.nvmrc" && -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]]; then
  # shellcheck disable=SC1090
  . "$_TOOLCHAIN"
  if bin_dir="$(cloud_agent_node_bin_dir "$_REPO_ROOT" 2>/dev/null)"; then
    PATH="${bin_dir}:$(cloud_agent_strip_exec_daemon_from_path "$PATH")"
    export PATH
  fi
fi

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"
