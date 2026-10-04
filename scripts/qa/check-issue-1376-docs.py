#!/usr/bin/env python3
"""Check receive-sizing invariant/agent/API crosslinks and reject obsolete policy."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
for number in range(1, 10):
    assert f"**E1376-{number}**" in (root / "docs/frontend.md").read_text()
for file in ("AGENTS.md", "docs/testing.md", "docs/route-solver.md",
             "docs/frontend.md", "docs/qa/issue-1376/README.md"):
    assert "AGENTS_SWAP_EXACT_OUTPUT.md" in (root / file).read_text(), file
for file in ("docs/frontend.md", "skills/AGENTS_FRONTEND_SWAP_QUERY_PARAMS.md",
             "skills/AGENTS_FRONTEND_SWAP_URL_SYNC.md", "skills/AGENTS_FRONTEND_SWAP_REVERSE_QUOTES.md"):
    text = (root / file).read_text()
    assert "silently ignores output" not in text, file
    assert "silently ignore output" not in text, file
    assert "No indexer exact-out solver" not in text, file
assert "amount_out" in (root / "docs/route-solver.md").read_text()
assert "PASS with explicit skips" in (root / "docs/qa/issue-1376/README.md").read_text()
assert "verify-issue-1376:" in (root / "Makefile").read_text()
print("PASS: #1376 invariants, current policy and agent/API crosslinks")
