#!/usr/bin/env python3
"""Fail if tokenlist.json has colliding symbols or execute ids (GitLab #715).

No network. Case-insensitive ASCII uniqueness: UST1 ≡ ust1 ≡ Ust1.
Non-ASCII tickers are rejected so Cyrillic homographs cannot ship as UST1.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DEFAULT_TOKENLIST = REPO / "tokenlist" / "tokenlist.json"
FIXTURE_DIR = REPO / "scripts" / "qa" / "fixtures" / "tokenlist-715"

ASCII_SYMBOL = re.compile(r"^[A-Za-z0-9]+$")


def fail(msg: str) -> None:
    print(f"  FAIL  {msg}", file=sys.stderr)
    raise SystemExit(1)


def fold_symbol(raw: str) -> str:
    """Unicode casefold; uniqueness CI also requires ASCII-only tickers first."""
    return raw.strip().casefold()


def validate_tokens(tokens: object, *, label: str) -> list[str]:
    errors: list[str] = []
    if not isinstance(tokens, list) or not tokens:
        return [f"{label}: tokens must be a non-empty list"]

    seen_sym: dict[str, str] = {}
    seen_id: dict[str, str] = {}

    for i, tok in enumerate(tokens):
        prefix = f"{label} token[{i}]"
        if not isinstance(tok, dict):
            errors.append(f"{prefix}: not an object")
            continue
        symbol = tok.get("symbol")
        if not isinstance(symbol, str) or not symbol.strip():
            errors.append(f"{prefix}: empty or whitespace-only symbol")
            continue
        if not ASCII_SYMBOL.fullmatch(symbol.strip()):
            errors.append(f"{prefix}: non-ASCII or illegal ticker {symbol!r}")
            continue
        folded = fold_symbol(symbol)
        if folded in seen_sym:
            errors.append(f"{prefix}: duplicate symbol {symbol!r} (matches {seen_sym[folded]!r})")
        else:
            seen_sym[folded] = symbol.strip()

        typ = tok.get("type")
        if typ == "native":
            denom = str(tok.get("denom") or "").strip().lower()
            if not denom:
                errors.append(f"{prefix}: native row missing denom")
                continue
            exec_id = f"native:{denom}"
        elif typ == "cw20":
            addr = str(tok.get("address") or "").strip().lower()
            if not addr:
                errors.append(f"{prefix}: cw20 row missing address")
                continue
            exec_id = f"cw20:{addr}"
        else:
            errors.append(f"{prefix}: type must be native or cw20")
            continue

        if exec_id in seen_id:
            errors.append(f"{prefix}: duplicate execute id {exec_id} (also {seen_id[exec_id]})")
        else:
            seen_id[exec_id] = prefix

    return errors


def load_tokens(path: Path) -> list:
    if not path.is_file():
        fail(f"missing {path}")
    data = json.loads(path.read_text())
    return data.get("tokens") if isinstance(data, dict) else data


def assert_valid(path: Path) -> None:
    errors = validate_tokens(load_tokens(path), label=str(path.relative_to(REPO) if path.is_relative_to(REPO) else path))
    if errors:
        for err in errors:
            print(f"  FAIL  {err}", file=sys.stderr)
        raise SystemExit(1)


def assert_invalid(path: Path, needle: str) -> None:
    errors = validate_tokens(load_tokens(path), label=path.name)
    joined = "\n".join(errors)
    if not errors:
        fail(f"fixture {path.name} was expected to fail ({needle})")
    if needle.casefold() not in joined.casefold():
        fail(f"fixture {path.name} failed, but not with {needle!r}: {joined}")


def run_self_test() -> None:
    if not FIXTURE_DIR.is_dir():
        fail(f"missing fixtures {FIXTURE_DIR.relative_to(REPO)}")
    cases = (
        ("dup-symbol-case.json", "duplicate symbol"),
        ("dup-address.json", "duplicate execute id"),
        ("dup-denom.json", "duplicate execute id"),
        ("empty-symbol.json", "empty or whitespace-only symbol"),
        ("spaceusd-case.json", "duplicate symbol"),
        ("non-ascii-symbol.json", "non-ASCII"),
    )
    for name, needle in cases:
        assert_invalid(FIXTURE_DIR / name, needle)
    print("  PASS  tokenlist uniqueness fixtures fail closed")


def main(argv: list[str]) -> int:
    args = argv[1:]
    if "--self-test" in args:
        run_self_test()
        args = [a for a in args if a != "--self-test"]
    paths = [Path(a) for a in args] if args else [DEFAULT_TOKENLIST]
    for path in paths:
        assert_valid(path)
        rel = path.relative_to(REPO) if path.is_relative_to(REPO) else path
        print(f"  PASS  unique symbols/ids in {rel}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
