#!/usr/bin/env python3
"""Validate the #640 Cosmostation CW20 fragment (no network)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
FRAG = REPO / "docs/listings/cosmostation/cw20_2.fragment.json"
ASSET = REPO / "docs/listings/cosmostation/asset"
KEPLR = REPO / "docs/listings/keplr-contract-registry/cosmos/columbus/tokens"
IMAGE_PREFIX = (
    "https://raw.githubusercontent.com/cosmostation/chainlist/master/chain/terra/asset/"
)
GEMS = frozenset(
    {
        "terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94",
        "terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena",
        "terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr",
        "terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9",
        "terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc",
        "terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme",
        "terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z",
        "terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs",
    }
)
REQUIRED = (
    "type",
    "contract",
    "name",
    "symbol",
    "description",
    "decimals",
    "image",
    "coinGeckoId",
)
SOURCE_IMAGE = {
    "CL8Y": "tokenlist/images/CL8Y.png",
    "UST1": "tokenlist/images/UST1.png",
    "USTR": "tokenlist/images/USTR.png",
    "cLUNC": "tokenlist/images/CLUNC.png",
    "cUSTC": "tokenlist/images/CUSTC.png",
    "vFDUSD": "tokenlist/images/VFDUSD.png",
}


def fail(msg: str) -> None:
    print(f"  FAIL  {msg}", file=sys.stderr)
    raise SystemExit(1)


def keplr_by_address() -> dict[str, dict]:
    out: dict[str, dict] = {}
    for path in KEPLR.glob("*.json"):
        data = json.loads(path.read_text())
        meta = data.get("metadata") or {}
        out[data["contractAddress"]] = {
            "symbol": meta.get("symbol"),
            "decimals": meta.get("decimals"),
            "coinGeckoId": data.get("coinGeckoId") or "",
        }
    return out


def main() -> None:
    if not FRAG.is_file():
        fail(f"missing {FRAG.relative_to(REPO)}")
    rows = json.loads(FRAG.read_text())
    if not isinstance(rows, list) or len(rows) != 6:
        fail(f"expected 6 fragment rows, got {type(rows)} len={getattr(rows, '__len__', lambda: None)()}")
    keplr = keplr_by_address()
    seen: set[str] = set()
    for row in rows:
        missing = [k for k in REQUIRED if k not in row]
        if missing:
            fail(f"{row.get('symbol')} missing {missing}")
        if row["type"] != "cw20":
            fail(f"{row['symbol']} type must be cw20")
        addr = row["contract"]
        if addr in seen:
            fail(f"duplicate {addr}")
        seen.add(addr)
        if addr in GEMS:
            fail(f"gem listed: {addr}")
        if addr not in keplr:
            fail(f"{addr} not in Keplr pack")
        if row["symbol"] != keplr[addr]["symbol"]:
            fail(f"{addr} symbol {row['symbol']!r} != Keplr {keplr[addr]['symbol']!r}")
        if row["decimals"] != keplr[addr]["decimals"]:
            fail(f"{addr} decimals {row['decimals']!r} != Keplr")
        if not row["image"].startswith(IMAGE_PREFIX):
            fail(f"{row['symbol']} image must be under chain/terra/asset/")
        png = row["image"].rsplit("/", 1)[1]
        if png != png.lower() or not png.endswith(".png"):
            fail(f"{row['symbol']} asset filename must be lowercase .png")
        if not (ASSET / png).is_file():
            fail(f"missing {ASSET.relative_to(REPO)}/{png}")
        src = REPO / SOURCE_IMAGE[row["symbol"]]
        if not src.is_file():
            fail(f"missing source {src.relative_to(REPO)}")
        cg = row["coinGeckoId"]
        if row["symbol"] == "CL8Y":
            if cg != "ceramicliberty-com":
                fail("CL8Y coinGeckoId must be ceramicliberty-com")
        elif cg != "":
            fail(f"{row['symbol']} coinGeckoId must be empty string")
        desc = row["description"]
        if row["symbol"] == "UST1" and "unstablecoin" not in desc.lower():
            fail("UST1 description must say unstablecoin")
        if row["symbol"] == "USTR" and "not a stablecoin" not in desc.lower():
            fail("USTR description must say not a stablecoin")
    extra = set(keplr) - seen
    if extra:
        fail(f"Keplr tokens missing from fragment: {sorted(extra)}")
    print("  OK    Cosmostation fragment pins, schema, logos, Keplr lockstep")


if __name__ == "__main__":
    main()
