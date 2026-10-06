#!/usr/bin/env python3
"""Check #1391 local documentation crosslinks and invariant registration."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
paths = [root / p for p in ("docs/qa/issue-1391/README.md", "docs/qa/issue-1391/verification.md",
          "skills/AGENTS_ORACLE_SAFETY.md", "docs/runbooks/indexer-external-oracle.md")]
for path in paths:
    assert path.is_file(), str(path)
    for target in re.findall(r"\]\(([^)]+)\)", path.read_text()):
        if "://" in target or target.startswith("#"):
            continue
        assert (path.parent / target.split("#")[0]).exists(), f"{path}: {target}"
text = (root / "docs/indexer-invariants.md").read_text()
assert all(f"O1391-{i}" in text for i in range(1,11))
assert "AGENTS_ORACLE_SAFETY.md" in (root / "AGENTS.md").read_text()
print("#1391 local links and O1391-1..10: PASS")
