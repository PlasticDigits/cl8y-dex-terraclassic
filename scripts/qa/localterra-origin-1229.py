#!/usr/bin/env python3
"""Owned #1229 smoke; prepare separately, run acceptance under timeout 59s.

Uses only a localterra chain and new random keys/contracts; never resets infra.
"""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
CONTAINER = "cl8y-dex-functional1400-localterra-1"
STATE = Path("/tmp/dex-origin-1229-localterra.json")
INCLUSION_SECONDS = 12
CW20_HASH = "C77FFCDB0039041FBD071C76EFB547232314CE8CCF219C3BF5B7FABD4CF7D4A4"


def command(*args, check=True):
    result = subprocess.run(["docker", "exec", CONTAINER, "terrad", *args],
                            capture_output=True, text=True, timeout=20)
    if check and result.returncode:
        raise RuntimeError("localterra command failed: " + result.stderr[-1200:])
    return result


def query(*args):
    return json.loads(command("query", *args, "--output", "json").stdout)


def attest():
    status = json.loads(command("status").stdout)
    assert status["node_info"]["network"] == "localterra"
    assert not status["sync_info"]["catching_up"]
    return status["sync_info"]["latest_block_height"]


def tx(key, *args, reject=False):
    result = command("tx", *args, "--from", key, "--keyring-backend", "test",
                     "--chain-id", "localterra", "--gas", "auto", "--gas-adjustment", "1.4",
                     "--gas-prices", "28.325uluna", "--broadcast-mode", "sync", "--yes", "--output", "json",
                     check=not reject)
    if result.returncode:
        assert reject and "Unauthorized" in result.stderr, result.stderr[-1200:]
        return {"rejected": True, "stage": "prebroadcast", "reason": "Unauthorized"}
    sent = json.loads(result.stdout)
    if int(sent.get("code", 0)):
        assert reject and "Unauthorized" in sent.get("raw_log", ""), sent
        return {"rejected": True, "stage": "check_tx", "reason": "Unauthorized"}
    deadline = time.monotonic() + INCLUSION_SECONDS
    while time.monotonic() < deadline:
        found = command("query", "tx", sent["txhash"], "--output", "json", check=False)
        if found.returncode == 0:
            included = json.loads(found.stdout)
            if reject:
                assert int(included["code"]) != 0 and "Unauthorized" in included["raw_log"], included
            else:
                assert int(included["code"]) == 0, included
            return included
        time.sleep(0.15)
    raise RuntimeError("localterra inclusion deadline expired")


def attributes(receipt):
    return [a for e in receipt.get("events", []) for a in e.get("attributes", [])]


def event_value(receipt, key):
    values = [a["value"] for a in attributes(receipt) if a["key"] == key]
    assert values, (key, receipt)
    return values[-1]


def smart(address, message):
    return query("wasm", "contract-state", "smart", address, json.dumps(message))["data"]


def init_token(state, launcher=None):
    return {"name": "OriginSmoke", "symbol": "ORIGIN", "decimals": 6,
            "initial_balances": [], "marketing": None, "manager": state["manager"],
            "treasury": state["manager"], "buy_bps": 0, "sell_bps": 0,
            "max_buy_bps": 0, "max_sell_bps": 0, "max_transfer_bps": 0,
            "factory": state["manager"], "router": None, "ust1": state["ust1"],
            "cmm_treasury": state["cmm"], "features": [], "mint": None,
            "transfer_bps": None, "sinks": None, "autolp": None, "launcher": launcher,
            "launch_guards": None, "initial_exempt": None}


def instantiate(state, code, message, label):
    receipt = tx(state["key"], "wasm", "instantiate", str(code), json.dumps(message),
                 "--label", state["run"] + label, "--admin", state["cmm"])
    return event_value(receipt, "_contract_address")


def prepare():
    assert not STATE.exists(), "use a new state path; preserve existing smoke receipts"
    attest()
    assert query("wasm", "code-info", "1")["checksum"] == CW20_HASH
    run = "origin1229-" + uuid.uuid4().hex[:12]
    addresses = []
    for suffix in ["manager", "cmm"]:
        key = run + suffix
        # Never print or save generated key material; only its public address.
        result = command("keys", "add", key, "--keyring-backend", "test", "--output", "json")
        addresses.append(json.loads(result.stdout)["address"])
    state = {"run": run, "key": run + "manager", "manager": addresses[0], "cmm": addresses[1],
             "container": CONTAINER, "chain_id": "localterra", "receipts": []}
    tx("test1", "bank", "send", "test1", state["manager"], "10000000000uluna")
    for role in ["token", "launcher"]:
        name = "cl8y_community_tax_token.wasm" if role == "token" else "cl8y_community_token_launcher.wasm"
        artifact = ROOT / "smartcontracts/artifacts" / name
        state[role + "_sha256"] = hashlib.sha256(artifact.read_bytes()).hexdigest()
        previous = [row for row in query("wasm", "list-code", "--limit", "200", "--reverse")["code_infos"]
                    if row["data_hash"].lower() == state[role + "_sha256"]]
        if previous:
            state[role + "_code"] = int(previous[-1]["code_id"])
        else:
            target = "/tmp/" + run + "-" + name
            subprocess.run(["docker", "cp", str(artifact), CONTAINER + ":" + target], check=True,
                           capture_output=True)
            receipt = tx(state["key"], "wasm", "store", target)
            state[role + "_code"] = int(event_value(receipt, "code_id"))
        assert query("wasm", "code-info", str(state[role + "_code"]))["checksum"].lower() == state[role + "_sha256"]
    state["ust1"] = instantiate(state, 1, {
        "name": "Owned UST1 Smoke", "symbol": "USTONE", "decimals": 6,
        "initial_balances": [{"address": state["manager"], "amount": "500000000"}],
        "mint": None, "marketing": None,
    }, "ust1")
    state["launcher"] = instantiate(state, state["launcher_code"], {
        "token_code_id": state["token_code"], "autolp_code_id": None,
        "ust1": state["ust1"], "cmm_treasury": state["cmm"],
        "cmm_governance": state["cmm"], "factory": state["manager"], "router": None,
    }, "launcher")
    STATE.write_text(json.dumps(state, indent=2) + "\n")
    STATE.chmod(0o600)
    print("Prepared owned LocalTerra contracts and matching code checksums:", STATE)


def run():
    began = time.monotonic()
    state = json.loads(STATE.read_text())
    start_height = attest()
    args = init_token(state)
    for field in ["marketing", "factory", "router", "ust1", "cmm_treasury", "autolp", "launcher"]:
        args.pop(field)
    args.update({"autolp_threshold": None, "autolp_lp_recipient": None})
    receipt = tx(state["key"], "wasm", "execute", state["launcher"], json.dumps({"create_token": args}))
    token = [a["value"] for a in attributes(receipt)
             if a["key"] == "community_token" and a["value"] != "pending"][-1]
    assert smart(token, {"get_launcher_origin": {}})["launcher"] == state["launcher"]
    assert query("wasm", "contract", token)["contract_info"]["admin"] == state["cmm"]
    before = int(smart(state["ust1"], {"balance": {"address": state["cmm"]}})["balance"])
    hook = base64.b64encode(json.dumps({"enable_feature": {"token": token, "sku": "transfer_tax"}}).encode()).decode()
    paid = tx(state["key"], "wasm", "execute", state["ust1"], json.dumps({"send": {
        "contract": state["launcher"], "amount": "50000000", "msg": hook}}))
    assert smart(token, {"get_features": {}})["transfer_tax"]
    after = int(smart(state["ust1"], {"balance": {"address": state["cmm"]}})["balance"])
    assert after - before == 50000000
    for address in [state["launcher"], token]:
        assert smart(state["ust1"], {"balance": {"address": address}})["balance"] == "0"
    rejected = tx(state["key"], "wasm", "instantiate", str(state["token_code"]),
                  json.dumps(init_token(state, state["launcher"])), "--label", state["run"] + "reject",
                  "--admin", state["cmm"], reject=True)
    direct = instantiate(state, state["token_code"], init_token(state), "direct")
    hook = base64.b64encode(json.dumps({"enable_feature": {"token": direct, "sku": "transfer_tax"}}).encode()).decode()
    uncreated = tx(state["key"], "wasm", "execute", state["ust1"], json.dumps({"send": {
        "contract": state["launcher"], "amount": "50000000", "msg": hook}}), reject=True)
    assert not smart(direct, {"get_features": {}})["transfer_tax"]
    assert smart(state["ust1"], {"balance": {"address": state["cmm"]}})["balance"] == str(after)
    evidence = {"status": "PASS", "utc": datetime.now(timezone.utc).isoformat(),
                "elapsed_seconds": time.monotonic() - began, "chain_id": "localterra",
                "start_height": start_height, "end_height": attest(),
                "token_code": state["token_code"], "launcher_code": state["launcher_code"],
                "token_sha256": state["token_sha256"], "launcher_sha256": state["launcher_sha256"],
                "owned_launcher": state["launcher"], "owned_token": token, "owned_direct_token": direct,
                "create_tx": receipt["txhash"], "enable_tx": paid["txhash"],
                "cmm_invoice_raw": after - before,
                "mismatched_origin": rejected, "uncreated_forward": uncreated,
                "scope": "new random keys and contracts only; no infra reset or existing contract changes"}
    output = STATE.with_name(STATE.stem + "-result.json")
    output.write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({"status": "PASS", "elapsed_seconds": evidence["elapsed_seconds"], "receipt": str(output)}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("prepare", "run"))
    parser.add_argument("--state", type=Path, default=STATE, help="Owned external fixture state; preparation refuses overwrite")
    args = parser.parse_args()
    STATE = args.state.resolve()
    if STATE.is_relative_to(ROOT):
        parser.error("owned fixture state must stay outside the repository")
    if args.mode == "prepare":
        INCLUSION_SECONDS = 60
        prepare()
    else:
        run()
