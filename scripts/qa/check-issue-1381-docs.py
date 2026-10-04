#!/usr/bin/env python3
"""Validate current tax-funding invariants and agent/deployment crosslinks."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
skill = (root / "skills/AGENTS_ROUTE_LOCAL_TAX.md").read_text()
for number in range(1, 11):
    assert f"**RT1381-{number}" in skill
for filename in ("AGENTS.md", "docs/testing.md", "docs/contracts-security-audit.md",
                 "docs/contracts-terraclassic.md", "docs/route-solver.md", "docs/frontend.md",
                 "docs/qa/issue-1381/README.md"):
    assert "AGENTS_ROUTE_LOCAL_TAX.md" in (root / filename).read_text(), filename
for filename in ("AGENTS_COMMUNITY_TAX_ROUTER.md", "AGENTS_INDEXER_TAX_AWARE_ROUTING.md",
                 "AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md", "AGENTS_COMMUNITY_TAX_CW20.md",
                 "AGENTS_HYBRID_QUOTING.md", "AGENTS_FRONTEND_PAY_INVOICE.md"):
    text = (root / "skills" / filename).read_text()
    assert "Current funding-version rule (#1381)" in text, filename
    assert "historical compatibility" in text, filename
assert "verify-issue-1381:" in (root / "Makefile").read_text()
assert "PASS with explicit skips" in (root / "docs/qa/issue-1381/README.md").read_text()
print("PASS #1381 invariants, versioned legacy policy and agent crosslinks")
