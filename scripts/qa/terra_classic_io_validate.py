#!/usr/bin/env python3
"""Validate the #1261 directory pack and catalog without network access."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

from listings_catalog_validate import GEMS

REPO = Path(__file__).resolve().parents[2]
PACK = REPO / "docs/listings/terra-classic-io"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> None:
    catalog = json.loads((REPO / "docs/listings/catalog.json").read_text())
    fragment = (PACK / "projects.fragment.ts").read_text()
    # Parse this literal-only TypeScript fragment as JSON; never execute pack code.
    literal = re.sub(r"([,{]\s*)([a-zA-Z]+)\s*:", r'\1"\2":', fragment)
    literal = literal.replace("'", '"')
    literal = re.sub(r",\s*([}\]])", r"\1", literal)
    project = json.loads(literal)
    expected = {
        "name": "CL8Y DEX",
        "url": "https://dex.cl8y.com",
        "description": "Native DEX",
        "indicator": "onchain",
        "logo": "/public/logos/dex/cl8y.png",
        "categories": ["dex", "applications"],
    }
    require(project == expected, "pack must contain exactly the pinned CL8Y DEX object")
    require(project["url"] == catalog["pins"]["dapp"], "dApp/catalog URL mismatch")
    require(not any(gem in fragment for gem in GEMS), "gem address in project pack")
    require(not re.search(r"ALPHA|USTRIX|SpaceUSD|stablecoin|\$1|coinGecko|0x", fragment, re.I),
            "forbidden token, peg, price ID or BSC copy in project pack")

    png = (PACK / "cl8y.png").read_bytes()
    require(png == (REPO / "tokenlist/images/CL8Y.png").read_bytes(), "logo differs from repository art")
    require(png[:8] == b"\x89PNG\r\n\x1a\n", "logo is not PNG")

    venues = [v for v in catalog["venues"] if v["id"] == "terra-classic-io"]
    children = [c for c in catalog["children"] if c["venue"] == "terra-classic-io"]
    require(len(venues) == len(children) == 1, "require one venue and one child")
    venue, child = venues[0], children[0]
    for key, value in {
        "kind": "dex", "channel": "pr", "agentic": True,
        "upstream": "https://github.com/terra-classic-io/website",
        "path": "src/data/projects.ts", "pack": "docs/listings/terra-classic-io/",
    }.items():
        require(venue.get(key) == value, f"catalog venue {key} mismatch")
    require(child["issue"] == 1261, "child must point to #1261")
    require(child.get("upstream_pr") == venue.get("upstream_pr"), "upstream PR pins differ")
    if venue.get("upstream_pr"):
        require(re.fullmatch(r"https://github.com/terra-classic-io/website/pull/[1-9]\d*",
                             venue["upstream_pr"]) is not None, "invalid upstream PR URL")

    for path in [PACK / "README.md", REPO / "skills/AGENTS_TERRA_CLASSIC_IO.md"]:
        text = path.read_text()
        for phrase in ["TC1261-1", "TC1261-8", "L639-7", "Do not open upstream PRs from CI",
                       "public/logos/dex/cl8y.png", "make verify-issue-1261"]:
            require(phrase in text, f"{path.name} missing {phrase}")
    print("  OK    #1261 single project, categories, dApp, catalog, logo bytes and no-CI-PR guidance")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError) as exc:
        print(f"  FAIL  {exc}", file=sys.stderr)
        raise SystemExit(1)
