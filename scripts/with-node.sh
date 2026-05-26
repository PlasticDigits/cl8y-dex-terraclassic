#!/usr/bin/env bash
# Run commands with Node/npm from nvm per repo .nvmrc (default local toolchain).
#
# Usage:
#   bash scripts/with-node.sh -- npm run test:run
#   bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
#   eval "$(bash scripts/with-node.sh --print-env)"
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CWD_REL="."

load_nvm() {
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    . "$NVM_DIR/nvm.sh"
  elif command -v brew >/dev/null 2>&1; then
    local brew_nvm
    brew_nvm="$(brew --prefix nvm 2>/dev/null || true)"
    if [[ -n "$brew_nvm" && -s "$brew_nvm/nvm.sh" ]]; then
      # shellcheck source=/dev/null
      . "$brew_nvm/nvm.sh"
    fi
  fi
  if ! command -v nvm >/dev/null 2>&1; then
    echo "with-node: nvm not found. Install nvm and run: nvm install $(cat "$REPO_ROOT/.nvmrc")" >&2
    return 127
  fi
  # shellcheck disable=SC2164
  cd "$REPO_ROOT"
  nvm use --silent
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cwd)
      CWD_REL="${2:?--cwd requires a path}"
      shift 2
      ;;
    --print-env)
      load_nvm || exit $?
      printf 'export PATH="%s"\n' "$(dirname "$(command -v node)")"
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      echo "with-node: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

load_nvm || exit $?
# shellcheck disable=SC2164
cd "$REPO_ROOT/$CWD_REL"

if [[ $# -eq 0 ]]; then
  echo "with-node: missing command after --" >&2
  exit 2
fi

exec "$@"
