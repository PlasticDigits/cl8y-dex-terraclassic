#!/usr/bin/env bash
# Install and configure GitLab CLI (glab) on Cloud Agent VMs.
#
# Cloud Agent checkouts use git remotes like:
#   https://x-access-token:…@gitlab.com/PlasticDigits/<repo>.git
# glab then fails project auto-detection (404) unless GITLAB_REPO is set or
# remote.origin_url is overridden. This script is idempotent.
#
# Usage (from repo root):
#   ./scripts/setup-glab-cloud-agent.sh
#   source .env.glab   # optional: default repo for glab in this shell
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

GLAB_VERSION="${GLAB_VERSION:-1.101.0}"
GLAB_ENV_FILE="${REPO_ROOT}/.env.glab"

install_glab() {
  if command -v glab >/dev/null 2>&1; then
    return 0
  fi
  echo "[setup-glab] installing glab v${GLAB_VERSION}…"
  local arch glab_arch tarball
  arch="$(uname -m)"
  case "$arch" in
    x86_64) glab_arch="amd64" ;;
    aarch64 | arm64) glab_arch="arm64" ;;
    *)
      echo "[setup-glab] unsupported architecture: ${arch}" >&2
      exit 1
      ;;
  esac
  tarball="glab_${GLAB_VERSION}_linux_${glab_arch}.tar.gz"
  curl -fsSL \
    "https://gitlab.com/gitlab-org/cli/-/releases/v${GLAB_VERSION}/downloads/${tarball}" \
    -o "/tmp/${tarball}"
  tar -xzf "/tmp/${tarball}" -C /tmp
  sudo install -m 0755 "/tmp/bin/glab" /usr/local/bin/glab
  rm -f "/tmp/${tarball}"
}

# Strip credentials; return https://gitlab.com/Group/repo.git
clean_gitlab_remote_url() {
  local raw="$1"
  raw="${raw#git@}" # ssh form handled below
  if [[ "$raw" == git@* ]]; then
    raw="${raw#git@}"
    raw="https://${raw/://}"
    raw="${raw%.git}"
    raw="${raw}.git"
  fi
  raw="$(printf '%s' "$raw" | sed -E 's#https://[^@]+@#https://#')"
  raw="${raw%/}"
  raw="${raw%.git}"
  printf '%s\n' "$raw"
}

# PlasticDigits/cl8y-dex-terraclassic from remote URL
gitlab_project_path_from_remote() {
  local url="$1"
  url="${url%.git}"
  url="${url%/}"
  if [[ "$url" =~ gitlab\.com[:/](.+)$ ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi
  return 1
}

if [[ -z "${GITLAB_TOKEN:-}" ]]; then
  echo "[setup-glab] GITLAB_TOKEN is not set; export it before running this script." >&2
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "[setup-glab] must run inside a git repository." >&2
  exit 1
fi

install_glab

origin_url="$(git remote get-url origin 2>/dev/null || true)"
if [[ -z "$origin_url" ]]; then
  echo "[setup-glab] no git remote named 'origin'." >&2
  exit 1
fi

clean_url="$(clean_gitlab_remote_url "$origin_url")"
project_path="$(gitlab_project_path_from_remote "$clean_url")" || {
  echo "[setup-glab] could not parse GitLab project path from: ${clean_url}" >&2
  exit 1
}

echo "[setup-glab] project: ${project_path}"
echo "[setup-glab] remote.origin_url: ${clean_url}"

glab auth login --hostname gitlab.com --token "$GITLAB_TOKEN" >/dev/null 2>&1 \
  || glab auth login --hostname gitlab.com --token "$GITLAB_TOKEN"

glab config set remote.origin_url "$clean_url"

mkdir -p "$(dirname "$GLAB_ENV_FILE")"
# shellcheck disable=SC2016
printf 'export GITLAB_REPO=%s\n' "$project_path" >"$GLAB_ENV_FILE"

export GITLAB_REPO="$project_path"

if ! glab api "projects/$(printf '%s' "$project_path" | sed 's|/|%2F|g')" >/dev/null 2>&1; then
  echo "[setup-glab] API check failed for project ${project_path} (token or path?)" >&2
  exit 1
fi

if ! GITLAB_REPO="$project_path" glab issue list --per-page 1 >/dev/null 2>&1; then
  echo "[setup-glab] warn: glab issue list failed; ensure GITLAB_REPO is exported in your shell." >&2
  exit 1
fi

echo "[setup-glab] OK — glab $(glab version 2>/dev/null | head -1)"
echo "[setup-glab] wrote ${GLAB_ENV_FILE} (gitignored); run: source .env.glab"
