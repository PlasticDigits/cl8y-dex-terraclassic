#!/usr/bin/env python3
"""Measure isolated synthetic reverse smart queries inside LocalTerra transactions.

Never resets the shared chain or queries production. Query-probe receipts include
transaction/wrapper overhead, so gas_used is a conservative query upper bound.
"""
import argparse
import base64
import importlib.util
import json
import time
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path

HELPER = Path(__file__).with_name("localterra-route-tax-1381.py")
SPEC = importlib.util.spec_from_file_location("localterra_tax", HELPER)
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--artifacts", type=Path, default=Path(__file__).resolve().parents[2] / "smartcontracts/artifacts")
    parser.add_argument("--probe", type=Path, default=Path(__file__).with_name("fixtures") / "reverse-query-probe/target/wasm32-unknown-unknown/release/reverse_query_probe.wasm")
    parser.add_argument("--router-version", default="1.3.1", help="Expected reviewed router cw2 version (historical comparison only)")
    parser.add_argument("--code-cache", type=Path)
    parser.add_argument("--evidence-file", required=True, type=Path)
    args = parser.parse_args()
    chain = helper.LocalTerra(args.container, args.code_cache)
    # Only bootstrap funding uses the shared genesis account. All subsequent
    # transactions use a unique test signer, avoiding concurrent QA nonce races.
    key = "issue1389-" + str(time.time_ns())
    chain.command("terrad", "keys", "add", key, "--keyring-backend", "test", "--output", "json")
    owner = chain.address(key)
    for attempt in range(5):
        try:
            chain.tx("bank", "send", "test1", owner, "100000000000uluna", key="test1")
            break
        except RuntimeError as error:
            # Only retry a rejected bootstrap send; never retry accepted txs or
            # arbitrary failures, and never use this logic outside LocalTerra.
            if "LocalTerra broadcast rejected: account sequence mismatch" not in str(error) or attempt == 4:
                raise
            time.sleep(1)
    chain.tx_key = key
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in (
        "cl8y_dex_factory", "cl8y_dex_pair", "cl8y_dex_router", "cw20_mintable")}
    probe_code = chain.store(args.probe)
    _, treasury = chain.fresh_trader()
    factory = chain.instantiate(codes["cl8y_dex_factory"]["code_id"], {
        "governance": owner, "treasury": treasury, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"]["code_id"],
        "lp_token_code_id": codes["cw20_mintable"]["code_id"],
        "whitelisted_code_ids": [codes["cw20_mintable"]["code_id"]],
        "pair_creation_fee_uluna": "0"}, "issue1389-factory")
    router = chain.instantiate(codes["cl8y_dex_router"]["code_id"], {"factory": factory}, "issue1389-router")
    probe = chain.instantiate(probe_code["code_id"], {}, "issue1389-test-query-probe")
    decimals = [6, 18, 6, 18, 6]
    tokens = [chain.instantiate(codes["cw20_mintable"]["code_id"], {
        "name": "Synthetic reverse " + str(i), "symbol": "REV", "decimals": d,
        "initial_balances": [{"address": owner, "amount": str(10 ** (d + 9))}],
        "mint": {"minter": owner}}, "issue1389-token-" + str(i)) for i, d in enumerate(decimals)]
    pairs = []
    for i in range(4):
        infos = [helper.asset(token) for token in tokens[i:i+2]]
        chain.execute(factory, {"create_pair": {"asset_infos": infos}})
        pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        pairs.append(pair)
        for j in (i, i+1):
            chain.execute(tokens[j], {"increase_allowance": {
                "spender": pair, "amount": str(10 ** (decimals[j] + 6))}})
        chain.execute(pair, {"provide_liquidity": {"assets": [
            {"info": helper.asset(tokens[j]), "amount": str(10 ** (decimals[j] + 6))}
            for j in (i, i+1)]}})

    checks = []
    def version(address):
        raw = chain.query("wasm", "contract-state", "raw", address, "contract_info", "--ascii")
        return json.loads(base64.b64decode(raw["data"]))

    pair_versions = {pair: version(pair) for pair in pairs}
    router_version = version(router)
    assert all(value == {"contract": "cl8y-dex-pair", "version": "1.20.0"}
               for value in pair_versions.values()), pair_versions
    assert router_version == {"contract": "cl8y-dex-router", "version": args.router_version}, router_version
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "chain_id": "localterra",
        "synthetic_only": True, "gas_scope": "transaction including test-wrapper overhead",
        "codes": codes, "probe_code": probe_code, "factory": factory, "router": router,
        "pairs": pairs, "pair_versions": pair_versions, "router_version": router_version,
        "expected_router_version": args.router_version,
        "tokens": tokens, "decimals": decimals, "checks": checks, "status": "PARTIAL"}

    def save():
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")

    def measure(label, contract, query, expected_error=None):
        receipt = chain.execute(probe, {"contract": contract,
            "query": base64.b64encode(json.dumps(query).encode()).decode()})
        status = chain.event(receipt, "wasm", "query_status")
        error = chain.event(receipt, "wasm", "query_error") if status == "error" else None
        row = {"label": label, "gas_used": int(receipt["gas_used"]), "height": receipt["height"],
            "txhash": receipt["txhash"], "query_status": status, "error": error}
        checks.append(row)
        save()
        if expected_error:
            assert status == "error" and expected_error in error, (label, status, error)
        else:
            assert status == "ok", (label, error)
        # A fixed test envelope, never a production gas guarantee.
        assert row["gas_used"] < 12_000_000, row
        row["status"] = "PASS"
        print(json.dumps(row), flush=True)

    def route(names, mode):
        ops = helper.operations(names)
        for op in ops:
            if mode == "greedy":
                op["terra_swap"]["greedy"] = {"max_maker_fills": 100}
            elif mode == "declared":
                op["terra_swap"]["hybrid"] = {"pool_input": "1", "book_input": "1", "max_maker_fills": 100}
        return ops

    def router_query(names, target, mode):
        return {"reverse_simulate_swap_operations": {
            "ask_amount": str(target), "operations": route(names, mode)}}

    def forward_query(names, mode, offer):
        ops = route(names, mode)
        if mode == "declared":
            # Reverse accepts ratio templates; executable forward operations
            # require each split to sum to that hop's actual economic offer.
            current = int(offer)
            for op, a, b in zip(ops, names, names[1:]):
                split = {"pool_input": str(current - current // 2),
                         "book_input": str(current // 2), "max_maker_fills": 100}
                op["terra_swap"]["hybrid"] = split
                pair_index = min(tokens.index(a), tokens.index(b))
                hop = chain.smart(pairs[pair_index], {"hybrid_simulation": {
                    "offer_asset": {"info": helper.asset(a), "amount": str(current)},
                    "hybrid": split}})
                current = int(hop["return_amount"])
        return {"simulate_swap_operations": {"offer_amount": offer, "operations": ops}}

    for direction in (False, True):
        names = tokens[::-1] if direction else tokens
        for mode in ("pool", "greedy", "declared"):
            for hops in ((1, 2) if mode == "declared" else (1, 4)):
                target_decimals = decimals[::-1][hops] if direction else decimals[hops]
                target = 100 * 10 ** target_decimals
                query = router_query(names[:hops+1], target, mode)
                measure(f"empty {mode} {hops} hops reverse={direction}", router, query)
                quote = chain.smart(router, query)
                forward = chain.smart(router, forward_query(names[:hops+1], mode, quote["amount"]))
                assert int(forward["amount"]) >= target, (quote, forward)
        # Three opaque 32-call reservations cannot fit alongside fresh validations.
        measure(f"declared whole-request denial reverse={direction}", router,
                router_query(names, 100 * 10 ** 6, "declared"), "REVERSE_QUERY_BUDGET_EXCEEDED")

    # Two opposing 20-order books per pair exercise maker walks in four capped hops.
    for pair in pairs:
        assets = chain.smart(pair, {"pool": {}})["assets"]
        ordered_tokens = [asset["info"]["token"]["contract_addr"] for asset in assets]
        ds = [decimals[tokens.index(token)] for token in ordered_tokens]
        for side, index in (("ask", 0), ("bid", 1)):
            # Strictly beat the pool in each orientation so greedy visits makers.
            price = format(Decimal(10) ** (ds[1] - ds[0]) *
                           (Decimal("0.99") if side == "ask" else Decimal("1.01")), "f")
            amount = 10 ** (ds[index] + 1)
            hook = {"place_limit_order_batch": {"side": side, "orders": [
                {"price": price, "amount": str(amount), "max_adjust_steps": 256}
                for _ in range(20)]}}
            chain.execute(ordered_tokens[index], {"send": {"contract": pair,
                "amount": str(amount * 20), "msg": base64.b64encode(json.dumps(hook).encode()).decode()}})
        for index in (0, 1):
            walk = chain.smart(pair, {"hybrid_simulation": {
                "offer_asset": {"info": helper.asset(ordered_tokens[index]),
                                "amount": str(100 * 10 ** ds[index])},
                "greedy": {"max_maker_fills": 100}}})
            assert int(walk["limit_book_offer_consumed"]) > 0, (pair, index, walk)
    for direction in (False, True):
        names = tokens[::-1] if direction else tokens
        # Greedy full-book precision may intentionally exhaust the small budget.
        query = router_query(names, 100 * 10 ** 6, "greedy")
        try:
            quote = chain.smart(router, query)
        except RuntimeError as error:
            assert "REVERSE_QUERY_BUDGET_EXCEEDED" in str(error), str(error)
            measure(f"full greedy four-hop budget reverse={direction}", router, query,
                    "REVERSE_QUERY_BUDGET_EXCEEDED")
        else:
            measure(f"full greedy four-hop sufficient reverse={direction}", router, query)
            forward = chain.smart(router, {"simulate_swap_operations": {
                "offer_amount": quote["amount"], "operations": route(names, "greedy")}})
            assert int(forward["amount"]) >= 100 * 10 ** 6
    measure("invalid Observe horizon", pairs[0], {"observe": {"seconds_ago": [2 ** 32 - 1]}},
            "seconds_ago")
    evidence["status"] = "PASS"
    save()


if __name__ == "__main__":
    main()
