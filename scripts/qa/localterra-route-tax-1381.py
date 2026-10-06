#!/usr/bin/env python3
"""Execute isolated route-funded tax fixtures on LocalTerra; never production.

Creates its own factory, router, assets and pools without resetting shared state.
Only public test receipts are emitted; key-generation output is discarded.
"""
import argparse
import base64
import hashlib
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path


class LocalTerra:
    def __init__(self, container, code_cache=None, tx_key="test1"):
        self.container = container
        self.tx_key = tx_key
        status = json.loads(self.command("curl", "-sf", "http://127.0.0.1:26657/status"))
        if status["result"]["node_info"]["network"] != "localterra":
            raise RuntimeError("Refusing every chain except localterra")
        self.receipts = []
        self.default_key = tx_key
        self.code_cache = code_cache
        self.stored_codes = json.loads(code_cache.read_text()) if code_cache and code_cache.exists() else {}

    def command(self, *parts, stdin=None):
        result = subprocess.run(["docker", "exec", "-i", self.container, *parts], input=stdin,
                                capture_output=True, timeout=90)
        if result.returncode:
            raise RuntimeError(result.stderr.decode()[:1500])
        return result.stdout.decode()

    def query(self, *parts):
        return json.loads(self.command("terrad", "query", *parts, "--node",
                                       "http://127.0.0.1:26657", "--output", "json"))

    def smart(self, address, message):
        return self.query("wasm", "contract-state", "smart", address, json.dumps(message))["data"]

    def address(self, key):
        return self.command("terrad", "keys", "show", key, "-a", "--keyring-backend", "test").strip()

    def tx(self, *parts, key=None, expect_failure=False):
        # Only an explicit CheckTx sequence rejection is safe to rebuild/retry.
        # Accepted or uncertain transactions always go to inclusion reconciliation.
        key = key or self.default_key
        for attempt in range(8):
            response = json.loads(self.command("terrad", "tx", *parts, "--from", key,
                "--keyring-backend", "test", "--chain-id", "localterra", "--gas", "12000000",
                "--fees", "500000000uluna", "--node", "http://127.0.0.1:26657",
                "--broadcast-mode", "sync", "-y", "--output", "json"))
            if (response.get("code", 0) and
                    "account sequence mismatch" in response.get("raw_log", "") and attempt < 7):
                time.sleep(0.25 * (attempt + 1))
                continue
            return self.wait(response, expect_failure)

    def wait(self, response, expect_failure=False):
        if response.get("code", 0):
            raise RuntimeError("LocalTerra broadcast rejected: " + response.get("raw_log", ""))
        for _ in range(45):
            try:
                receipt = self.query("tx", response["txhash"])
            except RuntimeError:
                time.sleep(1)
                continue
            failed = int(receipt.get("code", 0)) != 0
            if failed != expect_failure:
                raise RuntimeError("Unexpected LocalTerra result: " + receipt.get("raw_log", ""))
            self.receipts.append({"txhash": receipt["txhash"], "height": receipt["height"],
                "code": receipt.get("code", 0), "gas_used": receipt["gas_used"]})
            return receipt
        raise TimeoutError("LocalTerra inclusion timed out")

    def multi_execute(self, entries, key, expect_failure=False):
        """One actual Cosmos transaction, matching frontend atomic message ordering."""
        signer = self.address(key)
        messages = [{"@type": "/cosmwasm.wasm.v1.MsgExecuteContract", "sender": signer,
                     "contract": entry["contract"],
                     "msg": entry["msg"],
                     "funds": entry.get("coins", [])} for entry in entries]
        unsigned = {"body": {"messages": messages, "memo": "", "timeout_height": "0",
                             "extension_options": [], "non_critical_extension_options": []},
                    "auth_info": {"signer_infos": [], "fee": {"amount": [
                        {"denom": "uluna", "amount": "500000000"}],
                        "gas_limit": "12000000", "payer": "", "granter": ""}}, "signatures": []}
        stem = "/tmp/issue1381-tx-" + str(time.time_ns())
        try:
            self.command("sh", "-c", 'cat > "$1"', "sh", stem + ".json",
                         stdin=json.dumps(unsigned).encode())
            signed = self.command("terrad", "tx", "sign", stem + ".json", "--from", key,
                "--keyring-backend", "test", "--chain-id", "localterra", "--node",
                "http://127.0.0.1:26657", "--output", "json")
            self.command("sh", "-c", 'cat > "$1"', "sh", stem + ".signed.json", stdin=signed.encode())
            response = json.loads(self.command("terrad", "tx", "broadcast", stem + ".signed.json",
                "--node", "http://127.0.0.1:26657", "--broadcast-mode", "sync", "--output", "json"))
            return self.wait(response, expect_failure)
        finally:
            self.command("rm", "-f", stem + ".json", stem + ".signed.json")

    @staticmethod
    def event(receipt, event_type, key):
        events = receipt.get("events") or [e for log in receipt.get("logs", []) for e in log["events"]]
        return next(a["value"] for e in events if e["type"] == event_type
                    for a in e["attributes"] if a["key"] == key)

    def execute(self, address, message, **kwargs):
        return self.tx("wasm", "execute", address, json.dumps(message), **kwargs)

    def instantiate(self, code, message, label):
        receipt = self.tx("wasm", "instantiate", str(code), json.dumps(message),
                          "--label", label, "--admin", self.address(self.default_key))
        return self.event(receipt, "instantiate", "_contract_address")

    def store(self, artifact):
        binary = artifact.read_bytes()
        digest = hashlib.sha256(binary).hexdigest()
        cached = self.stored_codes.get(digest)
        if cached:
            info = self.query("wasm", "code-info", str(cached["code_id"]))
            assert info["checksum"].lower() == digest, "Stored code cache does not match artifact"
            print("REUSED LocalTerra " + artifact.name + " code " + str(cached["code_id"]), flush=True)
            return cached
        dest = "/tmp/issue1381-" + artifact.name
        self.command("sh", "-c", 'cat > "$1"', "sh", dest, stdin=binary)
        receipt = self.tx("wasm", "store", dest)
        stored = {"code_id": int(self.event(receipt, "store_code", "code_id")),
                  "sha256": digest}
        self.stored_codes[digest] = stored
        if self.code_cache:
            self.code_cache.parent.mkdir(parents=True, exist_ok=True)
            self.code_cache.write_text(json.dumps(self.stored_codes, indent=2) + "\n")
        print("STORED LocalTerra " + artifact.name + " code " + str(stored["code_id"]), flush=True)
        return stored

    def balance(self, token, address):
        return int(self.smart(token, {"balance": {"address": address}})["balance"])

    def fresh_trader(self, funding="10000000000uluna"):
        key = "issue1381-" + str(time.time_ns())
        self.command("terrad", "keys", "add", key, "--keyring-backend", "test", "--output", "json")
        address = self.address(key)
        self.tx("bank", "send", self.default_key, address, funding)
        return key, address


def asset(token):
    return {"token": {"contract_addr": token}}


def operations(tokens):
    return [{"terra_swap": {"offer_asset_info": asset(a), "ask_asset_info": asset(b),
                             "hybrid": None, "greedy": None, "min_return": None}}
            for a, b in zip(tokens, tokens[1:])]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--artifacts", type=Path,
                        default=Path(__file__).resolve().parents[2] / "smartcontracts/artifacts")
    parser.add_argument("--legacy-artifacts", type=Path,
                        help="Optional reviewed earlier router/token wasm for actual migration checks")
    parser.add_argument("--code-cache", type=Path, help="Reuse checksum-verified test-chain stores on retest")
    parser.add_argument("--evidence-file", required=True, type=Path)
    args = parser.parse_args()
    chain = LocalTerra(args.container, args.code_cache)
    # A dedicated synthetic owner prevents sequence collisions with other agent fleets.
    chain.default_key, owner = chain.fresh_trader("200000000000uluna")
    sink_key, sink = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in (
        "cl8y_dex_factory", "cl8y_dex_pair", "cl8y_dex_router", "cw20_mintable", "cl8y_community_tax_token")}
    legacy_codes = {}
    if args.legacy_artifacts:
        for name in ("cl8y_dex_router", "cl8y_community_tax_token"):
            legacy_codes[name] = chain.store(args.legacy_artifacts / (name + ".wasm"))
    code = lambda name: codes[name]["code_id"]
    factory = chain.instantiate(code("cl8y_dex_factory"), {
        "governance": owner, "treasury": sink, "default_fee_bps": 30,
        "pair_code_id": code("cl8y_dex_pair"), "lp_token_code_id": code("cw20_mintable"),
        "whitelisted_code_ids": [code("cw20_mintable"), code("cl8y_community_tax_token")]
            + ([legacy_codes["cl8y_community_tax_token"]["code_id"]] if legacy_codes else []),
        "pair_creation_fee_uluna": "0"}, "issue1381-factory")
    router = chain.instantiate(legacy_codes.get("cl8y_dex_router", codes["cl8y_dex_router"])["code_id"],
                               {"factory": factory}, "issue1381-router")
    migration_checks = []
    if legacy_codes:
        original_config = chain.smart(router, {"config": {}})
        assert original_config.get("funding_version") in (None, 0, 1)
        chain.tx("wasm", "migrate", router, str(code("cl8y_dex_router")), "{}")
        migrated_config = chain.smart(router, {"config": {}})
        assert migrated_config["funding_version"] == 1 and migrated_config["factory"] == factory
        assert migrated_config.get("settlement_version") == 1
        migration_checks.append({"contract": "router", "status": "PASS", "factory_preserved": True,
                                 "prior_funding_version": original_config.get("funding_version"),
                                 "settlement_version": migrated_config["settlement_version"]})
    tokens = {}
    decimals = {"CL8Y": 18, "ALPHA": 6, "BETA": 6, "UST1": 6, "USTR": 18}
    for name in ("CL8Y", "UST1", "USTR"):
        tokens[name] = chain.instantiate(code("cw20_mintable"), {
            "name": name + "synthetic", "symbol": name, "decimals": decimals[name],
            "initial_balances": [{"address": owner, "amount": str(10**(decimals[name] + 9))}],
            "mint": {"minter": owner}}, "issue1381-" + name)
    for name in ("ALPHA", "BETA"):
        tokens[name] = chain.instantiate(legacy_codes.get("cl8y_community_tax_token", codes["cl8y_community_tax_token"])["code_id"], {
            "name": name + "synthetic", "symbol": name, "decimals": 6,
            "initial_balances": [{"address": owner, "amount": "1000000000000000"}],
            "manager": owner, "treasury": sink, "buy_bps": 450, "sell_bps": 100,
            "max_buy_bps": 450, "max_sell_bps": 100, "max_transfer_bps": 0,
            "factory": factory, "router": router, "ust1": tokens["UST1"],
            "cmm_treasury": owner, "features": [], "mint": None, "transfer_bps": None,
            "sinks": None, "autolp": None, "launcher": None, "launch_guards": None},
            "issue1381-" + name)
    pairs = {}
    for a, b in (("CL8Y", "ALPHA"), ("ALPHA", "UST1"), ("UST1", "USTR"),
                 ("ALPHA", "BETA"), ("BETA", "USTR")):
        infos = [asset(tokens[a]), asset(tokens[b])]
        chain.execute(factory, {"create_pair": {"asset_infos": infos}})
        pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        pairs[a + "/" + b] = pair
        for name in (a, b):
            chain.execute(tokens[name], {"increase_allowance": {
                "spender": pair, "amount": str(10**(decimals[name] + 6))}})
        chain.execute(pair, {"provide_liquidity": {"assets": [
            {"info": asset(tokens[name]), "amount": str(10**(decimals[name] + 6))}
            for name in (a, b)], "slippage_tolerance": None}})
    if legacy_codes:
        before_pools = {pair: chain.smart(pair, {"pool": {}}) for pair in pairs.values()}
        for name in ("ALPHA", "BETA"):
            before_config = chain.smart(tokens[name], {"get_config": {}})
            before_balance = chain.balance(tokens[name], owner)
            chain.tx("wasm", "migrate", tokens[name], str(code("cl8y_community_tax_token")), "{}")
            assert chain.balance(tokens[name], owner) == before_balance
            assert chain.smart(tokens[name], {"get_config": {}}) == before_config
            migration_checks.append({"contract": name, "status": "PASS", "balances_config_preserved": True})
        for pair, before in before_pools.items():
            chain.execute(factory, {"refresh_pair_asset_code_ids": {"pair": pair}})
            assert chain.smart(pair, {"pool": {}}) == before
        migration_checks.append({"contract": "pair pins", "status": "PASS", "pool_state_preserved": True})
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "chain_id": "localterra",
                "synthetic_only": True, "owner_key": chain.default_key, "sink_key": sink_key, "codes": codes, "factory": factory, "router": router,
                "tokens": tokens, "decimals": decimals, "pairs": pairs, "checks": [], "legacy_codes": legacy_codes,
                "migration_checks": migration_checks, "status": "PARTIAL"}
    config = chain.smart(router, {"config": {}})
    assert config["funding_version"] == 1, "Router capability not deployed"
    for name in ("ALPHA", "BETA"):
        cap = chain.smart(tokens[name], {"route_tax_capability": {}})
        assert cap["funding_version"] == 1 and cap["router"] == router
    evidence["receipts"] = chain.receipts
    args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")

    def snapshot(trader):
        return {name: {"trader": chain.balance(token, trader), "router": chain.balance(token, router),
                      "sink": chain.balance(token, sink), "supply": chain.smart(token, {"token_info": {}})["total_supply"]}
                for name, token in tokens.items()}

    for label, names, dust, reserves, fail in (
        ("zero intermediates", ("CL8Y", "ALPHA", "UST1", "USTR"), False, False, False),
        ("multi tax intermediates", ("CL8Y", "ALPHA", "BETA", "USTR"), False, False, False),
        ("owned dust and existing balances", ("CL8Y", "ALPHA", "UST1", "USTR"), True, True, False),
        ("taxed initial gross", ("ALPHA", "UST1", "USTR"), False, False, False),
        ("taxed final buy", ("CL8Y", "ALPHA"), False, False, False),
        ("minimum return rollback", ("CL8Y", "ALPHA", "BETA", "USTR"), False, False, True)):
        key, trader = chain.fresh_trader()
        offer = 101 * 10**decimals[names[0]] // 100
        chain.execute(tokens[names[0]], {"transfer": {"recipient": trader, "amount": str(offer)}})
        for name in names[1:-1]:
            if reserves:
                chain.execute(tokens[name], {"transfer": {"recipient": trader, "amount": "7654321"}})
            if dust:
                chain.execute(tokens[name], {"transfer": {"recipient": router, "amount": "1234567"}})
        before = snapshot(trader)
        for name in names[1:]:
            if not reserves:
                assert before[name]["trader"] == 0
        ops = operations([tokens[name] for name in names])
        quote = chain.smart(router, {"simulate_swap_operations": {
            "offer_amount": str(offer), "operations": ops, "trader": trader, "sender": trader}})
        expected = int(quote["amount"])
        reverse = chain.smart(router, {"reverse_simulate_swap_operations": {
            "ask_amount": str(expected), "operations": ops, "trader": trader, "sender": trader}})
        forward_reverse = chain.smart(router, {"simulate_swap_operations": {
            "offer_amount": reverse["amount"], "operations": ops, "trader": trader, "sender": trader}})
        assert int(forward_reverse["amount"]) >= expected, "Gross reverse budget insufficient"
        hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": ops, "max_spread": "0.5",
            "minimum_receive": str(expected + 1 if fail else expected), "to": None,
            "deadline": None, "unwrap_output": False}}
        receipt = chain.execute(tokens[names[0]], {"send": {"contract": router, "amount": str(offer),
            "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}, key=key, expect_failure=fail)
        after = snapshot(trader)
        if fail:
            assert "Minimum receive assertion" in receipt.get("raw_log", ""), "Unexpected failure cause"
            assert before == after, "Tax distributions or token balances survived rollback"
        else:
            assert before[names[0]]["trader"] - after[names[0]]["trader"] <= offer
            assert after[names[-1]]["trader"] - before[names[-1]]["trader"] == expected
            for name in names[1:-1]:
                assert after[name]["trader"] >= before[name]["trader"], "Intermediate wallet used"
            for name in names:
                assert after[name]["router"] == before[name]["router"], "Router dust spent or unowned residue"
            for name in names:
                if name in ("ALPHA", "BETA"):
                    assert after[name]["sink"] > before[name]["sink"], "Configured tax not collected"
        evidence["checks"].append({"name": label, "status": "PASS", "quote": quote,
            "reverse": reverse, "before": before, "after": after, "txhash": receipt["txhash"],
            "gas_used": receipt["gas_used"]})
        print("PASS LocalTerra " + label, flush=True)
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
    evidence["status"] = "PASS"
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
