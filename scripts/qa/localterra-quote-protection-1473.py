#!/usr/bin/env python3
"""Prove quote-relative net floors with the existing router on owned LocalTerra."""
import argparse
import base64
import hashlib
import importlib.util
import json
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path

spec = importlib.util.spec_from_file_location("chain_fixture", Path(__file__).with_name("localterra-route-tax-1381.py"))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
monitor_spec = importlib.util.spec_from_file_location("owned_inclusion_monitor", Path(__file__).with_name("localterra-offline-gas-1480.py"))
monitor = importlib.util.module_from_spec(monitor_spec)
monitor_spec.loader.exec_module(monitor)
SCALE = 10 ** 18


def decimal_text(atomics):
    return (str(atomics // SCALE) + "." + str(atomics % SCALE).zfill(18)).rstrip("0").rstrip(".")


def ceiling(numerator, denominator, tolerance):
    n = (numerator * SCALE + denominator * tolerance) * SCALE
    d = denominator * (SCALE - tolerance)
    return min(SCALE, (n + d - 1) // d)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--code-cache", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    artifact_provenance = monitor.provenance(args.artifacts)
    chain = monitor.BudgetChain(args.container, args.code_cache)
    chain.pending_path = args.output.with_suffix(".pending.private.json")
    owner_key, owner = chain.fresh_trader("200000000000uluna")
    chain.default_key = owner_key
    _, treasury = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in
             ("cl8y_dex_factory", "cl8y_dex_pair", "cl8y_dex_router", "cw20_mintable")}
    factory = chain.instantiate(codes["cl8y_dex_factory"]["code_id"], {
        "governance": owner, "treasury": treasury, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"]["code_id"],
        "lp_token_code_id": codes["cw20_mintable"]["code_id"],
        "whitelisted_code_ids": [codes["cw20_mintable"]["code_id"]],
        "pair_creation_fee_uluna": "0"}, "quote1473-factory")
    router = chain.instantiate(codes["cl8y_dex_router"]["code_id"], {"factory": factory}, "quote1473-router")
    tokens, pairs = {}, {}
    decimals = {"RAW6": 6, "RAW18": 18, "RAW9": 9}
    for label, places in decimals.items():
        tokens[label] = chain.instantiate(codes["cw20_mintable"]["code_id"], {
            "name": "Quote fixture " + label, "symbol": label, "decimals": places,
            "initial_balances": [{"address": owner, "amount": str(10 ** (places + 9))}],
            "mint": {"minter": owner}}, "quote1473-" + label)
    for a, b in [("RAW6", "RAW18"), ("RAW18", "RAW9")]:
        infos = [fixture.asset(tokens[a]), fixture.asset(tokens[b])]
        chain.execute(factory, {"create_pair": {"asset_infos": infos}})
        pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        pairs[frozenset((a, b))] = pair
        for label in (a, b):
            chain.execute(tokens[label], {"increase_allowance": {"spender": pair, "amount": str(10 ** (decimals[label] + 7))}})
        chain.execute(pair, {"provide_liquidity": {"assets": [
            {"info": fixture.asset(tokens[label]), "amount": str((1000 if label == a else 1500) * 10 ** decimals[label])}
            for label in (a, b)], "slippage_tolerance": None}})
    started = datetime.now(timezone.utc).isoformat()
    rows = []

    def snapshot():
        return [chain.smart(pair, {"pool": {}}) for pair in pairs.values()]

    def execute(path, offer, operations, market, floor, fail=False):
        hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": operations,
                "max_spread": decimal_text(market), "minimum_receive": str(floor),
                "to": owner, "deadline": None, "unwrap_output": False}}
        return chain.execute(tokens[path[0]], {"send": {"contract": router, "amount": str(offer),
            "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}, expect_failure=fail)

    for path in [("RAW6", "RAW18"), ("RAW18", "RAW6"), ("RAW6", "RAW18", "RAW9"), ("RAW9", "RAW18", "RAW6")]:
        for size in ["100", "1000"]:
            for selected in ["0.01", "0.05", "0.30"]:
                tolerance = int(Decimal(selected) * SCALE)
                offer = int(size) * 10 ** decimals[path[0]]
                amount, operations, market, legacy_blocked = offer, [], tolerance, False
                expected_hops = []
                for a, b in zip(path, path[1:]):
                    pair = pairs[frozenset((a, b))]
                    pool = chain.smart(pair, {"pool": {}})
                    reserves = {x["info"]["token"]["contract_addr"]: int(x["amount"]) for x in pool["assets"]}
                    reserve_in, reserve_out = reserves[tokens[a]], reserves[tokens[b]]
                    gross = amount * reserve_out // (reserve_in + amount)
                    commission = gross * 30 // 10000
                    expected = gross - commission
                    assert expected > 0
                    spot = amount * reserve_out // reserve_in
                    numerator = min(spot - gross, gross)
                    simulation = chain.smart(pair, {"hybrid_simulation": {
                        "offer_asset": {"info": fixture.asset(tokens[a]), "amount": str(amount)},
                        "hybrid": None, "greedy": None, "trader": owner, "sender": owner,
                        "belief_price": None}})
                    assert int(simulation["return_amount"]) == expected, "Independent raw arithmetic differs from contract"
                    floor = max(1, (expected * (SCALE - tolerance) + SCALE - 1) // SCALE)
                    operations.append({"terra_swap": {"offer_asset_info": fixture.asset(tokens[a]),
                        "ask_asset_info": fixture.asset(tokens[b]), "hybrid": None, "greedy": None,
                        "min_return": str(floor)}})
                    market = max(market, ceiling(numerator, gross, tolerance))
                    legacy_blocked |= numerator * SCALE > gross * tolerance
                    expected_hops.append(expected)
                    amount = expected
                minimum = max(1, (amount * (SCALE - tolerance) + SCALE - 1) // SCALE)
                before = snapshot()
                if legacy_blocked:
                    execute(path, offer, operations, tolerance, minimum, fail=True)
                    assert snapshot() == before, "Legacy spread rejection changed reserves/LP shares"
                poisoned = json.loads(json.dumps(operations))
                poisoned[-1]["terra_swap"]["min_return"] = str(amount + 1)
                execute(path, offer, poisoned, market, minimum, fail=True)
                assert snapshot() == before, "Rejected net floor changed reserves/LP shares"
                wallet_before = chain.balance(tokens[path[-1]], owner)
                receipt = execute(path, offer, operations, market, minimum)
                delivered = chain.balance(tokens[path[-1]], owner) - wallet_before
                assert delivered == amount and delivered >= minimum, {"direction": "->".join(path), "quote": str(amount), "delivered": str(delivered), "floor": str(minimum), "contract_amounts": [a["value"] for e in receipt.get("events", []) for a in e.get("attributes", []) if a.get("key") in ("return_amount", "net_return_amount", "commission_amount")]}
                rows.append({"direction": "->".join(path), "human_offer": size, "selected_tolerance": selected,
                    "raw_offer": str(offer), "expected_hop_outputs": [str(x) for x in expected_hops],
                    "net_quote": str(amount), "minimum_receive": str(minimum), "delivered": str(delivered),
                    "execution_market_spread": decimal_text(market), "legacy_false_rejection_reproduced": legacy_blocked,
                    "rejected_floor_reserves_and_lp_unchanged": True, "status": "PASS"})
                print("PASS " + rows[-1]["direction"] + " " + size + " " + selected, flush=True)
    assert any(x["legacy_false_rejection_reproduced"] for x in rows)
    result = {"issue": 1473, "scope": "Owned LocalTerra hook-free 6/18/9 decimal direct and multihop existing-router execution; arithmetic independently checked against contract; no production signing or deployment claim",
              "utc_start": started, "utc_end": datetime.now(timezone.utc).isoformat(), "status": "PASS",
              "source_driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "inclusion_monitor_sha256": hashlib.sha256(Path(__file__).with_name("localterra-offline-gas-1480.py").read_bytes()).hexdigest(),
              "inclusion_monitor": "Bounded 120-second REST reconciliation of original accepted hash; private pending identity; no resend",
              "source_artifact_provenance": artifact_provenance,
              "artifacts": codes, "cases": rows, "case_count": len(rows),
              "skips": ["funded tax/hook parity of derived ceiling", "native wrapping", "deployed acceptance"]}
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
