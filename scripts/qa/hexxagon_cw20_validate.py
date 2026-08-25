#!/usr/bin/env python3
"""Validate the #641 Hexxagon terra.js fragment (no network)."""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
FRAG = REPO / "docs/listings/hexxagon/terra.fragment.json"
KEPLR = REPO / "docs/listings/keplr-contract-registry/cosmos/columbus/tokens"
USTR = "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv"
ICON_HOST = "gitlab.com/PlasticDigits/cl8y-dex-terraclassic"
REQUIRED = ("protocol", "symbol", "name", "token", "icon", "decimals")
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


def fail(msg: str) -> None:
    print(f"  FAIL  {msg}", file=sys.stderr)
    raise SystemExit(1)


def keplr() -> dict[str, dict]:
    out = {}
    for path in KEPLR.glob("*.json"):
        data = json.loads(path.read_text())
        meta = data["metadata"]
        out[data["contractAddress"]] = {
            "symbol": meta["symbol"],
            "decimals": meta["decimals"],
        }
    return out


def main() -> None:
    rows = json.loads(FRAG.read_text())
    if len(rows) != 5:
        fail(f"expected 5 submit rows (USTR already live), got {len(rows)}")
    pins = keplr()
    seen = set()
    for row in rows:
        for key in REQUIRED:
            if key not in row:
                fail(f"missing {key} on {row.get('symbol')}")
        addr = row["token"]
        if addr == USTR:
            fail("do not resubmit already-listed USTR")
        if addr in GEMS:
            fail(f"gem listed: {addr}")
        if addr not in pins:
            fail(f"{addr} not in Keplr pack")
        if row["symbol"] != pins[addr]["symbol"] or row["decimals"] != pins[addr]["decimals"]:
            fail(f"{addr} pin mismatch vs Keplr")
        if ICON_HOST not in row["icon"]:
            fail(f"{row['symbol']} icon must use in-repo tokenlist host")
        if row["symbol"] == "CL8Y":
            if row.get("coinGeckoID") != "ceramicliberty-com":
                fail("CL8Y must set coinGeckoID ceramicliberty-com")
        elif row.get("coinGeckoID"):
            fail(f"{row['symbol']} must not invent coinGeckoID")
        seen.add(addr)
    missing = set(pins) - seen - {USTR}
    if missing:
        fail(f"Keplr submit tokens missing: {sorted(missing)}")
    print("  OK    Hexxagon fragment pins, schema, Keplr lockstep, USTR omitted")


if __name__ == "__main__":
    main()
