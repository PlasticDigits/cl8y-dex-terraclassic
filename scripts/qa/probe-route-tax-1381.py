#!/usr/bin/env python3
"""Bounded, read-only public router/token capability probe for #1381."""
import argparse
import base64
import json
from datetime import datetime, timezone
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lcd", default="https://terra-classic-lcd.publicnode.com")
    parser.add_argument("--router", required=True)
    parser.add_argument("--tokens", nargs="+", required=True)
    parser.add_argument("--pair")
    parser.add_argument("--trader", help="Synthetic address only; do not use reporter identities")
    parser.add_argument("--gross", default="1010000")
    parser.add_argument("--expect-funded", action="store_true")
    args = parser.parse_args()
    if len(args.tokens) > 16 or not args.gross.isascii() or not args.gross.isdecimal() or len(args.gross) > 39:
        parser.error("Maximum 16 tokens and a Uint128 gross budget")
    if int(args.gross) > 2**128 - 1:
        parser.error("Gross budget exceeds Uint128")
    if bool(args.pair) != bool(args.trader):
        parser.error("Supply both pair and synthetic trader for preview checks")
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "probe_only": True,
                "lcd": args.lcd, "router": args.router, "tokens": []}

    def get(path):
        request = Request(args.lcd.rstrip("/") + path, headers={"User-Agent": "CL8Y-route-tax-probe/1"})
        with urlopen(request, timeout=10) as response:
            return json.load(response)

    def smart(address, msg):
        encoded = quote(base64.b64encode(json.dumps(msg).encode()).decode(), safe="")
        result = get(f"/cosmwasm/wasm/v1/contract/{address}/smart/{encoded}")["data"]
        return json.loads(base64.b64decode(result)) if isinstance(result, str) else result

    def inventory(address):
        info = get(f"/cosmwasm/wasm/v1/contract/{address}")["contract_info"]
        code = get(f'/cosmwasm/wasm/v1/code/{info["code_id"]}')
        return {"code_id": info["code_id"], "admin": info.get("admin"),
                "data_hash": code.get("code_info", {}).get("data_hash")}

    errors = []
    try:
        header = get("/cosmos/base/tendermint/v1beta1/blocks/latest")["block"]["header"]
        evidence["block"] = {key: header[key] for key in ("chain_id", "height", "time")}
        evidence["router_inventory"] = inventory(args.router)
        evidence["router_config"] = smart(args.router, {"config": {}})
        if evidence["router_config"].get("funding_version") != 1:
            errors.append("Router does not advertise funding version 1")
        for address in dict.fromkeys(args.tokens):
            record = {"contract": address, **inventory(address)}
            evidence["tokens"].append(record)
            try:
                record["capability"] = smart(address, {"route_tax_capability": {}})
                cap = record["capability"]
                if cap.get("funding_version") != 1 or cap.get("router") != args.router:
                    errors.append("Token funding/router capability mismatch: " + address)
                if args.pair:
                    record["preview"] = smart(address, {"route_tax_preview": {
                        "kind": "sell", "amount": args.gross, "trader": args.trader, "pair": args.pair}})
                    preview = record["preview"]
                    gross, economic, tax = (int(preview[key]) for key in ("gross", "economic", "tax"))
                    if preview.get("funding_version") != 1 or gross != int(args.gross) or economic + tax > gross:
                        errors.append("Invalid funded preview: " + address)
            except HTTPError as error:
                record["capability_error"] = f"HTTP {error.code}"
                errors.append("Token capability unavailable: " + address)
    except (HTTPError, URLError, TimeoutError, KeyError, ValueError) as error:
        errors.append(type(error).__name__ + ": public probe unavailable")
    evidence["limitations"] = ["Read-only probes do not establish execution success or immutable-code trust configuration"]
    evidence["errors"] = errors
    evidence["status"] = "FAIL" if errors else "PASS"
    print(json.dumps(evidence, indent=2))
    return 1 if args.expect_funded and errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
