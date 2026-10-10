#!/usr/bin/env python3
"""Verify isolated direct wrapper floors on LocalTerra only (#1459).

Uses fresh synthetic keys/contracts and public test funding. Never resets shared
chain state. Raw key-generation output stays private; receipts are public only.
"""
import argparse
import base64
import importlib.util
import hashlib
import json
import time
from pathlib import Path


def native_balance(chain, address):
    coins = chain.query("bank", "balances", address)["balances"]
    return next((int(coin["amount"]) for coin in coins if coin["denom"] == "uluna"), 0)


def snapshot(chain, treasury, mapper, token, owner):
    # Failed transactions still pay ante fees, so custody/supply/rate snapshots
    # intentionally omit the transaction payer's fee-denom balance.
    return {
        "treasury_native": native_balance(chain, treasury),
        "wallet_token": chain.balance(token, owner),
        "supply": int(chain.smart(token, {"token_info": {}})["total_supply"]),
        "rate": chain.smart(mapper, {"rate_limit": {"denom": "uluna"}})["amount_used"],
    }


def unwrap(chain, token, mapper, recipient, floor, fail=False):
    hook = {"unwrap_with_minimum": {"recipient": recipient, "minimum_receive": str(floor)}}
    message = {"send": {"contract": mapper, "amount": "500000",
                        "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}
    receipt = chain.execute(token, message, expect_failure=fail)
    if fail:
        assert "Minimum receive not met" in receipt.get("raw_log", ""), receipt
    return receipt


def wrap(chain, treasury, floor, fail=False):
    receipt = chain.tx("wasm", "execute", treasury,
                       json.dumps({"wrap_deposit_with_minimum": {"minimum_receive": str(floor)}}),
                       "--amount", "1000000uluna", expect_failure=fail)
    if fail:
        assert "Minimum receive not met" in receipt.get("raw_log", ""), receipt
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--cw20-artifact", type=Path)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--env-file", type=Path)
    parser.add_argument("--code-cache", type=Path)
    args = parser.parse_args()
    source = Path(__file__).with_name("localterra-route-tax-1381.py")
    spec = importlib.util.spec_from_file_location("localterra_fixture", source)
    fixture = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(fixture)
    chain = fixture.LocalTerra(args.container, args.code_cache)
    key, owner = chain.fresh_trader("30000000000uluna")
    chain.default_key = key
    provenance = {
        "driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "fixture_helper_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "container": args.container,
        "artifacts_path": str(args.artifacts),
        "cw20_artifact_path": str(args.cw20_artifact) if args.cw20_artifact else None,
        "scope": "Isolated synthetic LocalTerra; artifacts are not production release attestations",
    }
    codes = {}
    for name in ["treasury", "wrap_mapper", "cw20_mintable"]:
        artifact = args.cw20_artifact if name == "cw20_mintable" and args.cw20_artifact else args.artifacts / (name + ".wasm")
        codes[name] = chain.store(artifact)
    treasury = chain.instantiate(codes["treasury"]["code_id"], {"governance": owner}, "wrapper-floor-1459-treasury")
    mapper = chain.instantiate(codes["wrap_mapper"]["code_id"], {
        "governance": owner, "treasury": treasury, "fee_wrap_bps": 200, "fee_unwrap_bps": 50,
    }, "wrapper-floor-1459-mapper")
    token = chain.instantiate(codes["cw20_mintable"]["code_id"], {
        "name": "Wrapper Floor LUNC", "symbol": "WFLUNC", "decimals": 6,
        "initial_balances": [], "mint": {"minter": owner, "cap": None}, "marketing": None,
    }, "wrapper-floor-1459-token")
    chain.execute(token, {"add_minter": {"minter": mapper}})
    chain.execute(mapper, {"set_denom_mapping": {"denom": "uluna", "cw20_addr": token}})
    chain.execute(treasury, {"set_denom_wrapper": {"denom": "uluna", "wrapper": mapper}})
    chain.execute(mapper, {"set_rate_limit": {"denom": "uluna", "config": {
        "max_amount_per_window": "10000000", "window_seconds": 3600,
    }}})
    for contract in [treasury, mapper]:
        assert chain.smart(contract, {"execution_capabilities": {}}) == {"direct_wrapper_floor_version": 1}

    before = snapshot(chain, treasury, mapper, token, owner)
    wrap(chain, treasury, 980001, fail=True)
    assert snapshot(chain, treasury, mapper, token, owner) == before
    passed_wrap = wrap(chain, treasury, 980000)
    assert chain.balance(token, owner) == 980000
    before = snapshot(chain, treasury, mapper, token, owner)
    unwrap(chain, token, mapper, owner, 497501, fail=True)
    assert snapshot(chain, treasury, mapper, token, owner) == before

    # A separate recipient excludes payer ante fees from delivered bank delta.
    _, recipient = chain.fresh_trader("1000000uluna")
    recipient_before = native_balance(chain, recipient)
    passed_unwrap = unwrap(chain, token, mapper, recipient, 1)
    recipient_after = native_balance(chain, recipient)
    delivered = recipient_after - recipient_before
    assert 0 < delivered <= 497500
    wrap(chain, treasury, 980000)
    before = snapshot(chain, treasury, mapper, token, owner)
    unwrap(chain, token, mapper, recipient, delivered + 1, fail=True)
    assert snapshot(chain, treasury, mapper, token, owner) == before
    assert native_balance(chain, recipient) == recipient_after
    exact_unwrap = unwrap(chain, token, mapper, recipient, delivered)
    assert native_balance(chain, recipient) - recipient_after == delivered
    assert int(passed_wrap["gas_used"]) < 800000
    assert max(int(passed_unwrap["gas_used"]), int(exact_unwrap["gas_used"])) < 1200000

    receipt = {
        "time_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "chain": "localterra",
        "provenance": provenance,
        "codes": codes, "contracts": {"treasury": treasury, "mapper": mapper, "token": token},
        "wrap_gas_used": passed_wrap["gas_used"], "unwrap_gas_used": passed_unwrap["gas_used"],
        "exact_floor_unwrap_gas_used": exact_unwrap["gas_used"],
        "unwrap_nominal": 497500, "unwrap_delivered": delivered,
        "nonzero_bank_tax_observed": delivered < 497500,
        "rollback": {"wrap": True, "unwrap": True, "actual_net": True}, "receipts": chain.receipts,
    }
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + "\n")
    if args.env_file:
        args.env_file.write_text(f"VITE_TREASURY_ADDRESS={treasury}\nVITE_WRAP_MAPPER_ADDRESS={mapper}\nVITE_LUNC_C_TOKEN_ADDRESS={token}\n")
    print(json.dumps({key: value for key, value in receipt.items() if key not in ["receipts", "codes", "contracts"]}))


if __name__ == "__main__":
    main()
