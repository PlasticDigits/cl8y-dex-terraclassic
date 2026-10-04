#!/usr/bin/env python3
"""Read-only public pair inventory/reverse probes; never signs or reads keyrings."""
import argparse
import base64
import json
import sys
from datetime import datetime, timezone
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

PAIR = "terra1tz5vwrungh6drd9nt95qym3k892vs3as8nqmu7sg4ypek7wxvv4qm89upc"
FACTORY = "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
TARGET = "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lcd", default="https://terra-classic-lcd.publicnode.com")
    parser.add_argument("--factory", default=FACTORY)
    parser.add_argument("--pair", default=PAIR)
    parser.add_argument("--target-token", default=TARGET, help="public contract resolved by the deployed CL8Y ticker")
    parser.add_argument("--expect-fixed", action="store_true")
    args = parser.parse_args()
    lcd = args.lcd.rstrip("/")
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "lcd": lcd,
                "probe_only": True, "inventory": [], "reverse": []}

    def get(path):
        try:
            with urlopen(Request(lcd + path, headers={"User-Agent": "curl/8.0"}), timeout=15) as response:
                return json.load(response)
        except HTTPError as error:
            # Public contract errors only; no request headers/cookies/identities.
            try:
                detail = json.loads(error.read().decode()).get("message", "LCD error")
            except (ValueError, UnicodeError):
                detail = "non-JSON LCD error"
            raise RuntimeError(f"HTTP {error.code}: {detail[:400]}") from error

    def smart(contract, message):
        encoded = quote(base64.b64encode(json.dumps(message).encode()).decode(), safe="")
        result = get(f"/cosmwasm/wasm/v1/contract/{contract}/smart/{encoded}")["data"]
        return json.loads(base64.b64decode(result)) if isinstance(result, str) else result

    def version(contract):
        key = quote(base64.b64encode(b"contract_info").decode(), safe="")
        raw = get(f"/cosmwasm/wasm/v1/contract/{contract}/raw/{key}").get("data")
        return json.loads(base64.b64decode(raw)) if raw else None

    try:
        block = get("/cosmos/base/tendermint/v1beta1/blocks/latest")["block"]["header"]
        evidence["block"] = {key: block[key] for key in ("height", "chain_id", "time")}
        cursor = None
        for _ in range(10):  # Finite inventory; flag truncation rather than looping forever.
            pairs = smart(args.factory, {"pairs": {"start_after": cursor, "limit": 30}})["pairs"]
            for pair in pairs:
                address = pair["contract_addr"]
                info = get(f"/cosmwasm/wasm/v1/contract/{address}")["contract_info"]
                evidence["inventory"].append({"pair": address, "code_id": info["code_id"],
                                              "cw2": version(address), "asset_infos": pair["asset_infos"]})
            if len(pairs) < 30:
                break
            cursor = pairs[-1]["asset_infos"]
        else:
            evidence["inventory_truncated"] = True
        pool = smart(args.pair, {"pool": {}})
        config = smart(args.pair, {"get_fee_config": {}})
        evidence["pair"] = args.pair
        evidence["pool"] = pool
        evidence["fee_bps"] = config["fee_config"]["fee_bps"]
        # Identify token orientation from public token metadata, not a local ticker map.
        tokens = []
        for asset in pool["assets"]:
            contract = asset["info"]["token"]["contract_addr"]
            info = smart(contract, {"token_info": {}})
            tokens.append({"contract": contract, "symbol": info["symbol"], "decimals": info["decimals"]})
        evidence["tokens"] = tokens
        target_token = next(token for token in tokens if token["contract"] == args.target_token)
        ask_asset = next(a for a in pool["assets"] if a["info"]["token"]["contract_addr"] == target_token["contract"])
        offer_asset = next(a for a in pool["assets"] if a is not ask_asset)
        for human in (10, 100):
            raw = str(human * 10 ** target_token["decimals"])
            record = {"human_target": human, "raw_target": raw}
            try:
                record["quote"] = smart(args.pair, {"hybrid_reverse_simulation": {
                    "ask_asset": {"info": {"token": {"contract_addr": target_token["contract"]}}, "amount": raw},
                    "hybrid": {"pool_input": "1", "book_input": "0", "max_maker_fills": 1}}})
                record["status"] = "PASS"
            except RuntimeError as error:
                record.update(status="FAIL", reason=str(error))
            # Snapshot pool minimum independent of the failing deployed reverse implementation.
            fee = evidence["fee_bps"]
            gross = ((int(raw) - 1) * 10000) // (10000 - fee) + 1
            rin, rout = int(offer_asset["amount"]), int(ask_asset["amount"])
            if gross < rout:
                denominator = rout - gross
                seed = (rin * rout + denominator - 1) // denominator - rin
                record["snapshot_pool_offer"] = str(seed)
                try:
                    record["forward"] = smart(args.pair, {"hybrid_simulation": {
                        "offer_asset": {"info": offer_asset["info"], "amount": str(seed)},
                        "hybrid": {"pool_input": str(seed), "book_input": "0", "max_maker_fills": 1}}})
                    record["forward_sufficient"] = int(record["forward"]["return_amount"]) >= int(raw)
                except RuntimeError as error:
                    record["forward_error"] = str(error)
            evidence["reverse"].append(record)
    except (RuntimeError, URLError, TimeoutError, KeyError, ValueError, StopIteration) as error:
        evidence["unavailable"] = str(error)
        print(json.dumps(evidence, indent=2))
        return 2
    print(json.dumps(evidence, indent=2))
    return int(args.expect_fixed and any(row["status"] != "PASS" or not row.get("forward_sufficient") for row in evidence["reverse"]))


if __name__ == "__main__":
    sys.exit(main())
