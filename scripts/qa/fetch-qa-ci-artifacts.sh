#!/usr/bin/env bash
# fetch-qa-ci-artifacts — download prebuilt QA wasm tarball and/or indexer binary from GitLab generic packages.
# GitLab #325 — set QA_FETCH_CI_ARTIFACTS=1 before make start-qa to try cache hits.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

GIT_SHA="${1:-$(git rev-parse --short HEAD 2>/dev/null || true)}"
PROJECT="${GITLAB_REPO:-PlasticDigits/cl8y-dex-terraclassic}"
PROJECT_ENC="${PROJECT//\//%2F}"
API="${CI_API_V4_URL:-https://gitlab.com/api/v4}"
TOKEN="${GITLAB_TOKEN:-${CI_JOB_TOKEN:-}}"

if [ -z "$GIT_SHA" ]; then
  echo "[fetch-qa-artifacts] no git sha — skip" >&2
  exit 0
fi

if [ -z "$TOKEN" ]; then
  echo "[fetch-qa-artifacts] GITLAB_TOKEN unset — skip remote fetch" >&2
  exit 0
fi

_auth_header() {
  if [ -n "${CI_JOB_TOKEN:-}" ]; then
    printf 'JOB-TOKEN: %s' "$CI_JOB_TOKEN"
  else
    printf 'PRIVATE-TOKEN: %s' "$GITLAB_TOKEN"
  fi
}

_fetch_generic() {
  local pkg="$1" file="$2" dest="$3"
  local url="${API}/projects/${PROJECT_ENC}/packages/generic/${pkg}/${GIT_SHA}/${file}"
  if curl -fsSL -H "$(_auth_header)" "$url" -o "$dest"; then
    echo "[fetch-qa-artifacts] fetched ${pkg}/${GIT_SHA}/${file}"
    return 0
  fi
  return 1
}

WASM_TAR="${REPO_ROOT}/.qa-wasm-${GIT_SHA}.tar.gz"
INDEXER_BIN="${REPO_ROOT}/indexer/target/release/cl8y-dex-indexer"

if _fetch_generic "qa-wasm" "artifacts.tar.gz" "$WASM_TAR"; then
  mkdir -p "${REPO_ROOT}/smartcontracts/artifacts"
  tar xzf "$WASM_TAR" -C "${REPO_ROOT}/smartcontracts/artifacts" --strip-components=1 2>/dev/null \
    || tar xzf "$WASM_TAR" -C "${REPO_ROOT}/smartcontracts/artifacts"
  rm -f "$WASM_TAR"
fi

mkdir -p "${REPO_ROOT}/indexer/target/release"
if _fetch_generic "qa-indexer" "cl8y-dex-indexer" "$INDEXER_BIN"; then
  chmod +x "$INDEXER_BIN"
  export INDEXER_QA_BIN="$INDEXER_BIN"
  echo "[fetch-qa-artifacts] INDEXER_QA_BIN=$INDEXER_BIN"
fi
