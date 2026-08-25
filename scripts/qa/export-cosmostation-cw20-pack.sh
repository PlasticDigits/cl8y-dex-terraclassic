#!/usr/bin/env bash
# Assemble a drop-in tree for a cosmostation/chainlist fork (GitLab #640).
# Usage: ./scripts/qa/export-cosmostation-cw20-pack.sh DEST
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACK="$REPO_ROOT/docs/listings/cosmostation"
DEST="${1:-}"

if [[ -z "$DEST" || "$DEST" == "-h" || "$DEST" == "--help" ]]; then
  echo "Usage: $0 DEST" >&2
  exit 2
fi

python3 "$REPO_ROOT/scripts/qa/cosmostation_cw20_validate.py"

mkdir -p "$DEST/chain/terra/asset"
cp "$PACK/cw20_2.fragment.json" "$DEST/chain/terra/cw20_2.fragment.json"
cp "$PACK/asset/"*.png "$DEST/chain/terra/asset/"

echo "exported Cosmostation CW20 pack → $DEST"
find "$DEST" -type f | sort
