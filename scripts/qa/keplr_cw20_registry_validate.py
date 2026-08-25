#!/usr/bin/env python3
"""Validate the #629 Keplr CW20 listing pack (no network)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PACK = REPO / "docs/listings/keplr-contract-registry"
TOKENS_DIR = PACK / "cosmos/columbus/tokens"
TOKENLIST = REPO / "tokenlist/tokenlist.json"

FORBIDDEN = frozenset({"price", "priceUrl", "oracle", "marketId"})
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

EXPECTED = {
    "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3": {
        "symbol": "CL8Y",
        "name": "CL8Y",
        "decimals": 18,
        "imageFile": "CL8Y.png",
        "sourceImage": "tokenlist/images/CL8Y.png",
        "coinGeckoId": "ceramicliberty-com",
    },
    "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72": {
        "symbol": "UST1",
        "name": "UST1",
        "decimals": 6,
        "imageFile": "UST1.png",
        "sourceImage": "tokenlist/images/UST1.png",
    },
    "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv": {
        "symbol": "USTR",
        "name": "USTC Repeg",
        "decimals": 18,
        "imageFile": "USTR.png",
        "sourceImage": "tokenlist/images/USTR.png",
    },
    "terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg": {
        "symbol": "cLUNC",
        "name": "Wrapped Luna Classic",
        "decimals": 6,
        "imageFile": "CLUNC.png",
        "sourceImage": "tokenlist/images/CLUNC.png",
    },
    "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch": {
        "symbol": "cUSTC",
        "name": "Wrapped TerraClassicUSD",
        "decimals": 6,
        "imageFile": "CUSTC.png",
        "sourceImage": "tokenlist/images/CUSTC.png",
    },
    "terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3": {
        "symbol": "vFDUSD",
        "name": "Venus FDUSD (bridged)",
        "decimals": 6,
        "imageFile": "VFDUSD.png",
        "sourceImage": "tokenlist/images/VFDUSD.png",
    },
}

IMAGE_BASE = (
    "https://raw.githubusercontent.com/chainapsis/keplr-contract-registry/"
    "main/images/columbus"
)


def fail(msg: str) -> None:
    print(f"FAIL  {msg}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    files = sorted(TOKENS_DIR.glob("*.json"))
    if len(files) != len(EXPECTED):
        fail(f"expected {len(EXPECTED)} token JSON files, found {len(files)}")

    seen: set[str] = set()
    for path in files:
        data = json.loads(path.read_text())
        addr = data.get("contractAddress")
        if not isinstance(addr, str) or addr not in EXPECTED:
            fail(f"{path.name}: unexpected contractAddress {addr!r}")
        if path.name != f"{addr}.json":
            fail(f"{path.name}: filename must equal contractAddress.json")
        if addr in GEMS:
            fail(f"{addr} is a gem — must not be in the Keplr pack")
        extra = FORBIDDEN.intersection(data)
        if extra:
            fail(f"{path.name}: forbidden price keys {sorted(extra)}")
        exp = EXPECTED[addr]
        meta = data.get("metadata") or {}
        if meta.get("symbol") != exp["symbol"] or meta.get("name") != exp["name"]:
            fail(f"{path.name}: metadata name/symbol mismatch")
        if meta.get("decimals") != exp["decimals"]:
            fail(f"{path.name}: decimals {meta.get('decimals')} != {exp['decimals']}")
        want_url = f"{IMAGE_BASE}/{exp['imageFile']}"
        if data.get("imageUrl") != want_url:
            fail(f"{path.name}: imageUrl {data.get('imageUrl')!r} != {want_url!r}")
        src = REPO / exp["sourceImage"]
        if not src.is_file():
            fail(f"missing source logo {src}")
        if src.suffix.lower() != ".png":
            fail(f"{src} must be PNG")
        if exp.get("coinGeckoId"):
            if data.get("coinGeckoId") != exp["coinGeckoId"]:
                fail(f"{path.name}: coinGeckoId must be {exp['coinGeckoId']}")
        elif "coinGeckoId" in data:
            fail(f"{path.name}: coinGeckoId must be omitted")
        seen.add(addr)

    missing = set(EXPECTED) - seen
    if missing:
        fail(f"missing token files: {sorted(missing)}")

    tokenlist = json.loads(TOKENLIST.read_text())
    cl8y = next(t for t in tokenlist["tokens"] if t.get("symbol") == "CL8Y")
    if cl8y.get("decimals") != 18:
        fail(f"tokenlist CL8Y decimals are {cl8y.get('decimals')}, expected 18 (K629-3)")
    vfd = next(t for t in tokenlist["tokens"] if t.get("symbol") == "vFDUSD")
    if vfd.get("address") != "terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3":
        fail("tokenlist vFDUSD address drifted from K629-6")
    if vfd.get("decimals") != 6:
        fail("tokenlist vFDUSD decimals must be 6")

    print("PASS  keplr CW20 pack schema, pins, logos, tokenlist decimals")


if __name__ == "__main__":
    main()
