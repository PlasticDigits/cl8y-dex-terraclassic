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

# shellcheck source=scripts/lib/cloud-agent-toolchain.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-toolchain.sh"

load_nvm_node() {
  cloud_agent_ensure_node "$REPO_ROOT"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cwd)
      CWD_REL="${2:?--cwd requires a path}"
      shift 2
      ;;
    --print-env)
      cloud_agent_node_path_export "$REPO_ROOT"
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

load_nvm_node
# shellcheck disable=SC2164
cd "$REPO_ROOT/$CWD_REL"

if [[ $# -eq 0 ]]; then
  echo "with-node: missing command after --" >&2
  exit 2
fi

exec "$@"
