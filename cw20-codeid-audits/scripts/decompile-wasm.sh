#!/usr/bin/env bash
# Decompile a pinned LCD wasm with wabt (GitLab #589). Missing tools FAIL (C2).
#
# Usage:
#   decompile-wasm.sh <code_id>
#   decompile-wasm.sh --self-test
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUDIT_ROOT="$(cd "${CW20_AUDIT_ROOT:-$SCRIPT_DIR/..}" && pwd)"

need_wabt() {
  local missing=0
  for t in wasm2wat wasm-decompile wasm-objdump; do
    if ! command -v "$t" >/dev/null 2>&1; then
      echo "FAIL: missing $t (install wabt: apt install wabt / https://github.com/WebAssembly/wabt)" >&2
      missing=1
    fi
  done
  if [[ "$missing" -ne 0 ]]; then
    echo "FAIL: decompile tools missing; do not skip decomp (C2)" >&2
    return 1
  fi
}

run_self_test() {
  if ! need_wabt; then
    return 1
  fi
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  # Minimal wasm: (module)
  python3 - "$tmp/m.wasm" <<'PY'
import pathlib, sys
# wasm module with empty body: 00 61 73 6d 01 00 00 00
pathlib.Path(sys.argv[1]).write_bytes(b"\x00asm\x01\x00\x00\x00")
PY
  wasm2wat "$tmp/m.wasm" -o "$tmp/m.wat"
  wasm-objdump -x "$tmp/m.wasm" > "$tmp/m.objdump.txt"
  test -s "$tmp/m.wat"
  test -s "$tmp/m.objdump.txt"
  echo "PASS: decompile-wasm self-test (wabt present)"
}

if [[ "${1:-}" == "--self-test" ]]; then
  run_self_test
  exit $?
fi

ID="${1:-}"
if [[ -z "$ID" || ! "$ID" =~ ^[0-9]+$ ]]; then
  echo "Usage: $0 <code_id> | --self-test" >&2
  exit 1
fi

need_wabt

WASM="$AUDIT_ROOT/codeids/$ID/token.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "FAIL: $WASM missing — run fetch-lcd-wasm.sh $ID first" >&2
  exit 1
fi

DEST="$AUDIT_ROOT/codeids/$ID/decomp"
mkdir -p "$DEST"
wasm2wat "$WASM" -o "$DEST/token.wat"
wasm-decompile "$WASM" -o "$DEST/token.decompiled.c" || {
  echo "WARN: wasm-decompile failed; wat + objdump still required" >&2
}
wasm-objdump -x "$WASM" > "$DEST/objdump-headers.txt"
# Transfer-path symbols (best-effort; names often stripped)
wasm-objdump -d "$WASM" > "$DEST/objdump-disasm.txt" || true
if command -v strings >/dev/null 2>&1; then
  strings -n 6 "$WASM" > "$DEST/strings.txt"
fi
echo "PASS: decompiled code $ID -> $DEST"
# Never overwrite REPORT.md
exit 0
