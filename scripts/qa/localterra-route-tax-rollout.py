#!/usr/bin/env python3
"""Rehearse the production rollout transport on isolated LocalTerra contracts.

The actual terrad generate/sign/multisign/encode/broadcast path is used with new
synthetic 2-of-3 keys. Existing application contracts and workstation keys are
never used. Legacy code IDs require explicit checksum assertions.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from route_tax_rollout import Artifact, Rollout, SPECS, require, write_json
from route_tax_rollout_cli import TerradChain, parser as rollout_parser

spec = importlib.util.spec_from_file_location("localterra_fixture", Path(__file__).with_name("localterra-route-tax-1381.py"))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class DockerChain(TerradChain):
    """Transport adapter only: all signing and safety checks remain production code."""
    def __init__(self, args, local):
        super().__init__(args)
        self.local = local
        self.broadcasts = 0
        self.engine = None
        self.journal_checks = 0

    def get(self, path, rpc=False):
        port = 26657 if rpc else 1317
        result = subprocess.run(["docker", "exec", self.local.container, "curl", "-s", "-w", "\n%{http_code}",
                                 f"http://127.0.0.1:{port}{path}"], capture_output=True, text=True, timeout=20)
        require(result.returncode == 0, "LocalTerra read failed")
        body, status = result.stdout.rsplit("\n", 1)
        require(status == "200", f"Read-only endpoint returned HTTP {status}")
        return json.loads(body)

    def command(self, parts, timeout=180):
        mapped = []
        for part in parts:
            path = Path(part)
            if part.startswith("/") and path.is_file():
                target = "/tmp/rollout1381-" + hashlib.sha256(str(path).encode()).hexdigest()[:16] + path.suffix
                self.local.command("sh", "-c", 'cat > "$1"', "sh", target, stdin=path.read_bytes())
                mapped.append(target)
            else:
                mapped.append(part)
        if parts[:2] == ["tx", "broadcast"]:
            self.broadcasts += 1
            if self.engine:
                saved = json.loads(self.engine.journal_path.read_text())
                signed = str(parts[2])
                require(any(op.get("signed_path") == signed and op.get("tx_hash") and not op.get("receipt")
                            for op in saved["operations"].values()), "Broadcast preceded durable pending journal")
                self.journal_checks += 1
        result = subprocess.run(["docker", "exec", "-i", self.local.container, "terrad", *mapped],
                                capture_output=True, text=True, timeout=timeout)
        require(result.returncode == 0, result.stderr[:1500])
        return result.stdout


def instantiate(local, code, message, label, admin):
    receipt = local.tx("wasm", "instantiate", str(code), json.dumps(message), "--label", label, "--admin", admin)
    return local.event(receipt, "instantiate", "_contract_address")


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    p.add_argument("--artifacts", type=Path, required=True)
    p.add_argument("--state-dir", type=Path, required=True)
    p.add_argument("--ordinary-wasm", type=Path, help="Reviewed cw20 fixture wasm when external artifacts are not in optimizer output")
    p.add_argument("--ordinary-sha256")
    p.add_argument("--legacy-factory-code", type=int, required=True)
    p.add_argument("--legacy-factory-sha256", required=True)
    p.add_argument("--legacy-pair-code", type=int, required=True)
    p.add_argument("--legacy-pair-sha256", required=True)
    p.add_argument("--legacy-router-code", type=int, required=True)
    p.add_argument("--legacy-router-sha256", required=True)
    p.add_argument("--legacy-token-code", type=int, required=True)
    p.add_argument("--legacy-token-sha256", required=True)
    p.add_argument("--cmm-wasm", type=Path, help="Optional reviewed treasury 0.2.2 for isolated owned-contract migration")
    p.add_argument("--cmm-sha256")
    p.add_argument("--legacy-cmm-wasm", type=Path)
    p.add_argument("--legacy-cmm-sha256")
    p.add_argument("--evidence-file", type=Path, required=True)
    args = p.parse_args()
    cmm_inputs = (args.cmm_wasm, args.cmm_sha256, args.legacy_cmm_wasm, args.legacy_cmm_sha256)
    require(all(cmm_inputs) or not any(cmm_inputs), "Supply all four reviewed CMM artifact arguments together")
    require(bool(args.ordinary_wasm) == bool(args.ordinary_sha256), "Supply reviewed ordinary wasm and checksum together")
    require(not args.state_dir.exists(), "Use a fresh state directory for each isolated rehearsal")
    args.state_dir.mkdir(parents=True, mode=0o700)
    local = fixture.LocalTerra(args.container)
    owner, sink = local.address("test1"), local.address("test2")
    stamp = str(time.time_ns())
    members = ["rollout1381-" + stamp + "-" + str(i) for i in (1, 2, 3)]
    gov_key = "rollout1381-" + stamp + "-gov"
    for key in members:
        local.command("terrad", "keys", "add", key, "--no-backup", "--keyring-backend", "test", "--output", "json")
    local.command("terrad", "keys", "add", gov_key, "--multisig", ",".join([members[0], members[2], members[1]]),
                  "--multisig-threshold", "2", "--nosort", "--keyring-backend", "test", "--output", "json")
    gov = local.address(gov_key)
    local.tx("bank", "send", "test1", gov, "100000000000uluna")
    transport_args = rollout_parser().parse_args(["--state-dir", str(args.state_dir), "--chain-id", "localterra",
        "--node", "http://127.0.0.1:26657", "--lcd", "http://127.0.0.1:1317", "--home", "/home/terra/.terra",
        "--keyring-backend", "test", "--store-key", "test1", "--store-address", owner,
        "--multisig-key", gov_key, "--governance", gov, "--signer1", members[0], "--signer2", members[1]])
    chain = DockerChain(transport_args, local)
    require(chain.network() == "localterra", "Synthetic keys can only transact on LocalTerra")
    chain.unlock()
    record = json.loads(chain.command(["keys", "show", gov_key, "--keyring-backend", "test", "--output", "json"]))
    pubkey = json.loads(record["pubkey"]) if isinstance(record["pubkey"], str) else record["pubkey"]
    expected = [json.loads(chain.command(["keys", "show", members[i], "--keyring-backend", "test", "--output", "json"])) for i in (0, 2, 1)]
    require(pubkey["public_keys"] == [json.loads(r["pubkey"]) if isinstance(r["pubkey"], str) else r["pubkey"] for r in expected],
            "SDK did not preserve synthetic 1/3/2 multisig ordering")
    checksums = {line.split()[1].lstrip("*"): line.split()[0] for line in (args.artifacts / "checksums.txt").read_text().splitlines() if len(line.split()) == 2}
    artifacts = {role: Artifact.load(role, args.artifacts / SPECS[role][2], checksums[SPECS[role][2]])
                 for role in ("factory", "pair", "router", "token", "launcher")}
    legacy = {}
    for role in ("factory", "pair", "router", "token"):
        code = getattr(args, "legacy_" + role + "_code")
        require(chain.code_hash(code) == getattr(args, "legacy_" + role + "_sha256").lower(), "Unreviewed legacy " + role)
        legacy[role] = code
    ordinary_path = args.ordinary_wasm or args.artifacts / "cw20_mintable.wasm"
    ordinary_hash = args.ordinary_sha256 or checksums.get("cw20_mintable.wasm")
    require(ordinary_hash and hashlib.sha256(ordinary_path.read_bytes()).hexdigest() == ordinary_hash,
            "Unreviewed ordinary fixture wasm checksum")
    ordinary = local.store(ordinary_path)["code_id"]
    launcher_code = local.store(args.artifacts / SPECS["launcher"][2])["code_id"]
    cmm = owner
    if args.cmm_wasm:
        artifacts["cmm"] = Artifact.load("cmm", args.cmm_wasm, args.cmm_sha256)
        Artifact.load("cmm", args.legacy_cmm_wasm, args.legacy_cmm_sha256, version="0.2.1")
        cmm_code = local.store(args.legacy_cmm_wasm)["code_id"]
        cmm = instantiate(local, cmm_code, {"governance": gov}, "rollout1381-cmm", gov)
    factory = instantiate(local, legacy["factory"], {"governance": gov, "treasury": sink, "default_fee_bps": 30,
        "pair_code_id": legacy["pair"], "lp_token_code_id": ordinary, "whitelisted_code_ids": [ordinary, legacy["token"]],
        "pair_creation_fee_uluna": "0"}, "rollout1381-factory", gov)
    router = instantiate(local, legacy["router"], {"factory": factory}, "rollout1381-router", gov)
    ust1 = instantiate(local, ordinary, {"name": "RolloutSyntheticUST1", "symbol": "RUS", "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000000000000"}], "mint": {"minter": owner}}, "rollout1381-ust1", gov)
    launcher = instantiate(local, launcher_code, {"token_code_id": legacy["token"], "autolp_code_id": None,
        "ust1": ust1, "cmm_treasury": cmm, "cmm_governance": gov, "factory": factory, "router": router}, "rollout1381-launcher", gov)
    tokens = []
    token_specs = [("RTX", 450, 100, gov), ("RZERO", 0, 0, gov)]
    if args.cmm_wasm:
        token_specs.append(("RCMM", 100, 100, cmm))
    for name, buy, sell, token_admin in token_specs:
        tokens.append(instantiate(local, legacy["token"], {"name": "Rollout" + name, "symbol": name, "decimals": 6,
            "initial_balances": [{"address": owner, "amount": "1000000000000000"}], "manager": owner, "treasury": sink,
            "buy_bps": buy, "sell_bps": sell, "max_buy_bps": buy, "max_sell_bps": sell, "max_transfer_bps": 0,
            "factory": factory, "router": router, "ust1": ust1, "cmm_treasury": cmm, "features": [],
            "mint": None, "transfer_bps": None, "sinks": None, "autolp": None, "launcher": launcher, "launch_guards": None},
            "rollout1381-" + name, token_admin))
    infos = [fixture.asset(tokens[0]), fixture.asset(ust1)]
    local.execute(factory, {"create_pair": {"asset_infos": infos}})
    pair = local.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
    for token in (tokens[0], ust1):
        local.execute(token, {"increase_allowance": {"spender": pair, "amount": "1000000000"}})
    local.execute(pair, {"provide_liquidity": {"assets": [{"info": info, "amount": "1000000000"} for info in infos], "slippage_tolerance": None}})
    config = {"chain_id": "localterra", "factory": factory, "router": router, "launcher": launcher,
              "governance": gov, "alpha": tokens[0], "cmm": cmm, "tax_free_code_ids": [ordinary]}
    engine = Rollout(chain, artifacts, config, args.state_dir / "journal.json")
    chain.engine = engine
    planned = engine.prepare()
    require({t["address"] for t in planned["inventory"]["tokens"]} == set(tokens), "Zero-rate/unpaired legacy instance omitted")
    engine.execute()
    require(engine.verify()["contract_verification"] == "PASS", "Rollout verification failed")
    count = chain.broadcasts
    engine.execute()
    require(chain.broadcasts == count, "Repeated execution rebroadcast completed operations")
    engine.finalize()
    count = chain.broadcasts
    engine.finalize()
    require(chain.broadcasts == count, "Repeated finalize rebroadcast completed operations")
    engine.verify()
    require(chain.journal_checks == chain.broadcasts, "Transport did not checkpoint all transactions")
    require(local.smart(pair, {"is_paused": {}})["paused"] is False, "Finalization failed to restore trading")
    creation = local.execute(launcher, {"create_token": {"name": "RolloutNewTemplate", "symbol": "RNEW", "decimals": 6,
        "initial_balances": [{"address": owner, "amount": "1000000"}], "manager": owner, "treasury": sink,
        "buy_bps": 0, "sell_bps": 0, "max_buy_bps": 0, "max_sell_bps": 0, "max_transfer_bps": 0,
        "features": [], "mint": None, "transfer_bps": None, "sinks": None, "launch_guards": None,
        "autolp_threshold": None, "autolp_lp_recipient": None}})
    created_token = local.event(creation, "instantiate", "_contract_address")
    require(int(chain.info(created_token)["code_id"]) == engine.state["codes"]["token"], "New launch used stale token template")
    require(chain.smart(created_token, {"route_tax_capability": {}})["funding_version"] == 1, "New token lacks route-tax protocol")
    evidence = {"status": "PASS", "chain_id": "localterra", "synthetic_only": True, "factory": factory,
        "router": router, "launcher": launcher, "cmm": cmm, "tokens": tokens, "pair": pair, "multisig": gov,
        "created_token": created_token, "new_token_creation_receipt": {k: creation[k] for k in ("txhash", "code", "height", "gas_used")},
        "multisig_order": [1, 3, 2], "selected_members": [1, 2], "codes": engine.state["codes"],
        "rollout_broadcasts": chain.broadcasts, "durable_prebroadcast_checks": chain.journal_checks,
        "checks": ["actual SDK 2-of-3 signatures in 1/3/2 membership order", "empty migrate messages accepted",
                   "factory/router/pair migration", "token template rotation and actual new token creation", "taxed and zero-rate unpaired token discovery",
                   "token config/features/supply preservation", "pool reserve preservation", "pause/restore and live pair pins",
                   "execute/finalize idempotence", "durable signed transaction hash before every broadcast"],
        "receipts": [op["receipt"] for op in engine.state["operations"].values()],
        "skips": [] if args.cmm_wasm else [{"item": "CMM-owned contract migration", "reason": "No reviewed treasury 0.2.2/legacy artifacts supplied"}],
        "production_execution": "UNVERIFIED"}
    if args.cmm_wasm:
        evidence["checks"].append("CMM treasury 0.2.1 to 0.2.2 and actual migrate_owned_contract")
    write_json(args.evidence_file, evidence)
    print(json.dumps({"status": "PASS", "evidence_file": str(args.evidence_file), "rollout_broadcasts": chain.broadcasts}, indent=2))


if __name__ == "__main__":
    main()
