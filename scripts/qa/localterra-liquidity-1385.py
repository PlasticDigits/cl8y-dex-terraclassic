#!/usr/bin/env python3
"""Real owned LocalTerra LP control; consumes only synthetic test keys.

Provision a dedicated chain (never reset shared volumes), then run this fixture
with its container, optimized artifacts, private QA env and evidence directory.
The Rust consumer uses a disposable QA database and a local LCD outage bridge.
"""
import argparse
import base64
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("lp_fixture", Path(__file__).with_name("localterra-route-tax-1381.py"))
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--artifacts", type=Path, default=ROOT / "smartcontracts/artifacts")
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--qa-env", type=Path)
    parser.add_argument("--withdraw", action="store_true")
    args = parser.parse_args()
    args.evidence_dir.mkdir(parents=True, exist_ok=True)
    fixture_path = args.evidence_dir / "chain.json"
    if args.withdraw:
        fixture = json.loads(fixture_path.read_text())
        assert fixture["container"] == args.container
        chain = module.LocalTerra(args.container, tx_key=fixture["test_key"])
        if fixture.get("withdrawal_receipts"):
            assert chain.smart(fixture["pair"], {"pool": {}}) == fixture["after_withdrawal"]
            print("Existing synthetic withdrawal receipt reconciled; no repeat broadcast")
            return
        amount = chain.balance(fixture["lp_token"], chain.address(chain.tx_key)) * 9 // 10
        chain.execute(fixture["lp_token"], {"send": {"contract": fixture["pair"],
                      "amount": str(amount), "msg": base64.b64encode(b'{"withdraw_liquidity":{}}').decode()}})
        fixture["withdrawal_amount"] = str(amount)
        fixture["withdrawal_receipts"] = chain.receipts
        fixture["after_withdrawal"] = chain.smart(fixture["pair"], {"pool": {}})
        fixture_path.write_text(json.dumps(fixture, indent=2) + "\n")
        print("Executed synthetic LP withdrawal; receipt recorded")
        return
    if not args.qa_env:
        parser.error("--qa-env required for the disposable database consumer")
    chain = module.LocalTerra(args.container)
    codes = {name: chain.store(args.artifacts / (name + ".wasm"))
             for name in ("cl8y_dex_factory", "cl8y_dex_pair", "cw20_mintable")}
    for code in codes.values():
        assert chain.query("wasm", "code-info", str(code["code_id"]))["checksum"].lower() == code["sha256"]
    key, wallet = chain.fresh_trader("50000000000uluna")
    chain.tx("bank", "send", "test1", wallet, "50000000000uusd")
    chain.tx_key = key
    tokens = {ticker: chain.instantiate(codes["cw20_mintable"]["code_id"], {
        "name": "Synthetic wrapped " + ticker, "symbol": ticker, "decimals": 6,
        "initial_balances": [{"address": wallet, "amount": "10000000000"}], "mint": None},
        "ops1385-" + ticker + "-" + str(time.time_ns())) for ticker in ("CLUNC", "CUSTC")}
    factory = chain.instantiate(codes["cl8y_dex_factory"]["code_id"], {
        "governance": wallet, "treasury": wallet, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"]["code_id"],
        "lp_token_code_id": codes["cw20_mintable"]["code_id"],
        "whitelisted_code_ids": [codes["cw20_mintable"]["code_id"]],
        "pair_creation_fee_uluna": "0"}, "ops1385-" + str(time.time_ns()))
    infos = [{"token": {"contract_addr": tokens[d]}} for d in ("CLUNC", "CUSTC")]
    chain.execute(factory, {"create_pair": {"asset_infos": infos}})
    pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]
    for token in tokens.values():
        chain.execute(token, {"increase_allowance": {"spender": pair["contract_addr"], "amount": "1000000000"}})
    chain.execute(pair["contract_addr"], {
            "provide_liquidity": {"assets": [{"info": info, "amount": "1000000000"} for info in infos],
                                  "receiver": wallet}})
    fixture = {"scope": "dedicated LocalTerra synthetic LP; no production wallets or DB",
               "container": args.container, "test_key": key, "factory": factory,
               "pair": pair["contract_addr"], "lp_token": pair["liquidity_token"],
               "codes": codes, "tokens": tokens, "receipts": chain.receipts,
               "before": chain.smart(pair["contract_addr"], {"pool": {}})}
    fixture_path.write_text(json.dumps(fixture, indent=2) + "\n")
    flag = args.evidence_dir / "outage.flag"
    flag.unlink(missing_ok=True)

    class Bridge(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            if flag.exists():
                self.send_response(503)
                self.end_headers()
                self.wfile.write(b'{"error":"isolated LCD outage"}')
                return
            # Forward the actual node's response. No reserve/query response mock.
            result = subprocess.run(["docker", "exec", args.container, "curl", "-sS",
                                     "--max-time", "2", "http://127.0.0.1:1317" + self.path],
                                    capture_output=True, timeout=3)
            self.send_response(200 if result.returncode == 0 else 502)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(result.stdout)

    bridge = ThreadingHTTPServer(("127.0.0.1", 0), Bridge)
    threading.Thread(target=bridge.serve_forever, daemon=True).start()
    env = os.environ.copy()
    for line in args.qa_env.read_text().splitlines():
        if line and not line.startswith("#"):
            name, value = line.split("=", 1)
            env[name] = value
    env.update(LIQUIDITY1385_FIXTURE=str(fixture_path),
               HUB_CLUNC_ADDRESS=tokens["CLUNC"], HUB_CUSTC_ADDRESS=tokens["CUSTC"],
               LIQUIDITY1385_LCD_URL=f"http://127.0.0.1:{bridge.server_port}",
               LIQUIDITY1385_OUTAGE_FLAG=str(flag),
               LIQUIDITY1385_EVIDENCE=str(args.evidence_dir / "indexer.json"))
    try:
        subprocess.run(["cargo", "test", "--test", "ops_liquidity_1385", "--", "--ignored", "--nocapture"],
                       cwd=ROOT / "indexer", env=env, check=True)
    finally:
        bridge.shutdown()
        flag.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
