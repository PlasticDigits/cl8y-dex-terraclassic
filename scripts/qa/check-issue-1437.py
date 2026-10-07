#!/usr/bin/env python3
"""Source/docs boundaries complement executable #1437 API/ingestion tests."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
api = ROOT / "indexer/src/api"
consumers = []
for path in (ROOT / "indexer/src").rglob("*.rs"):
    if path.name == "listing_virtual_pairs.rs":
        continue
    text = path.read_text()
    if "listing_virtual_pairs" in text:
        if path == api / "mod.rs":
            assert "mod listing_virtual_pairs;" in text
            assert text.count("listing_virtual_pairs") == 1, "router must only declare the module"
        else:
            consumers.append(path.relative_to(ROOT).as_posix())
assert set(consumers) == {"indexer/src/api/cg.rs", "indexer/src/api/gt.rs"}, consumers
for name in ["cmc.rs", "defillama.rs", "pairs.rs", "route_solver.rs", "tokens.rs", "traders.rs"]:
    text = (api / name).read_text()
    assert "listing_wrap_principal" not in text and "listing_mapper_custody" not in text, name
    assert "cl8y-virtual:" not in text, name
projection = (api / "listing_virtual_pairs.rs").read_text()
for forbidden in [".lcd", "to_f64", "INSERT INTO", "UPDATE pairs", "UPDATE swap_events"]:
    assert forbidden not in projection, forbidden
ids = ["cl8y-virtual:clunc-uluna", "cl8y-virtual:custc-uusd", "cl8y-virtual:uluna-uusd", "cl8y-virtual:uluna-usdt", "cl8y-virtual:clunc-uusd"]
for file in ["docs/CG_CMC_COMPLIANCE.md", "docs/indexer-invariants.md", "docs/listings/forms/geckoterminal.md"]:
    text = (ROOT / file).read_text()
    for opaque_id in ids:
        assert opaque_id in text, (file, opaque_id)
for file in ["AGENTS.md", "docs/testing.md", "skills/AGENTS_LISTING_VIRTUAL_PAIRS.md", "docs/qa/issue-1437/README.md"]:
    assert "verify-issue-1437" in (ROOT / file).read_text(), file
assert "LV1437-10" in (ROOT / "skills/AGENTS_LISTING_VIRTUAL_PAIRS.md").read_text()
assert "PASS WITH SKIPS" in " ".join((ROOT / "docs/qa/issue-1437/README.md").read_text().split())
assert "is_listing_only_id" in (ROOT / "indexer/src/db/queries/pairs.rs").read_text()
assert "is_listing_only_id" in (ROOT / "indexer/src/indexer/pair_discovery.rs").read_text()
assert "get_at_height" in (ROOT / "indexer/src/indexer/listing_principal.rs").read_text()
assert "x-cosmos-block-height" in (ROOT / "indexer/src/lcd/mod.rs").read_text()
for table in ["listing_wrap_principal", "listing_mapper_custody"]:
    assert (ROOT / "scripts/indexer-reorg-recover.sh").read_text().count(f"DELETE FROM {table}") == 2, table
print("PASS: CG/GT-only consumers, economic isolation, ids/docs, early guards and replay cleanup")
