"""Verify availability invariants and navigable agent/QA entrypoints."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
text = (root / "docs/route-solver.md").read_text()
for number in range(1, 9):
    assert f"**RQ1389-{number}**" in text
for file in ("AGENTS.md", "docs/testing.md", "docs/route-solver.md",
             "docs/qa/issue-1389/README.md", "skills/AGENTS_SWAP_EXACT_OUTPUT.md"):
    assert "AGENTS_REVERSE_QUERY_BUDGET.md" in (root / file).read_text(), file
assert "PASS with explicit skips" in (root / "docs/qa/issue-1389/README.md").read_text()
assert "verify-issue-1389:" in (root / "Makefile").read_text()
limits = {
    "smartcontracts/contracts/router/src/reverse_query.rs": {
        "MAX_REVERSE_SIMULATIONS_PER_HOP": 33,
        "MAX_REVERSE_SIMULATIONS_PER_REQUEST": 96,
    },
    "smartcontracts/packages/dex-common/src/pair.rs": {"MAX_HYBRID_REVERSE_SIM_CALLS": 32},
    "indexer/src/api/route_exact_output_admission.rs": {"MAX_CONCURRENT_REQUESTS": 4},
}
for file, constants in limits.items():
    source = (root / file).read_text()
    for name, value in constants.items():
        assert re.search(rf"const {name}:\s*(?:u32|usize)\s*=\s*{value};", source), (file, name)
print("PASS: #1389 invariants, QA and agent crosslinks")
