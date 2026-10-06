#!/usr/bin/env python3
"""Check issue-specific documentation crosslinks and invariant inventory."""
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
files = [root / p for p in (
    "skills/AGENTS_BRIDGE_DEBT_MARKET_PINS.md", "docs/qa/issue-1393/README.md",
    "docs/qa/issue-1393/verification.md", "docs/runbooks/deploy-usdt-ust1-lp.md")]
for file in files:
    for target in re.findall(r"\]\(([^)]+)\)", file.read_text()):
        if "://" not in target and not target.startswith("#"):
            assert (file.parent / target.split("#")[0]).exists(), (file, target)
skill = files[0].read_text()
for number in range(1, 11):
    assert f"BD1393-{number} |" in skill
for entry in ("AGENTS.md", "docs/contracts-security-audit.md"):
    assert "AGENTS_BRIDGE_DEBT_MARKET_PINS.md" in (root / entry).read_text()
assert "PASS with skips" in files[1].read_text()
print("PASS issue1393 invariant inventory and documentation links")
