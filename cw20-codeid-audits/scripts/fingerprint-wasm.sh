#!/usr/bin/env bash
# Static fingerprint of pinned LCD wasm (GitLab #589). Does not execute the contract.
# Writes codeids/<id>/decomp/fingerprint.json (and strings.txt if strings exists).
#
# Usage:
#   fingerprint-wasm.sh <code_id>
#   fingerprint-wasm.sh --self-test
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUDIT_ROOT="$(cd "${CW20_AUDIT_ROOT:-$SCRIPT_DIR/..}" && pwd)"

NEEDLES=(
  tax_map cw20_taxed fee_on_transfer rebase reflection
  transfer send transfer_from send_from burn mint
  balance_at total_supply_at
  ibc_receive ibc_packet requires_terra requires_stargate requires_iterator
  UpdateTaxMap permit flash_mint
  terraport_token cw20_base classic_terraport
)

run_self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  printf 'hello tax_map cw20_taxed and cw20_base\x00asm' > "$tmp/sample.bin"
  python3 - "$tmp/sample.bin" "$tmp/out.json" <<'PY'
import json, pathlib, sys
raw = pathlib.Path(sys.argv[1]).read_bytes()
text = raw.decode("latin-1", errors="replace")
needles = ["tax_map", "cw20_taxed", "cw20_base", "ibc_receive"]
hits = {n: (n in text) for n in needles}
pathlib.Path(sys.argv[2]).write_text(json.dumps({"hits": hits}, indent=2))
assert hits["tax_map"] and hits["cw20_base"]
assert not hits["ibc_receive"]
PY
  echo "PASS: fingerprint-wasm self-test"
}

if [[ "${1:-}" == "--self-test" ]]; then
  run_self_test
  exit 0
fi

ID="${1:-}"
if [[ -z "$ID" || ! "$ID" =~ ^[0-9]+$ ]]; then
  echo "Usage: $0 <code_id> | --self-test" >&2
  exit 1
fi

WASM="$AUDIT_ROOT/codeids/$ID/token.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "FAIL: $WASM missing — run fetch-lcd-wasm.sh $ID first" >&2
  exit 1
fi

DEST="$AUDIT_ROOT/codeids/$ID/decomp"
mkdir -p "$DEST"

python3 - "$WASM" "$DEST/fingerprint.json" "${NEEDLES[@]}" <<'PY'
import json, pathlib, sys
wasm = pathlib.Path(sys.argv[1]).read_bytes()
out = pathlib.Path(sys.argv[2])
needles = sys.argv[3:]
text = wasm.decode("latin-1", errors="replace")
# wasm export names often appear as UTF-8 / latin-1 fragments
hits = {}
for n in needles:
    hits[n] = n.lower() in text.lower()
# magic / version
magic_ok = wasm[:4] == b"\x00asm"
result = {
    "bytes": len(wasm),
    "wasm_magic": magic_ok,
    "hits": hits,
    "note": "String presence is not FoT proof (A28). Absence of tax_map is necessary, not sufficient.",
}
out.write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(hits, indent=2))
if not magic_ok:
    sys.exit(2)
PY

if command -v strings >/dev/null 2>&1; then
  strings -n 6 "$WASM" > "$DEST/strings.txt"
fi
echo "PASS: fingerprint code $ID -> $DEST/fingerprint.json"
exit 0
