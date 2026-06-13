#!/usr/bin/env bash
# Scan git-tracked files only (excludes build artifacts and the gitleaks fixture tree).
# Prevents global build-dir allowlists from bypassing mandatory gitleaks (#380 / M-13).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GITLEAKS_IMAGE="${GITLEAKS_IMAGE:-ghcr.io/gitleaks/gitleaks:v8.24.2}"
CONFIG="${ROOT}/.gitleaks.toml"
FIXTURE_PREFIX="scripts/ci/gitleaks-fixture/"

stage_tracked_tree() {
  local dest="$1"
  cd "$ROOT"
  while IFS= read -r -d '' path; do
    [[ "$path" == "${FIXTURE_PREFIX}"* ]] && continue
    mkdir -p "${dest}/$(dirname "$path")"
    cp -a "$path" "${dest}/${path}"
  done < <(git -C "$ROOT" ls-files -z)
}

run_gitleaks() {
  local source_dir="$1"
  shift
  if command -v gitleaks >/dev/null 2>&1; then
    gitleaks detect --source "$source_dir" --no-git --config "$CONFIG" --redact "$@"
  else
    docker run --rm \
      -v "${ROOT}:${ROOT}" \
      -v "${source_dir}:${source_dir}" \
      -w "$ROOT" --entrypoint gitleaks \
      "$GITLEAKS_IMAGE" detect --source "$source_dir" --no-git --config "$CONFIG" --redact "$@"
  fi
}

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
stage_tracked_tree "$tmpdir"
run_gitleaks "$tmpdir" "$@"
