#!/usr/bin/env python3
"""Create isolated LocalTerra #1391 fixtures and record real reserve/depth attacks.

Uses only LocalTerra test keyring/code IDs; never resets chain or shared contracts.
Replay receipts with ORACLE1391_CHAIN_RECEIPT=<output> cargo test --test oracle_1391_chain_receipt.
"""
import argparse
import base64
from datetime import datetime, timezone
import json
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--output", required=True)
    parser.add_argument("--cw20-code", type=int, default=1)
    parser.add_argument("--factory-code", type=int, default=3)
    parser.add_argument("--pair-code", type=int, default=4)
    args = parser.parse_args()
    prefix = f"oracle1391-{time.time_ns()}"
    receipts = []
    def cli(*command):
        result = subprocess.run(["docker", "exec", args.container, "terrad", *map(str, command)],
                                capture_output=True, text=True, check=True, timeout=20)
        return json.loads(result.stdout)
    def query(*command):
        return cli("query", *command, "--node", "http://127.0.0.1:26657", "--output", "json")
    def tx(*command, key=None):
        result = cli("tx", *command, "--from", key or prefix, "--keyring-backend", "test",
                     "--chain-id", "localterra", "--node", "http://127.0.0.1:26657", "--gas", "5000000",
                     "--fees", "500000000uluna", "--broadcast-mode", "sync", "-y", "--output", "json")
        if result.get("code", 0):
            raise RuntimeError(f"LocalTerra check failed: {result.get('raw_log')}")
        # Inclusion reconciliation stays finite under shared CI host contention;
        # an accepted transaction is never blindly rebroadcast after timeout.
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            try:
                included = query("tx", result["txhash"])
                break
            except subprocess.CalledProcessError:
                time.sleep(0.2)
        else:
            raise RuntimeError(f"LocalTerra inclusion timeout for synthetic tx {result['txhash']}")
        if included.get("code", 0):
            raise RuntimeError(f"LocalTerra execution failed: {included.get('raw_log')}")
        receipts.append(result["txhash"])
        return included
    def smart(address, msg):
        return query("wasm", "contract-state", "smart", address, json.dumps(msg))["data"]
    def execute(address, msg):
        return tx("wasm", "execute", address, json.dumps(msg))
    def instantiated(result):
        events = result.get("events", [])
        events += [e for log in result.get("logs", []) for e in log.get("events", [])]
        found = [a["value"] for event in events if event["type"] == "instantiate"
                 for a in event["attributes"] if a["key"] == "_contract_address"]
        if not found:
            raise RuntimeError("Missing instantiated contract address")
        return found[0]
    status = cli("status", "--node", "http://127.0.0.1:26657")
    node = status.get("node_info") or status.get("NodeInfo") or {}
    sync = status.get("sync_info") or status.get("SyncInfo") or {}
    if node.get("network") != "localterra":
        raise RuntimeError("Refusing fixture mutation outside LocalTerra")
    stamp = sync.get("latest_block_time", "")
    # RFC3339 nanoseconds normalize to Python's microsecond precision.
    if "." in stamp:
        left, rest = stamp.split(".", 1)
        stamp = left + "." + rest.rstrip("Z")[:6] + "+00:00"
    observed = datetime.fromisoformat(stamp.replace("Z", "+00:00"))
    age = (datetime.now(timezone.utc) - observed).total_seconds()
    if age < 0 or age > 60:
        raise RuntimeError(f"LocalTerra not live: latest block is {int(age)}s old")
    for code in (args.cw20_code, args.factory_code, args.pair_code):
        query("wasm", "code-info", code)
    # Mnemonic-bearing key-add output is consumed only for address and never printed.
    wallet = cli("keys", "add", prefix, "--keyring-backend", "test", "--output", "json")["address"]
    tx("bank", "send", "test1", wallet, "50000000000uluna", key="test1")
    tokens = {}
    for ticker in ("custc", "ust1", "ustr"):
        msg = {"name": f"Synthetic {ticker} 1391", "symbol": ticker.upper(), "decimals": 6,
               "initial_balances": [{"address": wallet, "amount": "1000000000000000"}], "mint": None}
        tokens[ticker] = instantiated(tx("wasm", "instantiate", args.cw20_code, json.dumps(msg),
                                            "--label", prefix + ticker, "--no-admin"))
    factory_msg = {"governance": wallet, "treasury": wallet, "default_fee_bps": 30,
                   "pair_code_id": args.pair_code, "lp_token_code_id": args.cw20_code,
                   "whitelisted_code_ids": [args.cw20_code], "pair_creation_fee_uluna": "0"}
    factory = instantiated(tx("wasm", "instantiate", args.factory_code, json.dumps(factory_msg),
                              "--label", prefix + "factory", "--no-admin"))
    pairs = {}
    for name, ticker0, ticker1, amount0, amount1 in (
        ("ust1-custc", "ust1", "custc", 1000000000, 200000000000),
        ("ustr-custc", "ustr", "custc", 1000000000, 400000000000),
        ("ustr-ust1", "ustr", "ust1", 1000000000, 1000000000),
    ):
        infos = [{"token": {"contract_addr": tokens[t]}} for t in (ticker0, ticker1)]
        execute(factory, {"create_pair": {"asset_infos": infos}})
        pair_response = smart(factory, {"pair": {"asset_infos": infos}})
        pair = pair_response.get("pair", pair_response)
        address = pair["contract_addr"]
        for ticker in (ticker0, ticker1):
            execute(tokens[ticker], {"increase_allowance": {"spender": address, "amount": "1000000000000000"}})
        execute(address, {"provide_liquidity": {"assets": [
            {"info": infos[0], "amount": str(amount0)}, {"info": infos[1], "amount": str(amount1)}],
            "slippage_tolerance": None, "receiver": wallet}})
        pairs[name] = {"address": address, "lp_token": pair["liquidity_token"]}
    def snapshot(stage):
        return {"stage": stage, "recorded_at": datetime.now(timezone.utc).isoformat(),
                "pairs": {name: {**info, "pool": smart(info["address"], {"pool": {}})} for name, info in pairs.items()}}
    stages = [snapshot("before")]
    swap = base64.b64encode(json.dumps({"swap": {"max_spread": "0.5"}}).encode()).decode()
    # Keep the deliberate reserve-shift attack below the unchanged 50% trade
    # protection: exact half-reserve input exceeds it by integer rounding dust.
    execute(tokens["custc"], {"send": {"contract": pairs["ust1-custc"]["address"],
                                     "amount": "90000000000", "msg": swap}})
    stages.append(snapshot("canonical_reserve_shift"))
    lp = pairs["ustr-custc"]["lp_token"]
    balance = int(smart(lp, {"balance": {"address": wallet}})["balance"])
    withdraw = base64.b64encode(b'{"withdraw_liquidity":{}}').decode()
    execute(lp, {"send": {"contract": pairs["ustr-custc"]["address"],
                         "amount": str(balance * 9999 // 10000), "msg": withdraw}})
    stages.append(snapshot("candidate_depth_loss"))
    out = {"scope": "LocalTerra isolated test wallet/tokens/factory; no production deployment",
           "wallet": wallet, "tokens": tokens, "factory": factory, "stages": stages,
           "transaction_hashes": receipts, "code_ids": {"cw20": args.cw20_code,
           "factory": args.factory_code, "pair": args.pair_code}}
    Path(args.output).write_text(json.dumps(out, indent=2) + "\n")
    print(f"Recorded {len(receipts)} LocalTerra transactions and {len(stages)} snapshots at {args.output}")


if __name__ == "__main__":
    main()
