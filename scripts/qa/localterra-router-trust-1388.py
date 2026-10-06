#!/usr/bin/env python3
"""Rehearse compiled-wasm trust rotation on an isolated #1381 LocalTerra fleet."""
import argparse
import base64
import importlib.util
import json
from datetime import datetime, timezone
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "funding", Path(__file__).with_name("localterra-route-tax-1381.py"))
funding = importlib.util.module_from_spec(spec)
spec.loader.exec_module(funding)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--evidence-file", type=Path, required=True)
    args = parser.parse_args()
    fixture = json.loads(args.fixture.read_text())
    assert fixture["chain_id"] == "localterra" and fixture["synthetic_only"]
    assert fixture["status"] == "PASS"
    chain = funding.LocalTerra(args.container)
    chain.default_key = fixture.get("owner_key", "test1")
    old = fixture["router"]
    factory = fixture["factory"]
    token = fixture["tokens"]["ALPHA"]
    code = fixture["codes"]["cl8y_dex_router"]
    candidate = chain.instantiate(code["code_id"], {"factory": factory}, "issue1388-router")
    bad_factory = chain.instantiate(code["code_id"], {"factory": old}, "issue1388-wrong-factory")
    owner_key = chain.default_key
    chain.default_key = fixture.get("sink_key", "test2")
    attacker_owned = chain.instantiate(code["code_id"], {"factory": factory}, "issue1388-attacker-admin")
    chain.default_key = owner_key
    # Simulate CMM custody without granting a generic execute dispatcher.
    admin_receipt = chain.tx("wasm", "set-contract-admin", token, old)
    checks = [{"name": "opaque contract admin recovery setup", "status": "PASS",
               "txhash": admin_receipt["txhash"]}]

    def rotate(target=candidate, checksum=code["sha256"], key=None, fail=False):
        payload = {"rotate_router": {"router": target, "expected_code_id": code["code_id"],
                                      "expected_checksum": checksum}}
        before = chain.smart(token, {"get_config": {}})
        receipt = chain.execute(token, payload, key=key, expect_failure=fail)
        if fail:
            assert before == chain.smart(token, {"get_config": {}})
        checks.append({"name": "rotation " + ("rejection" if fail else "success"),
                       "status": "PASS", "candidate": target, "txhash": receipt["txhash"]})

    rotate(key=fixture.get("sink_key", "test2"), fail=True)
    rotate(checksum="00" * 32, fail=True)
    rotate(target=bad_factory, fail=True)
    rotate(target=attacker_owned, fail=True)
    rotate()
    assert chain.smart(token, {"route_tax_capability": {}})["router"] == candidate
    ops = funding.operations([token, fixture["tokens"]["UST1"]])

    def swap(router, fail=False):
        key, trader = chain.fresh_trader()
        offer = 1010000
        chain.execute(token, {"transfer": {"recipient": trader, "amount": str(offer)}})
        before = chain.balance(token, trader)
        hook = {"execute_route_swap_operations": {"funding_version": 1, "operations": ops,
                "max_spread": "0.5", "minimum_receive": "1", "to": None, "deadline": None,
                "unwrap_output": False}}
        receipt = chain.execute(token, {"send": {"contract": router, "amount": str(offer),
                "msg": base64.b64encode(json.dumps(hook).encode()).decode()}},
                key=key, expect_failure=fail)
        if fail:
            assert chain.balance(token, trader) == before
        else:
            assert chain.balance(fixture["tokens"]["UST1"], trader) > 0
        checks.append({"name": "stale route rollback" if fail else "recovered funded route",
                       "status": "PASS", "txhash": receipt["txhash"]})

    swap(old, fail=True)
    swap(candidate)
    receipt = chain.execute(token, {"rotate_router": {
        "router": None, "expected_code_id": None, "expected_checksum": None}})
    assert chain.smart(token, {"route_tax_capability": {}})["router"] is None
    checks.append({"name": "revoke router", "status": "PASS", "txhash": receipt["txhash"]})
    swap(candidate, fail=True)
    rotate()
    swap(candidate)
    args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
    args.evidence_file.write_text(json.dumps({"utc": datetime.now(timezone.utc).isoformat(),
        "status": "PASS", "chain_id": "localterra", "synthetic_only": True,
        "fixture": str(args.fixture), "router": old, "replacement": candidate,
        "code": code, "checks": checks, "receipts": chain.receipts,
        "production_execution": False}, indent=2) + "\n")
    print(f"PASS LocalTerra #1388 rotation/revocation: {len(checks)} checks")


if __name__ == "__main__":
    main()
