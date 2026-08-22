#!/usr/bin/env bash
# Fetch CosmWasm LCD wasm and pin SHA-256 to CodeInfo.data_hash (GitLab #589).
# Fail closed on mismatch, truncation, missing ID, or endpoint disagreement (G4).
#
# Usage:
#   fetch-lcd-wasm.sh <code_id>
#   fetch-lcd-wasm.sh --self-test
#
# Env:
#   LCD_URL              default https://terra-classic-lcd.publicnode.com
#   LCD_URL_SECONDARY    optional second endpoint; hash must match
#   CW20_AUDIT_ROOT      default: repo cw20-codeid-audits/
#   FETCH_FORCE=1        re-download even if cache pin matches
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUDIT_ROOT="$(cd "${CW20_AUDIT_ROOT:-$SCRIPT_DIR/..}" && pwd)"
LCD_URL="${LCD_URL:-https://terra-classic-lcd.publicnode.com}"
LCD_URL="${LCD_URL%/}"

sha256_file() {
  sha256sum "$1" | awk '{print toupper($1)}'
}

write_meta() {
  local dest="$1"
  python3 - "$dest" <<'PY'
import json, sys
path = sys.argv[1]
meta = json.loads(sys.stdin.read())
with open(path, "w", encoding="utf-8") as f:
    json.dump(meta, f, indent=2)
    f.write("\n")
PY
}

fetch_one() {
  local lcd="$1" id="$2" out_json="$3"
  local url="${lcd}/cosmwasm/wasm/v1/code/${id}"
  if ! curl -fsS --max-time 60 "$url" -o "$out_json"; then
    echo "FAIL: LCD error fetching code ${id} from ${lcd}" >&2
    return 1
  fi
}

decode_and_pin() {
  local json_path="$1" wasm_path="$2"
  python3 - "$json_path" "$wasm_path" <<'PY'
import base64, hashlib, json, sys
jpath, wpath = sys.argv[1], sys.argv[2]
with open(jpath, encoding="utf-8") as f:
    body = json.load(f)
info = body.get("code_info") or {}
data_b64 = body.get("data")
if not data_b64:
    sys.stderr.write("FAIL: LCD JSON missing data (truncated or wrong endpoint)\n")
    sys.exit(2)
raw = base64.b64decode(data_b64)
if len(raw) < 8 or raw[:4] != b"\x00asm":
    sys.stderr.write("FAIL: downloaded bytes are not wasm\n")
    sys.exit(2)
digest = hashlib.sha256(raw).hexdigest().upper()
lcd_hash = (info.get("data_hash") or "").replace("0x", "").upper()
if not lcd_hash:
    sys.stderr.write("FAIL: CodeInfo.data_hash missing\n")
    sys.exit(2)
if digest != lcd_hash:
    sys.stderr.write(f"FAIL: sha256 {digest} != data_hash {lcd_hash} (C1)\n")
    sys.exit(3)
with open(wpath, "wb") as f:
    f.write(raw)
print(digest)
print(info.get("creator") or "")
print((info.get("instantiate_permission") or {}).get("permission") or "")
print(info.get("code_id") or "")
PY
}

run_self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  # Valid wasm header + padding
  printf '\x00asm\x01\x00\x00\x00' > "$tmp/ok.wasm"
  dd if=/dev/zero bs=64 count=1 >> "$tmp/ok.wasm" 2>/dev/null
  local good
  good="$(sha256_file "$tmp/ok.wasm")"
  python3 - "$tmp" "$good" <<'PY'
import base64, json, hashlib, sys, pathlib
tmp, good = sys.argv[1], sys.argv[2]
raw = pathlib.Path(tmp, "ok.wasm").read_bytes()
body = {
  "code_info": {"code_id": "1", "creator": "terra1test", "data_hash": good,
                "instantiate_permission": {"permission": "Everybody"}},
  "data": base64.b64encode(raw).decode(),
}
pathlib.Path(tmp, "good.json").write_text(json.dumps(body))
bad = dict(body)
bad["code_info"] = dict(body["code_info"])
bad["code_info"]["data_hash"] = "0" * 64
pathlib.Path(tmp, "bad.json").write_text(json.dumps(bad))
trunc = dict(body)
trunc["data"] = ""
pathlib.Path(tmp, "trunc.json").write_text(json.dumps(trunc))
garbage = dict(body)
garbage["data"] = base64.b64encode(b"not-wasm-xxxx").decode()
pathlib.Path(tmp, "garbage.json").write_text(json.dumps(garbage))
PY
  local out="$tmp/out.wasm"
  local digest
  digest="$(decode_and_pin "$tmp/good.json" "$out" | head -1)"
  [[ "$digest" == "$good" ]] || { echo "FAIL self-test: good pin" >&2; return 1; }
  if decode_and_pin "$tmp/bad.json" "$tmp/should-not.wasm" >/dev/null 2>"$tmp/err"; then
    echo "FAIL self-test: mismatch should fail closed (C1)" >&2
    return 1
  fi
  grep -q "sha256" "$tmp/err" || true
  if decode_and_pin "$tmp/trunc.json" "$tmp/should-not.wasm" >/dev/null 2>"$tmp/err2"; then
    echo "FAIL self-test: truncated download should fail" >&2
    return 1
  fi
  if decode_and_pin "$tmp/garbage.json" "$tmp/should-not.wasm" >/dev/null 2>"$tmp/err3"; then
    echo "FAIL self-test: non-wasm should fail (G9)" >&2
    return 1
  fi
  echo "PASS: fetch-lcd-wasm self-test (pin match, mismatch, truncated, non-wasm)"
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

DEST="$AUDIT_ROOT/codeids/$ID"
mkdir -p "$DEST"
WASM="$DEST/token.wasm"
META="$DEST/meta.json"
PIN="$DEST/wasm.sha256"
JSON="$DEST/code.json"

if [[ -f "$WASM" && -f "$PIN" && "${FETCH_FORCE:-}" != "1" ]]; then
  local_hash="$(sha256_file "$WASM")"
  pinned="$(tr -d '[:space:]' < "$PIN" | tr '[:lower:]' '[:upper:]')"
  if [[ "$local_hash" == "$pinned" ]]; then
    echo "CACHE: $WASM already pinned $pinned (set FETCH_FORCE=1 to re-fetch)"
    # Do not touch REPORT.md
    exit 0
  fi
  echo "FAIL: cached wasm sha256 $local_hash != pin $pinned" >&2
  exit 3
fi

fetch_one "$LCD_URL" "$ID" "$JSON"
mapfile -t FIELDS < <(decode_and_pin "$JSON" "$WASM")
DIGEST="${FIELDS[0]}"
CREATOR="${FIELDS[1]:-}"
PERM="${FIELDS[2]:-}"
CODE_ID_FIELD="${FIELDS[3]:-}"

if [[ -n "${LCD_URL_SECONDARY:-}" ]]; then
  JSON2="$DEST/code.secondary.json"
  fetch_one "${LCD_URL_SECONDARY%/}" "$ID" "$JSON2"
  mapfile -t FIELDS2 < <(decode_and_pin "$JSON2" "$DEST/token.secondary.wasm")
  if [[ "${FIELDS2[0]}" != "$DIGEST" ]]; then
    echo "FAIL: secondary LCD data_hash ${FIELDS2[0]} != primary $DIGEST (G4)" >&2
    exit 3
  fi
  rm -f "$DEST/token.secondary.wasm" "$JSON2"
fi

echo "$DIGEST" > "$PIN"
python3 - "$META" "$ID" "$DIGEST" "$CREATOR" "$PERM" "$LCD_URL" "$CODE_ID_FIELD" <<'PY'
import json, sys
path, code_id, digest, creator, perm, lcd, lcd_id = sys.argv[1:]
meta = {
  "code_id": int(code_id),
  "data_hash": digest,
  "creator": creator,
  "instantiate_permission": perm,
  "lcd": lcd,
  "lcd_code_id": lcd_id,
  "note": "SHA-256 of LCD wasm bytes; not an optimizer rebuild. Do not overwrite REPORT.md.",
}
with open(path, "w", encoding="utf-8") as f:
    json.dump(meta, f, indent=2)
    f.write("\n")
PY

echo "PASS: code $ID pinned $DIGEST -> $WASM"
# Never overwrite REPORT.md
exit 0
