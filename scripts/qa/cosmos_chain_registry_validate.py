#!/usr/bin/env python3
"""Validate the #642/#7910 listing fragments against recorded chain evidence."""

import base64
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / "docs/listings/cosmos-chain-registry"
EVIDENCE = ROOT / "docs/qa/issue-642/evidence-2026-10-04.json"


def validate():
    evidence = json.loads(EVIDENCE.read_text())
    terra = json.loads((PACK / "terra/assetlist.fragment.json").read_text())
    bsc = json.loads((PACK / "_non-cosmos/binancesmartchain/assetlist.fragment.json").read_text())
    pins = {t["contract"] for t in json.loads((ROOT / "docs/listings/catalog.json").read_text())["tokens"]}
    usdt = "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4"
    expected = pins | {usdt}
    assert len(pins) == 6 and len(expected) == 7
    assert terra["chain_name"] == "terra" and bsc["chain_name"] == "binancesmartchain"
    assert evidence["node_info"]["network"] == "columbus-5"
    launch = evidence["cl8y_launch"]["response"]["entries"][0]
    initial = launch["msg"]
    assert launch["updated"]["block_height"] == "24103161"
    assert sum(int(b["amount"]) for b in initial["initial_balances"]) == 500000 * 10**18
    assert evidence["cl8y_terra_bridge_mode"]["response"]["data"]["token_type"] == "mint_burn"
    assert evidence["cl8y_bsc_bridge_mode"]["response"] == 0
    assert len(terra["assets"]) == 7
    assert {a["address"] for a in terra["assets"]} == expected, "unexpected CW20 or missing pin"
    tokens = {t["address"]: t for t in evidence["tokens"]}
    origins = {a["base"]: a for a in bsc["assets"]}
    mappings = evidence["bridge"]["mappings"]["response"]["data"]["mappings"]
    native = {"cLUNC": "uluna", "cUSTC": "uusd"}
    bridges = {"CL8Y", "vFDUSD", "USDT"}
    mapper = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2"
    for asset in terra["assets"]:
        address = asset["address"]
        recorded = tokens[address]
        info = recorded["token_info"]["response"]["data"]
        assert recorded["contract_info"]["response"]["contract_info"]["code_id"] == "10184"
        assert asset["type_asset"] == "cw20" and asset["base"] == "cw20:" + address
        assert asset["denom_units"][0] == {"denom": asset["base"], "exponent": 0}
        assert asset["denom_units"][1] == {"denom": asset["display"], "exponent": info["decimals"]}
        assert "coingecko_id" not in asset, "CoinGecko identity belongs on the BSC origin"
        symbol = asset["symbol"]
        if symbol in bridges:
            trace, = asset["traces"]
            assert trace["type"] == "bridge" and trace["provider"] == "CL8Y Bridge"
            assert trace["counterparty"]["chain_name"] == "binancesmartchain"
            base = trace["counterparty"]["base_denom"]
            assert base in origins, "unregistered bridge origin"
            mapping, = [m for m in mappings if m["token"] == address and m["dest_chain"] == "00000038"]
            assert "0x" + base64.b64decode(mapping["dest_token"])[-20:].hex() == base.lower()
            assert origins[base]["denom_units"][1]["exponent"] == mapping["dest_decimals"]
            assert asset["images"][0]["image_sync"] == trace["counterparty"]
            if symbol == "CL8Y":
                assert "500,000-token launch allocation" in asset["description"]
        elif symbol in native:
            trace, = asset["traces"]
            assert trace == {"type": "wrapped", "counterparty": {"chain_name": "terra", "base_denom": native[symbol]}, "provider": "CL8Y"}
            assert mapper in recorded["minters"]["response"]["data"]["minters"]
            assert "mint administrator" in asset["description"]
        else:
            assert "traces" not in asset
        for image in asset["images"]:
            for extension in ("png", "svg"):
                if extension not in image:
                    continue
                prefix = "https://raw.githubusercontent.com/cosmos/chain-registry/master/"
                assert image[extension].startswith(prefix)
                path = PACK / image[extension][len(prefix):]
                assert path.is_file(), f"missing image: {path}"
                data = path.read_bytes()
                assert len(data) < 250000
                if extension == "png":
                    assert data[:8] == b"\x89PNG\r\n\x1a\n"
                    assert data[12:16] == b"IHDR"
                    width, height = struct.unpack(">II", data[16:24])
                    assert abs(width - height) <= 1 and width > 0 and height > 0
    cl8y, = [a for a in origins.values() if a["symbol"] == "CL8Y"]
    assert cl8y["coingecko_id"] == "ceramicliberty-com"
    assert evidence["coingecko"]["platforms"] == {"binance-smart-chain": cl8y["base"]}
    assert sum("coingecko_id" in a for a in origins.values() if a["symbol"] != "USDT") == 1
    usdt_asset, = [a for a in terra["assets"] if a["address"] == usdt]
    assert usdt_asset["name"] == tokens[usdt]["token_info"]["response"]["data"]["name"] == "Tether USD"
    assert usdt_asset["symbol"] == tokens[usdt]["token_info"]["response"]["data"]["symbol"] == "USDT"
    assert "Bridge-Debt" in usdt_asset["description"]
    assert evidence["additional"]["usdt_market"]["response"]["data"]["outstanding"] == "10000000000000000000000"
    assert (PACK / "source-images/USDT.png").read_bytes() == (ROOT / "tokenlist/images/USDT.png").read_bytes()
    assert "unstablecoin" in next(a for a in terra["assets"] if a["symbol"] == "UST1")["description"]
    assert "Not a stablecoin" in next(a for a in terra["assets"] if a["symbol"] == "USTR")["description"]
    assert evidence["code"]["response"]["code_info"]["data_hash"] == "C77FFCDB0039041FBD071C76EFB547232314CE8CCF219C3BF5B7FABD4CF7D4A4"
    print("PASS: seven pinned CW20s, recorded LCD metadata, bridge origins/scales, wrapped traces, images, and debt disclosure")


if __name__ == "__main__":
    validate()
