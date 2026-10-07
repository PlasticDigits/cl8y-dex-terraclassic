#!/usr/bin/env bash
# Stores/migrates the inventoried existing fleet; leaves affected markets paused.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/route-common.sh"
route_ops --execute
route_ops --verify
printf 'Contracts migrated and contained. Apply the public pins from %s/deployment.env to the indexer/frontend before pause restoration.\n' "$ROUTE_OPS_STATE"
