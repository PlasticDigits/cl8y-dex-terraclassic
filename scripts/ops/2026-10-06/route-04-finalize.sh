#!/usr/bin/env bash
# Run after matching application pins and live funded provider quotes are applied.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/route-common.sh"
route_ops --finalize
route_ops --verify
