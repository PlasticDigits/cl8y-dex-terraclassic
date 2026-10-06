#!/usr/bin/env python3
"""Synthetic #1390 launch-guard execution using an isolated #1381 fixture."""
import argparse
import base64
import importlib.util
import json
from datetime import datetime, timezone
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "route_tax1381", Path(__file__).with_name("localterra-route-tax-1381.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--evidence-file", type=Path, required=True)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    assert fixture["chain_id"] == "localterra" and fixture["status"] == "PASS"
    chain = module.LocalTerra(args.container)
    chain.owner_key = fixture.get("owner_key", "test1")
    owner, sink = chain.address(chain.owner_key), chain.address("test2")
    router, factory = fixture["router"], fixture["factory"]
    ust1, cl8y = fixture["tokens"]["UST1"], fixture["tokens"]["CL8Y"]
    guard = chain.instantiate(fixture["codes"]["cl8y_community_tax_token"]["code_id"], {
        "name": "GuardSynthetic", "symbol": "GRD", "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000000000000"}],
        "manager": owner, "treasury": sink, "buy_bps": 450, "sell_bps": 100,
        "max_buy_bps": 450, "max_sell_bps": 100, "max_transfer_bps": 0,
        "factory": factory, "router": router, "ust1": ust1, "cmm_treasury": owner,
        "features": ["launch_guards", "exemption_directory", "variable_rates"], "mint": None,
        "transfer_bps": None, "sinks": None, "autolp": None, "launcher": None,
        # The first paid settings save changes this to zero (no-op saves must fail).
        "launch_guards": {"max_wallet": None, "cooldown_blocks": 2,
                          "trading_enabled": True}}, "issue1390-guard")
    pairs = []
    for other, amount in ((ust1, "1000000000000"), (cl8y, "1000000000000000000000000")):
        infos = [module.asset(guard), module.asset(other)]
        chain.execute(factory, {"create_pair": {"asset_infos": infos}})
        pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        pairs.append(pair)
        chain.execute(guard, {"register_listed_pair": {"pair": pair}})
        for token, raw in ((guard, "1000000000000"), (other, amount)):
            chain.execute(token, {"increase_allowance": {"spender": pair, "amount": raw}})
        chain.execute(pair, {"provide_liquidity": {"assets": [
            {"info": module.asset(guard), "amount": "1000000000000"},
            {"info": module.asset(other), "amount": amount}], "slippage_tolerance": None}})
    tokens = (guard, ust1, cl8y)
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "chain_id": "localterra",
                "synthetic_only": True, "router": router, "token": guard,
                "codes": fixture["codes"], "checks": [], "status": "PARTIAL"}

    def record(name, extra=None):
        evidence["checks"].append({"name": name, "status": "PASS", **(extra or {})})
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
        print("PASS LocalTerra " + name, flush=True)

    def snapshot(trader):
        return {"tokens": {t: {"wallet": chain.balance(t, trader),
            "router": chain.balance(t, router), "sink": chain.balance(t, sink),
            "supply": chain.smart(t, {"token_info": {}})["total_supply"]} for t in tokens},
            "pools": {p: chain.smart(p, {"pool": {}}) for p in pairs}}

    def quote(ops, trader, reverse=False, amount=100):
        query = "reverse_simulate_swap_operations" if reverse else "simulate_swap_operations"
        field = "ask_amount" if reverse else "offer_amount"
        return chain.smart(router, {query: {field: str(amount), "operations": ops,
                                           "trader": trader, "sender": trader}})

    def send(token, ops, amount, to=None):
        hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": ops,
            "max_spread": "0.5", "minimum_receive": None, "to": to,
            "deadline": None, "unwrap_output": False}}
        return {"contract": token, "msg": {"send": {"contract": router,
            "amount": str(amount), "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}}

    def settings(max_wallet=None, cooldown=0, buy=450, sell=100):
        hook = {"update_settings": {"settings": {"buy_bps": buy, "sell_bps": sell,
            "launch_guards": {"max_wallet": max_wallet, "cooldown_blocks": cooldown,
                              "trading_enabled": True}}}}
        chain.execute(ust1, {"send": {"contract": guard, "amount": "50000000",
            "msg": base64.b64encode(json.dumps(hook).encode()).decode()}})

    key, trader = chain.fresh_trader()
    chain.execute(guard, {"transfer": {"recipient": trader, "amount": "1000"}})
    chain.execute(cl8y, {"transfer": {"recipient": trader, "amount": "1000000000000000000"}})
    for cooldown in (0, 1, 5):
        settings(cooldown=cooldown)
        for names in ((guard, ust1, guard), (cl8y, guard, ust1, guard)):
            ops = module.operations(names)
            for reverse in (False, True):
                try:
                    quote(ops, trader, reverse)
                except RuntimeError as error:
                    assert "repeated route-tax-capable asset" in str(error), str(error)
                else:
                    raise AssertionError("Cyclic capable route quoted successfully")
            before = snapshot(trader)
            receipt = chain.multi_execute([send(names[0], ops, 100)], key, expect_failure=True)
            assert "repeated route-tax-capable asset" in receipt.get("raw_log", "")
            assert snapshot(trader) == before, "Cyclic failure changed funded balances/pools"
            record("cyclic forward/reverse/execute cooldown " + str(cooldown),
                   {"path": names, "txhash": receipt["txhash"]})
    settings(buy=0, sell=0)
    try:
        quote(module.operations((guard, ust1, guard)), trader)
    except RuntimeError as error:
        assert "repeated route-tax-capable asset" in str(error)
    else:
        raise AssertionError("Zero-rate capable cycle escaped policy")
    record("zero-rate capable cycle rejected")

    settings(max_wallet="10")
    for amount, residue in ((100, 1), (101, 0), (100, 1)):
        ops = module.operations((guard, ust1))
        expected = int(quote(ops, trader, amount=amount)["amount"])
        before = snapshot(trader)
        receipt = chain.multi_execute([send(guard, ops, amount)], key)
        after = snapshot(trader)
        assert before["tokens"][guard]["wallet"] - after["tokens"][guard]["wallet"] == amount - residue
        assert after["tokens"][ust1]["wallet"] - before["tokens"][ust1]["wallet"] == expected
        assert after["tokens"][guard]["router"] == before["tokens"][guard]["router"]
        record("owned input restoration above lowered cap", {"gross": amount,
            "residue": residue, "txhash": receipt["txhash"]})

    for recipient in (None, chain.fresh_trader()[1]):
        ops = module.operations((cl8y, guard))
        gross = 1000000000000000000
        assert int(quote(ops, trader, amount=gross)["amount"]) > 10
        before = snapshot(trader)
        recipient_before = chain.balance(guard, recipient) if recipient else None
        receipt = chain.multi_execute([send(cl8y, ops, gross, recipient)], key, expect_failure=True)
        assert "max wallet" in receipt.get("raw_log", "").lower(), receipt.get("raw_log", "")
        assert snapshot(trader) == before
        if recipient:
            assert chain.balance(guard, recipient) == recipient_before
        record("bought output capped " + ("payer" if recipient is None else "other recipient"),
               {"txhash": receipt["txhash"]})

    # Acquire exactly G=100 at the intermediate, whose next sell leaves one
    # purchased unit. It must not inherit the original-input cap exemption.
    first_hop = module.operations((cl8y, guard))
    gross = int(quote(first_hop, trader, reverse=True, amount=100)["amount"])
    assert int(quote(first_hop, trader, amount=gross)["amount"]) == 100
    before = snapshot(trader)
    receipt = chain.multi_execute([send(cl8y, module.operations((cl8y, guard, ust1)), gross)],
                                  key, expect_failure=True)
    assert "max wallet" in receipt.get("raw_log", "").lower(), receipt.get("raw_log", "")
    assert snapshot(trader) == before
    record("purchased intermediate residue capped and atomic rollback",
           {"purchased_gross": 100, "residue": 1, "txhash": receipt["txhash"]})

    for protocol_trader in (router, pairs[0], factory, guard):
        for kind in ("sell", "buy"):
            try:
                chain.smart(guard, {"route_tax_preview": {"kind": kind, "amount": "100",
                    "trader": protocol_trader, "pair": pairs[0]}})
            except RuntimeError as error:
                assert "trusted non-exempt trader" in str(error), str(error)
            else:
                raise AssertionError("Protocol-exempt trader preview succeeded")
        for path in ((guard, ust1), (ust1, guard)):
            for reverse in (False, True):
                try:
                    quote(module.operations(path), protocol_trader, reverse)
                except RuntimeError as error:
                    assert "trusted non-exempt trader" in str(error), str(error)
                else:
                    raise AssertionError("Protocol-exempt trader router quote succeeded")
    record("protocol-exempt buy/sell previews rejected")

    # An ordinary intermediate buy/sell shares one authenticated route check.
    settings(cooldown=100)
    intermediate_key, intermediate_trader = chain.fresh_trader()
    chain.execute(cl8y, {"transfer": {"recipient": intermediate_trader,
                                     "amount": "3000000000000000000"}})
    intermediate_ops = module.operations((cl8y, guard, ust1))
    expected = int(quote(intermediate_ops, intermediate_trader,
                         amount=1000000000000000000)["amount"])
    receipt = chain.multi_execute([send(cl8y, intermediate_ops, 1000000000000000000)],
                                  intermediate_key)
    assert chain.balance(ust1, intermediate_trader) == expected
    record("ordinary guarded intermediate buy/sell", {"txhash": receipt["txhash"]})

    # Two independent routes in one actual transaction cannot share acquired state.
    key, trader = chain.fresh_trader()
    chain.execute(guard, {"transfer": {"recipient": trader, "amount": "1000"}})
    ops = module.operations((guard, ust1))
    before = snapshot(trader)
    receipt = chain.multi_execute([send(guard, ops, 100), send(guard, ops, 100)],
                                  key, expect_failure=True)
    assert "cooldown" in receipt.get("raw_log", "").lower()
    assert snapshot(trader) == before
    record("independent same-transaction routes cooldown and atomic rollback",
           {"txhash": receipt["txhash"]})
    evidence["status"] = "PASS"
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
