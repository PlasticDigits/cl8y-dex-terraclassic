#!/usr/bin/env python3
"""Audit and amend only unsigned, unstored artifacts in a contained rollout.

check is read-only. apply archives the exact prior bytes and uses a recoverable
two-file transaction; it never signs, broadcasts, unlocks or restores pauses.
"""
from __future__ import annotations

import argparse
import base64
import copy
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from types import SimpleNamespace
from urllib.error import HTTPError, URLError
from urllib.parse import quote
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts/lib"))
from route_tax_rollout import Artifact, Rollout, SPECS, digest, require, validate_pair_membership
from route_tax_rollout_cli import TerradChain

PENDING = "artifact-amendment-pending.json"
LINKS = "artifact_amendments"
CHECKPOINT = "preservation_checkpoint"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, indent=2) + "\n").encode()


def sync_directory(path):
    directory = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def read(path):
    path = Path(path)
    require(path.is_file() and not path.is_symlink(), "Expected a regular amendment/evidence file")
    return path.read_bytes()


def atomic(path, data):
    """Replace one file durably. The transaction log protects the file pair."""
    path = Path(path)
    require(not path.is_symlink(), "Refusing a symlink amendment destination")
    fd, temporary = tempfile.mkstemp(prefix=".artifact-amendment-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as stream:
            os.fchmod(stream.fileno(), 0o600)
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        sync_directory(path.parent)
    finally:
        Path(temporary).unlink(missing_ok=True)


def immutable(path, data):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def canonical_transaction(tx):
    """Normalize only SDK defaults/number rendering, never contract JSON."""
    body = tx["body"]
    require(set(body) <= {"messages", "memo", "timeout_height", "extension_options", "non_critical_extension_options", "unordered", "timeout_timestamp"},
            "Unknown saved transaction body field")
    require(body.get("unordered", False) is False and body.get("timeout_timestamp") is None,
            "Unexpected unordered/timestamp transaction semantics")
    messages = copy.deepcopy(body["messages"])
    for msg in messages:
        if "funds" not in msg and msg["@type"].endswith("MsgExecuteContract"):
            msg["funds"] = []
        if isinstance(msg.get("msg"), str):
            msg["msg"] = json.loads(base64.b64decode(msg["msg"], validate=True))
        if "code_id" in msg:
            msg["code_id"] = str(int(msg["code_id"]))
    def sdk_numbers(value):
        if isinstance(value, list):
            return [sdk_numbers(v) for v in value]
        if isinstance(value, dict):
            return {k: str(int(v)) if k in {"sequence", "threshold", "extra_bits_stored", "gas_limit", "amount"}
                    and isinstance(v, (str, int)) else sdk_numbers(v) for k, v in value.items()}
        return value
    auth = copy.deepcopy(tx["auth_info"])
    require(set(auth) <= {"signer_infos", "fee", "tip"} and not auth.get("tip"), "Unexpected transaction tip/auth field")
    auth.pop("tip", None)
    auth["fee"].setdefault("payer", "")
    auth["fee"].setdefault("granter", "")
    return {"body": {"messages": messages, "memo": body.get("memo", ""),
                     "timeout_height": str(int(body.get("timeout_height", 0))),
                     "extension_options": body.get("extension_options", []),
                     "non_critical_extension_options": body.get("non_critical_extension_options", [])},
            "auth_info": sdk_numbers(auth), "signatures": tx["signatures"]}


class ReadOnlyChain:
    """Expose only HTTP reads; no terrad command, keyring or process env access."""
    METHODS = {"network", "info", "version", "smart", "code_hash", "receipt", "get"}

    def __init__(self, state_dir, config):
        transport = config["transport"]
        self.config = config
        self.client = TerradChain(SimpleNamespace(state_dir=str(state_dir),
            node=transport["node"], lcd=transport["lcd"], store_key=transport["store_key"],
            multisig_key=transport["multisig_key"], admin_key=[]))
        # Included transaction bytes and uploaded code bytes are immutable.
        # Reuse only these within this check; pools, pauses, config, membership,
        # supply and contract code/admin bindings are always queried fresh.
        underlying_get = self.client.get
        cache = {}
        def cached_get(path, rpc=False):
            immutable_read = re.fullmatch(r"/cosmos/tx/v1beta1/txs/[0-9A-Fa-f]{64}|/tx\?hash=0x[0-9A-Fa-f]{64}|/cosmwasm/wasm/v1/code/[0-9]+", path)
            key = (rpc, path)
            if immutable_read and key in cache:
                return copy.deepcopy(cache[key])
            result = underlying_get(path, rpc=rpc)
            if immutable_read:
                cache[key] = copy.deepcopy(result)
            return result
        self.client.get = cached_get

    def __getattr__(self, name):
        if name in self.METHODS:
            return getattr(self.client, name)
        raise AttributeError(name)

    def validate_saved(self, action, entry):
        saved = json.loads(read(entry["signed_path"]))
        live = self.get("/cosmos/tx/v1beta1/txs/" + entry["tx_hash"])
        rpc = self.get("/tx?hash=0x" + entry["tx_hash"], rpc=True)["result"]
        require(sha(base64.b64decode(rpc["tx"], validate=True)).upper() == entry["tx_hash"].upper(),
                "RPC transaction bytes differ from the saved hash")
        require(canonical_transaction(saved) == canonical_transaction(live["tx"]),
                "Saved signed transaction differs from included transaction")
        messages = canonical_transaction(saved)["body"]["messages"]
        require(len(messages) == 1, "Saved action has multiple messages")
        msg = messages[0]
        sender = self.config["governance"] if action["role"] == "gov" else (
            self.config["transport"]["store_address"] if action["role"] == "store" else
            next((a for a, key in self.config.get("eoa_admins", {}).items() if key == action["role"]), None))
        types = {"store": "MsgStoreCode", "migrate": "MsgMigrateContract", "execute": "MsgExecuteContract"}
        require(msg["@type"] == "/cosmwasm.wasm.v1." + types[action["op"]]
                and msg["sender"] == sender and not msg.get("funds"), "Included action/sender differs")
        if action["op"] == "store":
            binary = base64.b64decode(msg["wasm_byte_code"], validate=True)
            if binary.startswith(b"\x1f\x8b"):
                binary = gzip.decompress(binary)
            require(binary == read(action["path"]), "Previously signed store artifact changed")
        else:
            require(msg["contract"] == action["contract"] and msg["msg"] == action["msg"], "Saved contract action differs")
            if action["op"] == "migrate":
                require(int(msg["code_id"]) == action["code_id"], "Saved migration code differs")
        fee = saved["auth_info"]["fee"]
        require(0 < int(fee["gas_limit"]) <= int(self.config["transport"]["max_gas"])
                and len(fee["amount"]) == 1 and fee["amount"][0]["denom"] == "uluna"
                and int(fee["amount"][0]["amount"]) == entry["fee_uluna"], "Saved fee/gas differs")

    def historical_pool(self, address, height):
        height = str(int(height))
        query = quote(base64.b64encode(b'{"pool":{}}').decode(), safe="")
        request = Request(self.client.args.lcd.rstrip("/") + "/cosmwasm/wasm/v1/contract/" + address + "/smart/" + query,
                          headers={"x-cosmos-block-height": height, "User-Agent": "CL8Y-rollout-amendment/1"})
        try:
            with urlopen(request, timeout=15) as response:
                observed = response.headers.get("x-cosmos-block-height")
                if observed != height:
                    return {"status": "unavailable", "requested_height": height, "returned_height": observed,
                            "reason": "Provider did not attest the exact requested response height"}
                value = json.load(response)["data"]
                pool = json.loads(base64.b64decode(value, validate=True)) if isinstance(value, str) else value
                return {"status": "available", "requested_height": height, "returned_height": observed, "pool": pool}
        except (HTTPError, URLError, TimeoutError, ValueError, KeyError) as error:
            reason = "HTTP " + str(error.code) if isinstance(error, HTTPError) else type(error).__name__
            return {"status": "unavailable", "requested_height": height, "returned_height": None,
                    "reason": "Historical state unavailable: " + reason}


def integrity(journal, manifest):
    require(journal["identity_hash"] == digest(journal["identity"]), "Journal identity changed")
    require(journal["inventory_hash"] == digest(journal["inventory"]), "Original inventory changed")
    require(manifest == {k: journal[k] for k in ("identity", "identity_hash", "inventory", "inventory_hash")},
            "Manifest and journal differ; recover the staged amendment first")


def artifacts(identity):
    for value in identity["artifacts"].values():
        read(value["path"])
    return {role: Artifact.load(role, value["path"], value["sha256"], value["version"])
            for role, value in identity["artifacts"].items()}


def validate_live(state_dir, journal, after, chain, preservation=None, check_balances=True):
    require(chain.network() == journal["identity"]["config"]["chain_id"], "Amendment chain mismatch")
    engine = Rollout(chain, artifacts(after), journal["identity"]["config"], state_dir / "journal.json")
    engine.state = journal
    engine._journal_proofs()
    engine._live_bindings()
    for entry in journal["operations"].values():
        if entry.get("receipt") and entry["action"]["op"] == "migrate":
            action = entry["action"]
            require(int(chain.info(action["contract"])["code_id"]) == action["code_id"], "Completed migration drifted")
    if not check_balances:
        return
    preservation = preservation or journal.get(CHECKPOINT, {})
    for pair in journal["inventory"]["pairs"]:
        if pair["address"] in journal["inventory"]["affected_pairs"]:
            expected = preservation.get("pairs", {}).get(pair["address"], pair["pool"])
            require(chain.smart(pair["address"], {"pool": {}}) == expected, "Contained pool changed")
    for token in journal["inventory"]["tokens"]:
        expected = preservation.get("token_supplies", {}).get(token["address"], token["supply"])
        require(chain.smart(token["address"], {"token_info": {}})["total_supply"] == expected, "Token supply changed")


def capture_checkpoint(journal, chain):
    """Freeze existing legacy economic state, without rewriting planned inventory."""
    require(journal["status"] == "executing" and CHECKPOINT not in journal,
            "Capture one checkpoint only before pair/token migration")
    for name, entry in journal["operations"].items():
        require(not (name.startswith(("migrate:pair:", "migrate:token:")) and
                     any(k in entry for k in ("tx_hash", "signed_path", "receipt", "fee_uluna"))),
                "Pair/token migration already signed; cannot capture a new baseline")
    for role in ("factory", "router"):
        entry = journal["operations"].get("migrate:" + role, {})
        require(entry.get("receipt") and entry.get("tx_hash"), "Prerequisite factory/router migration is incomplete")
    inv = journal["inventory"]
    config = journal["identity"]["config"]
    validate_pair_membership(chain, config, inv)
    checkpoint = {"schema": 1, "inventory_hash": journal["inventory_hash"], "pairs": {}, "token_supplies": {},
                  "proofs": {"pairs": {}, "tokens": {}},
                  "qualification": "Current paused legacy state before pair/token migration; historical attribution only where the exact response height is verified"}
    for pair in inv["pairs"]:
        address = pair["address"]
        require(int(chain.info(address)["code_id"]) == pair["code_id"]
                and chain.code_hash(pair["code_id"]).lower() == pair["sha256"], "Pair no longer uses original legacy code")
        if address not in inv["affected_pairs"]:
            continue
        require(chain.smart(address, {"is_paused": {}})["paused"] is True, "Checkpoint pair is not contained")
        pause = journal["operations"].get("pause:" + address, {})
        require(pause.get("receipt") and str(pause["receipt"].get("code")) == "0", "Included pause receipt missing")
        height = str(int(pause["receipt"]["height"]))
        historical = chain.historical_pool(address, height)
        current = chain.smart(address, {"pool": {}})
        if historical.get("status") == "available":
            require(historical.get("requested_height") == historical.get("returned_height") == height,
                    "Historical height differs from pause receipt")
            require(historical["pool"] == current, "Pool changed after the historical pause-block snapshot")
        else:
            require(historical.get("status") == "unavailable", "Malformed historical query result")
        checkpoint["pairs"][address] = current
        checkpoint["proofs"]["pairs"][address] = {"original_code_id": pair["code_id"], "original_sha256": pair["sha256"],
            "paused": True, "pause_tx_hash": pause["tx_hash"], "pause_height": height, "historical": historical,
            "differs_from_original_inventory": current != pair["pool"]}
    for token in inv["tokens"]:
        address = token["address"]
        require(int(chain.info(address)["code_id"]) == token["code_id"]
                and chain.code_hash(token["code_id"]).lower() == token["sha256"], "Token no longer uses original legacy code")
        checkpoint["token_supplies"][address] = chain.smart(address, {"token_info": {}})["total_supply"]
        checkpoint["proofs"]["tokens"][address] = {"original_code_id": token["code_id"], "original_sha256": token["sha256"],
            "differs_from_original_inventory": checkpoint["token_supplies"][address] != token["supply"]}
    return checkpoint


def evidence(path, replacements):
    data = read(path)
    record = json.loads(data)
    require(record.get("status") == "PASS" and record.get("business_source_unchanged") is True,
            "Reviewed successful artifact evidence with unchanged business source is required")
    for role, checksum in replacements.items():
        require(record.get("artifacts", {}).get(role, {}).get("sha256") == checksum,
                "Artifact evidence does not bind the proposed checksum")
    return {"path": str(Path(path).resolve()), "sha256": sha(data)}


def eligible(journal, replacements):
    require(journal["status"] in ("planned", "executing"), "Amend only an unfinished contained rollout")
    require(replacements and set(replacements) <= set(journal["identity"]["artifacts"]), "Unknown/empty artifact roles")
    for role, checksum in replacements.items():
        require(re.fullmatch(r"[0-9a-f]{64}", checksum or ""), "Malformed replacement checksum")
        require(role not in journal["codes"], "Cannot amend an already stored/reused artifact role")
    for name, entry in journal["operations"].items():
        if entry.get("tx_hash"):
            require(entry.get("receipt"), "Reconcile every signed pending transaction before amendment")
        elif entry.get("receipt") or any(k in entry for k in ("signed_path", "fee_uluna")):
            require(False, "Malformed or partially signed operation prevents amendment")
        else:
            require(set(entry) == {"action"} and name.startswith("store:"), "Only an unsigned store action may remain pending")
        for role in replacements:
            if name == "store:" + role:
                require(set(entry) == {"action"} and entry["action"] == {
                    "op": "store", "role": "store", "path": journal["identity"]["artifacts"][role]["path"]},
                    "Cannot change a signed or altered store operation")
            elif name.startswith("migrate:" + role) or (entry["action"].get("path") == journal["identity"]["artifacts"][role]["path"]):
                require(False, "Artifact already participates in another operation")


def frozen_files(journal):
    return {entry["signed_path"]: sha(read(entry["signed_path"]))
            for entry in journal["operations"].values() if entry.get("signed_path")}


def reject_orphan_signatures(state_dir, journal):
    included_folders = {Path(e["signed_path"]).resolve().parent for e in journal["operations"].values() if e.get("signed_path")}
    transactions = state_dir / "transactions"
    if transactions.exists():
        require(not transactions.is_symlink(), "Symlink transaction evidence directory")
        for path in transactions.rglob("*.json"):
            if path.name == "signed.json" or path.name.startswith("signature"):
                require(path.resolve().parent in included_folders,
                        "Uncheckpointed signed/component transaction must be reconciled before amendment")


def verify_frozen(receipt):
    require(sha(read(receipt["artifact_evidence"]["path"])) == receipt["artifact_evidence"]["sha256"], "Artifact evidence changed")
    for path, checksum in receipt["preserved_signed_files"].items():
        require(sha(read(path)) == checksum, "Prior signed evidence changed")


def archive_path(state_dir, name):
    require(re.fullmatch(r"[0-9a-f]{64}", name or ""), "Invalid amendment archive reference")
    parent = state_dir / "artifact-amendments"
    folder = parent / name
    require(not parent.is_symlink() and not folder.is_symlink(), "Symlink amendment archive")
    return folder


def recover(state_dir, chain, expected_identity, fault=None):
    pending = json.loads(read(state_dir / PENDING))
    name = pending["archive"]
    folder = archive_path(state_dir, name)
    transaction = json.loads(read(folder / "transaction.json"))
    require(sha(read(folder / "transaction.json")) == pending["transaction_sha256"], "Amendment transaction record changed")
    receipt_bytes = read(folder / "receipt.json")
    require(sha(receipt_bytes) == transaction["receipt_sha256"] == name, "Amendment receipt changed")
    receipt = json.loads(receipt_bytes)
    require(receipt["prior_identity_hash"] == expected_identity, "Recovery belongs to another intended identity")
    images = {}
    for filename, hashes in transaction["files"].items():
        require(filename in ("manifest.json", "journal.json"), "Unknown amendment destination")
        images[filename] = {}
        for phase in ("before", "after"):
            data = read(folder / (phase + "-" + filename))
            require(sha(data) == hashes[phase], "Amendment byte image changed")
            images[filename][phase] = data
        require(sha(read(state_dir / filename)) in hashes.values(), "Current rollout file changed outside the staged amendment")
    require(set(images) == {"manifest.json", "journal.json"}, "Incomplete amendment file pair")
    before, after = json.loads(images["journal.json"]["before"]), json.loads(images["journal.json"]["after"])
    integrity(before, json.loads(images["manifest.json"]["before"]))
    integrity(after, json.loads(images["manifest.json"]["after"]))
    require(before["identity_hash"] == receipt["prior_identity_hash"] and after["identity_hash"] == receipt["next_identity_hash"], "Archive identities differ")
    eligible(before, receipt["replacements"])
    expected_after = copy.deepcopy(before)
    for role, checksum in receipt["replacements"].items():
        expected_after["identity"]["artifacts"][role]["sha256"] = checksum
    expected_after["identity_hash"] = digest(expected_after["identity"])
    preservation = receipt.get(CHECKPOINT)
    if preservation is not None:
        require(preservation["inventory_hash"] == before["inventory_hash"], "Checkpoint original inventory changed")
        expected_after[CHECKPOINT] = preservation
    expected_after.setdefault(LINKS, []).append({"receipt": "artifact-amendments/" + name + "/receipt.json", "sha256": name})
    require(after == expected_after, "Staged amendment changes more than the approved artifact hashes/link")
    require(sha(images["manifest.json"]["before"]) == receipt["original_manifest_sha256"]
            and sha(images["journal.json"]["before"]) == receipt["original_journal_sha256"]
            and sha(images["manifest.json"]["after"]) == receipt["next_manifest_sha256"], "Archived original/current bytes differ from receipt")
    require(before["inventory"] == after["inventory"] and before["operations"] == after["operations"]
            and before["codes"] == after["codes"] and before["status"] == after["status"], "Prior rollout evidence changed")
    verify_frozen(receipt)
    reject_orphan_signatures(state_dir, before)
    validate_live(state_dir, before, after["identity"], chain, preservation)
    for filename in ("manifest.json", "journal.json"):
        atomic(state_dir / filename, images[filename]["after"])
        if fault:
            fault(filename + "-replaced")
    integrity(json.loads(read(state_dir / "journal.json")), json.loads(read(state_dir / "manifest.json")))
    atomic(folder / "completed.json", encoded({"identity_hash": after["identity_hash"], "transaction_sha256": pending["transaction_sha256"]}))
    (state_dir / PENDING).unlink()
    sync_directory(state_dir)
    return {"status": "amended", "identity_hash": after["identity_hash"], "receipt": str(folder / "receipt.json")}


def amend(state_dir, replacements, evidence_path, expected_identity, reason, chain=None, apply=False, fault=None,
          capture_preservation=False):
    state_dir = Path(state_dir).resolve()
    require(not state_dir.is_relative_to(ROOT), "Amendment evidence must remain outside git")
    require(reason.strip() and len(reason) <= 1000, "A concise amendment reason is required")
    require(re.fullmatch(r"[0-9a-f]{64}", expected_identity or ""), "Expected original identity is required")
    journal_bytes, manifest_bytes = read(state_dir / "journal.json"), read(state_dir / "manifest.json")
    journal = json.loads(journal_bytes)
    chain = chain or ReadOnlyChain(state_dir, journal["identity"]["config"])
    review = evidence(evidence_path, replacements)
    if (state_dir / PENDING).exists():
        require(apply, "Interrupted amendment is staged; use apply to recover its file pair")
        pending = json.loads(read(state_dir / PENDING))
        folder = archive_path(state_dir, pending["archive"])
        saved = json.loads(read(folder / "receipt.json"))
        require(saved["artifact_evidence"] == review and saved["replacements"] == replacements, "Recovery request changed")
        return recover(state_dir, chain, expected_identity, fault)
    integrity(journal, json.loads(manifest_bytes))
    reject_orphan_signatures(state_dir, journal)
    if journal["identity_hash"] != expected_identity:
        links = journal.get(LINKS, [])
        require(links, "Unexpected current rollout identity")
        name = links[-1]["sha256"]
        folder = archive_path(state_dir, name)
        require(links[-1]["receipt"] == "artifact-amendments/" + name + "/receipt.json", "Invalid amendment link path")
        saved = json.loads(read(folder / "receipt.json"))
        require(sha(read(folder / "receipt.json")) == name, "Amendment link changed")
        require(saved["prior_identity_hash"] == expected_identity and saved["next_identity_hash"] == journal["identity_hash"]
                and saved["replacements"] == replacements and saved["artifact_evidence"] == review, "Different amendment already applied")
        require(saved.get(CHECKPOINT) == journal.get(CHECKPOINT), "Checkpoint differs from immutable amendment receipt")
        verify_frozen(saved)
        validate_live(state_dir, journal, journal["identity"], chain)
        return {"status": "already_amended", "identity_hash": journal["identity_hash"]}
    eligible(journal, replacements)
    updated = copy.deepcopy(journal)
    for role, checksum in replacements.items():
        require(checksum != journal["identity"]["artifacts"][role]["sha256"], "Replacement has unchanged checksum")
        updated["identity"]["artifacts"][role]["sha256"] = checksum
    updated["identity_hash"] = digest(updated["identity"])
    preservation = journal.get(CHECKPOINT)
    if capture_preservation:
        validate_live(state_dir, journal, updated["identity"], chain, check_balances=False)
        preservation = capture_checkpoint(journal, chain)
        updated[CHECKPOINT] = preservation
    validate_live(state_dir, journal, updated["identity"], chain, preservation)
    next_manifest = encoded({k: updated[k] for k in ("identity", "identity_hash", "inventory", "inventory_hash")})
    receipt = {"schema": 1, "reason": reason, "prior_identity_hash": journal["identity_hash"],
        "next_identity_hash": updated["identity_hash"], "original_identity": journal["identity"],
        "original_manifest_sha256": sha(manifest_bytes), "original_journal_sha256": sha(journal_bytes),
        "next_manifest_sha256": sha(next_manifest), "inventory_hash": journal["inventory_hash"],
        "preserved_operations_hash": digest(journal["operations"]), "preserved_signed_files": frozen_files(journal),
        "artifact_evidence": review, "replacements": replacements,
        "changes": {r: {"before": journal["identity"]["artifacts"][r]["sha256"], "after": h} for r, h in replacements.items()}}
    if preservation is not None:
        receipt[CHECKPOINT] = preservation
    receipt_bytes = encoded(receipt)
    archive = sha(receipt_bytes)
    relative = "artifact-amendments/" + archive + "/receipt.json"
    updated.setdefault(LINKS, []).append({"receipt": relative, "sha256": archive})
    result = {"status": "eligible", "prior_identity_hash": expected_identity, "identity_hash": updated["identity_hash"], "roles": sorted(replacements)}
    if not apply:
        return result
    folder = archive_path(state_dir, archive)
    folder.parent.mkdir(mode=0o700, exist_ok=True)
    require(not folder.parent.is_symlink(), "Symlink amendment evidence directory")
    # A previous crash before the pending marker may leave this exact archive.
    folder.mkdir(mode=0o700, exist_ok=True)
    require(not folder.is_symlink(), "Symlink amendment archive")
    byte_images = {"before-manifest.json": manifest_bytes, "before-journal.json": journal_bytes,
                   "after-manifest.json": next_manifest, "after-journal.json": encoded(updated), "receipt.json": receipt_bytes}
    transaction = {"receipt_sha256": archive, "files": {name: {phase: sha(byte_images[phase + "-" + name])
        for phase in ("before", "after")} for name in ("manifest.json", "journal.json")}}
    byte_images["transaction.json"] = encoded(transaction)
    for name, data in byte_images.items():
        if (folder / name).exists():
            require(read(folder / name) == data, "Existing immutable archive differs")
        else:
            immutable(folder / name, data)
    sync_directory(folder)
    sync_directory(folder.parent)
    atomic(state_dir / PENDING, encoded({"archive": archive, "transaction_sha256": sha(byte_images["transaction.json"])}))
    if fault:
        fault("staged")
    return recover(state_dir, chain, expected_identity, fault)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("mode", choices=("check", "apply"))
    p.add_argument("--state-dir", type=Path, required=True)
    p.add_argument("--replacements", type=Path, required=True, help="JSON object mapping artifact roles to reviewed SHA-256 values")
    p.add_argument("--evidence", type=Path, required=True, help="PASS JSON with business_source_unchanged=true and artifacts[role].sha256")
    p.add_argument("--expected-identity", required=True)
    p.add_argument("--reason", required=True)
    p.add_argument("--capture-preservation-checkpoint", action="store_true", help="Freeze verified current paused legacy pools/supply before any pair/token migration; original inventory stays immutable")
    args = p.parse_args()
    locks = []
    try:
        # Existing execution locks are opened read-only; check creates no files.
        for name in ("custody.lock", "rollout.lock"):
            fd = os.open(args.state_dir / name, os.O_RDONLY | os.O_NOFOLLOW)
            locks.append(fd)
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        print(json.dumps(amend(args.state_dir, json.loads(read(args.replacements)), args.evidence,
            args.expected_identity, args.reason, apply=args.mode == "apply",
            capture_preservation=args.capture_preservation_checkpoint), indent=2))
        return 0
    except (OSError, ValueError, RuntimeError, KeyError, TypeError) as error:
        print("Artifact amendment stopped: " + str(error), file=sys.stderr)
        return 1
    finally:
        for fd in locks:
            os.close(fd)


if __name__ == "__main__":
    raise SystemExit(main())
