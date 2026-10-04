#!/usr/bin/env python3
"""Test-chain-only migration of final pair artifact, with unchanged pool-state checks."""
import argparse
import base64
import json
import hashlib
import subprocess
import time
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--factory", required=True)
    parser.add_argument("--code-id", type=int, help="resume using the already-stored final artifact")
    parser.add_argument("--evidence-file", type=Path, help="checkpoint public receipts after each pair")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]

    def command(*parts):
        return subprocess.check_output(parts, text=True, stderr=subprocess.PIPE)

    def query(*parts):
        return json.loads(command("docker", "exec", args.container, "terrad", "query", *parts,
                                  "--node", "http://127.0.0.1:26657", "--output", "json"))

    def smart(address, msg):
        return query("wasm", "contract-state", "smart", address, json.dumps(msg))["data"]

    # Refuse every chain except LocalTerra; this helper cannot migrate production.
    status = json.loads(command("docker", "exec", args.container, "curl", "-sf", "http://127.0.0.1:26657/status"))
    assert status["result"]["node_info"]["network"] == "localterra", "Refusing non-LocalTerra migration"
    account = command("docker", "exec", args.container, "terrad", "keys", "show", "test1",
                      "-a", "--keyring-backend", "test").strip()
    pairs, cursor = [], None
    for _ in range(10):
        page = smart(args.factory, {"pairs": {"limit": 30, "start_after": cursor}})["pairs"]
        pairs.extend(page)
        if len(page) < 30:
            break
        cursor = page[-1]["asset_infos"]
    else:
        raise RuntimeError("Inventory exceeds 300-pair test bound")
    before = {}
    for pair in pairs:
        address = pair["contract_addr"]
        assert query("wasm", "contract", address)["contract_info"]["admin"] == account, "Unexpected pair admin"
        before[address] = smart(address, {"pool": {}})

    def tx(*parts):
        response = json.loads(command("docker", "exec", args.container, "terrad", "tx", *parts,
                                      "--from", "test1", "--keyring-backend", "test", "--chain-id", "localterra",
                                      "--gas", "auto", "--gas-adjustment", "1.3", "--gas-prices", "28.325uluna",
                                      "--node", "http://127.0.0.1:26657", "--broadcast-mode", "sync", "-y", "--output", "json"))
        assert response.get("code", 0) == 0, "Test migration broadcast rejected"
        for _ in range(30):
            try:
                receipt = query("tx", response["txhash"])
                assert receipt.get("code", 0) == 0, "Test migration transaction failed"
                return receipt
            except subprocess.CalledProcessError:
                time.sleep(1)
        raise TimeoutError("Test migration transaction not included")

    artifact = repo / "smartcontracts/artifacts/cl8y_dex_pair.wasm"
    binary = artifact.read_bytes()
    name = binary.find(b"cl8y-dex-pair")
    assert name >= 0 and b"1.19.0" in binary[name:name + 48], "Wrong pair artifact version"
    if args.code_id:
        assert args.code_id > 0
        code = str(args.code_id)
        stored_info = query("wasm", "code-info", code)
        assert stored_info.get("checksum", "").lower() == hashlib.sha256(binary).hexdigest(), "Stored code differs from final artifact"
    else:
        command("docker", "cp", str(artifact), f"{args.container}:/tmp/issue1376-pair.wasm")
        stored = tx("wasm", "store", "/tmp/issue1376-pair.wasm")
        events = stored.get("events") or [event for log in stored.get("logs", []) for event in log["events"]]
        code = next(attr["value"] for event in events if event["type"] == "store_code"
                    for attr in event["attributes"] if attr["key"] == "code_id")
    tx("wasm", "execute", args.factory, json.dumps({"update_config": {"pair_code_id": int(code)}}))
    evidence = {"chain_id": "localterra", "pair_code_id": code, "version": "1.19.0", "pairs": []}
    def checkpoint():
        if args.evidence_file:
            args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")
    checkpoint()
    for pair in pairs:
        address = pair["contract_addr"]
        receipt = tx("wasm", "migrate", address, code, "{}")
        assert smart(address, {"pool": {}}) == before[address], "Migration changed pool state"
        raw = query("wasm", "contract-state", "raw", address, "contract_info", "--ascii")
        cw2 = json.loads(base64.b64decode(raw["data"]))
        assert cw2["version"] == "1.19.0", "Migration did not update cw2"
        info = query("wasm", "contract", address)["contract_info"]
        assert str(info["code_id"]) == str(code)
        evidence["pairs"].append({"pair": address, "code_id": code, "cw2": cw2, "txhash": receipt["txhash"], "pool_state_preserved": True})
        checkpoint()
    print(json.dumps(evidence, indent=2))


if __name__ == "__main__":
    main()
