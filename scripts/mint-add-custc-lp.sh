#!/usr/bin/env bash
# Add oracle TVL at the live pool ratio (no price rebalance) and send LP to the
# ustr-cmm CMM treasury.
#
#   UST1/cUSTC   +$50,000  mint UST1 + cUSTC (DEX 2-of-3 extra minter)
#   cLUNC/cUSTC  +$20,000  mint cLUNC + cUSTC (primary minter; keeps cLUNC premium)
#
# UST1 is valued at $1. cUSTC and cLUNC use the indexer oracles. Legs follow the
# current reserves, so the two sides are not forced to equal USD.
#
# Usage:
#   DRY_RUN=1 ./scripts/mint-add-custc-lp.sh
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
#   UST1_LP_YES=1 CLUNC_LP_YES=1 ./scripts/mint-add-custc-lp.sh
#
# Overrides (set to 0 to skip that pair):
#   UST1_CUSTC_ADD_USD=50000
#   CLUNC_CUSTC_ADD_USD=20000
#
# Never commit TERRAD_HOST_KEYRING_PASS. File-keyring keys used by the child
# scripts (multisig_2of3, multisig1, multisig2, cl8ydeploy, cl8y2_admin) share
# one passphrase.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
  exit 0
fi

UST1_ADD="${UST1_CUSTC_ADD_USD:-50000}"
CLUNC_ADD="${CLUNC_CUSTC_ADD_USD:-20000}"

positive() {
  python3 -c "from decimal import Decimal; import sys; sys.exit(0 if Decimal(sys.argv[1]) > 0 else 1)" "$1"
}

echo "=============================================="
echo "Add cUSTC LP at live ratio → CMM"
echo "=============================================="
echo "UST1/cUSTC add USD:  $UST1_ADD"
echo "cLUNC/cUSTC add USD: $CLUNC_ADD"
echo "DRY_RUN:             ${DRY_RUN:-0}"
echo ""

if positive "$UST1_ADD"; then
  echo "-------- UST1/cUSTC +\$$UST1_ADD (no swap) --------"
  UST1_LP_PAIRS=custc \
  UST1_LP_SKIP_SWAP=1 \
  UST1_LP_USD_CUSTC="$UST1_ADD" \
  UST1_LP_USD_USTR=0 \
    "$SCRIPT_DIR/rebalance-mint-ust1-lp.sh"
else
  echo "skip UST1/cUSTC (UST1_CUSTC_ADD_USD=$UST1_ADD)"
fi

echo ""
if positive "$CLUNC_ADD"; then
  echo "-------- cLUNC/cUSTC +\$$CLUNC_ADD (no swap) --------"
  CLUNC_LP_ADD_USD="$CLUNC_ADD" \
    "$SCRIPT_DIR/mint-clunc-custc-lp.sh"
else
  echo "skip cLUNC/cUSTC (CLUNC_CUSTC_ADD_USD=$CLUNC_ADD)"
fi
