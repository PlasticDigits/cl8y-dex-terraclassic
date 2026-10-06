#!/usr/bin/env python3
"""Validate trust lifecycle invariants and local documentation crosslinks."""
import re
from pathlib import Path

root = Path(__file__).resolve().parents[2]
files = ["skills/AGENTS_ROUTER_TRUST_LIFECYCLE.md",
         "docs/runbooks/router-trust-1388.md", "docs/qa/issue-1388/README.md"]
skill = (root / files[0]).read_text()
for number in range(1, 11):
    assert f"**RTTRUST1388-{number}" in skill
for filename in files:
    path = root / filename
    for target in re.findall(r"\]\(([^)]+)\)", path.read_text()):
        if "://" not in target:
            assert (path.parent / target.split("#", 1)[0]).exists(), (filename, target)
for filename in ["AGENTS.md", "docs/testing.md", "docs/contracts-security-audit.md",
                 "docs/contracts-terraclassic.md", "docs/route-solver.md",
                 "skills/AGENTS_ROUTE_LOCAL_TAX.md", "skills/AGENTS_ROUTE_TAX_ROLLOUT.md"]:
    assert "AGENTS_ROUTER_TRUST_LIFECYCLE.md" in (root / filename).read_text(), filename
assert "PASS with explicit" in (root / files[2]).read_text()
assert "verify-issue-1388:" in (root / "Makefile").read_text()
print("PASS #1388 invariants, recovery and agent verification crosslinks")
