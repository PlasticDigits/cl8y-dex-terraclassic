#!/usr/bin/env bash
# Mint ~$10k oracle TVL of cLUNC/cUSTC v2 LP at the CURRENT pool ratio (no swap),
# send LP to the CMM treasury, then holder-burn leftover wrap on the ops wallet.
#
# Does NOT rebalance. Keeps a cLUNC premium so selling LUNC into the pool stays
# attractive. "$5k+$5k" is ~$10k total oracle TVL; USD legs follow the live
# ratio and are not 50/50 while the pool is off peg.
#
# For an on-oracle deepen ($5k each side after swapping to LUNC/USTC), use
# scripts/rebalance-mint-clunc-custc-lp.sh instead.
#
# Usage:
#   DRY_RUN=1 ./scripts/mint-clunc-custc-lp.sh
#   CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
#   CLUNC_LP_ADD_USD=10000  # default; add this much oracle TVL at live ratio
#   CLUNC_LP_BURN_ONLY=1 CLUNC_LP_YES=1 ./scripts/mint-clunc-custc-lp.sh
#
# Unlock once (non-interactive):
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#
# Never commit TERRAD_HOST_KEYRING_PASS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi

export CLUNC_LP_SKIP_SWAP=1
export CLUNC_LP_ADD_USD="${CLUNC_LP_ADD_USD:-10000}"
exec "$SCRIPT_DIR/rebalance-mint-clunc-custc-lp.sh" "$@"
