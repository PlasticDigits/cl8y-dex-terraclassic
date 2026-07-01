#!/bin/bash
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=lib/governance-multisig.sh
source "$SCRIPT_DIR/lib/governance-multisig.sh"
cd "$SCRIPT_DIR/../smartcontracts"
./scripts/deploy.sh mainnet "${1:-$GOVERNANCE_MULTISIG_ADDR}"
