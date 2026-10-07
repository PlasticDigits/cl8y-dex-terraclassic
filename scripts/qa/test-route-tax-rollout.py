#!/usr/bin/env python3
"""Behavioral rollout regressions. The fake chain never invokes terrad or a network."""

from __future__ import annotations

import copy
import base64
import fcntl
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import unquote, urlparse


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))
SPEC = importlib.util.spec_from_file_location(
    "route_tax_rollout", ROOT / "scripts/lib/route_tax_rollout.py"
)
rollout = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = rollout
SPEC.loader.exec_module(rollout)
sys.path.insert(0, str(ROOT / "scripts/lib"))
CLI_SPEC = importlib.util.spec_from_file_location(
    "route_tax_rollout_cli", ROOT / "scripts/lib/route_tax_rollout_cli.py"
)
cli = importlib.util.module_from_spec(CLI_SPEC)
sys.modules[CLI_SPEC.name] = cli
CLI_SPEC.loader.exec_module(cli)
PROBE_SPEC = importlib.util.spec_from_file_location("route_tax_probe", ROOT / "scripts/qa/probe-route-tax-1381.py")
probe = importlib.util.module_from_spec(PROBE_SPEC)
PROBE_SPEC.loader.exec_module(probe)

GOV = "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7"
DEPLOYER = "terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv"
FACTORY = "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
ROUTER = "terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw"
LAUNCHER = "terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
CMM = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2"
ALPHA = "terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz"
ZERO_TAX = "terra1ps34wcgyjjp93hf2wvmt3t7v9ky9xm43yhzf6anq8kyp777egsdq0eua67"
PLAIN = "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72"
PAIR = "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2"
PAIR2 = "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch"
PUBKEYS = {
    "multisig1": "A+7iz2WaOj/1BQb1H71NddOiPjYygfHVImHz4nOEFWmk",
    "multisig2": "AmS14yFlZT8Z68M3IkeQyfBk0KoI1dm9pLR6KIin8tEq",
    "multisig3": "AnoHI0WAvhg4mx3B4hVIdeiRjRVZvTLbRXk8SiEIPLx/",
    "cl8ydeploy": "AiTnHQ19SF+H1Lp8fqzNerQuJF1NteqjXQ0cVYW13P3g",
    "cl8y2_admin": "AhGe+8a39iwdybpaZWGpAM8PyElmpNxNAMflniCvOCRO",
}
KEY_ADDRESSES = {
    "multisig1": "terra13d6jycp9hv8u64t92j2htdr53sn9f88r4uqtxm",
    "multisig2": "terra1lsewv7zjf2pe535lpdgh9dx2n928yn3ker76mq",
    "multisig3": "terra13hmvua4rdpd4un98lxrwhvj4sy50l44ffhpza7",
    "multisig_2of3": GOV,
    "cl8ydeploy": DEPLOYER,
    "cl8y2_admin": "terra1xsecn4snv94ezcez0z3vq8an9j4h4kxxcydp8l",
}


def key_record(name):
    if name == "multisig_2of3":
        # Actual user keyring order is 1, 3, 2; do not equate index with name.
        pubkey = {"@type": "/cosmos.crypto.multisig.LegacyAminoPubKey",
                  "threshold": 2, "public_keys": [
                      {"@type": "/cosmos.crypto.secp256k1.PubKey", "key": PUBKEYS[key]}
                      for key in ("multisig1", "multisig3", "multisig2")]}
        kind = "multi"
    else:
        pubkey = {"@type": "/cosmos.crypto.secp256k1.PubKey", "key": PUBKEYS[name]}
        kind = "local"
    return {"name": name, "address": KEY_ADDRESSES[name],
            "pubkey": json.dumps(pubkey), "type": kind}


def clone(value):
    return copy.deepcopy(value)


def digest(value):
    return hashlib.sha256(value).hexdigest()


class FakeChain:
    """Mutable chain state: migrations change identity, refresh changes pair pins.

    Queries return copies so implementation cannot accidentally mutate chain state
    without dispatching a transaction. Unknown queries fail instead of silently
    satisfying a newly introduced production check.
    """

    def __init__(self):
        self.chain_id = "columbus-5"
        self.actions = []
        self.codes = {}
        self.instances = {}
        self.configs = {}
        self.pair_rows = []
        self.pauses = {PAIR: False, PAIR2: True}
        self.pins = {PAIR: [100, 200], PAIR2: [101, 200]}
        self.whitelist = [100, 101, 200]
        self.transactions = {}
        self.artifact_metadata = {}
        self.pools = {}
        self.features = {}
        self.supply = {}
        self.fault = None
        self.fee = 100
        self.journal_path = None
        self.rebroadcasts = []
        self.preview_queries = []

    def network(self):
        return self.chain_id

    def info(self, address):
        return clone(self.instances[address])

    def code(self, code_id):
        return clone(self.codes[int(code_id)])

    def code_hash(self, code_id):
        return self.codes[int(code_id)]["sha256"]

    def version(self, address):
        code = self.codes[int(self.instances[address]["code_id"])]
        return {"contract": code["contract"], "version": code["version"]}

    def contracts(self, code, limit):
        result = [addr for addr, info in self.instances.items() if info["code_id"] == code]
        if len(result) > limit:
            raise rollout.RolloutError("Token contracts exceed bound")
        return result

    def smart(self, address, message):
        if message in ({"config": {}}, {"get_config": {}}):
            return clone(self.configs[address])
        if "pairs" in message and address == FACTORY:
            params = message["pairs"]
            rows = self.pair_rows
            cursor = params.get("start_after")
            if cursor:
                for index, row in enumerate(rows):
                    if row["asset_infos"] == cursor:
                        rows = rows[index + 1:]
                        break
                else:
                    raise AssertionError("Invalid factory pagination cursor")
            return {"pairs": clone(rows[:params.get("limit", 10)])}
        if "get_pair_count" in message:
            return {"count": len(self.pair_rows)}
        if "get_whitelisted_code_ids" in message:
            params = message["get_whitelisted_code_ids"]
            ids = [x for x in self.whitelist if x > (params.get("start_after") or 0)]
            count = params.get("limit", 10)
            page = ids[:count]
            return {"code_ids": page, "next": page[-1] if len(ids) > count else None}
        if "is_code_id_whitelisted" in message:
            code_id = message["is_code_id_whitelisted"]["code_id"]
            return {"code_id": code_id, "whitelisted": code_id in self.whitelist}
        if "is_paused" in message:
            return {"paused": self.pauses[address]}
        if "get_asset_code_ids" in message:
            return {"code_ids": clone(self.pins[address])}
        if "pool" in message:
            return clone(self.pools[address])
        if "get_features" in message:
            return clone(self.features[address])
        if "token_info" in message:
            return {"total_supply": self.supply[address]}
        if "route_tax_capability" in message:
            cfg = self.configs[address]
            funded = rollout.version_tuple(self.version(address)["version"]) >= (1, 2, 0)
            if not funded:
                raise RuntimeError("unknown query route_tax_capability")
            return {"funding_version": 1, "router": cfg["router"],
                    "buy_bps": cfg["buy_bps"], "sell_bps": cfg["sell_bps"]}
        if "route_tax_preview" in message:
            params = message["route_tax_preview"]
            self.preview_queries.append((address, clone(params)))
            gross = int(params["amount"])
            kind = params["kind"]
            bps = self.configs[address][kind + "_bps"]
            if kind == "sell":
                low, high = 0, gross
                while low < high:
                    middle = (low + high + 1) // 2
                    if middle + middle * bps // 10000 <= gross:
                        low = middle
                    else:
                        high = middle - 1
                economic = net = low
                tax = economic * bps // 10000
            else:
                economic = gross
                tax = gross * bps // 10000
                net = gross - tax
            return {"funding_version": 1, "gross": str(gross), "economic": str(economic),
                    "tax": str(tax), "net": str(net)}
        if "is_protocol_exempt" in message:
            return {"protocol": True}
        raise AssertionError((address, message))

    def mutate(self, action):
        self.actions.append(clone(action))
        return action

    def receipt(self, tx_hash):
        return clone(self.transactions.get(tx_hash))

    def rebroadcast(self, path, tx_hash):
        self.rebroadcasts.append((path, tx_hash))
        raise AssertionError("An uncertain transaction must not be blindly resent")

    def transaction(self, action, on_signed):
        tx_hash = digest(json.dumps({"action": action, "sequence": len(self.actions)}, sort_keys=True).encode()).upper()
        self.actions.append(clone(action))
        on_signed(tx_hash, "/external/signed-" + tx_hash + ".json", self.fee)
        if self.journal_path:
            written = json.loads(self.journal_path.read_text())
            self.checkpoint_observed = any(
                entry.get("tx_hash") == tx_hash for entry in written["operations"].values())
            if not self.checkpoint_observed:
                raise AssertionError("Broadcast preceded durable checkpoint")
        if self.fault == "after-sign":
            self.fault = None
            raise rollout.RolloutError("Fake connection lost after signing")
        receipt = {"txhash": tx_hash, "code": 0}
        if self.fault == "failed-receipt":
            receipt["code"] = 7
            self.fault = None
        elif action["op"] == "store":
            code_id = max(self.codes) + 1000
            metadata = self.artifact_metadata[action["path"]]
            self.codes[code_id] = clone(metadata)
            receipt["code_id"] = code_id
        elif action["op"] == "migrate":
            self._migrate(action["contract"], action["code_id"])
        elif action["op"] == "execute":
            msg = action["msg"]
            if "set_pair_paused" in msg:
                self.pauses[msg["set_pair_paused"]["pair"]] = msg["set_pair_paused"]["paused"]
            elif "add_whitelisted_code_id" in msg:
                self.whitelist.append(msg["add_whitelisted_code_id"]["code_id"])
            elif "update_config" in msg:
                self.configs[action["contract"]].update(msg["update_config"])
            elif "migrate_owned_contract" in msg:
                self._migrate(msg["migrate_owned_contract"]["contract"], msg["migrate_owned_contract"]["new_code_id"])
            elif "refresh_pair_asset_code_ids" in msg:
                target = msg["refresh_pair_asset_code_ids"]["pair"]
                row = next(row for row in self.pair_rows if row["contract_addr"] == target)
                self.pins[target] = [self.instances[asset["token"]["contract_addr"]]["code_id"]
                                     for asset in row["asset_infos"]]
            else:
                raise AssertionError(action)
        else:
            raise AssertionError(action)
        self.transactions[tx_hash] = clone(receipt)
        if self.fault == "after-inclusion":
            self.fault = None
            raise rollout.RolloutError("Fake connection lost after inclusion")
        return receipt

    def _migrate(self, address, code_id):
        self.instances[address]["code_id"] = code_id
        if address == ROUTER:
            self.configs[address].update(funding_version=1, settlement_version=1, last_route_id=0)


class RolloutTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.chain = FakeChain()
        self.journal = self.folder / "journal.json"
        self.chain.journal_path = self.journal
        self.artifacts = {}
        for role in ("factory", "pair", "router", "token", "launcher"):
            identity, version, filename = rollout.SPECS[role]
            binary = b"\0asm\x01\0\0\0" + identity.encode() + b"\0" + version.encode()
            path = self.folder / filename
            path.write_bytes(binary)
            self.artifacts[role] = rollout.Artifact.load(role, path, digest(binary))
            self.chain.artifact_metadata[str(path)] = {
                "contract": identity, "version": version, "sha256": digest(binary)}

        def add(address, role, code, version=None, admin=GOV):
            identity, target, _ = rollout.SPECS[role]
            live_version = version or target
            self.chain.codes[code] = {"contract": identity, "version": live_version,
                                      "sha256": self.artifacts[role].sha256 if live_version == target else digest(str(code).encode())}
            self.chain.instances[address] = {"code_id": code, "admin": admin}

        add(FACTORY, "factory", 10)
        add(ROUTER, "router", 20, "1.1.0")
        add(LAUNCHER, "launcher", 40)
        add(PAIR, "pair", 30)
        add(PAIR2, "pair", 30)
        add(ALPHA, "token", 100, "1.1.0", CMM)
        add(ZERO_TAX, "token", 101, "1.1.0")
        self.chain.instances[PLAIN] = {"code_id": 200, "admin": GOV}
        self.chain.codes[200] = {"contract": "crates.io:cw20-base", "version": "0.16.0", "sha256": digest(b"plain")}
        self.chain.instances[CMM] = {"code_id": 50, "admin": GOV}
        self.chain.codes[50] = {"contract": rollout.SPECS["cmm"][0], "version": "0.2.2", "sha256": digest(b"cmm")}
        self.chain.configs[FACTORY] = {"governance": GOV, "treasury": CMM, "pair_code_id": 30,
                                       "lp_token_code_id": 200, "default_fee_bps": 30}
        self.chain.configs[ROUTER] = {"funding_version": 0, "factory": FACTORY, "wrap_mapper": None}
        self.chain.configs[LAUNCHER] = {"token_code_id": 100, "autolp_code_id": 70,
                                       "factory": FACTORY, "router": ROUTER, "ust1": PLAIN,
                                       "cmm_treasury": CMM, "cmm_governance": GOV}
        self.chain.configs[CMM] = {"governance": GOV}
        for address, bps in ((ALPHA, 100), (ZERO_TAX, 0)):
            self.chain.configs[address] = {"buy_bps": bps, "sell_bps": bps, "transfer_bps": 0,
                                          "factory": FACTORY, "router": ROUTER, "treasury": CMM,
                                          "sinks": [{"kind": "treasury", "addr": CMM, "bps": 10000}]}
            self.chain.features[address] = {"features": ["buy_tax", "sell_tax"]}
            self.chain.supply[address] = "1000000000000"
        for pair, token in ((PAIR, ALPHA), (PAIR2, ZERO_TAX)):
            self.chain.pair_rows.append({"contract_addr": pair, "asset_infos": [
                {"token": {"contract_addr": token}}, {"token": {"contract_addr": PLAIN}}],
                "liquidity_token": PLAIN})
            self.chain.pools[pair] = {"assets": [{"amount": "1000000"}, {"amount": "2000000"}], "total_share": "10000"}
        self.original_settings = clone(self.chain.configs)
        self.engine = rollout.Rollout(self.chain, self.artifacts, {}, self.journal)

    def resume(self, **config):
        return rollout.Rollout(self.chain, self.artifacts, config, self.journal)

    def test_inventory_includes_zero_rate_tokens_and_readonly_planning(self):
        state = self.engine.prepare()
        self.assertEqual({ALPHA, ZERO_TAX}, {token["address"] for token in state["inventory"]["tokens"]})
        self.assertEqual([], self.chain.actions)
        self.assertEqual("planned", state["status"])

    def test_full_rollout_preserves_settings_and_uses_cmm_dispatch(self):
        state = self.engine.execute()
        self.assertEqual("contracts_ready_paused", state["status"])
        self.assertTrue(all(self.chain.pauses.values()))
        code = state["codes"]["token"]
        self.assertEqual(code, self.chain.instances[ALPHA]["code_id"])
        self.assertEqual(code, self.chain.instances[ZERO_TAX]["code_id"])
        self.assertIn({"op": "execute", "role": "gov", "contract": CMM,
                       "msg": {"migrate_owned_contract": {"contract": ALPHA, "new_code_id": code}}}, self.chain.actions)
        self.assertFalse(any(action["op"] == "migrate" and action.get("contract") == ALPHA for action in self.chain.actions))
        stores = [action for action in self.chain.actions if action["op"] == "store"]
        self.assertEqual({self.artifacts["router"].path, self.artifacts["token"].path}, {action["path"] for action in stores})
        self.assertEqual(self.original_settings[ALPHA], self.chain.configs[ALPHA])
        self.assertEqual(self.original_settings[ZERO_TAX], self.chain.configs[ZERO_TAX])
        self.assertEqual(70, self.chain.configs[LAUNCHER]["autolp_code_id"])
        self.assertEqual([code, 200], self.chain.pins[PAIR])
        self.assertTrue({100, 101, 200}.issubset(self.chain.whitelist))
        self.assertTrue(self.chain.checkpoint_observed)
        refresh_at = next(index for index, action in enumerate(self.chain.actions)
                          if "refresh_pair_asset_code_ids" in action.get("msg", {}))
        token_at = next(index for index, action in enumerate(self.chain.actions)
                        if action.get("contract") == ZERO_TAX and action["op"] == "migrate")
        self.assertGreater(refresh_at, token_at)
        self.assertEqual("UNVERIFIED", self.engine.verify()["production_execution"])

    def test_behind_factory_pair_launcher_and_cmm_upgrade_before_dependents(self):
        for code, version in ((10, "1.10.0"), (30, "1.18.0"), (40, "0.9.0"), (50, "0.2.1")):
            self.chain.codes[code]["version"] = version
            self.chain.codes[code]["sha256"] = digest(str(code).encode())
        identity, version, filename = rollout.SPECS["cmm"]
        path = self.folder / filename
        binary = b"\0asm\x01\0\0\0" + identity.encode() + b"\0" + version.encode()
        path.write_bytes(binary)
        self.artifacts["cmm"] = rollout.Artifact.load("cmm", path, digest(binary))
        self.chain.artifact_metadata[str(path)] = {"contract": identity, "version": version, "sha256": digest(binary)}
        state = self.engine.execute()
        pair_code = state["codes"]["pair"]
        self.assertEqual(pair_code, self.chain.configs[FACTORY]["pair_code_id"])
        self.assertEqual(pair_code, self.chain.instances[PAIR]["code_id"])
        self.assertEqual(pair_code, self.chain.instances[PAIR2]["code_id"])
        migrated = {action["contract"] for action in self.chain.actions if action["op"] == "migrate"}
        self.assertTrue({FACTORY, ROUTER, LAUNCHER, CMM, PAIR, PAIR2}.issubset(migrated))
        cmm_at = next(index for index, action in enumerate(self.chain.actions)
                      if action["op"] == "migrate" and action["contract"] == CMM)
        token_at = next(index for index, action in enumerate(self.chain.actions)
                        if "migrate_owned_contract" in action.get("msg", {}))
        self.assertLess(cmm_at, token_at)
        self.assertEqual(70, self.chain.configs[LAUNCHER]["autolp_code_id"])

    def test_reviewed_eoa_admin_is_used_only_for_its_proven_token(self):
        self.chain.instances[ZERO_TAX]["admin"] = DEPLOYER
        self.engine.config["eoa_admins"] = {DEPLOYER: "cl8ydeploy"}
        self.engine.execute()
        direct = next(action for action in self.chain.actions
                      if action["op"] == "migrate" and action.get("contract") == ZERO_TAX)
        self.assertEqual("cl8ydeploy", direct["role"])

    def test_already_funded_fleet_does_not_store_or_migrate_again(self):
        for code, role in ((20, "router"), (100, "token"), (101, "token")):
            self.chain.codes[code]["version"] = self.artifacts[role].version
            self.chain.codes[code]["sha256"] = self.artifacts[role].sha256
        self.chain.configs[ROUTER].update(funding_version=1, settlement_version=1, last_route_id=0)
        self.engine.execute()
        self.assertFalse(any(action["op"] in ("store", "migrate") for action in self.chain.actions))
        self.assertFalse(any("migrate_owned_contract" in action.get("msg", {}) for action in self.chain.actions))
        self.assertEqual(70, self.chain.configs[LAUNCHER]["autolp_code_id"])

    def test_finalization_restores_only_original_pause_states_and_is_idempotent(self):
        self.engine.execute()
        self.engine.finalize()
        self.assertEqual({PAIR: False, PAIR2: True}, self.chain.pauses)
        count = len(self.chain.actions)
        self.resume().finalize()
        self.assertEqual(count, len(self.chain.actions))

    def test_resume_completed_execution_does_not_repeat_transactions(self):
        self.engine.execute()
        count = len(self.chain.actions)
        self.resume().execute()
        self.assertEqual(count, len(self.chain.actions))

    def test_unknown_pending_hash_is_not_resent(self):
        self.chain.fault = "after-sign"
        with self.assertRaises(rollout.RolloutError):
            self.engine.execute()
        persisted = json.loads(self.journal.read_text())
        self.assertTrue(next(iter(persisted["operations"].values()))["tx_hash"])
        count = len(self.chain.actions)
        with self.assertRaisesRegex(rollout.RolloutError, "Pending transaction"):
            self.resume().execute()
        self.assertEqual(count, len(self.chain.actions))
        self.assertEqual([], self.chain.rebroadcasts)

    def test_resume_after_inclusion_reconciles_receipt_without_duplicate(self):
        self.chain.fault = "after-inclusion"
        with self.assertRaises(rollout.RolloutError):
            self.engine.execute()
        first_action = clone(self.chain.actions[0])
        self.resume().execute()
        self.assertEqual(1, self.chain.actions.count(first_action))

    def test_failed_receipt_remains_failure_on_resume(self):
        self.chain.fault = "failed-receipt"
        with self.assertRaisesRegex(rollout.RolloutError, "failed on chain"):
            self.engine.execute()
        count = len(self.chain.actions)
        with self.assertRaisesRegex(rollout.RolloutError, "failed on chain"):
            self.resume().execute()
        self.assertEqual(count, len(self.chain.actions))

    def test_failure_mid_fleet_keeps_markets_contained_and_blocks_finalize(self):
        original = self.chain.transaction

        def fail_token(action, checkpoint):
            if "migrate_owned_contract" in action.get("msg", {}):
                self.chain.fault = "failed-receipt"
            return original(action, checkpoint)

        with patch.object(self.chain, "transaction", side_effect=fail_token):
            with self.assertRaisesRegex(rollout.RolloutError, "failed on chain"):
                self.engine.execute()
        self.assertTrue(all(self.chain.pauses.values()))
        with self.assertRaisesRegex(rollout.RolloutError, "not ready"):
            self.resume().finalize()

    def test_resume_external_unpause_stops_before_another_fleet_transaction(self):
        original = self.chain.transaction

        def interrupt_refresh(action, checkpoint):
            if action.get("msg", {}).get("refresh_pair_asset_code_ids", {}).get("pair") == PAIR:
                self.chain.fault = "after-inclusion"
            return original(action, checkpoint)

        with patch.object(self.chain, "transaction", side_effect=interrupt_refresh):
            with self.assertRaises(rollout.RolloutError):
                self.engine.execute()
        self.chain.pauses[PAIR] = False
        count = len(self.chain.actions)
        with self.assertRaises(rollout.RolloutError):
            self.resume().execute()
        self.assertEqual(count, len(self.chain.actions))

    def test_chain_identity_admin_and_hash_fail_before_any_transaction(self):
        for change in ("chain", "admin", "hash"):
            with self.subTest(change=change):
                original = clone(self.chain.instances[FACTORY])
                old_hash = self.chain.codes[10]["sha256"]
                if change == "chain":
                    self.chain.chain_id = "phoenix-1"
                elif change == "admin":
                    self.chain.instances[FACTORY]["admin"] = DEPLOYER
                else:
                    self.chain.codes[10]["sha256"] = "0" * 64
                with self.assertRaises(rollout.RolloutError):
                    self.engine.inventory()
                self.chain.chain_id = "columbus-5"
                self.chain.instances[FACTORY] = original
                self.chain.codes[10]["sha256"] = old_hash
        self.assertEqual([], self.chain.actions)

    def test_same_version_different_hash_and_downgrade_are_rejected(self):
        self.chain.codes[20]["version"] = rollout.SPECS["router"][1]
        with self.assertRaisesRegex(rollout.RolloutError, "Same-version"):
            self.engine.inventory()
        self.chain.codes[20]["version"] = "99.0.0"
        with self.assertRaisesRegex(rollout.RolloutError, "downgrade"):
            self.engine.inventory()

    def test_source_artifact_changes_and_resume_input_changes_are_rejected(self):
        self.engine.prepare()
        with self.assertRaisesRegex(rollout.RolloutError, "Changed rollout inputs"):
            self.resume(max_pairs=25).prepare()
        path = Path(self.artifacts["router"].path)
        path.write_bytes(path.read_bytes() + b"changed")
        with self.assertRaisesRegex(rollout.RolloutError, "Artifact changed"):
            self.resume().prepare()

    def test_saved_inventory_edit_is_rejected_on_resume(self):
        self.engine.prepare()
        saved = json.loads(self.journal.read_text())
        saved["inventory"]["tokens"][0]["admin"] = GOV
        saved["inventory"]["pairs"][0]["paused"] = True
        self.journal.write_text(json.dumps(saved))
        with self.assertRaises(rollout.RolloutError):
            self.resume().prepare()

    def test_complete_execution_cannot_replay_cached_pause_receipts(self):
        self.engine.execute()
        self.engine.finalize()
        before = len(self.chain.actions)
        with self.assertRaises(rollout.RolloutError):
            self.resume().execute()
        self.assertEqual(before, len(self.chain.actions))

    def test_paired_managed_token_wrong_router_is_not_silently_skipped(self):
        self.chain.configs[ZERO_TAX]["router"] = FACTORY
        with self.assertRaises(rollout.RolloutError):
            self.engine.inventory()

    def test_unpaired_launcher_inventory_router_mismatch_blocks_all_transactions(self):
        unpaired = "synthetic-unpaired-token"
        self.chain.instances[unpaired] = clone(self.chain.instances[ALPHA])
        self.chain.configs[unpaired] = {**clone(self.chain.configs[ALPHA]), "router": FACTORY}
        with self.assertRaisesRegex(rollout.RolloutError, "requires in-place migration"):
            self.engine.execute()
        self.assertEqual([], self.chain.actions)
        self.assertFalse(self.journal.exists())

    def test_binding_change_after_planning_blocks_migration_before_transactions(self):
        self.engine.prepare()
        self.chain.configs[ZERO_TAX]["router"] = FACTORY
        with self.assertRaisesRegex(rollout.RolloutError, "Token settings changed"):
            self.resume().execute()
        self.assertEqual([], self.chain.actions)

    def test_router_monotonic_counter_is_not_an_operator_setting(self):
        self.engine.execute()
        self.chain.configs[ROUTER]["last_route_id"] = 7
        self.assertEqual("PASS", self.engine.verify()["contract_verification"])
        self.chain.configs[ROUTER]["settlement_version"] = 0
        with self.assertRaisesRegex(rollout.RolloutError, "settlement version"):
            self.engine.verify()

    def test_router_replacement_is_rejected_even_if_launcher_was_repointed(self):
        replacement = "synthetic-new-router"
        self.chain.instances[replacement] = clone(self.chain.instances[ROUTER])
        self.chain.configs[replacement] = clone(self.chain.configs[ROUTER])
        self.chain.configs[LAUNCHER]["router"] = replacement
        engine = rollout.Rollout(self.chain, self.artifacts, {"router": replacement}, self.journal)
        with self.assertRaisesRegex(rollout.RolloutError, "configured router mismatch"):
            engine.execute()
        self.assertEqual([], self.chain.actions)

    def test_funded_capability_preflight_is_inventory_wide(self):
        unpaired = "synthetic-unpaired-funded-token"
        self.chain.instances[unpaired] = {"code_id": 102, "admin": GOV}
        self.chain.codes[102] = {"contract": rollout.SPECS["token"][0], "version": "1.2.0",
                                 "sha256": digest(b"legacy-funded-token")}
        self.chain.configs[unpaired] = clone(self.chain.configs[ALPHA])
        # Discovered under the launcher template even though it has no pool.
        self.chain.configs[LAUNCHER]["token_code_id"] = 102
        original = self.chain.smart

        def malformed_capability(address, msg):
            if address == unpaired and "route_tax_capability" in msg:
                return {"funding_version": 1, "router": FACTORY}
            return original(address, msg)

        with patch.object(self.chain, "smart", side_effect=malformed_capability):
            with self.assertRaisesRegex(rollout.RolloutError, "funding/router capability mismatch"):
                self.engine.execute()
        self.assertEqual([], self.chain.actions)

    def test_same_version_fake_hash_fails_before_migration_and_on_resume(self):
        self.chain.codes[20].update(version=self.artifacts["router"].version, sha256=self.artifacts["router"].sha256)
        self.chain.configs[ROUTER].update(funding_version=1, settlement_version=1, last_route_id=0)
        self.engine.prepare()
        self.chain.codes[20]["sha256"] = "0" * 64
        # A preserved CW2 label cannot establish immutable artifact identity.
        with self.assertRaisesRegex(rollout.RolloutError, "reviewed|checksum|Same-version"):
            self.resume().execute()
        self.assertEqual([], self.chain.actions)

    def test_explicit_noncommunity_contract_is_rejected(self):
        engine = rollout.Rollout(self.chain, self.artifacts, {}, self.journal, explicit_tokens=[PLAIN])
        with self.assertRaises(rollout.RolloutError):
            engine.inventory()

    def test_factory_creation_template_rotates_even_if_existing_pairs_current(self):
        self.chain.codes[31] = {"contract": rollout.SPECS["pair"][0], "version": "1.18.0", "sha256": digest(b"oldpair")}
        self.chain.configs[FACTORY]["pair_code_id"] = 31
        self.engine.execute()
        target = self.chain.configs[FACTORY]["pair_code_id"]
        self.assertEqual(self.artifacts["pair"].sha256, self.chain.code_hash(target))

    def test_optional_autolp_rotates_only_reviewed_template_and_preserves_instances(self):
        identity, version, filename = rollout.SPECS["autolp"]
        binary = b"\0asm\x01\0\0\0" + identity.encode() + b"\0" + version.encode()
        path = self.folder / filename
        path.write_bytes(binary)
        self.artifacts["autolp"] = rollout.Artifact.load("autolp", path, digest(binary))
        self.chain.artifact_metadata[str(path)] = {"contract": identity, "version": version, "sha256": digest(binary)}
        self.chain.codes[70] = {"contract": identity, "version": "1.0.0", "sha256": digest(b"legacy-autolp")}
        self.chain.instances["legacy-sister"] = {"code_id": 70, "admin": GOV}
        original = clone(self.chain.configs[LAUNCHER])
        self.engine.prepare()
        self.engine.execute()
        target = self.engine.state["codes"]["autolp"]
        self.assertEqual(self.artifacts["autolp"].sha256, self.chain.code_hash(target))
        self.assertEqual({**original, "token_code_id": self.engine.state["codes"]["token"],
                          "autolp_code_id": target}, self.chain.configs[LAUNCHER])
        self.assertEqual(70, self.chain.instances["legacy-sister"]["code_id"])
        self.engine.verify()
        self.engine.finalize()
        self.engine.verify()

    def test_optional_autolp_reuses_exact_current_code_without_store(self):
        identity, version, filename = rollout.SPECS["autolp"]
        binary = b"\0asm\x01\0\0\0" + identity.encode() + b"\0" + version.encode()
        path = self.folder / filename
        path.write_bytes(binary)
        self.artifacts["autolp"] = rollout.Artifact.load("autolp", path, digest(binary))
        self.chain.codes[70] = {"contract": identity, "version": version, "sha256": digest(binary)}
        self.engine.prepare()
        self.engine.execute()
        self.assertEqual(70, self.engine.state["codes"]["autolp"])
        self.assertFalse(any(action["op"] == "store" and action["path"] == str(path)
                             for action in self.chain.actions))
        self.engine.verify()

    def test_verification_detects_pins_whitelist_factory_and_pair_identity_drift(self):
        self.engine.execute()
        original_pins = clone(self.chain.pins)
        original_whitelist = clone(self.chain.whitelist)
        original_factory = clone(self.chain.configs[FACTORY])
        original_instances = clone(self.chain.instances)
        for drift in ("pins", "whitelist", "factory", "pair-admin", "pair-code", "token-admin", "paused"):
            with self.subTest(drift=drift):
                if drift == "pins":
                    self.chain.pins[PAIR] = [100, 200]
                elif drift == "whitelist":
                    self.chain.whitelist.remove(100)
                elif drift == "factory":
                    self.chain.configs[FACTORY]["treasury"] = DEPLOYER
                elif drift == "pair-admin":
                    self.chain.instances[PAIR]["admin"] = DEPLOYER
                elif drift == "pair-code":
                    self.chain.instances[PAIR]["code_id"] = 200
                elif drift == "token-admin":
                    self.chain.instances[ALPHA]["admin"] = GOV
                else:
                    self.chain.pauses[PAIR] = False
                with self.assertRaises(rollout.RolloutError):
                    self.engine.verify()
                self.chain.pins = clone(original_pins)
                self.chain.whitelist = clone(original_whitelist)
                self.chain.configs[FACTORY] = clone(original_factory)
                self.chain.instances = clone(original_instances)
                self.chain.pauses[PAIR] = True

    def test_missing_receipt_execution_code_is_rejected(self):
        original = self.chain.transaction

        def malformed(action, checkpoint):
            receipt = original(action, checkpoint)
            receipt.pop("code")
            return receipt

        with patch.object(self.chain, "transaction", side_effect=malformed):
            with self.assertRaises(rollout.RolloutError):
                self.engine.execute()

    def test_cmm_requires_dispatcher_support_and_real_governance(self):
        self.chain.codes[50]["version"] = "0.2.1"
        with self.assertRaisesRegex(rollout.RolloutError, "requires reviewed treasury"):
            self.engine.inventory()
        self.chain.codes[50]["version"] = "0.2.2"
        self.chain.configs[CMM]["governance"] = DEPLOYER
        with self.assertRaisesRegex(rollout.RolloutError, "CMM governance"):
            self.engine.inventory()

    def test_pair_count_and_inventory_bound_are_enforced(self):
        self.engine.config["max_pairs"] = 1
        with self.assertRaisesRegex(rollout.RolloutError, "Pair inventory exceeds"):
            self.engine.inventory()
        self.engine.config["max_pairs"] = 256
        original = self.chain.smart

        def inconsistent_count(address, msg):
            return {"count": 3} if "get_pair_count" in msg else original(address, msg)

        with patch.object(self.chain, "smart", side_effect=inconsistent_count):
            with self.assertRaisesRegex(rollout.RolloutError, "Pair-count inventory mismatch"):
                self.engine.inventory()

    def test_token_inventory_bound_rejects_even_no_pair_unlisted_instances(self):
        self.engine.config["max_tokens"] = 2
        with self.assertRaisesRegex(rollout.RolloutError, "Token discovery exceeds"):
            self.engine.inventory()

    def test_factory_pagination_duplicate_is_rejected(self):
        original = self.chain.smart

        def duplicate_page(address, msg):
            if "pairs" in msg:
                return {"pairs": [clone(self.chain.pair_rows[0])] * 30}
            return original(address, msg)

        with patch.object(self.chain, "smart", side_effect=duplicate_page):
            with self.assertRaisesRegex(rollout.RolloutError, "Duplicate/stalled"):
                self.engine.inventory()

    def test_explicit_token_binding_and_admin_are_validated(self):
        self.chain.configs[ALPHA]["router"] = FACTORY
        with self.assertRaisesRegex(rollout.RolloutError, "configured router mismatch"):
            self.engine.inventory()
        self.chain.configs[ALPHA]["router"] = ROUTER
        self.chain.instances[ALPHA]["admin"] = DEPLOYER
        with self.assertRaisesRegex(rollout.RolloutError, "Unsupported token wasm admin"):
            self.engine.inventory()

    def test_fee_budget_stops_before_new_submission(self):
        self.engine.config["max_total_fee_uluna"] = 99
        with self.assertRaisesRegex(rollout.RolloutError, "fee budget"):
            self.engine.execute()
        self.assertEqual({}, self.chain.transactions)

    def test_negative_zero_boolean_and_noninteger_callback_fees_stop_before_broadcast(self):
        for fee in (-1, 0, True, "100"):
            with self.subTest(fee=fee):
                self.chain.fee = fee
                with self.assertRaisesRegex(rollout.RolloutError, "Malformed transaction fee"):
                    self.engine.execute()
                self.assertEqual({}, self.chain.transactions)

    def test_completed_rollout_verification_allows_normal_burn_and_pool_activity(self):
        self.engine.execute()
        self.engine.finalize()
        self.chain.supply[ALPHA] = "999999999999"
        self.chain.pools[PAIR]["assets"][0]["amount"] = "1000001"
        self.assertEqual("PASS", self.resume().verify()["contract_verification"])

    def test_saved_receipt_must_still_exist_on_the_current_chain(self):
        self.engine.execute()
        first_hash = next(iter(self.engine.state["operations"].values()))["tx_hash"]
        del self.chain.transactions[first_hash]
        with self.assertRaisesRegex(rollout.RolloutError, "no longer exists"):
            self.resume().prepare()

    def test_changed_tax_settings_or_supply_fail_verification(self):
        self.engine.execute()
        self.chain.configs[ALPHA]["buy_bps"] += 1
        with self.assertRaisesRegex(rollout.RolloutError, "Token settings"):
            self.engine.verify()
        self.chain.configs[ALPHA] = clone(self.original_settings[ALPHA])
        self.chain.supply[ALPHA] = "0"
        with self.assertRaisesRegex(rollout.RolloutError, "Token supply"):
            self.engine.verify()

    def test_funded_previews_round_sell_to_maximal_input_and_check_zero_rate_buy(self):
        self.chain.configs[ALPHA].update(buy_bps=137, sell_bps=137)
        self.engine.execute()
        self.assertTrue({(ALPHA, "sell"), (ALPHA, "buy"), (ZERO_TAX, "sell"), (ZERO_TAX, "buy")}.issubset(
            {(address, params["kind"]) for address, params in self.chain.preview_queries}))
        for address, params in self.chain.preview_queries:
            self.assertIsNone(params["trader"])
            self.assertEqual("1010000", params["amount"])
            self.assertEqual(PAIR if address == ALPHA else PAIR2, params["pair"])
        sell = self.chain.smart(ALPHA, {"route_tax_preview": {
            "kind": "sell", "amount": "1010000", "trader": None, "pair": PAIR}})
        economic, tax = int(sell["economic"]), int(sell["tax"])
        self.assertLessEqual(economic + tax, 1010000)
        self.assertGreater(economic + 1 + (economic + 1) * 137 // 10000, 1010000)

    def test_malformed_or_spoofed_funded_preview_is_rejected(self):
        self.engine.execute()
        original = self.chain.smart
        for spoof in ("version", "gross", "negative", "maximal", "buy-conservation", "wallet-tax-waiver"):
            with self.subTest(spoof=spoof):
                def incorrect(address, msg):
                    result = original(address, msg)
                    params = msg.get("route_tax_preview")
                    if address != ALPHA or not params:
                        return result
                    if spoof == "version":
                        result["funding_version"] = 0
                    elif spoof == "gross":
                        result["gross"] = "1010001"
                    elif spoof == "negative":
                        result["net"] = "-1"
                    elif spoof == "maximal" and params["kind"] == "sell":
                        result.update(economic="1", tax="0", net="1")
                    elif spoof == "buy-conservation" and params["kind"] == "buy":
                        result["net"] = str(int(result["net"]) + 1)
                    elif spoof == "wallet-tax-waiver":
                        result.update(economic="1010000", net="1010000", tax="0")
                    return result
                with patch.object(self.chain, "smart", side_effect=incorrect):
                    with self.assertRaises(rollout.RolloutError):
                        self.engine.verify()


class PureValidationTest(unittest.TestCase):
    def test_2of3_signer_membership_accepts_actual_non_numeric_order(self):
        multisig = json.loads(key_record("multisig_2of3")["pubkey"])
        for names in (("multisig1", "multisig2"), ("multisig3", "multisig1")):
            rollout.validate_signers(multisig, [json.loads(key_record(name)["pubkey"]) for name in names])

    def test_duplicate_nonmember_and_wrong_threshold_are_rejected(self):
        multisig = json.loads(key_record("multisig_2of3")["pubkey"])
        first = json.loads(key_record("multisig1")["pubkey"])
        other = json.loads(key_record("cl8ydeploy")["pubkey"])
        for keys in ((first, first), (first, other), (first,)):
            with self.subTest(keys=keys), self.assertRaises(rollout.RolloutError):
                rollout.validate_signers(multisig, keys)
        multisig["threshold"] = 1
        with self.assertRaises(rollout.RolloutError):
            rollout.validate_signers(multisig, [first, other])

    def test_wasm_checksum_magic_and_cw2_identity_are_required(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "router.wasm"
            for binary in (b"cl8y-dex-router1.2.0", b"\0asm\x01\0\0\0wrong1.2.0",
                           b"\0asm\x01\0\0\0cl8y-dex-router1.1.0"):
                path.write_bytes(binary)
                with self.subTest(binary=binary), self.assertRaises(rollout.RolloutError):
                    rollout.Artifact.load("router", path, digest(binary))
            valid = b"\0asm\x01\0\0\0cl8y-dex-router1.2.0"
            path.write_bytes(valid)
            with self.assertRaisesRegex(rollout.RolloutError, "checksum"):
                rollout.Artifact.load("router", path, "0" * 64)
            with self.assertRaisesRegex(rollout.RolloutError, "reviewed SHA"):
                rollout.Artifact.load("router", path, "")


class TransportTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        with patch.dict(os.environ, {"TERRAD_HOST_KEYRING_BACKEND": "file"}):
            self.args = cli.parser().parse_args(["--state-dir", str(self.folder)])
        self.chain = cli.TerradChain(self.args)

    def test_default_key_names_and_file_backend_match_available_custody(self):
        self.assertEqual("cl8ydeploy", self.args.store_key)
        self.assertEqual("multisig_2of3", self.args.multisig_key)
        self.assertEqual(("multisig1", "multisig2"), (self.args.signer1, self.args.signer2))
        self.assertEqual("file", self.args.keyring_backend)
        self.assertFalse(self.args.execute or self.args.finalize)

    def test_passphrase_prompt_once_and_actual_multisig_members_accepted(self):
        records = []

        def query(parts):
            self.assertEqual(["keys", "show"], parts[:2])
            records.append(parts[2])
            return json.dumps(key_record(parts[2]))

        with patch.dict(os.environ, {"TERRAD_HOST_KEYRING_PASS": ""}), \
                patch.object(sys.stdin, "isatty", return_value=True), \
                patch.object(cli.getpass, "getpass", return_value="synthetic-password") as prompt, \
                patch.object(self.chain, "command", side_effect=query):
            self.chain.unlock()
            self.chain.unlock()
            prompt.assert_called_once()
        self.assertEqual({"cl8ydeploy", "multisig_2of3", "multisig1", "multisig2"}, set(records))
        self.assertEqual(4, len(records))

    def test_duplicate_signer_or_wrong_store_address_is_rejected(self):
        for problem in ("signer", "store"):
            with self.subTest(problem=problem):
                args = clone(self.args)
                if problem == "signer":
                    args.signer2 = args.signer1
                else:
                    args.store_address = GOV
                chain = cli.TerradChain(args)
                with patch.dict(os.environ, {"TERRAD_HOST_KEYRING_PASS": "synthetic-password"}), \
                        patch.object(chain, "command", side_effect=lambda parts: json.dumps(key_record(parts[2]))):
                    with self.assertRaises(rollout.RolloutError):
                        chain.unlock()

    def test_subprocess_diagnostics_redact_password_and_never_pass_it_in_argv(self):
        secret = "synthetic-password-with-$-and-quote"
        self.chain.password = secret
        failure = subprocess.CompletedProcess(["terrad"], 1, "", "Failed unlock: " + secret)
        with patch.object(cli.subprocess, "run", return_value=failure) as command:
            with self.assertRaises(rollout.RolloutError) as error:
                self.chain.command(["keys", "show", "cl8ydeploy"])
        self.assertNotIn(secret, str(error.exception))
        self.assertIn("[redacted]", str(error.exception))
        self.assertNotIn(secret, json.dumps(command.call_args.args[0]))
        self.assertIn(secret, command.call_args.kwargs["input"])

    def test_subprocess_success_output_stays_internal_and_timeout_never_exposes_password(self):
        secret = "synthetic-secret-for-output"
        self.chain.password = secret
        success = subprocess.CompletedProcess(["terrad"], 0, '{"wrapper_echo":"' + secret + '"}', "")
        with patch.object(cli.subprocess, "run", return_value=success), \
                patch.object(sys, "stdout", io.StringIO()) as public_output:
            self.chain.command(["keys", "show", "cl8ydeploy"])
        self.assertEqual("", public_output.getvalue())
        timeout = subprocess.TimeoutExpired(["terrad"], 1, output=secret, stderr=secret)
        with patch.object(cli.subprocess, "run", side_effect=timeout):
            with self.assertRaises(rollout.RolloutError) as error:
                self.chain.command(["keys", "show", "cl8ydeploy"])
        self.assertNotIn(secret, str(error.exception))

    def test_successful_sdk_json_is_unchanged_when_password_matches_public_fields(self):
        output = json.dumps({"address": GOV, "sequence": "1", "pubkey": json.loads(key_record("multisig1")["pubkey"])})
        for password in ("terra", "1"):
            with self.subTest(password=password):
                self.chain.password = password
                result = subprocess.CompletedProcess(["terrad"], 0, output, "")
                with patch.object(cli.subprocess, "run", return_value=result):
                    self.assertEqual(json.loads(output), json.loads(self.chain.command(["keys", "show", "multisig1"])))

    def test_eoa_admin_key_is_bound_to_its_reviewed_public_address(self):
        self.args.admin_key = ["cl8y2_admin"]
        chain = cli.TerradChain(self.args)

        def wrong_admin(parts):
            record = key_record(parts[2])
            if parts[2] == "cl8y2_admin":
                record["address"] = GOV
            return json.dumps(record)

        with patch.dict(os.environ, {"TERRAD_HOST_KEYRING_PASS": "synthetic-password"}), \
                patch.object(chain, "command", side_effect=wrong_admin):
            with self.assertRaisesRegex(rollout.RolloutError, "EOA admin key address"):
                chain.unlock()

    def test_rpc_lcd_identity_disagreement_stops_before_signing(self):
        with patch.object(self.chain, "get", side_effect=[
                {"result": {"node_info": {"network": "columbus-5"}}},
                {"block": {"header": {"chain_id": "phoenix-1"}}}]), \
                patch.object(self.chain, "command") as command:
            with self.assertRaisesRegex(rollout.RolloutError, "different chains"):
                self.chain.network()
        command.assert_not_called()

    def test_code_instance_discovery_follows_cursor_and_rejects_repeats(self):
        responses = [{"contracts": [ALPHA], "pagination": {"next_key": "second/+="}},
                     {"contracts": [ZERO_TAX], "pagination": {"next_key": None}}]
        with patch.object(self.chain, "get", side_effect=responses) as get:
            self.assertEqual([ALPHA, ZERO_TAX], self.chain.contracts(100, 3))
        self.assertIn("pagination.key=second%2F%2B%3D", get.call_args_list[1].args[0])
        with patch.object(self.chain, "get", side_effect=[responses[0], responses[0]]):
            with self.assertRaises(rollout.RolloutError):
                self.chain.contracts(100, 3)

    def test_changed_signed_bytes_cannot_be_rebroadcast(self):
        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_encoded_hash", return_value="A" * 64), \
                patch.object(self.chain, "command") as command:
            with self.assertRaisesRegex(rollout.RolloutError, "signed bytes changed"):
                self.chain.rebroadcast("/external/signed.json", "B" * 64)
        command.assert_not_called()

    def test_identical_saved_bytes_already_included_do_not_broadcast(self):
        receipt = {"txhash": "A" * 64, "code": 0, "height": "25"}
        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_encoded_hash", return_value="A" * 64), \
                patch.object(self.chain, "receipt", return_value=receipt), \
                patch.object(self.chain, "command") as command:
            self.assertEqual(receipt, self.chain.rebroadcast("/external/signed.json", "A" * 64))
        command.assert_not_called()

    def test_ambiguous_store_code_receipt_is_rejected(self):
        receipt = {"txhash": "A" * 64, "code": 0, "height": "12", "events": [
            {"type": "store_code", "attributes": [{"key": "code_id", "value": "1000"}]},
            {"type": "store_code", "attributes": [{"key": "code_id", "value": "1001"}]}]}
        with patch.object(self.chain, "get", return_value={"tx_response": receipt}):
            with self.assertRaisesRegex(rollout.RolloutError, "Ambiguous"):
                self.chain.receipt("A" * 64)

    def transaction_harness(self, tamper=None):
        self.chain.unlocked = True
        self.chain.public_keys = {name: json.loads(key_record(name)["pubkey"])
                                  for name in ("multisig_2of3", "multisig1", "multisig2")}
        action = {"op": "execute", "role": "gov", "contract": FACTORY,
                  "msg": {"set_pair_paused": {"pair": PAIR, "paused": True}}}
        unsigned = {"body": {"messages": [{"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
                                            "sender": GOV, "contract": FACTORY, "msg": action["msg"]}]},
                    "auth_info": {"fee": {"amount": [{"denom": "uluna", "amount": "100"}], "gas_limit": "1000"}}}
        commands = []

        def command(parts):
            commands.append(parts)
            if "--generate-only" in parts:
                value = clone(unsigned)
                if tamper == "generated":
                    value["body"]["messages"][0]["contract"] = ROUTER
                if tamper == "type":
                    value["body"]["messages"][0]["@type"] = "/cosmwasm.wasm.v1.MsgClearAdmin"
                if tamper == "memo":
                    value["body"]["memo"] = "unapproved"
                if tamper == "extension":
                    value["body"]["extension_options"] = [{"@type": "/unknown.Extension"}]
                if tamper == "tip":
                    value["auth_info"]["tip"] = {"tipper": GOV, "amount": [{"denom": "uluna", "amount": "100"}]}
                if tamper == "unordered":
                    value["body"]["unordered"] = True
                if tamper == "timestamp":
                    value["body"]["timeout_timestamp"] = "2026-10-04T00:00:00Z"
                if tamper == "defaults":
                    value["body"].update(memo="", timeout_height="0", unordered=False, timeout_timestamp=None,
                                          extension_options=[], non_critical_extension_options=[])
                    unsigned["body"] = clone(value["body"])
                return json.dumps(value)
            if parts[:2] == ["tx", "sign"]:
                return json.dumps({"signature": "synthetic"})
            if parts[:2] == ["tx", "multisign"]:
                value = clone(unsigned)
                value["signatures"] = ["synthetic"]
                value["auth_info"]["signer_infos"] = [{
                    "public_key": self.chain.public_keys["multisig_2of3"], "sequence": "2",
                    "mode_info": {"multi": {
                        "bitarray": {"extra_bits_stored": 3, "elems": "oA=="},
                        "mode_infos": [{"single": {"mode": "SIGN_MODE_LEGACY_AMINO_JSON"}}] * 2}}}]
                if tamper == "signed":
                    value["body"]["messages"][0]["contract"] = ROUTER
                if tamper == "fee":
                    value["auth_info"]["fee"]["amount"][0]["amount"] = "9000"
                if tamper == "sequence":
                    value["auth_info"]["signer_infos"][0]["sequence"] = "3"
                if tamper == "key":
                    value["auth_info"]["signer_infos"][0]["public_key"] = json.loads(key_record("cl8ydeploy")["pubkey"])
                if tamper == "signature-count":
                    value["signatures"] = ["synthetic", "extra"]
                if tamper == "members":
                    value["auth_info"]["signer_infos"][0]["mode_info"]["multi"]["bitarray"]["elems"] = "wA=="
                return json.dumps(value)
            if parts[:2] == ["tx", "encode"]:
                return base64.b64encode(b"synthetic-exact-txraw").decode()
            raise AssertionError(parts)

        return action, command, commands

    def test_generated_operation_substitution_is_rejected_before_signing(self):
        action, command, commands = self.transaction_harness("generated")
        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_account", return_value=("1", "2")), \
                patch.object(self.chain, "command", side_effect=command), \
                patch.object(self.chain, "rebroadcast") as broadcast:
            with self.assertRaises(rollout.RolloutError):
                self.chain.transaction(action, lambda *_: None)
        self.assertFalse(any(parts[:2] == ["tx", "sign"] for parts in commands))
        broadcast.assert_not_called()

    def test_generated_sdk_message_type_is_bound_to_approved_operation(self):
        action, command, commands = self.transaction_harness("type")
        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_account", return_value=("1", "2")), \
                patch.object(self.chain, "command", side_effect=command), \
                patch.object(self.chain, "rebroadcast") as broadcast:
            with self.assertRaises(rollout.RolloutError):
                self.chain.transaction(action, lambda *_: None)
        self.assertFalse(any(parts[:2] == ["tx", "sign"] for parts in commands))
        broadcast.assert_not_called()

    def test_unapproved_generated_memo_tip_extensions_or_timeout_cannot_be_signed(self):
        for tamper in ("memo", "tip", "extension", "unordered", "timestamp"):
            with self.subTest(tamper=tamper):
                action, command, commands = self.transaction_harness(tamper)
                with patch.object(self.chain, "network", return_value="columbus-5"), \
                        patch.object(self.chain, "_account", return_value=("1", "2")), \
                        patch.object(self.chain, "command", side_effect=command), \
                        patch.object(self.chain, "rebroadcast") as broadcast:
                    with self.assertRaises(rollout.RolloutError):
                        self.chain.transaction(action, lambda *_: None)
                self.assertFalse(any(parts[:2] == ["tx", "sign"] for parts in commands))
                broadcast.assert_not_called()

    def test_normal_sdk_body_defaults_are_accepted(self):
        action, command, _ = self.transaction_harness("defaults")
        receipt = {"txhash": digest(b"synthetic-exact-txraw").upper(), "code": 0}
        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_account", return_value=("1", "2")), \
                patch.object(self.chain, "command", side_effect=command), \
                patch.object(self.chain, "rebroadcast", return_value=receipt) as broadcast, \
                patch.object(sys, "stdout", io.StringIO()):
            self.assertEqual(receipt, self.chain.transaction(action, lambda *_: None))
        broadcast.assert_called_once()

    def test_saved_fee_accounting_and_intent_are_checked_without_unlocking(self):
        action, command, _ = self.transaction_harness()
        signed = json.loads(command(["tx", "multisign"]))
        path = self.folder / "saved-signed.json"
        path.write_text(json.dumps(signed))
        entry = {"signed_path": str(path), "tx_hash": digest(path.read_bytes()).upper(), "fee_uluna": 100}
        with patch.object(self.chain, "_encoded_hash", side_effect=lambda p: digest(Path(p).read_bytes()).upper()), \
                patch.object(self.chain, "unlock") as unlock:
            self.chain.validate_saved(action, entry)
            tampered_fee = {**entry, "fee_uluna": 1}
            with self.assertRaisesRegex(rollout.RolloutError, "fee accounting"):
                self.chain.validate_saved(action, tampered_fee)
            tampered_action = clone(action)
            tampered_action["msg"]["set_pair_paused"]["paused"] = False
            with self.assertRaisesRegex(rollout.RolloutError, "Saved operation changed"):
                self.chain.validate_saved(tampered_action, entry)
            signed["body"]["messages"][0]["contract"] = ROUTER
            path.write_text(json.dumps(signed))
            with self.assertRaisesRegex(rollout.RolloutError, "signed transaction changed"):
                self.chain.validate_saved(action, entry)
        unlock.assert_not_called()

    def test_signed_operation_and_fee_substitution_are_rejected(self):
        for tamper in ("signed", "fee", "sequence", "key", "signature-count", "members"):
            with self.subTest(tamper=tamper):
                action, command, _ = self.transaction_harness(tamper)
                with patch.object(self.chain, "network", return_value="columbus-5"), \
                        patch.object(self.chain, "_account", return_value=("1", "2")), \
                        patch.object(self.chain, "command", side_effect=command), \
                        patch.object(self.chain, "rebroadcast") as broadcast:
                    with self.assertRaises(rollout.RolloutError):
                        self.chain.transaction(action, lambda *_: None)
                broadcast.assert_not_called()

    def test_checkpoint_failure_prevents_broadcast_and_saved_files_contain_no_password(self):
        action, command, commands = self.transaction_harness()
        self.chain.password = "synthetic-private-passphrase"

        def checkpoint(tx_hash, path, fee):
            self.assertEqual(100, fee)
            self.assertTrue(Path(path).exists())
            raise OSError("Simulated full receipt disk")

        with patch.object(self.chain, "network", return_value="columbus-5"), \
                patch.object(self.chain, "_account", return_value=("1", "2")), \
                patch.object(self.chain, "command", side_effect=command), \
                patch.object(self.chain, "rebroadcast") as broadcast:
            with self.assertRaises(OSError):
                self.chain.transaction(action, checkpoint)
        broadcast.assert_not_called()
        self.assertEqual(2, sum(parts[:2] == ["tx", "sign"] for parts in commands))
        signs = [parts for parts in commands if parts[:2] == ["tx", "sign"]]
        self.assertEqual({"multisig1", "multisig2"}, {parts[parts.index("--from") + 1] for parts in signs})
        for parts in signs:
            self.assertIn("--offline", parts)
            self.assertEqual(GOV, parts[parts.index("--multisig") + 1])
            self.assertEqual("1", parts[parts.index("--account-number") + 1])
            self.assertEqual("2", parts[parts.index("--sequence") + 1])
        for path in self.folder.rglob("*.json"):
            self.assertNotIn(self.chain.password, path.read_text())

    def test_cli_default_plans_without_signing_or_production_broadcast(self):
        fixture = RolloutTest("test_inventory_includes_zero_rate_tokens_and_readonly_planning")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        checksum = fixture.folder / "checksums.txt"
        checksum.write_text("".join(f"{item.sha256}  {Path(item.path).name}\n" for item in fixture.artifacts.values()))
        output = io.StringIO()
        with patch.object(cli, "TerradChain", return_value=fixture.chain), \
                patch.object(sys, "stdout", output):
            self.assertEqual(0, cli.main(["--state-dir", str(self.folder), "--artifacts", str(fixture.folder)]))
        self.assertEqual([], fixture.chain.actions)
        self.assertEqual("planned", json.loads(output.getvalue())["status"])
        self.assertFalse((self.folder / "deployment.env").exists())

    def test_partial_rollout_never_emits_legacy_ids_as_trusted_route_tax_handoff(self):
        fixture = RolloutTest("test_inventory_includes_zero_rate_tokens_and_readonly_planning")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        checksum = fixture.folder / "checksums.txt"
        checksum.write_text("".join(f"{item.sha256}  {Path(item.path).name}\n" for item in fixture.artifacts.values()))
        arguments = ["--state-dir", str(self.folder), "--artifacts", str(fixture.folder)]
        with patch.object(cli, "TerradChain", return_value=fixture.chain), patch.object(sys, "stdout", io.StringIO()):
            self.assertEqual(0, cli.main(arguments))
            journal = self.folder / "journal.json"
            state = json.loads(journal.read_text())
            state["codes"]["token"] = 2500
            state["status"] = "executing"
            journal.write_text(json.dumps(state))
            fixture.chain.codes[2500] = {"contract": rollout.SPECS["token"][0], "version": "1.2.0",
                                         "sha256": fixture.artifacts["token"].sha256}
            self.assertEqual(0, cli.main(arguments))
        self.assertFalse((self.folder / "deployment.env").exists())

    def test_exclusive_state_lock_prevents_a_second_process_from_reading_or_writing_journal(self):
        fixture = RolloutTest("test_inventory_includes_zero_rate_tokens_and_readonly_planning")
        fixture.setUp()
        self.addCleanup(fixture.doCleanups)
        checksum = fixture.folder / "checksums.txt"
        checksum.write_text("".join(f"{item.sha256}  {Path(item.path).name}\n" for item in fixture.artifacts.values()))
        with (self.folder / "rollout.lock").open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with patch.object(cli, "TerradChain", return_value=fixture.chain), \
                    patch.object(sys, "stderr", io.StringIO()) as errors:
                self.assertEqual(1, cli.main(["--state-dir", str(self.folder), "--artifacts", str(fixture.folder)]))
        self.assertIn("Another rollout process", errors.getvalue())
        self.assertFalse((self.folder / "manifest.json").exists())
        self.assertEqual([], fixture.chain.actions)


class PublicProbeTest(unittest.TestCase):
    """Exercise the actual public probe with synthetic LCD responses only."""

    def setUp(self):
        self.hashes = {20: "a" * 64, 100: "b" * 64}
        self.token_router = ROUTER
        self.cap_router = ROUTER
        self.funding_version = 1

    def response(self, request, timeout):
        path = urlparse(request.full_url).path
        if path.endswith("/blocks/latest"):
            value = {"block": {"header": {"chain_id": "localterra", "height": "123", "time": "synthetic"}}}
        elif "/smart/" in path:
            address = path.split("/contract/", 1)[1].split("/", 1)[0]
            msg = json.loads(base64.b64decode(unquote(path.split("/smart/", 1)[1])))
            if address == ROUTER:
                value = {"funding_version": self.funding_version, "factory": FACTORY}
            elif "get_config" in msg:
                value = {"factory": FACTORY, "router": self.token_router}
            else:
                value = {"funding_version": self.funding_version, "router": self.cap_router,
                         "buy_bps": 100, "sell_bps": 100}
            value = {"data": value}
        elif "/contract/" in path:
            address = path.rsplit("/", 1)[1]
            value = {"contract_info": {"code_id": "20" if address == ROUTER else "100", "admin": GOV}}
        else:
            code = int(path.rsplit("/", 1)[1])
            value = {"code_info": {"data_hash": self.hashes[code]}}
        return io.BytesIO(json.dumps(value).encode())

    def run_probe(self, *extra):
        output = io.StringIO()
        with patch.object(sys, "argv", ["probe", "--router", ROUTER, "--tokens", ALPHA, *extra]), \
                patch.object(probe, "urlopen", side_effect=self.response), \
                patch.object(sys, "stdout", output):
            status = probe.main()
        return status, json.loads(output.getvalue())

    def reviewed_args(self):
        return ["--expect-funded", "--expect-reviewed-artifacts", "--reviewed-code", "20=" + "a" * 64,
                "--reviewed-code", "100=" + "b" * 64]

    def test_actual_code_id_and_data_hash_required_for_artifact_acceptance(self):
        status, result = self.run_probe(*self.reviewed_args())
        self.assertEqual(0, status)
        self.assertEqual("PASS", result["artifact_verification"])
        self.hashes[100] = "c" * 64
        status, result = self.run_probe(*self.reviewed_args())
        self.assertEqual(1, status)
        self.assertEqual("FAIL", result["artifact_verification"])
        self.assertTrue(any("data hash differs" in error for error in result["errors"]))

    def test_unreviewed_live_code_id_cannot_pass_with_matching_capability(self):
        status, result = self.run_probe("--expect-reviewed-artifacts", "--reviewed-code", "20=" + "a" * 64)
        self.assertEqual(1, status)
        self.assertTrue(any("no reviewed artifact" in error for error in result["errors"]))

    def test_capability_alone_does_not_establish_artifact_verification(self):
        status, result = self.run_probe("--expect-funded")
        self.assertEqual(0, status)
        self.assertEqual("UNVERIFIED", result["artifact_verification"])

    def test_config_binding_and_capability_both_reject_router_replacement(self):
        for field in ("token_router", "cap_router"):
            with self.subTest(field=field):
                setattr(self, field, FACTORY)
                status, result = self.run_probe("--expect-funded")
                self.assertEqual(1, status)
                self.assertTrue(any("router mismatch" in error or "capability mismatch" in error for error in result["errors"]))
                setattr(self, field, ROUTER)

    def test_boolean_funding_version_is_not_a_protocol_attestation(self):
        self.funding_version = True
        status, result = self.run_probe("--expect-funded")
        self.assertEqual(1, status)
        self.assertEqual("FAIL", result["status"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
