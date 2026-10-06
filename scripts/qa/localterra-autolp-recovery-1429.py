#!/usr/bin/env python3
"""Synthetic legacy AutoLP migration/recovery smoke on an owned LocalTerra.

Imports existing bounded CLI transaction helpers. Never starts, resets or removes
containers, touches production, or reads production signing credentials.
"""
import argparse
import base64
import importlib.util
import json
import time
from datetime import datetime, timezone
from pathlib import Path


spec = importlib.util.spec_from_file_location(
    "route_tax1381", Path(__file__).with_name("localterra-route-tax-1381.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--legacy-autolp", type=Path, required=True)
    parser.add_argument("--legacy-token", type=Path, required=True)
    parser.add_argument("--legacy-token-provenance", type=Path, required=True,
                        help="Reviewed build-provenance manifest containing legacy token artifact hash")
    parser.add_argument("--evidence-file", type=Path, required=True)
    parser.add_argument("--code-cache", type=Path,
                        help="Reuse only checksum-verified code IDs on the same owned chain")
    parser.add_argument("--inclusion-timeout", type=int, default=180)
    args = parser.parse_args()
    if not 0 < args.inclusion_timeout <= 300:
        parser.error("inclusion timeout must be between 1 and 300 seconds")
    class OwnedChain(module.LocalTerra):
        def wait(self, response, expect_failure=False):
            if response.get("code", 0):
                raise RuntimeError("LocalTerra broadcast rejected: " + response.get("raw_log", ""))
            deadline = time.monotonic() + args.inclusion_timeout
            while time.monotonic() < deadline:
                try:
                    receipt = self.query("tx", response["txhash"])
                except RuntimeError:
                    time.sleep(1)
                    continue
                if (int(receipt.get("code", 0)) != 0) != expect_failure:
                    raise RuntimeError("Unexpected LocalTerra result: " + receipt.get("raw_log", ""))
                self.receipts.append({key: receipt[key] for key in ("txhash", "height", "code", "gas_used")})
                return receipt
            raise TimeoutError("LocalTerra inclusion uncertain; reconcile without rebroadcast: " + response["txhash"])
    chain = OwnedChain(args.container, args.code_cache)
    owner = chain.address("test1")
    foreign_key, foreign = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in (
        "cw20_mintable", "cl8y_dex_factory", "cl8y_dex_pair",
        "cl8y_community_tax_token", "cl8y_community_tax_autolp")}
    codes["legacy_token"] = chain.store(args.legacy_token)
    codes["legacy_autolp"] = chain.store(args.legacy_autolp)
    assert codes["legacy_autolp"]["sha256"] == "a5e56f61fcca59232e37dca283f2e300731b999e60c6813d274fd8a5cee95934", "Expected historical AutoLP 11633 bytecode"
    provenance = json.loads(args.legacy_token_provenance.read_text())
    assert any(entry["path"].endswith("cl8y_community_tax_token.wasm") and
               entry["sha256"] == codes["legacy_token"]["sha256"]
               for entry in provenance["artifacts"]), "Legacy token provenance hash mismatch"
    for stored in codes.values():
        checksum = chain.query("wasm", "code-info", str(stored["code_id"]))["checksum"].lower()
        assert checksum == stored["sha256"], "Stored wasm checksum mismatch"
        stored["onchain_sha256"] = checksum
    def version(address):
        raw = chain.query("wasm", "contract-state", "raw", address, "contract_info", "--ascii")
        return json.loads(base64.b64decode(raw["data"]))
    quote = chain.instantiate(codes["cw20_mintable"]["code_id"], {
        "name": "Recovery Invoice", "symbol": "INV", "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000000000000"}],
        "mint": None}, "issue1429-quote")
    factory = chain.instantiate(codes["cl8y_dex_factory"]["code_id"], {
        "governance": owner, "treasury": owner, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"]["code_id"],
        "lp_token_code_id": codes["cw20_mintable"]["code_id"],
        "whitelisted_code_ids": [codes["legacy_token"]["code_id"],
                                 codes["cl8y_community_tax_token"]["code_id"],
                                 codes["cw20_mintable"]["code_id"]],
        "pair_creation_fee_uluna": "0"}, "issue1429-factory")
    token = chain.instantiate(codes["legacy_token"]["code_id"], {
        "name": "RecoveryLegacy", "symbol": "LEG", "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000000000000"}],
        "manager": owner, "treasury": owner, "buy_bps": 0, "sell_bps": 0,
        "max_buy_bps": 0, "max_sell_bps": 0, "max_transfer_bps": 0,
        "factory": factory, "router": None, "ust1": quote,
        "cmm_treasury": owner, "features": ["auto_v2_lp"], "mint": None,
        "transfer_bps": None, "sinks": None, "autolp": None,
        "launcher": owner, "launch_guards": None}, "issue1429-legacy-token")
    infos = [module.asset(token), module.asset(quote)]
    chain.execute(factory, {"create_pair": {"asset_infos": infos}})
    pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
    sister = chain.instantiate(codes["legacy_autolp"]["code_id"], {
        "token": token, "manager": token, "factory": factory,
        "router": None, "pair": pair, "quote_token": quote,
        "threshold": "1000", "lp_recipient": owner,
        "skim_max_spread": "0.01", "skim_min_return": None}, "issue1429-legacy-sister")
    legacy_versions = {"token": version(token), "autolp": version(sister)}
    assert legacy_versions["token"]["version"] == "1.3.0", legacy_versions
    assert legacy_versions["autolp"]["version"] == "1.0.0", legacy_versions
    chain.execute(token, {"bind_autolp": {"autolp": sister}})
    for asset in (token, quote):
        chain.execute(asset, {"increase_allowance": {"spender": pair, "amount": "1000000000000"}})
    chain.execute(pair, {"provide_liquidity": {"assets": [
        {"info": module.asset(token), "amount": "1000000000000"},
        {"info": module.asset(quote), "amount": "1000000000000"}],
        "slippage_tolerance": None}})
    checks = []
    evidence = {"schema": 1, "utc": datetime.now(timezone.utc).isoformat(),
        "chain_id": "localterra", "synthetic_only": True, "container": args.container,
        "codes": codes, "token": token, "sister": sister, "factory": factory,
        "pair": pair, "manager": owner, "checks": checks, "status": "PARTIAL"}
    evidence["legacy_versions"] = legacy_versions
    evidence["legacy_token_provenance"] = provenance["metadata"]
    evidence["legacy_autolp_provenance"] = {
        "kind": "historical production bytecode used only in synthetic LocalTerra",
        "endpoint": "https://lcd.terra-classic.hexxagon.io/cosmwasm/wasm/v1/code/11633",
        "code_id": 11633, "registry_sha256": "a5e56f61fcca59232e37dca283f2e300731b999e60c6813d274fd8a5cee95934",
        "receipt_sha256": codes["legacy_autolp"]["sha256"]}

    def record(name, extra=None):
        checks.append({"name": name, "status": "PASS", **(extra or {})})
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
        print("PASS LocalTerra " + name, flush=True)

    try:
        chain.smart(sister, {"get_lp_policy": {}})
    except RuntimeError as error:
        assert "unknown variant" in str(error), str(error)
        record("legacy policy query unsupported")
    else:
        raise AssertionError("Legacy fixture unexpectedly supports LP policy")
    chain.tx("wasm", "migrate", token, str(codes["cl8y_community_tax_token"]["code_id"]), "{}")
    chain.tx("wasm", "migrate", sister, str(codes["cl8y_community_tax_autolp"]["code_id"]), "{}")
    evidence["migrated_versions"] = {"token": version(token), "autolp": version(sister)}
    assert evidence["migrated_versions"]["token"]["version"] == "1.4.0"
    assert evidence["migrated_versions"]["autolp"]["version"] == "1.1.0"
    assert chain.smart(sister, {"get_lp_policy": {}}) is None
    config = chain.smart(sister, {"get_config": {}})
    assert config["manager"] == token
    record("migration preserves actual token manager and missing policy", {"config": config})
    refresh_receipt = chain.execute(factory, {"refresh_pair_asset_code_ids": {"pair": pair}})
    record("canonical factory refreshes pair asset pins after token migration",
           {"transaction": refresh_receipt["txhash"]})
    lp = chain.smart(pair, {"pair": {}})["liquidity_token"]

    def snapshot():
        return {"pool": chain.smart(pair, {"pool": {}}),
                "config": chain.smart(sister, {"get_config": {}}),
                "policy": chain.smart(sister, {"get_lp_policy": {}}),
                "pending_skim_raw": chain.query("wasm", "contract-state", "raw", sister,
                                                "pending_skim", "--ascii")["data"],
                "token_config": chain.smart(token, {"get_config": {}}),
                "total_supply": {asset: chain.smart(asset, {"token_info": {}})["total_supply"]
                                 for asset in (token, quote, lp)},
                "allowances": {asset: {wallet: chain.smart(asset, {"allowance": {
                    "owner": wallet, "spender": pair}}) for wallet in (owner, sister)}
                    for asset in (token, quote)},
                "balances": {asset: {wallet: chain.balance(asset, wallet)
                             for wallet in (owner, sister, pair, foreign)}
                             for asset in (token, quote, lp)}}

    def rejected(name, contract, message, expected_error, key=None):
        before = snapshot()
        receipt = chain.execute(contract, message, key=key, expect_failure=True)
        assert expected_error in receipt.get("raw_log", ""), receipt.get("raw_log")
        assert snapshot() == before, name + " changed economic/config state"
        record(name, {"transaction": receipt["txhash"], "expected_error": expected_error,
                      "raw_log": receipt["raw_log"], "snapshot_unchanged": True})

    settings = {"pair": pair, "threshold": "1000", "lp_recipient": owner,
                "skim_max_spread": "0.01", "skim_min_return": None}
    def invoice(settings):
        return {"send": {"contract": token, "amount": "50000000",
            "msg": base64.b64encode(json.dumps({"update_settings": {"settings": {
                "autolp": settings}}}).encode()).decode()}}

    rejected("unchanged invoice remains no-op and cannot initialize policy", quote, invoice(settings),
             "Settings batch is empty or identical to on-chain state")
    pool = chain.smart(pair, {"pool": {}})
    amounts = {asset["info"]["token"]["contract_addr"]: asset["amount"] for asset in pool["assets"]}
    policy = {"pair": pair, "quote": quote, "lp_token": lp,
              "tax_reserve": amounts[token], "quote_reserve": amounts[quote],
              "total_share": pool["total_share"]}
    refresh = {"refresh_autolp_policy": {"expected_policy": policy}}
    rejected("foreign proxy refresh rejected atomically", token, refresh, "Unauthorized", foreign_key)
    rejected("human cannot impersonate sister manager", sister, {"update_config": {"pair": pair}}, "Unauthorized")
    mismatch = dict(policy, tax_reserve=str(int(policy["tax_reserve"]) + 1))
    rejected("unapproved reserve snapshot rejected", token,
             {"refresh_autolp_policy": {"expected_policy": mismatch}},
             "AutoLP policy recovery requires the bound token-owned sister and exact nonzero live economics")
    chain.execute(token, refresh)
    assert chain.smart(sister, {"get_lp_policy": {}}) == policy
    assert chain.smart(sister, {"get_config": {}}) == config
    record("manager token-mediated refresh captures exact approved anchor", {"policy": policy})
    chain.execute(token, {"transfer": {"recipient": sister, "amount": "1000000"}})
    before_lp = chain.balance(lp, owner)
    chain.execute(sister, {"skim_to_lp": {}}, key=foreign_key)
    minted = chain.balance(lp, owner) - before_lp
    assert minted > 0
    assert chain.smart(sister, {"get_lp_policy": {}}) == policy
    assert chain.smart(sister, {"get_config": {}})["skimming"] is False
    record("permissionless skim mints positive LP without refreshing policy", {"minted_lp": str(minted)})
    hostile = dict(settings, skim_min_return="999999999999999999999999")
    chain.execute(quote, invoice(hostile))
    chain.execute(token, {"transfer": {"recipient": sister, "amount": "1000000"}})
    rejected("failed swap reply rolls back reserves balances lock and policy",
             sister, {"skim_to_lp": {}}, "Min return assertion", foreign_key)
    evidence["status"] = "PASS"
    evidence["receipts"] = chain.receipts
    evidence["completed_at_utc"] = datetime.now(timezone.utc).isoformat()
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
