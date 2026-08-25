#!/usr/bin/env bash
# Assemble a drop-in tree for a keplr-contract-registry fork (GitLab #629).
# Usage: ./scripts/qa/export-keplr-cw20-pack.sh DEST [--include-registered]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PACK="$REPO_ROOT/docs/listings/keplr-contract-registry"
INCLUDE_REGISTERED=0
DEST=""

for arg in "$@"; do
  case "$arg" in
    --include-registered) INCLUDE_REGISTERED=1 ;;
    --help|-h)
      echo "Usage: $0 DEST [--include-registered]"
      exit 0
      ;;
    *)
      if [[ -z "$DEST" ]]; then
        DEST="$arg"
      else
        echo "unexpected argument: $arg" >&2
        exit 2
      fi
      ;;
  esac
done

if [[ -z "$DEST" ]]; then
  echo "DEST directory required" >&2
  exit 2
fi

python3 "$REPO_ROOT/scripts/qa/keplr_cw20_registry_validate.py"

# USTR is already live upstream — omit unless the operator wants a verify copy.
USTR=terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv

mkdir -p "$DEST/cosmos/columbus/tokens" "$DEST/images/columbus"
for src in "$PACK/cosmos/columbus/tokens/"*.json; do
  base="$(basename "$src")"
  if [[ "$base" == "${USTR}.json" && "$INCLUDE_REGISTERED" -eq 0 ]]; then
    echo "skip already-registered $base"
    continue
  fi
  cp "$src" "$DEST/cosmos/columbus/tokens/"
  image_file="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['imageUrl'].rsplit('/',1)[1])" "$src")"
  src_png="$REPO_ROOT/tokenlist/images/$image_file"
  if [[ ! -f "$src_png" ]]; then
    echo "missing $src_png" >&2
    exit 1
  fi
  cp "$src_png" "$DEST/images/columbus/$image_file"
done

echo "exported Keplr CW20 pack → $DEST"
find "$DEST" -type f | sort
