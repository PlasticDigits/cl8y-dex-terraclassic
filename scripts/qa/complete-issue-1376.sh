#!/usr/bin/env bash
# Optional operator-only pair rollout; default is read-only. See issue-1376 QA.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
mode="${1:---check}"
case "$mode" in
  --check|--migrate|--dry-run) ;;
  *) echo 'Usage: complete-issue-1376.sh [--check|--migrate|--dry-run]' >&2; exit 2 ;;
esac
[[ $# -le 1 ]] || exit 2
export UPGRADE1324_PAIR_VERSION=1.19.0
export TERRAD_HOST_CHAIN_ID=columbus-5
export UPGRADE1324_FACTORY_ADDRESS=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea
export UPGRADE1324_PAIR_WASM="$PWD/smartcontracts/artifacts/cl8y_dex_pair.wasm"
if [[ "$mode" != --check ]]; then
  # Never substitute a newly built/different artifact into a reviewed release.
  printf '%s  %s\n' 4ad54cd3b9a5ff3d28b80bfec21800b39fee97ca9295bb524353db7590005bba "$UPGRADE1324_PAIR_WASM" | sha256sum --check --status
  git diff --quiet 3f9310652ac83818749f5d32c4e32057b08817bc -- smartcontracts
  if [[ "$mode" == --dry-run ]]; then
    DRY_RUN=1 bash scripts/upgrade-1324-pair-twap.sh
    exit 0
  fi
  if [[ "${UPGRADE1324_SKIP_STORE:-0}" == 1 ]]; then
    # Resume only the identical stored artifact, never an unchecked code ID.
    python3 - <<'PY'
import base64, hashlib, json, os, urllib.request
code = os.environ.get('UPGRADE1324_PAIR_CODE_ID', '')
if not code.isascii() or not code.isdigit():
    raise SystemExit('Resume requires a numeric UPGRADE1324_PAIR_CODE_ID')
lcd = os.environ.get('LCD_URL', os.environ.get('TERRA_LCD_URL', 'https://terra-classic-lcd.publicnode.com')).rstrip('/')
req = urllib.request.Request(lcd + '/cosmwasm/wasm/v1/code/' + code, headers={'User-Agent': 'curl/8.0'})
with urllib.request.urlopen(req, timeout=30) as response:
    stored = json.load(response)
if hashlib.sha256(base64.b64decode(stored['data'])).hexdigest() != '4ad54cd3b9a5ff3d28b80bfec21800b39fee97ca9295bb524353db7590005bba':
    raise SystemExit('Stored code checksum does not match the reviewed pair artifact')
PY
  fi
  # The existing tool verifies governance/admin, count, cw2 and observations.
  bash scripts/upgrade-1324-pair-twap.sh
fi
UPGRADE1324_PROBE_ONLY=1 bash scripts/upgrade-1324-pair-twap.sh
python3 scripts/qa/probe-issue-1376.py --expect-fixed
node scripts/qa/browser-probe-issue-1376.cjs --expect-fixed --matrix
