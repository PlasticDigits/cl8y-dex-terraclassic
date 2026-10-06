#!/usr/bin/env python3
"""Standalone optimized Bridge-Debt fixtures; creates contracts only on localterra."""
import argparse
import base64
import importlib.util
import json
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("localterra_fixture", Path(__file__).with_name("localterra-route-tax-1381.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
LocalTerra = module.LocalTerra


def atomic_upgrade(chain, receipt, code_id, issuer, underlying):
    """Sign one synthetic admin transaction containing child migrate + accept."""
    owner = chain.address("test1")
    messages = [
        {"@type": "/cosmwasm.wasm.v1.MsgMigrateContract", "sender": owner,
         "contract": receipt, "code_id": str(code_id), "msg": {}},
        {"@type": "/cosmwasm.wasm.v1.MsgExecuteContract", "sender": owner,
         "contract": issuer, "msg": {"accept_market_upgrade": {"underlying": underlying}}, "funds": []},
    ]
    transaction = {"body": {"messages": messages, "memo": "", "timeout_height": "0",
                           "extension_options": [], "non_critical_extension_options": []},
                   "auth_info": {"signer_infos": [], "fee": {"amount": [
                       {"denom": "uluna", "amount": "500000000"}], "gas_limit": "12000000",
                       "payer": "", "granter": ""}}, "signatures": []}
    stem = "/tmp/issue1393-upgrade-" + str(time.time_ns())
    try:
        chain.command("sh", "-c", 'cat > "$1"', "sh", stem + ".json", stdin=json.dumps(transaction).encode())
        signed = chain.command("terrad", "tx", "sign", stem + ".json", "--from", "test1",
            "--keyring-backend", "test", "--chain-id", "localterra", "--node",
            "http://127.0.0.1:26657", "--output", "json")
        chain.command("sh", "-c", 'cat > "$1"', "sh", stem + ".signed.json", stdin=signed.encode())
        response = json.loads(chain.command("terrad", "tx", "broadcast", stem + ".signed.json",
            "--node", "http://127.0.0.1:26657", "--broadcast-mode", "sync", "--output", "json"))
        chain.wait(response)
    finally:
        chain.command("rm", "-f", stem + ".json", stem + ".signed.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--artifacts", type=Path, default=ROOT / "smartcontracts/artifacts")
    parser.add_argument("--evidence-file", type=Path, default=Path("/tmp/issue1393-localterra.json"))
    parser.add_argument("--code-cache", type=Path, default=Path("/tmp/issue1393-localterra-codes.json"))
    args = parser.parse_args()
    chain = LocalTerra(args.container, args.code_cache)
    owner = chain.address("test1")
    recipient_key, recipient = chain.fresh_trader()
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in
             ("bridge_debt", "bridge_debt_token", "cw20_mintable")}
    issuer = chain.instantiate(codes["bridge_debt"]["code_id"], {"owner": owner}, "issue1393-issuer")
    underlying_code = codes["cw20_mintable"]["code_id"]
    debt_code = codes["bridge_debt_token"]["code_id"]
    chain.execute(issuer, {"set_debt_code_id": {"code_id": debt_code}})
    chain.execute(issuer, {"approve_underlying_code": {"code_id": underlying_code, "multi_minter": True}})
    limits = dict(max_per_issue="1000", max_outstanding="2000", epoch_seconds=86400, max_per_epoch="2000")

    def market(name, receipt_code):
        token = chain.instantiate(underlying_code, {"name": "Synthetic " + name, "symbol": name,
            "decimals": 6, "initial_balances": [], "mint": {"minter": owner}}, "issue1393-" + name)
        chain.execute(token, {"add_minter": {"minter": issuer}})
        receipt = chain.instantiate(receipt_code, {"name": name + "-Bridge-Debt", "symbol": name + "-Bridge-Debt",
            "decimals": 6, "minter": issuer, "debtor": owner, "cap": None}, "issue1393-receipt-" + name)
        chain.execute(issuer, {"register_market": {"underlying": token, "debt_token": receipt}})
        return token, receipt

    def snapshot(token, receipt):
        return {"market": chain.smart(issuer, {"market": {"underlying": token}}),
                "underlying": chain.smart(token, {"token_info": {}}),
                "debt": chain.smart(receipt, {"token_info": {}}),
                "debtor": chain.balance(receipt, owner), "recipient": chain.balance(token, recipient),
                "issuer_underlying": chain.balance(token, issuer)}

    def issue(token, amount, **kwargs):
        chain.execute(issuer, {"issue": {"underlying": token, "recipient": recipient, "amount": str(amount)}}, **kwargs)

    def repay(token, amount):
        hook = base64.b64encode(b'{"repay":{}}').decode()
        chain.execute(token, {"send": {"contract": issuer, "amount": str(amount), "msg": hook}}, key=recipient_key)

    first, receipt = market("BDONE", debt_code)
    before = snapshot(first, receipt)
    issue(first, 1, expect_failure=True)
    assert snapshot(first, receipt) == before, "Disabled Issue changed balances/accounting"
    chain.execute(issuer, {"configure_market": {"underlying": first, "limits": limits, "paused": False}})
    issue(first, 1000)
    before = snapshot(first, receipt)
    issue(first, 1001, expect_failure=True)
    issue(first, 1, key=recipient_key, expect_failure=True)
    chain.execute(receipt, {"update_minter": {"new_minter": recipient}}, expect_failure=True)
    assert snapshot(first, receipt) == before, "Rejected mutation changed accounting"

    # Same approved wasm stored under another chain code ID exercises default rotation.
    code_file = args.artifacts / "bridge_debt_token.wasm"
    chain.stored_codes.pop(codes["bridge_debt_token"]["sha256"], None)
    next_code = chain.store(code_file)["code_id"]
    assert next_code != debt_code
    chain.execute(issuer, {"set_debt_code_id": {"code_id": next_code}})
    second, second_receipt = market("BDTWO", next_code)
    chain.execute(issuer, {"configure_market": {"underlying": second, "limits": limits, "paused": False}})
    issue(first, 500)
    issue(second, 500)
    assert chain.smart(issuer, {"market": {"underlying": first}})["debt_code_id"] == debt_code
    assert chain.smart(issuer, {"market": {"underlying": second}})["debt_code_id"] == next_code

    # Upgrade acceptance is atomic with the child migration by the shared admin.
    before = snapshot(first, receipt)
    chain.execute(issuer, {"propose_market_upgrade": {"underlying": first,
        "debt_code_id": next_code, "underlying_code_id": underlying_code}})
    atomic_upgrade(chain, receipt, next_code, issuer, first)
    after = snapshot(first, receipt)
    for field in ("underlying", "debt", "debtor", "recipient", "issuer_underlying"):
        assert after[field] == before[field], "Upgrade changed " + field
    assert after["market"]["outstanding"] == before["market"]["outstanding"]

    chain.execute(issuer, {"configure_market": {"underlying": first, "limits": limits, "paused": True}})
    repay(first, 1500)
    repay(second, 500)
    final = snapshot(first, receipt)
    second_final = snapshot(second, second_receipt)
    for result in (final, second_final):
        assert result["market"]["outstanding"] == "0"
        assert result["debt"]["total_supply"] == "0" and result["debtor"] == 0
        assert result["underlying"]["total_supply"] == "0" and result["recipient"] == 0
    assert final["market"]["epoch_issued"] == "1500", "Repay refunded epoch quota"
    toolchain = {tool: subprocess.check_output([tool, "--version"], text=True).strip()
                 for tool in ("rustc", "cargo", "wasm-opt")}
    evidence = {"status": "PASS", "chain": "localterra", "utc": datetime.now(timezone.utc).isoformat(),
                "toolchain": toolchain,
                "build": json.loads((args.artifacts / "issue1393-build.json").read_text())
                         if (args.artifacts / "issue1393-build.json").exists() else None,
                "artifacts": codes, "rotation_receipt_code_id": next_code,
                "issuer": issuer, "markets": [final, second_final],
                "checks": ["disabled issue rollback", "finite caps", "unauthorized issue rollback",
                           "receipt minter immutable", "default rotation and second market",
                           "approved code migration preserves supply", "paused repay and quota preservation"],
                "receipts": chain.receipts}
    args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
    args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
    print("PASS optimized LocalTerra Bridge-Debt smoke; evidence " + str(args.evidence_file))


if __name__ == "__main__":
    main()
