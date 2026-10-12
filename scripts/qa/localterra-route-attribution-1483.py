#!/usr/bin/env python3
"""Owned atomic native-wrap/pool settlement and exact economic rollback (#1483).

Reuses the existing #1381 signer/atomic-message fixture. Synthetic LocalTerra only;
12M fixture gas is separate from production offline budget/wallet calibration.
"""
import argparse
import base64
import hashlib
import importlib.util
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


FIXTURE = load("route1483_fixture", ROOT / "scripts/qa/localterra-route-tax-1381.py")
GAS = load("route1483_provenance", ROOT / "scripts/qa/localterra-offline-gas-1480.py")
SCALE = 10**18


def floor(raw, tolerance):
    return (raw * (SCALE - tolerance) + SCALE - 1) // SCALE


def decimal_text(atomics):
    return (str(atomics // SCALE) + "." + str(atomics % SCALE).zfill(18)).rstrip("0").rstrip(".")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--wrapper-artifacts", type=Path, required=True)
    parser.add_argument("--wrapper-source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    label = subprocess.run(["docker", "inspect", "--format", '{{index .Config.Labels "cl8y.qa.owner"}}',
                            args.container], capture_output=True, text=True, check=True).stdout.strip()
    assert label == "dex-1479-1480", "Owned isolated fixture family label required"
    artifacts = GAS.provenance(args.artifacts)
    source_inputs = {}
    for p in sorted(args.wrapper_source.rglob("*")):
        if p.is_file() and "target" not in p.parts and (p.suffix == ".rs" or p.name in ("Cargo.toml", "Cargo.lock")):
            source_inputs[str(p.relative_to(args.wrapper_source))] = hashlib.sha256(p.read_bytes()).hexdigest()
    assert source_inputs, "Wrapper source provenance unavailable"
    wrapper_revision = subprocess.run(["git", "-C", str(args.wrapper_source), "rev-parse", "HEAD"],
                                      capture_output=True, text=True, check=True).stdout.strip()
    utc_start = datetime.now(timezone.utc).isoformat()
    chain = GAS.BudgetChain(args.container)
    chain.pending_path = args.output.with_suffix(".pending.private.json")
    key, owner = chain.fresh_trader("400000000000uluna")
    chain.default_key = key
    _, recipient = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + ".wasm"))["code_id"]
             for name in ("cl8y_dex_factory", "cl8y_dex_router", "cl8y_dex_pair", "cw20_mintable")}
    wrapper_hashes = {}
    for name in ("treasury", "wrap_mapper"):
        path = args.wrapper_artifacts / (name + ".wasm")
        wrapper_hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        codes[name] = chain.store(path)["code_id"]
    treasury = chain.instantiate(codes["treasury"], {"governance": owner}, "native1483-treasury")
    mapper = chain.instantiate(codes["wrap_mapper"], {
        "governance": owner, "treasury": treasury, "fee_wrap_bps": 200, "fee_unwrap_bps": 50}, "native1483-mapper")
    tokens = [chain.instantiate(codes["cw20_mintable"], {
        "name": "Native Attribution Fixture " + symbol, "symbol": symbol, "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000000000000"}],
        "mint": {"minter": owner, "cap": None}, "marketing": None}, "native1483-" + symbol)
        for symbol in ("WRAP", "OUT")]
    chain.execute(tokens[0], {"add_minter": {"minter": mapper}})
    chain.execute(mapper, {"set_denom_mapping": {"denom": "uluna", "cw20_addr": tokens[0]}})
    chain.execute(treasury, {"set_denom_wrapper": {"denom": "uluna", "wrapper": mapper}})
    chain.execute(mapper, {"set_rate_limit": {"denom": "uluna", "config": {
        "max_amount_per_window": "1000000000000000", "window_seconds": 3600}}})
    for contract in (treasury, mapper):
        assert chain.smart(contract, {"execution_capabilities": {}}) == {"direct_wrapper_floor_version": 1}
    factory = chain.instantiate(codes["cl8y_dex_factory"], {
        "governance": owner, "treasury": recipient, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"], "lp_token_code_id": codes["cw20_mintable"],
        "whitelisted_code_ids": [codes["cw20_mintable"]], "pair_creation_fee_uluna": "0"}, "native1483-factory")
    router = chain.instantiate(codes["cl8y_dex_router"], {"factory": factory}, "native1483-router")
    infos = [FIXTURE.asset(token) for token in tokens]
    chain.execute(factory, {"create_pair": {"asset_infos": infos}})
    pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
    for token in tokens:
        chain.execute(token, {"increase_allowance": {"spender": pair, "amount": "100000000000"}})
    chain.execute(pair, {"provide_liquidity": {"assets": [
        {"info": info, "amount": "10000000000"} for info in infos], "slippage_tolerance": None}})

    def bank(address):
        return next((int(c["amount"]) for c in chain.query("bank", "balances", address)["balances"]
                     if c["denom"] == "uluna"), 0)

    def economics():
        return {"pool": chain.smart(pair, {"pool": {}}), "treasury_custody": bank(treasury),
                "wallet_tokens": [chain.balance(token, owner) for token in tokens],
                "wrapper_supply": chain.smart(tokens[0], {"token_info": {}})["total_supply"],
                "wrapper_rate": chain.smart(mapper, {"rate_limit": {"denom": "uluna"}})["amount_used"]}

    rows = []
    for native in (True, False):
        for gross in (1_000_000, 1_000_000_000):
            for tolerance in (10**16, 5 * 10**16, 3 * 10**17):
                credit = gross - gross * 200 // 10_000 if native else gross
                reserves = {x["info"]["token"]["contract_addr"]: int(x["amount"])
                            for x in chain.smart(pair, {"pool": {}})["assets"]}
                rin, rout = (reserves[token] for token in tokens)
                raw = credit * rout // (rin + credit)
                net = raw - raw * 30 // 10_000
                minimum = floor(net, tolerance)
                ideal = credit * rout // rin
                numerator = min(ideal - raw, raw)
                market = min(SCALE, ((numerator * SCALE + raw * tolerance) * SCALE
                                    + raw * (SCALE - tolerance) - 1) // (raw * (SCALE - tolerance)))

                def entries(hop_min, final_min):
                    op = {"terra_swap": {"offer_asset_info": infos[0], "ask_asset_info": infos[1],
                          "hybrid": None, "min_return": str(hop_min)}}
                    hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": [op],
                            "max_spread": decimal_text(market), "minimum_receive": str(final_min),
                            "to": owner, "deadline": None, "unwrap_output": False}}
                    result = [{"contract": tokens[0], "msg": {"send": {"contract": router,
                              "amount": str(credit), "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}}]
                    if native:
                        result.insert(0, {"contract": treasury, "msg": {"wrap_deposit_with_minimum": {
                            "minimum_receive": str(credit)}}, "coins": [{"denom": "uluna", "amount": str(gross)}]})
                    return result

                original = economics()
                rejects = []
                for hop_min, final_min in ((net + 1, minimum), (minimum, net + 1)):
                    failed = chain.multi_execute(entries(hop_min, final_min), key, expect_failure=True)
                    assert economics() == original, "Atomic floor rejection changed wrapper/pool/wallet economic state"
                    rejects.append(int(failed["code"]))
                before = chain.balance(tokens[1], owner)
                included = chain.multi_execute(entries(minimum, minimum), key)
                delivered = chain.balance(tokens[1], owner) - before
                assert delivered == net and delivered >= minimum
                assert int(included["code"]) == 0
                settled = economics()
                if native:
                    assert settled["treasury_custody"] - original["treasury_custody"] == gross
                    assert int(settled["wrapper_supply"]) - int(original["wrapper_supply"]) == credit
                    assert settled["wallet_tokens"][0] == original["wallet_tokens"][0]
                    assert int(settled["wrapper_rate"]) - int(original["wrapper_rate"]) == gross
                else:
                    assert original["wallet_tokens"][0] - settled["wallet_tokens"][0] == credit
                    for field in ("treasury_custody", "wrapper_supply", "wrapper_rate"):
                        assert settled[field] == original[field]
                rows.append({"native": native, "gross_input": str(gross), "credited_route_input": str(credit),
                             "tolerance_atomics": str(tolerance), "expected_net": str(net), "delivered_net": str(delivered),
                             "minimum_receive": str(minimum), "included_code": 0, "gas_used": int(included["gas_used"]),
                             "rejected_codes": rejects, "atomic_wrapper_pool_wallet_rollback": True})
                print(json.dumps({"native": native, "gross_input": str(gross), "status": "PASS"}), flush=True)
    result = {"issue": 1483, "status": "PASS", "utc_start": utc_start,
              "utc_end": datetime.now(timezone.utc).isoformat(), "driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "dex_artifacts": artifacts, "wrapper_source_revision": wrapper_revision,
              "wrapper_source_inputs_sha256": hashlib.sha256(json.dumps(source_inputs, sort_keys=True).encode()).hexdigest(),
              "wrapper_source_input_count": len(source_inputs), "wrapper_artifact_sha256": wrapper_hashes,
              "wrapper_build": "Rust 1.83.0 locked release libraries; no wasm-opt; owned synthetic fixture only",
              "scope": "Owned native-wrap plus one pool hop and wrapped one-hop route; exact independent net and atomic rollback",
              "synthetic_requested_gas": 12_000_000, "gas_is_not_production_calibration": True,
              "cases": rows, "case_count": len(rows), "economic_rollback_excludes_paid_ante_fees": True,
              "skips": ["native output composition", "tax/hook/populated-book", "physical wallets", "production settlement"]}
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as handle:
        handle.write(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
