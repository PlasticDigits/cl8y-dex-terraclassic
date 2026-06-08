#!/usr/bin/env bash
# Lightweight per-shell Cloud Agent init (sourced from ~/.bashrc).
# Full provisioning runs via scripts/setup-cloud-agent-env.sh on VM startup.
set -euo pipefail

_REPO_ROOT="${CLOUD_AGENT_REPO_ROOT:-/workspace}"
_ENV_GLIB="${_REPO_ROOT}/.env.glab"
_ENV_GIT="${_REPO_ROOT}/.env.git"
_LIB="${_REPO_ROOT}/scripts/lib/cloud-agent-env.sh"

if [[ -f "$_ENV_GIT" ]]; then
  # shellcheck disable=SC1090
  . "$_ENV_GIT"
fi

if [[ -f "$_ENV_GLIB" ]]; then
  # shellcheck disable=SC1090
  . "$_ENV_GLIB"
fi

if [[ -f "$_LIB" ]]; then
  # shellcheck disable=SC1090
  . "$_LIB"
  cloud_agent_configure_git_identity 2>/dev/null || true
fi
