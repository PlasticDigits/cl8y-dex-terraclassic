#!/usr/bin/env bash
# Static check: CL8Y DEX CosmWasm contracts must not expose IBC hook entry points (SEC-D02 / #407).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACTS_DIR="${REPO_ROOT}/smartcontracts/contracts"

if [[ ! -d "$CONTRACTS_DIR" ]]; then
  echo "ERROR: missing contracts dir: $CONTRACTS_DIR" >&2
  exit 1
fi

# CosmWasm IBC callback surface — grep is the deploy-time gate documented in launch-checklist.md.
PATTERNS=(
  'ibc_receive'
  'ibc_ack'
  'ibc_timeout'
  'IbcChannel'
  'ibc_packet'
)

found=0
for pattern in "${PATTERNS[@]}"; do
  if rg -n --glob '*.rs' "$pattern" "$CONTRACTS_DIR" >/tmp/ibc-hooks-grep.txt 2>/dev/null; then
    echo "ERROR: IBC hook pattern '$pattern' found in smartcontracts/contracts/:" >&2
    cat /tmp/ibc-hooks-grep.txt >&2
    found=1
  fi
done

if [[ "$found" -ne 0 ]]; then
  echo "FAIL: app contracts must not expose IBC receive/ack/timeout entry points (SEC-D02)" >&2
  exit 1
fi

echo "OK: no IBC hook entry-point patterns in smartcontracts/contracts/ (SEC-D02)"
