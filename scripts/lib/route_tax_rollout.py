#!/usr/bin/env python3
"""Existing-address contract rollout for route-local tax protocol v1.

Planning and verification never unlock keys. The transport signs exact operations
and checkpoints their transaction hash before a single broadcast.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
from dataclasses import dataclass
from pathlib import Path

from route_tax_rollout_checks import funded_capability_error, router_binding_error, router_settings, router_settlement_error


class RolloutError(RuntimeError):
    pass


SPECS = {
    "factory": ("cl8y-dex-factory", "1.11.0", "cl8y_dex_factory.wasm"),
    "pair": ("cl8y-dex-pair", "1.19.0", "cl8y_dex_pair.wasm"),
    "router": ("cl8y-dex-router", "1.3.0", "cl8y_dex_router.wasm"),
    "token": ("crates.io:cl8y-community-tax-token", "1.3.0", "cl8y_community_tax_token.wasm"),
    "launcher": ("crates.io:cl8y-community-token-launcher", "1.0.0", "cl8y_community_token_launcher.wasm"),
    "cmm": ("crates.io:treasury", "0.2.2", "treasury.wasm"),
}
DEFAULTS = {
    "chain_id": "columbus-5",
    "factory": "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea",
    "router": "terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw",
    "launcher": "terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze",
    "cmm": "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2",
    "governance": "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7",
    "alpha": "terra1x6e64es6yhauhvs3prvpdg2gkqdtfru840wgnhs935x8axr7zxkqzysuxz",
    "tax_free_code_ids": [6036, 8266, 10184],
    "max_pairs": 256, "max_tokens": 128,
    "max_total_fee_uluna": 5_000_000_000,
}


def require(condition, message):
    if not condition:
        raise RolloutError(message)


def version_tuple(value):
    require(isinstance(value, str) and re.fullmatch(r"\d+\.\d+\.\d+", value), "Unrecognized cw2 version")
    return tuple(int(part) for part in value.split("."))


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    temporary = path.with_suffix(path.suffix + ".tmp")
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    os.fchmod(fd, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)


@dataclass(frozen=True)
class Artifact:
    role: str
    path: str
    version: str
    sha256: str

    @classmethod
    def load(cls, role, path, expected_sha256, version=None):
        require(role in SPECS, "Unknown artifact role")
        binary = Path(path).read_bytes()
        checksum = hashlib.sha256(binary).hexdigest()
        require(re.fullmatch(r"[0-9a-f]{64}", expected_sha256 or ""), "A reviewed SHA-256 is required")
        require(checksum == expected_sha256, f"Unreviewed/stale {role} artifact checksum")
        require(binary.startswith(b"\0asm\x01\0\0\0"), f"Invalid {role} wasm header")
        target = version or SPECS[role][1]
        version_tuple(target)
        require(SPECS[role][0].encode() in binary and target.encode() in binary,
                f"{role} artifact lacks expected cw2 identity/version")
        return cls(role, str(Path(path).resolve()), target, checksum)

    def recheck(self):
        require(hashlib.sha256(Path(self.path).read_bytes()).hexdigest() == self.sha256,
                f"Artifact changed after planning: {self.role}")


def validate_signers(multisig, signer_keys):
    """SDK orders signatures by public-key membership, not the keyring name suffix."""
    require(multisig.get("@type") == "/cosmos.crypto.multisig.LegacyAminoPubKey", "Governance key is not legacy multisig")
    require(multisig.get("threshold") in (2, "2") and len(multisig.get("public_keys", [])) == 3,
            "Governance must be exactly 2-of-3")
    members = {digest(key) for key in multisig["public_keys"]}
    selected = [digest(key) for key in signer_keys]
    require(len(selected) == 2 and len(set(selected)) == 2 and set(selected) <= members,
            "Two distinct member signer keys are required")


class Rollout:
    def __init__(self, chain, artifacts, config, journal_path, explicit_tokens=()):
        self.chain, self.artifacts = chain, artifacts
        self.config = {**DEFAULTS, **config}
        self.journal_path = Path(journal_path)
        self.explicit_tokens = list(explicit_tokens)
        self.state = json.loads(self.journal_path.read_text()) if self.journal_path.exists() else {}

    def _save(self):
        write_json(self.journal_path, self.state)

    def _integrity(self):
        require(self.state.get("inventory_hash") == digest(self.state.get("inventory")), "Rollout inventory was modified")
        require(self.state.get("identity_hash") == digest(self.state.get("identity")), "Rollout identity was modified")
        manifest = json.loads(self.journal_path.with_name("manifest.json").read_text())
        require(manifest == {k: self.state[k] for k in ("identity", "identity_hash", "inventory", "inventory_hash")},
                "Immutable rollout manifest differs from journal")

    def _journal_proofs(self):
        total = 0
        for item in self.state["operations"].values():
            fee = item.get("fee_uluna", 0)
            require(isinstance(fee, int) and not isinstance(fee, bool) and fee >= 0, "Malformed saved fee accounting")
            total += fee
            if item.get("tx_hash"):
                require(fee > 0, "Signed transaction has no reserved fee")
                checker = getattr(self.chain, "validate_saved", None)
                if checker:
                    checker(item["action"], item)
            if item.get("receipt"):
                self._receipt_ok(item["receipt"])
                require(item["receipt"]["txhash"] == item.get("tx_hash"), "Saved receipt/hash mismatch")
                live = self.chain.receipt(item["tx_hash"])
                require(live is not None, "Saved included receipt no longer exists on this chain")
                self._receipt_ok(live)
                require(live["txhash"] == item["tx_hash"], "Live receipt/hash mismatch")
        require(total <= self.config["max_total_fee_uluna"], "Saved fee budget exceeds bound")

    @staticmethod
    def _receipt_ok(receipt):
        require(isinstance(receipt, dict) and "code" in receipt and not isinstance(receipt["code"], bool), "Receipt status is missing/malformed")
        require(str(receipt["code"]) == "0", "Rollout transaction failed on chain")
        require(isinstance(receipt.get("txhash"), str) and re.fullmatch(r"[0-9A-Fa-f]{64}", receipt["txhash"]), "Malformed transaction hash")

    def _instance(self, address, role):
        info, cw2 = self.chain.info(address), self.chain.version(address)
        require(cw2["contract"] == SPECS[role][0], f"Unexpected {role} cw2 identity at {address}")
        version_tuple(cw2["version"])
        result = {"address": address, "role": role, "code_id": int(info["code_id"]),
                  "admin": info.get("admin"), "cw2": cw2,
                  "sha256": self.chain.code_hash(int(info["code_id"])).lower()}
        if role in self.artifacts:
            artifact = self.artifacts[role]
            require(version_tuple(cw2["version"]) <= version_tuple(artifact.version), f"Refusing {role} downgrade")
            if cw2["version"] == artifact.version:
                require(result["sha256"] == artifact.sha256, f"Same-version differing {role} wasm; use a genuine version bump")
            result["needs_migrate"] = cw2["version"] != artifact.version
        else:
            result["needs_migrate"] = False
        return result

    def inventory(self):
        c = self.config
        require(self.chain.network() == c["chain_id"], "RPC/LCD chain identity mismatch")
        require(c["chain_id"] in ("columbus-5", "localterra"), "Unsupported rollout chain")
        for artifact in self.artifacts.values():
            artifact.recheck()
        require(all(role in self.artifacts for role in ("factory", "pair", "router", "token")), "Missing reviewed core artifacts")
        instances = {role: self._instance(c[role], role) for role in ("factory", "router", "launcher")}
        for item in instances.values():
            require(item["admin"] == c["governance"], f"Unexpected {item['role']} wasm admin")
        factory_config = self.chain.smart(c["factory"], {"config": {}})
        router_config = self.chain.smart(c["router"], {"config": {}})
        launcher_config = self.chain.smart(c["launcher"], {"get_config": {}})
        require(str(factory_config["governance"]) == c["governance"], "Factory governance mismatch")
        require(router_config["factory"] == launcher_config["factory"] == c["factory"], "Configured factory mismatch")
        require(launcher_config.get("router") == c["router"], "Launcher router mismatch; immediate rollout requires in-place migration")
        # Version selects an available query schema; actual wasm trust uses hashes.
        if version_tuple(instances["router"]["cw2"]["version"]) >= (1, 2, 0):
            error = funded_capability_error(router_config)
            require(error is None, error)
        if not instances["router"]["needs_migrate"]:
            error = router_settlement_error(router_config)
            require(error is None, error)
        whitelist, cursor = [], None
        for _ in range(16):
            page = self.chain.smart(c["factory"], {"get_whitelisted_code_ids": {"start_after": cursor, "limit": 30}})
            ids = page["code_ids"]
            require(len(ids) <= 30 and all(isinstance(i, int) and not isinstance(i, bool) and i > (cursor or 0) for i in ids), "Malformed whitelist inventory")
            whitelist.extend(ids)
            require(len(set(whitelist)) == len(whitelist), "Duplicate whitelist IDs")
            if page.get("next") is None:
                break
            require(ids and page["next"] == ids[-1], "Whitelist cursor stalled")
            cursor = page["next"]
        else:
            raise RolloutError("Whitelist exceeds inventory bound")
        pairs, cursor, seen = [], None, set()
        for _ in range((c["max_pairs"] + 29) // 30 + 1):
            page = self.chain.smart(c["factory"], {"pairs": {"start_after": cursor, "limit": 30}})["pairs"]
            require(len(page) <= 30, "Oversized pair inventory page")
            for pair in page:
                address = pair["contract_addr"]
                require(address not in seen, "Duplicate/stalled pair inventory")
                seen.add(address)
                require(len(seen) <= c["max_pairs"], "Pair inventory exceeds rollout bound")
                item = self._instance(address, "pair")
                require(item["admin"] == c["governance"], "Unexpected pair wasm admin")
                item.update(assets=pair["asset_infos"], pool=self.chain.smart(address, {"pool": {}}),
                            paused=self.chain.smart(address, {"is_paused": {}})["paused"])
                require(isinstance(item["paused"], bool), "Malformed pause state")
                pairs.append(item)
            if len(page) < 30:
                break
            require(page[-1]["asset_infos"] != cursor, "Pair cursor did not advance")
            cursor = page[-1]["asset_infos"]
        else:
            raise RolloutError("Unbounded pair inventory")
        require(len(pairs) == int(self.chain.smart(c["factory"], {"get_pair_count": {}})["count"]), "Pair-count inventory mismatch")
        candidates = set(self.explicit_tokens)
        if c.get("alpha"):
            candidates.add(c["alpha"])
        paired_assets = {a["token"]["contract_addr"] for pair in pairs for a in pair["assets"]}
        candidates.update(paired_assets)
        discovery_codes = {int(launcher_config["token_code_id"])}
        for address in list(candidates):
            if self.chain.version(address)["contract"] == SPECS["token"][0]:
                discovery_codes.add(int(self.chain.info(address)["code_id"]))
        for code in sorted(discovery_codes):
            candidates.update(self.chain.contracts(code, c["max_tokens"]))
        require(len(candidates) <= c["max_tokens"], "Token discovery exceeds rollout bound")
        tokens = []
        for address in sorted(candidates):
            if self.chain.version(address)["contract"] != SPECS["token"][0]:
                require(address not in self.explicit_tokens and address != c.get("alpha"), "Explicit token is not community-tax implementation")
                continue
            cfg = self.chain.smart(address, {"get_config": {}})
            if cfg["factory"] != c["factory"]:
                require(address not in paired_assets and address not in self.explicit_tokens and address != c.get("alpha"), "Explicit/paired token factory mismatch")
                continue
            # Also validate unpaired instances discovered through launcher code IDs.
            # Silently skipping these would leave obsolete settlement authority live.
            error = router_binding_error(cfg, c["router"])
            require(error is None, f"{error}: {address}")
            item = self._instance(address, "token")
            if version_tuple(item["cw2"]["version"]) >= (1, 2, 0):
                error = funded_capability_error(self.chain.smart(address, {"route_tax_capability": {}}), c["router"])
                require(error is None, f"{error}: {address}")
            require(item["admin"] in (c["governance"], c["cmm"], *c.get("eoa_admins", {}).keys()), "Unsupported token wasm admin")
            item.update(config=cfg, features=self.chain.smart(address, {"get_features": {}}),
                        supply=self.chain.smart(address, {"token_info": {}})["total_supply"])
            tokens.append(item)
        require(tokens, "No managed community-tax instances discovered")
        if any(t["admin"] == c["cmm"] and t["needs_migrate"] for t in tokens):
            item = self._instance(c["cmm"], "cmm")
            require(item["admin"] == c["governance"], "CMM wasm admin mismatch")
            require(self.chain.smart(c["cmm"], {"config": {}})["governance"] == c["governance"], "CMM governance mismatch")
            require(version_tuple(item["cw2"]["version"]) >= (0, 2, 2) or "cmm" in self.artifacts,
                    "CMM requires reviewed treasury >=0.2.2 before owned-contract migration")
            instances["cmm"] = item
        affected = [p["address"] for p in pairs if instances["factory"]["needs_migrate"] or instances["router"]["needs_migrate"] or p["needs_migrate"] or
                    any(a["token"]["contract_addr"] in {t["address"] for t in tokens} for a in p["assets"])]
        return {"instances": instances, "factory_config": factory_config, "router_config": router_config,
                "launcher_config": launcher_config, "whitelist": whitelist,
                "pairs": pairs, "tokens": tokens, "affected_pairs": affected}

    def prepare(self):
        identity = {"config": self.config, "artifacts": {k: vars(v) for k, v in sorted(self.artifacts.items())},
                    "explicit_tokens": sorted(self.explicit_tokens)}
        identity_hash = digest(identity)
        if self.state:
            self._integrity()
            require(self.state["identity_hash"] == identity_hash, "Changed rollout inputs; use a separate journal")
            require(self.chain.network() == self.config["chain_id"], "Resume chain mismatch")
            for artifact in self.artifacts.values():
                artifact.recheck()
            self._journal_proofs()
            return self.state
        inventory = self.inventory()
        self.state = {"identity_hash": identity_hash, "identity": identity, "inventory": inventory,
                      "inventory_hash": digest(inventory),
                      "operations": {}, "codes": {}, "status": "planned"}
        write_json(self.journal_path.with_name("manifest.json"),
                   {k: self.state[k] for k in ("identity", "identity_hash", "inventory", "inventory_hash")})
        self._save()
        return self.state

    def _operation(self, name, action, rebroadcast=False):
        self._integrity()
        operations = self.state["operations"]
        item = operations.get(name)
        if item:
            require(item["action"] == action, "Journal action changed")
            if item.get("receipt"):
                self._receipt_ok(item["receipt"])
                require(item["receipt"]["txhash"] == item.get("tx_hash"), "Saved receipt/hash mismatch")
                return item["receipt"]
            if item.get("tx_hash"):
                receipt = self.chain.receipt(item["tx_hash"])
                if receipt is None:
                    require(rebroadcast, f"Pending transaction {item['tx_hash']}; reconcile it or explicitly rebroadcast the same signed bytes")
                    receipt = self.chain.rebroadcast(item["signed_path"], item["tx_hash"])
                self._receipt_ok(receipt)
                require(receipt["txhash"] == item["tx_hash"], "Pending receipt hash mismatch")
                item["receipt"] = receipt
                self._save()
                return receipt
        item = operations.setdefault(name, {"action": action})
        self._save()

        def signed(tx_hash, signed_path, fee):
            require(isinstance(fee, int) and not isinstance(fee, bool) and fee > 0, "Malformed transaction fee")
            require(re.fullmatch(r"[0-9A-Fa-f]{64}", tx_hash), "Malformed signed transaction hash")
            require(all(isinstance(op.get("fee_uluna", 0), int) and not isinstance(op.get("fee_uluna", 0), bool)
                        and op.get("fee_uluna", 0) >= 0 for op in operations.values()), "Malformed saved fee accounting")
            total = sum(int(op.get("fee_uluna", 0)) for op in operations.values())
            require(total + fee <= self.config["max_total_fee_uluna"], "Total rollout fee budget exceeded")
            item.update(tx_hash=tx_hash, signed_path=signed_path, fee_uluna=fee)
            self._save()

        self._live_bindings()
        receipt = self.chain.transaction(action, signed)
        self._receipt_ok(receipt)
        require(item.get("tx_hash") == receipt["txhash"], "Transport receipt differs from signed transaction")
        item["receipt"] = receipt
        self._save()
        return receipt

    def _store(self, role, rebroadcast):
        artifact = self.artifacts[role]
        artifact.recheck()
        if role not in self.state["codes"]:
            receipt = self._operation("store:" + role, {"op": "store", "role": "store", "path": artifact.path}, rebroadcast)
            self.state["codes"][role] = int(receipt["code_id"])
            self._save()
        code = self.state["codes"][role]
        require(self.chain.code_hash(code).lower() == artifact.sha256, f"Stored {role} checksum differs")
        return code

    def _code(self, role, candidates, rebroadcast):
        if role not in self.state["codes"]:
            for code in sorted(set(int(i) for i in candidates)):
                if self.chain.code_hash(code).lower() == self.artifacts[role].sha256:
                    self.state["codes"][role] = code
                    self._save()
                    break
        return self._store(role, rebroadcast)

    def _live_bindings(self):
        """Reject a changed authority or unrelated governance update before spending."""
        inv, c = self.state["inventory"], self.config
        for item in [*inv["instances"].values(), *inv["pairs"], *inv["tokens"]]:
            current = self.chain.info(item["address"])
            require(current.get("admin") == item["admin"], "Wasm admin changed after planning")
            allowed = {item["code_id"]}
            if item["role"] in self.state["codes"]:
                allowed.add(self.state["codes"][item["role"]])
            require(int(current["code_id"]) in allowed, "Unexpected code changed after planning")
            live_version = self.chain.version(item["address"])
            allowed_version = item["cw2"]
            if int(current["code_id"]) == item["code_id"]:
                require(self.chain.code_hash(item["code_id"]).lower() == item["sha256"],
                        "Planned code checksum changed after planning")
            if int(current["code_id"]) != item["code_id"]:
                role = item["role"]
                require(role in self.artifacts and self.chain.code_hash(int(current["code_id"])).lower() == self.artifacts[role].sha256,
                        "Changed code is not the reviewed target")
                allowed_version = {"contract": SPECS[role][0], "version": self.artifacts[role].version}
            require(live_version == allowed_version, "Contract cw2 identity changed after planning")
        fc = self.chain.smart(c["factory"], {"config": {}})
        expected = dict(inv["factory_config"])
        if self.state["operations"].get("factory:pair-template", {}).get("tx_hash"):
            expected["pair_code_id"] = self.state["codes"]["pair"]
        require(fc == expected, "Factory settings changed after planning")
        rc = self.chain.smart(c["router"], {"config": {}})
        require(router_settings(rc) == router_settings(inv["router_config"]), "Router settings changed after planning")
        router_version = self.chain.version(c["router"])["version"]
        if version_tuple(router_version) >= (1, 2, 0):
            error = funded_capability_error(rc)
            require(error is None, error)
        if version_tuple(router_version) >= (1, 3, 0):
            error = router_settlement_error(rc)
            require(error is None, error)
        lc = self.chain.smart(c["launcher"], {"get_config": {}})
        expected = dict(inv["launcher_config"])
        if self.state["operations"].get("launcher:template", {}).get("tx_hash"):
            expected["token_code_id"] = self.state["codes"]["token"]
        require(lc == expected, "Launcher settings changed after planning")
        if "cmm" in inv["instances"]:
            require(self.chain.smart(c["cmm"], {"config": {}})["governance"] == c["governance"],
                    "CMM governance changed after planning")
        for token in inv["tokens"]:
            require(self.chain.smart(token["address"], {"get_config": {}}) == token["config"], "Token settings changed after planning")
            require(self.chain.smart(token["address"], {"get_features": {}}) == token["features"], "Token features changed after planning")
            if version_tuple(self.chain.version(token["address"])["version"]) >= (1, 2, 0):
                error = funded_capability_error(self.chain.smart(token["address"], {"route_tax_capability": {}}), c["router"])
                require(error is None, error)
        for pair in inv["pairs"]:
            if pair["address"] not in inv["affected_pairs"]:
                continue
            original = pair["paused"]
            paused = self.state["operations"].get("pause:" + pair["address"], {})
            restored = self.state["operations"].get("restore:" + pair["address"], {})
            if self.state["status"] == "complete" or restored.get("receipt"):
                expected_pause = original
            elif paused.get("receipt"):
                expected_pause = True
            else:
                expected_pause = original
            require(self.chain.smart(pair["address"], {"is_paused": {}})["paused"] is expected_pause,
                    "Affected pair pause changed outside rollout")

    def _execute_msg(self, name, address, msg, rebroadcast=False):
        return self._operation(name, {"op": "execute", "role": "gov", "contract": address, "msg": msg}, rebroadcast)

    def execute(self, rebroadcast_pending=False):
        self.prepare()
        require(self.state.get("status") != "complete", "Completed rollout cannot execute again; use verify or a new manifest")
        c, inv = self.config, self.state["inventory"]
        self.state["status"] = "executing"
        self._save()
        for pair in inv["pairs"]:
            if pair["address"] in inv["affected_pairs"]:
                self._execute_msg("pause:" + pair["address"], c["factory"],
                                  {"set_pair_paused": {"pair": pair["address"], "paused": True}}, rebroadcast_pending)
        for role in ("factory", "router", "launcher", "cmm"):
            item = inv["instances"].get(role)
            if not item:
                continue
            if item["needs_migrate"]:
                code = self._store(role, rebroadcast_pending)
                self._operation("migrate:" + role, {"op": "migrate", "role": "gov", "contract": item["address"],
                    "code_id": code, "msg": {}}, rebroadcast_pending)
            else:
                self.state["codes"][role] = item["code_id"]
                self._save()
        token_code = self._code("token", [inv["launcher_config"]["token_code_id"],
            *[t["code_id"] for t in inv["tokens"]]], rebroadcast_pending)
        require(self.chain.code_hash(token_code).lower() == self.artifacts["token"].sha256, "Token template checksum mismatch")
        self.state["codes"]["token"] = token_code
        self._save()
        listed = self.chain.smart(c["factory"], {"is_code_id_whitelisted": {"code_id": token_code}})["whitelisted"]
        if not listed:
            self._execute_msg("whitelist:token", c["factory"], {"add_whitelisted_code_id": {"code_id": token_code}}, rebroadcast_pending)
        for token in inv["tokens"]:
            if not token["needs_migrate"]:
                continue
            if token["admin"] == c["cmm"]:
                self._execute_msg("migrate:token:" + token["address"], c["cmm"],
                    {"migrate_owned_contract": {"contract": token["address"], "new_code_id": token_code}}, rebroadcast_pending)
            else:
                role = "gov" if token["admin"] == c["governance"] else c["eoa_admins"][token["admin"]]
                self._operation("migrate:token:" + token["address"], {"op": "migrate", "role": role,
                    "contract": token["address"], "code_id": token_code, "msg": {}}, rebroadcast_pending)
        self._execute_msg("launcher:template", c["launcher"], {"update_config": {"token_code_id": token_code}}, rebroadcast_pending)
        pair_code = self._code("pair", [inv["factory_config"]["pair_code_id"],
            *[p["code_id"] for p in inv["pairs"]]], rebroadcast_pending)
        if int(inv["factory_config"]["pair_code_id"]) != pair_code:
            self._execute_msg("factory:pair-template", c["factory"], {"update_config": {"pair_code_id": pair_code}}, rebroadcast_pending)
        for pair in inv["pairs"]:
            if pair["needs_migrate"]:
                self._operation("migrate:pair:" + pair["address"], {"op": "migrate", "role": "gov",
                    "contract": pair["address"], "code_id": pair_code, "msg": {}}, rebroadcast_pending)
        for pair in inv["affected_pairs"]:
            self._execute_msg("refresh:" + pair, c["factory"], {"refresh_pair_asset_code_ids": {"pair": pair}}, rebroadcast_pending)
        self.verify()
        self.state["status"] = "contracts_ready_paused"
        self._save()
        return self.state

    def verify(self):
        self._integrity()
        self._journal_proofs()
        require(self.state and self.chain.network() == self.config["chain_id"], "Missing/mismatched rollout journal")
        self._live_bindings()
        c, inv = self.config, self.state["inventory"]
        for role, item in inv["instances"].items():
            if role in self.artifacts:
                actual = self._instance(item["address"], role)
                require(not actual["needs_migrate"], f"{role} migration incomplete")
        router_config = self.chain.smart(c["router"], {"config": {}})
        require(router_settings(router_config) == router_settings(inv["router_config"]), "Router configuration changed")
        error = funded_capability_error(router_config) or router_settlement_error(router_config)
        require(error is None, error)
        launcher = self.chain.smart(c["launcher"], {"get_config": {}})
        require(launcher == {**inv["launcher_config"], "token_code_id": self.state["codes"]["token"]}, "Launcher configuration changed unexpectedly")
        for token in inv["tokens"]:
            actual = self._instance(token["address"], "token")
            require(not actual["needs_migrate"], "Token migration incomplete")
            require(actual["admin"] == token["admin"], "Token wasm admin changed")
            require(self.chain.smart(c["factory"], {"is_code_id_whitelisted": {"code_id": actual["code_id"]}})["whitelisted"] is True, "Migrated token not whitelisted")
            require(self.chain.smart(token["address"], {"get_config": {}}) == token["config"], "Token settings changed during migration")
            require(self.chain.smart(token["address"], {"get_features": {}}) == token["features"], "Token features changed")
            if self.state["status"] != "complete":
                require(self.chain.smart(token["address"], {"token_info": {}})["total_supply"] == token["supply"],
                        "Token supply changed; reconcile concurrent activity before cutover")
            cap = self.chain.smart(token["address"], {"route_tax_capability": {}})
            error = funded_capability_error(cap, c["router"])
            require(error is None, error)
            require(cap["buy_bps"] == token["config"]["buy_bps"] and cap["sell_bps"] == token["config"]["sell_bps"], "Tax capability rates changed")
            for pair in inv["pairs"]:
                if not any(asset["token"]["contract_addr"] == token["address"] for asset in pair["assets"]):
                    continue
                for kind in ("sell", "buy"):
                    preview = self.chain.smart(token["address"], {"route_tax_preview": {
                        "kind": kind, "amount": "1010000", "trader": None, "pair": pair["address"]}})
                    gross, economic, tax, net = (int(preview[key]) for key in ("gross", "economic", "tax", "net"))
                    require(preview["funding_version"] == 1 and gross == 1_010_000
                            and min(economic, tax, net) >= 0, "Malformed funded tax preview")
                    bps = int(cap[kind + "_bps"])
                    if kind == "sell":
                        require(tax == economic * bps // 10_000 and net == economic and economic + tax <= gross
                                and (economic == gross or economic + 1 + (economic + 1) * bps // 10_000 > gross),
                                "Sell preview is not the maximal funded economic input")
                    else:
                        require(economic == gross and tax == gross * bps // 10_000 and net + tax == gross,
                                "Buy preview does not reconcile gross/net/tax")
        require(int(self.chain.smart(c["factory"], {"config": {}})["pair_code_id"]) == self.state["codes"]["pair"], "Factory pair template not rotated")
        for old in inv["whitelist"]:
            require(self.chain.smart(c["factory"], {"is_code_id_whitelisted": {"code_id": old}})["whitelisted"] is True, "Old whitelist entry was removed")
        for pair in inv["pairs"]:
            actual = self._instance(pair["address"], "pair")
            require(not actual["needs_migrate"] and actual["admin"] == pair["admin"], "Pair code/admin verification failed")
            pins = self.chain.smart(pair["address"], {"get_asset_code_ids": {}})["code_ids"]
            live = [int(self.chain.info(asset["token"]["contract_addr"])["code_id"]) for asset in pair["assets"]]
            require(pins == live, "Pair asset code pins are stale")
            for code in live:
                require(self.chain.smart(c["factory"], {"is_code_id_whitelisted": {"code_id": code}})["whitelisted"] is True, "Pair asset is no longer whitelisted")
            if pair["address"] in inv["affected_pairs"]:
                if self.state["status"] != "complete":
                    require(self.chain.smart(pair["address"], {"pool": {}}) == pair["pool"], "Affected pool liquidity changed during rollout")
                restored = self.state["operations"].get("restore:" + pair["address"], {}).get("receipt")
                expected_pause = pair["paused"] if self.state["status"] == "complete" or restored else True
                require(self.chain.smart(pair["address"], {"is_paused": {}})["paused"] is expected_pause, "Pair pause state drifted")
        return {"contract_verification": "PASS", "production_execution": "UNVERIFIED"}

    def finalize(self, rebroadcast_pending=False):
        require(self.state.get("status") in ("contracts_ready_paused", "complete"), "Contracts not ready for application cutover")
        self.verify()
        for pair in self.state["inventory"]["pairs"]:
            if pair["address"] in self.state["inventory"]["affected_pairs"]:
                self._execute_msg("restore:" + pair["address"], self.config["factory"],
                    {"set_pair_paused": {"pair": pair["address"], "paused": pair["paused"]}}, rebroadcast_pending)
        self.state["status"] = "complete"
        self._save()
        return self.state
