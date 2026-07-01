#!/usr/bin/env python3
"""Verify canonical governance multisig address is documented consistently."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MULTISIG = "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7"
LIB = ROOT / "scripts/lib/governance-multisig.sh"
REFERENCE = ROOT / "docs/reference/governance-multisig.md"

REQUIRED_IN_MULTISIG: tuple[Path, ...] = (
    LIB,
    REFERENCE,
    ROOT / "docs/security-model.md",
    ROOT / "docs/deployment-guide.md",
    ROOT / "docs/runbooks/key-custody.md",
    ROOT / "smartcontracts/scripts/deploy.sh",
)


def main() -> int:
    errors: list[str] = []

    for path in REQUIRED_IN_MULTISIG:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
            continue
        if path.name == "deploy.sh" and "governance-multisig.sh" in path.read_text():
            continue
        if MULTISIG not in path.read_text():
            errors.append(f"{path.relative_to(ROOT)} missing governance multisig address")

    lib_text = LIB.read_text() if LIB.is_file() else ""
    if "GOVERNANCE_MULTISIG_ADDR" not in lib_text:
        errors.append("scripts/lib/governance-multisig.sh must export GOVERNANCE_MULTISIG_ADDR")

    deploy = (ROOT / "smartcontracts/scripts/deploy.sh").read_text() if (ROOT / "smartcontracts/scripts/deploy.sh").is_file() else ""
    if "governance-multisig.sh" not in deploy:
        errors.append("smartcontracts/scripts/deploy.sh must source governance-multisig.sh")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: governance multisig address documented")
    return 0


if __name__ == "__main__":
    sys.exit(main())
