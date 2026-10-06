#!/usr/bin/env python3
"""Check that route launch-guard rules and reproduction instructions stay linked."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
skill = (root / "skills/AGENTS_ROUTE_LAUNCH_GUARDS.md").read_text()
for number in range(1, 9):
    assert f"**LG1390-{number}" in skill
for filename in ("AGENTS.md", "docs/testing.md", "docs/contracts-security-audit.md",
                 "docs/route-solver.md", "docs/frontend.md", "skills/AGENTS_ROUTE_LOCAL_TAX.md",
                 "docs/qa/issue-1390/README.md"):
    assert "AGENTS_ROUTE_LAUNCH_GUARDS.md" in (root / filename).read_text(), filename
assert "verify-issue-1390:" in (root / "Makefile").read_text()
assert "PASS with explicit skips" in (root / "docs/qa/issue-1390/README.md").read_text()
print("PASS #1390 invariants, agent instructions and explicit skip crosslinks")
