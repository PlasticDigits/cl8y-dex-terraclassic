#!/usr/bin/env bash
# Scan git-tracked files only (excludes build artifacts; BIP39 fixture phrase allowlisted in .gitleaks.toml).
# Prevents global build-dir allowlists from bypassing mandatory gitleaks (#380 / M-13).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${GITLEAKS_SCAN_ROOT:-$(cd "${SCRIPT_DIR}/../.." && pwd)}"
GITLEAKS_IMAGE="${GITLEAKS_IMAGE:-ghcr.io/gitleaks/gitleaks:v8.24.2}"
CONFIG="${GITLEAKS_CONFIG:-${ROOT}/.gitleaks.toml}"

# gitleaks useDefault inherits a global path allowlist for node_modules/ that cannot be
# removed via TOML. Remap tracked dependency-tree paths so force-committed secrets still fail CI.
ci_stage_dest_path() {
  local path="$1"
  if [[ "$path" == */node_modules/* || "$path" == node_modules/* ]]; then
    printf '%s\n' "_gitleaks-tracked/${path//\/node_modules\//\/__tracked-nm__/}"
  else
    printf '%s\n' "$path"
  fi
}

stage_tracked_tree() {
  local dest="$1"
  cd "$ROOT"
  while IFS= read -r -d '' path; do
    local dest_path
    dest_path="$(ci_stage_dest_path "$path")"
    mkdir -p "${dest}/$(dirname "$dest_path")"
    cp -a "$path" "${dest}/${dest_path}"
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
