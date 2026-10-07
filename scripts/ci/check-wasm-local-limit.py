#!/usr/bin/env python3
"""Check declared function locals in binary Wasm; parameters are excluded.

This bounds the code-section local declarations, not instruction validity or
CosmWasm compatibility. Run the matching VM validator/simulation separately.
"""
import argparse
import hashlib
import json
from pathlib import Path


class WasmFormatError(ValueError):
    pass


class Reader:
    def __init__(self, data):
        self.data = data
        self.offset = 0

    @property
    def remaining(self):
        return len(self.data) - self.offset

    def byte(self):
        if not self.remaining:
            raise WasmFormatError("truncated binary payload")
        value = self.data[self.offset]
        self.offset += 1
        return value

    def u32(self):
        value = 0
        for index in range(5):
            byte = self.byte()
            if index == 4 and byte & 0xF0:
                raise WasmFormatError("unsigned LEB128 exceeds u32")
            value |= (byte & 0x7F) << (index * 7)
            if not byte & 0x80:
                return value
        raise WasmFormatError("unterminated unsigned LEB128")

    def payload(self, size):
        if size > self.remaining:
            raise WasmFormatError("payload length exceeds its enclosing boundary")
        start = self.offset
        self.offset += size
        return Reader(self.data[start:self.offset])

    def finish(self):
        if self.remaining:
            raise WasmFormatError("unexpected trailing section bytes")


def declared_function_locals(data):
    if data[:8] != b"\0asm\1\0\0\0":
        raise WasmFormatError("expected a version-1 Wasm core module")
    module = Reader(data[8:])
    functions = None
    locals_by_function = None
    while module.remaining:
        section_id = module.byte()
        section = module.payload(module.u32())
        if section_id == 3:
            if functions is not None:
                raise WasmFormatError("duplicate function section")
            functions = section.u32()
            for _ in range(functions):
                section.u32()  # Type index; parameters are not local declarations.
            section.finish()
        elif section_id == 10:
            if locals_by_function is not None:
                raise WasmFormatError("duplicate code section")
            count = section.u32()
            if count > section.remaining // 3:
                raise WasmFormatError("function count exceeds available code bodies")
            locals_by_function = []
            for _ in range(count):
                body = section.payload(section.u32())
                groups = body.u32()
                if groups > body.remaining // 2:
                    raise WasmFormatError("local group count exceeds available body bytes")
                declared = 0
                for _ in range(groups):
                    declared += body.u32()
                    # Numeric, vector and shorthand reference value types. More
                    # recent typed-reference encodings are outside this chain's ABI.
                    if body.byte() not in (0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x70, 0x6F):
                        raise WasmFormatError("unsupported local value type")
                if not body.remaining or body.data[-1] != 0x0B:
                    raise WasmFormatError("function body lacks its final end opcode")
                locals_by_function.append(declared)
            section.finish()
    if (functions or 0) != len(locals_by_function or []):
        raise WasmFormatError("function and code section counts differ")
    return locals_by_function or []


def inspect_artifact(path, limit=100):
    data = Path(path).read_bytes()
    counts = declared_function_locals(data)
    failures = [
        {"defined_function_index": index, "declared_locals": count}
        for index, count in enumerate(counts) if count > limit
    ]
    return {
        "path": str(path), "sha256": hashlib.sha256(data).hexdigest(),
        "defined_functions": len(counts), "max_declared_locals": max(counts, default=0),
        "limit": limit, "violations": failures, "status": "FAIL" if failures else "PASS",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wasm", nargs="+", type=Path)
    parser.add_argument("--limit", type=int, default=100)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if args.limit < 0:
        parser.error("--limit must be nonnegative")
    reports = []
    for path in args.wasm:
        try:
            reports.append(inspect_artifact(path, args.limit))
        except (OSError, WasmFormatError) as error:
            reports.append({"path": str(path), "status": "FAIL", "error": str(error)})
    failed = any(report["status"] != "PASS" for report in reports)
    if args.json:
        print(json.dumps({"status": "FAIL" if failed else "PASS", "artifacts": reports}, indent=2))
    else:
        for report in reports:
            detail = report.get("error") or f"max declared locals {report['max_declared_locals']} / {args.limit}"
            print(f"{report['status']} {report['path']}: {detail}")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
