#!/usr/bin/env python3
"""Binary code-section fixtures for the declared-local limit checker."""
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check-wasm-local-limit.py")
SPEC = importlib.util.spec_from_file_location("local_limit", SCRIPT)
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


def leb(value):
    output = bytearray()
    while True:
        byte = value & 127
        value >>= 7
        output.append(byte | (128 if value else 0))
        if not value:
            return bytes(output)


def section(kind, payload):
    return bytes([kind]) + leb(len(payload)) + payload


def fixture(groups, parameters=0, opcodes=b"\x0b"):
    types = section(1, b"\x01\x60" + leb(parameters) + b"\x7f" * parameters + b"\x00")
    functions = section(3, b"\x01\x00")
    body = leb(len(groups)) + b"".join(leb(count) + bytes([kind]) for count, kind in groups) + opcodes
    return b"\0asm\1\0\0\0" + types + functions + section(10, b"\x01" + leb(len(body)) + body)


class LocalsTests(unittest.TestCase):
    def test_boundary_excludes_parameters_and_sums_all_type_groups(self):
        data = fixture([(92, 0x7F), (8, 0x7E)], parameters=3)
        self.assertEqual(CHECKER.declared_function_locals(data), [100])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "boundary.wasm"
            path.write_bytes(data)
            self.assertEqual(CHECKER.inspect_artifact(path)["status"], "PASS")

    def test_real_multibyte_leb137_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "token.wasm"
            path.write_bytes(fixture([(129, 0x7F), (8, 0x7E)], parameters=3))
            report = CHECKER.inspect_artifact(path)
            self.assertEqual(report["max_declared_locals"], 137)
            self.assertEqual(report["violations"], [{"defined_function_index": 0, "declared_locals": 137}])
            self.assertEqual(subprocess.run([sys.executable, SCRIPT, "--json", path], capture_output=True).returncode, 1)

    def test_custom_sections_and_instruction_bytes_do_not_count_as_declarations(self):
        data = fixture([(1, 0x7F)], opcodes=b"\x41\x00\x21\x00\x0b")
        data += section(0, b"\x01x" + leb(137) + b"\x7f")
        self.assertEqual(CHECKER.declared_function_locals(data), [1])

    def test_zero_local_body_and_empty_module(self):
        self.assertEqual(CHECKER.declared_function_locals(fixture([])), [0])
        self.assertEqual(CHECKER.declared_function_locals(b"\0asm\1\0\0\0"), [])

    def test_truncated_sections_and_overflowing_leb_fail_closed(self):
        for data in [b"", b"\0asm\2\0\0\0", fixture([(137, 0x7F)])[:-1],
                     b"\0asm\1\0\0\0\x0a\x80\x80\x80\x80\x10",
                     b"\0asm\1\0\0\0\x0a\x80\x80\x80\x80\x80"]:
            with self.subTest(data=data):
                with self.assertRaises(CHECKER.WasmFormatError):
                    CHECKER.declared_function_locals(data)

    def test_duplicate_mismatched_and_malformed_code_bodies_fail_closed(self):
        valid = fixture([(1, 0x7F)])
        for data in [valid + section(10, b"\x00"),
                     valid + section(3, b"\x00"),
                     b"\0asm\1\0\0\0" + section(3, b"\x02\x00\x00") + section(10, b"\x00"),
                     fixture([(1, 0x7F)], opcodes=b""), fixture([(1, 0xFF)]),
                     b"\0asm\1\0\0\0" + section(10, b"\x01\x02\x80\x01")]:
            with self.subTest(data=data):
                with self.assertRaises(CHECKER.WasmFormatError):
                    CHECKER.declared_function_locals(data)


if __name__ == "__main__":
    unittest.main()
