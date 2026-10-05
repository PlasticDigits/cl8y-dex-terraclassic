#!/usr/bin/env python3
"""Actual atomic invoice/native-wrap flows using an isolated #1381 LocalTerra fixture."""
import argparse
import base64
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location("route_tax1381", Path(__file__).with_name("localterra-route-tax-1381.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def send(token, router, amount, ops, minimum, unwrap=False):
    hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": ops,
        "max_spread": "0.5", "minimum_receive": str(minimum), "to": None,
        "deadline": None, "unwrap_output": unwrap}}
    return {"contract": token, "msg": {"send": {"contract": router, "amount": str(amount),
        "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--stack-env", type=Path, help="Existing LocalTerra public wrap addresses; only selected keys read")
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--evidence-file", required=True, type=Path)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    assert fixture["chain_id"] == "localterra" and len(fixture["checks"]) >= 6
    chain = module.LocalTerra(args.container)
    chain.owner_key = fixture.get("owner_key", "test1")
    tokens, router, factory = fixture["tokens"], fixture["router"], fixture["factory"]
    assert chain.smart(router, {"config": {}})["funding_version"] == 1
    owner, sink = chain.address(chain.owner_key), chain.address("test2")
    evidence = {"chain_id": "localterra", "synthetic_only": True, "checks": [], "status": "PARTIAL"}

    def snapshot(trader):
        return {name: {"wallet": chain.balance(token, trader), "router": chain.balance(token, router),
                      "sink": chain.balance(token, sink), "supply": chain.smart(token, {"token_info": {}})["total_supply"]}
                for name, token in tokens.items()}

    def record(label, receipt, extra=None):
        evidence["checks"].append({"name": label, "status": "PASS", "txhash": receipt["txhash"],
                                  "gas_used": receipt["gas_used"], **(extra or {})})
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
        print("PASS LocalTerra " + label, flush=True)

    # A real 50-UST1 feature invoice paid atomically after a taxed route, with no
    # previously held ALPHA/UST1. A rejected invoice must roll the route back too.
    key, trader = chain.fresh_trader()
    payee = chain.instantiate(fixture["codes"]["cl8y_community_tax_token"]["code_id"], {
        "name": "InvoiceTest", "symbol": "INV", "decimals": 6, "initial_balances": [],
        "manager": trader, "treasury": sink, "buy_bps": 0, "sell_bps": 0,
        "max_buy_bps": 0, "max_sell_bps": 0, "max_transfer_bps": 0,
        "factory": factory, "router": router, "ust1": tokens["UST1"], "cmm_treasury": owner,
        "features": [], "mint": None, "transfer_bps": None, "sinks": None,
        "autolp": None, "launcher": None, "launch_guards": None}, "issue1381-invoice")
    ops = module.operations([tokens[n] for n in ("CL8Y", "ALPHA", "UST1")])
    target = 50_000_000
    reverse = chain.smart(router, {"reverse_simulate_swap_operations": {
        "ask_amount": str(target), "operations": ops, "trader": trader, "sender": trader}})
    gross = (int(reverse["amount"]) * 10050 + 9999) // 10000
    chain.execute(tokens["CL8Y"], {"transfer": {"recipient": trader, "amount": str(gross)}})
    assert chain.balance(tokens["ALPHA"], trader) == chain.balance(tokens["UST1"], trader) == 0
    quote = chain.smart(router, {"simulate_swap_operations": {
        "offer_amount": str(gross), "operations": ops, "trader": trader, "sender": trader}})
    assert int(quote["amount"]) >= target
    feature_hook = base64.b64encode(json.dumps({"enable_feature": {"sku": "split_router"}}).encode()).decode()
    for fail in (True, False):
        before = snapshot(trader)
        owner_before = chain.balance(tokens["UST1"], owner)
        invoice = {"contract": tokens["UST1"], "msg": {"send": {"contract": payee,
                   "amount": str(target - 1 if fail else target), "msg": feature_hook}}}
        receipt = chain.multi_execute([send(tokens["CL8Y"], router, gross, ops, target), invoice],
                                      key, expect_failure=fail)
        after = snapshot(trader)
        if fail:
            assert "Invoice amount must be exactly" in receipt.get("raw_log", "")
            assert before == after and chain.balance(tokens["UST1"], owner) == owner_before
        else:
            assert before["CL8Y"]["wallet"] - after["CL8Y"]["wallet"] == gross
            assert after["UST1"]["wallet"] == int(quote["amount"]) - target
            assert after["ALPHA"]["wallet"] >= before["ALPHA"]["wallet"]
            assert chain.balance(tokens["UST1"], owner) - owner_before == target
            assert chain.smart(payee, {"get_features": {}})["split_router"]
            assert all(after[n]["router"] == before[n]["router"] for n in tokens)
        record("invoice atomic rollback" if fail else "taxed route plus exact feature invoice", receipt,
               {"authorized_gross": str(gross), "invoice_raw": str(target), "quote": quote})

    if args.stack_env:
        keys = {"VITE_TREASURY_ADDRESS", "VITE_WRAP_MAPPER_ADDRESS", "VITE_LUNC_C_TOKEN_ADDRESS"}
        public = {}
        for line in args.stack_env.read_text().splitlines():
            name, sep, value = line.partition("=")
            if sep and name in keys:
                public[name] = value.strip().strip('"\'')
        treasury, mapper, wrapped = (public[k] for k in (
            "VITE_TREASURY_ADDRESS", "VITE_WRAP_MAPPER_ADDRESS", "VITE_LUNC_C_TOKEN_ADDRESS"))
        assert all(value.startswith("terra1") for value in (treasury, mapper, wrapped))
        info = chain.query("wasm", "contract", wrapped)["contract_info"]
        checksum = chain.query("wasm", "code-info", info["code_id"])["checksum"].lower()
        assert checksum == fixture["codes"]["cw20_mintable"]["sha256"], "Unreviewed ordinary wrapped token code"
        chain.execute(factory, {"add_whitelisted_code_id": {"code_id": int(info["code_id"])}})
        chain.execute(router, {"set_wrap_mapper": {"wrap_mapper": mapper}})
        config = chain.smart(mapper, {"config": {}})
        assert config["treasury"] == treasury
        fee = int(config["fee_wrap_bps"])
        assert 0 <= fee < 10_000
        wrapped_decimals = chain.smart(wrapped, {"token_info": {}})["decimals"]
        assert wrapped_decimals == 6
        liquidity = 1_000_000_000
        infos = [module.asset(wrapped), module.asset(tokens["ALPHA"])]
        try:
            pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        except RuntimeError:
            wrap_gross = (liquidity - 1) * 10000 // (10000 - fee) + 1
            chain.tx("wasm", "execute", treasury, '{"wrap_deposit":{}}', "--amount", str(wrap_gross) + "uluna")
            chain.execute(factory, {"create_pair": {"asset_infos": infos}})
            pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
            for token in (wrapped, tokens["ALPHA"]):
                chain.execute(token, {"increase_allowance": {"spender": pair, "amount": str(liquidity)}})
            chain.execute(pair, {"provide_liquidity": {"assets": [
                {"info": info, "amount": str(liquidity)} for info in infos], "slippage_tolerance": None}})
        tokens["CLUNC"] = wrapped
        key, trader = chain.fresh_trader()
        native_gross = 1_010_000
        funded = native_gross - native_gross * fee // 10000
        ops = module.operations([tokens[n] for n in ("CLUNC", "ALPHA", "UST1", "USTR")])
        quote = chain.smart(router, {"simulate_swap_operations": {
            "offer_amount": str(funded), "operations": ops, "trader": trader, "sender": trader}})
        assert all(chain.balance(tokens[n], trader) == 0 for n in ("CLUNC", "ALPHA", "UST1", "USTR"))
        for fail in (True, False):
            before = snapshot(trader)
            receipt = chain.multi_execute([
                {"contract": treasury, "msg": {"wrap_deposit": {}},
                 "coins": [{"denom": "uluna", "amount": str(native_gross)}]},
                send(wrapped, router, funded, ops, int(quote["amount"]) + (1 if fail else 0))],
                key, expect_failure=fail)
            after = snapshot(trader)
            if fail:
                assert "Minimum receive assertion" in receipt.get("raw_log", "")
                assert before == after, "Wrap mint or route tax survived atomic failure"
            else:
                assert after["USTR"]["wallet"] - before["USTR"]["wallet"] == int(quote["amount"])
                for n in ("CLUNC", "ALPHA", "UST1"):
                    assert after[n]["wallet"] >= before[n]["wallet"]
                assert all(after[n]["router"] == before[n]["router"] for n in tokens)
            record("native wrap atomic rollback" if fail else "native wrap plus taxed intermediate", receipt,
                   {"authorized_native_gross": str(native_gross), "route_gross": str(funded), "quote": quote})
        # The reverse native path settles only the route's received wrapped output,
        # then burns through the existing mapper; intermediate wallets remain unused.
        key, trader = chain.fresh_trader()
        offer = 1_010_000
        chain.execute(tokens["UST1"], {"transfer": {"recipient": trader, "amount": str(offer)}})
        ops = module.operations([tokens[n] for n in ("UST1", "ALPHA", "CLUNC")])
        quote = chain.smart(router, {"simulate_swap_operations": {
            "offer_amount": str(offer), "operations": ops, "trader": trader, "sender": trader}})
        before = snapshot(trader)
        native_before = int(next(c["amount"] for c in chain.query("bank", "balances", trader)["balances"]
                                 if c["denom"] == "uluna"))
        # Router unwrap minimum is mapper-net, before Classic native burn tax;
        # its arithmetic route quote is the net CW20 output before mapper fees.
        unwrap_fee = int(config["fee_unwrap_bps"])
        mapper_net = int(quote["amount"]) - int(quote["amount"]) * unwrap_fee // 10000
        receipt = chain.multi_execute([send(tokens["UST1"], router, offer, ops, mapper_net, True)], key)
        after = snapshot(trader)
        native_after = int(next(c["amount"] for c in chain.query("bank", "balances", trader)["balances"]
                                if c["denom"] == "uluna"))
        native_credit = native_after - native_before + 500_000_000
        assert 0 < native_credit <= int(quote["amount"])
        assert before["UST1"]["wallet"] - after["UST1"]["wallet"] == offer
        assert after["ALPHA"]["wallet"] >= before["ALPHA"]["wallet"]
        assert after["CLUNC"]["wallet"] == before["CLUNC"]["wallet"] == 0
        assert int(before["CLUNC"]["supply"]) - int(after["CLUNC"]["supply"]) == int(quote["amount"])
        assert all(after[n]["router"] == before[n]["router"] for n in tokens)
        record("taxed intermediate plus native unwrap", receipt,
               {"authorized_gross": str(offer), "wrapped_burn": quote["amount"],
                "mapper_minimum_receive": str(mapper_net), "native_credit": str(native_credit)})
    evidence["status"] = "PASS"
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
