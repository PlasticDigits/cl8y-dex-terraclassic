#!/usr/bin/env bash
# Cloud Agent VM startup / install script.
#
# Cursor runs this from .cursor/environment.json on every VM boot (after git pull).
# Must be idempotent. Ensures glab, GITLAB_TOKEN, GIT_USERNAME, GIT_EMAIL,
# git identity, VM toolchain (Docker, Node), Chrome + Keplr, Playwright, and
# LocalTerra stack — overwriting the default GitLab project-bot clone identity.
#
# Usage (from repo root):
#   ./scripts/setup-cloud-agent-env.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/cloud-agent-env.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-env.sh"

BASHRC="${HOME}/.bashrc"
BEGIN_MARKER='# >>> cl8y cloud-agent env >>>'
END_MARKER='# <<< cl8y cloud-agent env <<<'

install_shell_init() {
  local block tmp
  block="${BEGIN_MARKER}
# Managed by scripts/setup-cloud-agent-env.sh — do not edit
export CLOUD_AGENT_REPO_ROOT=\"${REPO_ROOT}\"
if [ -f \"${REPO_ROOT}/scripts/cloud-agent-shell-init.sh\" ]; then
  # shellcheck disable=SC1091
  . \"${REPO_ROOT}/scripts/cloud-agent-shell-init.sh\"
fi
${END_MARKER}"

  if [[ ! -f "$BASHRC" ]]; then
    printf '%s\n' "$block" >"$BASHRC"
    return 0
  fi

  # Drop legacy ad-hoc Cloud Agent block from earlier agent sessions.
  if grep -qF '# Cloud Agent GitLab / git defaults (PlasticDigits)' "$BASHRC" \
    && grep -qE '^if ! command -v glab' "$BASHRC" \
    && grep -qE '^fi$' "$BASHRC"; then
    tmp="$(mktemp)"
    awk '
        /^# Cloud Agent GitLab \/ git defaults \(PlasticDigits\)/ { skip=1; next }
        skip == 1 && /^if ! command -v glab/ { skip=2; next }
        skip == 2 && /^fi$/ { skip=0; next }
        skip == 2 { next }
        skip == 1 { next }
        { print }
    ' "$BASHRC" >"$tmp"
    mv "$tmp" "$BASHRC"
  fi

  if grep -qF "$BEGIN_MARKER" "$BASHRC"; then
    tmp="$(mktemp)"
    awk -v begin="$BEGIN_MARKER" -v end="$END_MARKER" -v block="$block" '
      $0 == begin { print block; skip=1; next }
      skip && $0 == end { skip=0; next }
      !skip { print }
    ' "$BASHRC" >"$tmp"
    mv "$tmp" "$BASHRC"
  else
    printf '\n%s\n' "$block" >>"$BASHRC"
  fi
}

echo "[cloud-agent-env] repo: ${REPO_ROOT}"

if [[ -z "${GITLAB_TOKEN:-}" ]]; then
  echo "[cloud-agent-env] ERROR: GITLAB_TOKEN is not set." >&2
  echo "[cloud-agent-env]        Configure GITLAB_TOKEN in Cursor Cloud Agent secrets." >&2
  exit 1
fi

cloud_agent_require_git_identity

echo "[cloud-agent-env] configuring git hooks…"
git config core.hooksPath .githooks 2>/dev/null || true

echo "[cloud-agent-env] configuring git identity from GIT_USERNAME / GIT_EMAIL…"
cloud_agent_configure_git_identity
cloud_agent_write_git_env_file "$REPO_ROOT"

echo "[cloud-agent-env] configuring glab…"
chmod +x "$REPO_ROOT/scripts/setup-glab-cloud-agent.sh"
"$REPO_ROOT/scripts/setup-glab-cloud-agent.sh"

if [[ -f "$REPO_ROOT/.env.glab" ]]; then
  # shellcheck disable=SC1091
  . "$REPO_ROOT/.env.glab"
fi

install_shell_init

echo "[cloud-agent-env] verifying git + glab…"
cloud_agent_verify_git_identity
cloud_agent_verify_glab

echo "[cloud-agent-env] VM toolchain (Docker, Node)…"
chmod +x "$REPO_ROOT/scripts/setup-cloud-agent-toolchain.sh" \
  "$REPO_ROOT/scripts/lib/cloud-agent-docker.sh"
if ! "$REPO_ROOT/scripts/setup-cloud-agent-toolchain.sh"; then
  echo "[cloud-agent-env] WARNING: toolchain setup incomplete (Docker/Node)." >&2
fi

echo "[cloud-agent-env] Chrome + Keplr…"
chmod +x "$REPO_ROOT/scripts/setup-browser-cloud-agent.sh" \
  "$REPO_ROOT/scripts/lib/keplr-chrome-extension.sh"
"$REPO_ROOT/scripts/setup-browser-cloud-agent.sh"

echo "[cloud-agent-env] LocalTerra + Playwright + deploy…"
chmod +x "$REPO_ROOT/scripts/setup-cloud-agent-localterra.sh" \
  "$REPO_ROOT/scripts/setup-cloud-agent-indexer-postgres.sh"
if ! "$REPO_ROOT/scripts/setup-cloud-agent-localterra.sh"; then
  echo "[cloud-agent-env] ERROR: LocalTerra stack setup failed." >&2
  exit 1
fi

echo "[cloud-agent-env] OK"
echo "[cloud-agent-env]   glab:     $(command -v glab) ($(glab version 2>/dev/null | head -1))"
echo "[cloud-agent-env]   git name: $(git config user.name)"
echo "[cloud-agent-env]   git mail: $(git config user.email)"
echo "[cloud-agent-env]   GITLAB_REPO: ${GITLAB_REPO:-unset}"
