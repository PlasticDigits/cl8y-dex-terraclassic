#!/usr/bin/env bash
# Public release inputs only; encrypted keyring custody remains in the terminal.
set -euo pipefail
ROUTE_OPS_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
ROUTE_OPS_STATE="${ROUTE_OPS_STATE:-$HOME/.local/state/cl8y-dex/release-20261006-route}"
export ROUTE_OPS_REPO ROUTE_OPS_STATE
python3 - "$ROUTE_OPS_REPO" <<'PY'
import hashlib, pathlib, sys
root = pathlib.Path(sys.argv[1])
expected = [
    ('cl8y_dex_factory.wasm', '849400a805d90976b93260ac23107887ae36b5a40127bc6b489bb2d12151ede9'),
    ('cl8y_dex_pair.wasm', 'eef6a38ec8a7bba402644c808ac192b0928bb6e9cd67bebf4b7835abce236955'),
    ('cl8y_dex_router.wasm', 'fcb38d8532c94f97905cf03a86e323f87a468be365e627be16e89335f5f739cb'),
    ('cl8y_community_tax_token.wasm', '64f94a96b294c0396c7bf630ddba74e8891d7f76bd3059cc21e7c69ceac2cd15'),
    ('cl8y_community_tax_autolp.wasm', '829a4355647f28a1b0e2ea21bfdc786179a91718e3e1bc62ea036ce5df43613c'),
]
manifest = dict((name.lstrip('*'), digest) for digest, name in
                (line.split() for line in (root/'smartcontracts/artifacts/checksums.txt').read_text().splitlines()))
for name, digest in expected:
    assert manifest[name] == digest, f'Reviewed checksum differs: {name}'
    assert hashlib.sha256((root/'smartcontracts/artifacts'/name).read_bytes()).hexdigest() == digest, f'Artifact differs: {name}'
sys.path.insert(0, str(root/'scripts/lib'))
from route_tax_rollout import SPECS
assert SPECS['router'][1] == '1.5.0' and SPECS['token'][1] == '1.4.0', 'Rollout engine is stale'
PY
route_ops() {
  python3 "$ROUTE_OPS_REPO/scripts/lib/route_tax_rollout_cli.py" \
    --artifacts "$ROUTE_OPS_REPO/smartcontracts/artifacts" \
    --checksums "$ROUTE_OPS_REPO/smartcontracts/artifacts/checksums.txt" \
    --autolp-wasm "$ROUTE_OPS_REPO/smartcontracts/artifacts/cl8y_community_tax_autolp.wasm" \
    --autolp-sha256 829a4355647f28a1b0e2ea21bfdc786179a91718e3e1bc62ea036ce5df43613c \
    --state-dir "$ROUTE_OPS_STATE" \
    --chain-id columbus-5 \
    --node https://terra-classic-rpc.publicnode.com:443 \
    --lcd https://terra-classic-lcd.publicnode.com \
    --home "$HOME/.terra" --keyring-backend file \
    --store-key cl8ydeploy --multisig-key multisig_2of3 \
    --signer1 multisig1 --signer2 multisig2 --admin-key cl8y2_admin \
    --max-total-fee-uluna 5000000000 "$@"
}
