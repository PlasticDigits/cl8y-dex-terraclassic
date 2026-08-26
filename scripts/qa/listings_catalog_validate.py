#!/usr/bin/env python3
"""Validate the #639 listing venue catalog (no network)."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG_PATH = REPO / "docs/listings/catalog.json"
KEPLR_TOKENS = REPO / "docs/listings/keplr-contract-registry/cosmos/columbus/tokens"

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

REQUIRED_INVARIANTS = [f"L639-{i}" for i in range(1, 9)]
REQUIRED_OWNED = {631, 629, 224}
REQUIRED_SKIP = {
    "coinhall",
    "dexscreener",
    "luncscan-telegram",
    "leap-own-repo",
    "terra-money-assets",
    "token-terminal",
    "cmc-dexscan",
}
REQUIRED_FORM_PACKS = {
    "coingecko-exchange": "docs/listings/forms/coingecko-exchange.md",
    "coingecko-terra-classic-platform": "docs/listings/forms/coingecko-terra-classic-platform.md",
    "coinmarketcap-exchange": "docs/listings/forms/coinmarketcap-exchange.md",
    "geckoterminal": "docs/listings/forms/geckoterminal.md",
}
FORBIDDEN_PRO_HOSTS = ("api.coingecko.com", "pro-api.coingecko.com")
PEG_RE = re.compile(r"\$1(?!\S)|peg(?:ged)?\s*(?:to\s*)?\$?1\b", re.I)
USTR_STABLE_RE = re.compile(r"\bUSTR\b.{0,40}\bstablecoin\b|\bstablecoin\b.{0,40}\bUSTR\b", re.I)


def fail(msg: str) -> None:
    print(f"  FAIL  {msg}", file=sys.stderr)
    raise SystemExit(1)


def load_catalog() -> dict:
    if not CATALOG_PATH.is_file():
        fail(f"missing {CATALOG_PATH.relative_to(REPO)}")
    return json.loads(CATALOG_PATH.read_text())


def keplr_by_address() -> dict[str, dict]:
    if not KEPLR_TOKENS.is_dir():
        fail(f"missing Keplr pack {KEPLR_TOKENS.relative_to(REPO)}")
    out: dict[str, dict] = {}
    for path in sorted(KEPLR_TOKENS.glob("*.json")):
        data = json.loads(path.read_text())
        addr = data.get("contractAddress", "")
        meta = data.get("metadata") or {}
        out[addr] = {
            "symbol": meta.get("symbol"),
            "decimals": meta.get("decimals"),
            "coinGeckoId": data.get("coinGeckoId"),
        }
    return out


def check_invariants(catalog: dict) -> None:
    got = catalog.get("invariants") or []
    missing = [i for i in REQUIRED_INVARIANTS if i not in got]
    if missing:
        fail(f"catalog.json missing invariants {missing}")


def check_pins(catalog: dict) -> None:
    pins = catalog.get("pins") or {}
    expected = {
        "dapp": "https://dex.cl8y.com",
        "indexer": "https://indexer.dex.cl8y.com",
        "factory": "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea",
        "cg_base": "https://indexer.dex.cl8y.com/cg/",
        "cmc_base": "https://indexer.dex.cl8y.com/cmc/",
        "coingecko_id": "ceramicliberty-com",
    }
    for key, value in expected.items():
        if pins.get(key) != value:
            fail(f"pin {key}: expected {value!r}, got {pins.get(key)!r}")
    platforms = pins.get("coingecko_id_platforms_today") or []
    if platforms != ["binance-smart-chain"]:
        fail(f"CL8Y CG platforms today must be BSC-only, got {platforms!r}")
    if "0x8f452a1fdd388a45e1080992eff051b4dd9048d2" not in (pins.get("cl8y_bsc") or "").lower():
        fail("pin cl8y_bsc must be the live BSC CL8Y contract")
    for path in pins.get("cg_paths") or []:
        if not str(path).startswith("/"):
            fail(f"cg path must be relative: {path}")
    for path in pins.get("cmc_paths") or []:
        if not str(path).startswith("/"):
            fail(f"cmc path must be relative: {path}")


def check_tokens(catalog: dict, keplr: dict[str, dict]) -> None:
    tokens = catalog.get("tokens") or []
    if len(tokens) != 6:
        fail(f"expected 6 permanent CW20s, got {len(tokens)}")
    seen: set[str] = set()
    for tok in tokens:
        addr = tok.get("contract") or ""
        if addr in seen:
            fail(f"duplicate token {addr}")
        seen.add(addr)
        if addr in GEMS:
            fail(f"gem address listed: {addr}")
        if addr not in keplr:
            fail(f"token {addr} missing from Keplr pack (K629-2)")
        if tok.get("symbol") != keplr[addr]["symbol"]:
            fail(
                f"{addr} symbol {tok.get('symbol')!r} != Keplr {keplr[addr]['symbol']!r}"
            )
        if tok.get("decimals") != keplr[addr]["decimals"]:
            fail(
                f"{addr} decimals {tok.get('decimals')!r} != Keplr {keplr[addr]['decimals']!r}"
            )
        notes = tok.get("notes") or ""
        if tok.get("symbol") == "UST1":
            if "unstablecoin" not in notes.lower():
                fail("UST1 notes must say unstablecoin")
            if PEG_RE.search(notes) and "never" not in notes.lower():
                fail("UST1 notes must not advertise a $1 peg")
        if tok.get("symbol") == "USTR" and re.search(
            r"\bis a stablecoin\b|\bas a stablecoin\b", notes, re.I
        ):
            fail("USTR notes must not list it as a stablecoin")
        if tok.get("symbol") == "CL8Y" and tok.get("coinGeckoId") != "ceramicliberty-com":
            fail("CL8Y must keep coinGeckoId ceramicliberty-com")
        elif tok.get("symbol") != "CL8Y" and tok.get("coinGeckoId"):
            fail(f"{tok.get('symbol')} must not invent a CoinGecko id")
    extra_keplr = set(keplr) - seen
    if extra_keplr:
        fail(f"Keplr pack tokens missing from catalog: {sorted(extra_keplr)}")


def check_owned_and_skip(catalog: dict) -> None:
    owned = {row.get("issue") for row in catalog.get("owned_surfaces") or []}
    if not REQUIRED_OWNED <= owned:
        fail(f"owned_surfaces must include {sorted(REQUIRED_OWNED)}, got {sorted(owned)}")
    for row in catalog.get("owned_surfaces") or []:
        if not row.get("do_not_reopen"):
            fail(f"owned surface {row.get('id')} must set do_not_reopen")
    skip_ids = {row.get("id") for row in catalog.get("skip") or []}
    missing = REQUIRED_SKIP - skip_ids
    if missing:
        fail(f"skip list missing {sorted(missing)}")


def check_venues(catalog: dict) -> None:
    venues = {row.get("id"): row for row in catalog.get("venues") or []}
    for venue_id, pack in REQUIRED_FORM_PACKS.items():
        row = venues.get(venue_id)
        if not row:
            fail(f"missing venue {venue_id}")
        if row.get("channel") != "form":
            fail(f"{venue_id} channel must be form")
        if row.get("form_pack") != pack:
            fail(f"{venue_id} form_pack must be {pack}")
        path = REPO / pack
        if not path.is_file():
            fail(f"missing form pack {pack}")
        text = path.read_text()
        if row.get("kind") == "dex" and "indexer.dex.cl8y.com" not in text:
            fail(f"{pack} must pin indexer.dex.cl8y.com")
        if venue_id == "coingecko-terra-classic-platform":
            if "ceramicliberty-com" not in text:
                fail(f"{pack} must keep ceramicliberty-com")
            if "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3" not in text:
                fail(f"{pack} must pin the columbus-5 CL8Y CW20")
        for host in FORBIDDEN_PRO_HOSTS:
            if re.search(rf"https?://{re.escape(host)}", text):
                fail(f"{pack} must not use {host} as an API URL")
    children = catalog.get("children") or []
    if [row.get("order") for row in children] != list(range(1, len(children) + 1)):
        fail("children.order must be 1..n sequential")
    child_venues = {row.get("venue") for row in children}
    if "cosmostation" not in child_venues:
        fail("children must start with Cosmostation")
    cs = venues.get("cosmostation")
    if not cs:
        fail("missing venue cosmostation")
    if cs.get("agentic") is not False:
        fail("cosmostation venue must set agentic false while chainlist is archived")
    if "archived" not in (cs.get("blocked") or "").lower():
        fail("cosmostation venue must document archived blocked reason")
    cs_child = next((row for row in children if row.get("venue") == "cosmostation"), None)
    if not cs_child or "archived" not in (cs_child.get("blocked") or "").lower():
        fail("children cosmostation must stay blocked while chainlist is archived")
    hx = venues.get("hexxagon")
    if not hx:
        fail("missing venue hexxagon")
    hx_pr = hx.get("upstream_pr") or ""
    if "hexxagon-io/chain-registry/pull/" not in hx_pr:
        fail("hexxagon venue must pin the live upstream PR")
    hx_child = next((row for row in children if row.get("venue") == "hexxagon"), None)
    if not hx_child or hx_child.get("upstream_pr") != hx_pr:
        fail("children hexxagon upstream_pr must match the venue pin")


def check_peg_language(catalog: dict) -> None:
    blob = json.dumps(catalog)
    if USTR_STABLE_RE.search(blob) and "not a stablecoin" not in blob.lower():
        fail("catalog must not list USTR as a stablecoin")
    # Allow "never advertise $1" / "never $1" on UST1; forbid a positive peg claim.
    for tok in catalog.get("tokens") or []:
        notes = tok.get("notes") or ""
        if tok.get("symbol") == "UST1" and re.search(
            r"pegs?\s+to\s+\$1|is\s+\$1|\$1\s+peg", notes, re.I
        ):
            fail("UST1 must not be advertised as a $1 peg")


def main() -> None:
    catalog = load_catalog()
    keplr = keplr_by_address()
    check_invariants(catalog)
    check_pins(catalog)
    check_tokens(catalog, keplr)
    check_owned_and_skip(catalog)
    check_venues(catalog)
    check_peg_language(catalog)
    print("  OK    listings catalog pins, Keplr lockstep, forms, skip list")


if __name__ == "__main__":
    main()
