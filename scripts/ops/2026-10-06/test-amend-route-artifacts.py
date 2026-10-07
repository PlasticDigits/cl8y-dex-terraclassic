#!/usr/bin/env python3
"""Synthetic amendment/recovery tests; no keyring, broadcasts or production writes."""
import base64
import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    sys.modules[name] = result
    spec.loader.exec_module(result)
    return result


fixture_module = module("amendment_fixture", ROOT / "scripts/qa/test-route-tax-rollout.py")
amendment = module("artifact_amendment", Path(__file__).with_name("amend-route-artifacts.py"))


class AmendmentTest(unittest.TestCase):
    def setUp(self):
        self.fixture = fixture_module.RolloutTest()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root, self.chain, self.engine = self.fixture.folder, self.fixture.chain, self.fixture.engine
        # Both pair and factory start behind the reviewed target in this fleet.
        for code, version in ((10, "1.10.0"), (30, "1.18.0")):
            self.chain.codes[code]["version"] = version
            self.chain.codes[code]["sha256"] = amendment.sha(str(code).encode())
        original_transaction = self.chain.transaction
        def transaction(action, signed):
            if action["op"] == "store" and action["path"] == self.fixture.artifacts["token"].path:
                raise RuntimeError("synthetic validation failure before signing")
            def checkpoint(tx_hash, _path, fee):
                path = self.root / (tx_hash + ".signed.json")
                path.write_text(json.dumps({"action": action, "tx_hash": tx_hash}))
                signed(tx_hash, str(path), fee)
            result = original_transaction(action, checkpoint)
            result["height"] = "42"
            self.chain.transactions[result["txhash"]] = copy.deepcopy(result)
            return result
        self.chain.transaction = transaction
        def validate_saved(action, entry):
            signed = json.loads(Path(entry["signed_path"]).read_text())
            amendment.require(signed == {"action": action, "tx_hash": entry["tx_hash"]}, "Saved signed transaction changed")
        self.chain.validate_saved = validate_saved
        self.chain.historical_pool = lambda address, height: {
            "status": "available", "requested_height": str(height), "returned_height": str(height),
            "pool": copy.deepcopy(self.chain.pools[address])}
        with self.assertRaisesRegex(RuntimeError, "before signing"):
            self.engine.execute()
        self.original_journal = (self.root / "journal.json").read_bytes()
        self.original_manifest = (self.root / "manifest.json").read_bytes()
        self.before = json.loads(self.original_journal)
        self.expected = self.before["identity_hash"]
        self.actions = copy.deepcopy(self.chain.actions)
        self.replacements = {}
        for role in ("token", "pair"):
            path = Path(self.fixture.artifacts[role].path)
            path.write_bytes(path.read_bytes() + b"\0synthetic-locals-normalization")
            self.replacements[role] = amendment.sha(path.read_bytes())
        self.evidence = self.root / "reviewed-evidence.json"
        self.evidence.write_text(json.dumps({"status": "PASS", "business_source_unchanged": True,
            "artifacts": {r: {"sha256": h} for r, h in self.replacements.items()}}))

    def run_amend(self, apply=False, fault=None, capture=False):
        return amendment.amend(self.root, self.replacements, self.evidence, self.expected,
            "Normalize only rejected unstored artifacts", self.chain, apply, fault, capture)

    def assert_no_dispatch(self):
        self.assertEqual(self.actions, self.chain.actions)
        self.assertEqual([], self.chain.rebroadcasts)

    def snapshot(self):
        return {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}

    def write_journal(self, state):
        (self.root / "journal.json").write_bytes(amendment.encoded(state))

    def test_check_is_read_only_and_retains_original_pauses(self):
        before = self.snapshot()
        self.assertEqual("eligible", self.run_amend()["status"])
        self.assertEqual(before, self.snapshot())
        self.assertEqual([False, True], [p["paused"] for p in self.before["inventory"]["pairs"]])
        self.assert_no_dispatch()

    def test_apply_preserves_original_byte_images_operations_and_signed_files(self):
        signed = amendment.frozen_files(self.before)
        result = self.run_amend(True)
        folder = Path(result["receipt"]).parent
        self.assertEqual(self.original_manifest, (folder / "before-manifest.json").read_bytes())
        self.assertEqual(self.original_journal, (folder / "before-journal.json").read_bytes())
        state = json.loads((self.root / "journal.json").read_text())
        for key in ("inventory", "inventory_hash", "operations", "codes", "status"):
            self.assertEqual(self.before[key], state[key])
        self.assertEqual(signed, amendment.frozen_files(state))
        original_operations_bytes = self.original_journal.split(b'  "operations": ', 1)[1].split(b'  "codes": ', 1)[0]
        current_operations_bytes = (self.root / "journal.json").read_bytes().split(b'  "operations": ', 1)[1].split(b'  "codes": ', 1)[0]
        self.assertEqual(original_operations_bytes, current_operations_bytes)
        self.assertEqual(self.before["identity"]["config"], state["identity"]["config"])
        for role in self.replacements:
            old = self.before["identity"]["artifacts"][role]
            self.assertEqual({**old, "sha256": self.replacements[role]}, state["identity"]["artifacts"][role])
        amendment.integrity(state, json.loads((self.root / "manifest.json").read_text()))
        # Existing engine accepts the amended shape and identity on normal resume.
        engine = amendment.Rollout(self.chain, amendment.artifacts(state["identity"]), state["identity"]["config"], self.root / "journal.json")
        self.assertEqual(state, engine.prepare())
        self.assert_no_dispatch()

    def test_repeat_apply_is_idempotent(self):
        self.run_amend(True)
        before = self.snapshot()
        self.assertEqual("already_amended", self.run_amend(True)["status"])
        self.assertEqual(before, self.snapshot())
        self.assert_no_dispatch()

    def test_already_uploaded_role_is_rejected_before_any_write(self):
        self.replacements = {"factory": "a" * 64}
        self.evidence.write_text(json.dumps({"status": "PASS", "business_source_unchanged": True,
            "artifacts": {"factory": {"sha256": "a" * 64}}}))
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "already stored"):
            self.run_amend(True)
        self.assertEqual(before, self.snapshot())

    def test_signed_pending_transaction_blocks_amendment(self):
        state = copy.deepcopy(self.before)
        state["operations"]["store:token"].update(tx_hash="a" * 64, signed_path="pending", fee_uluna=2)
        self.write_journal(state)
        with self.assertRaisesRegex(RuntimeError, "signed pending"):
            self.run_amend(True)
        self.assertFalse((self.root / "artifact-amendments").exists())

    def test_partial_signed_data_even_zero_fee_is_rejected(self):
        state = copy.deepcopy(self.before)
        state["operations"]["store:token"]["fee_uluna"] = 0
        self.write_journal(state)
        with self.assertRaisesRegex(RuntimeError, "partially signed"):
            self.run_amend(True)

    def test_unsigned_store_path_cannot_change(self):
        state = copy.deepcopy(self.before)
        state["operations"]["store:token"]["action"]["path"] += ".different"
        self.write_journal(state)
        with self.assertRaisesRegex(RuntimeError, "altered store"):
            self.run_amend(True)

    def test_new_review_must_bind_every_replacement(self):
        record = json.loads(self.evidence.read_text())
        del record["artifacts"]["pair"]
        self.evidence.write_text(json.dumps(record))
        with self.assertRaisesRegex(RuntimeError, "does not bind"):
            self.run_amend(True)

    def test_chain_identity_failure_leaves_files_unchanged(self):
        self.chain.chain_id = "other-chain"
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "chain mismatch"):
            self.run_amend(True)
        self.assertEqual(before, self.snapshot())

    def test_unexpected_pause_drift_blocks_amendment(self):
        self.chain.pauses[fixture_module.PAIR] = False
        with self.assertRaisesRegex(RuntimeError, "pause changed"):
            self.run_amend(True)

    def test_completed_migration_must_still_be_live(self):
        self.chain.instances[fixture_module.ROUTER]["code_id"] = 20
        with self.assertRaisesRegex(RuntimeError, "Completed migration drifted"):
            self.run_amend(True)

    def test_pool_and_supply_drift_block_amendment(self):
        self.chain.pools[fixture_module.PAIR]["total_share"] = "999"
        with self.assertRaisesRegex(RuntimeError, "Contained pool changed"):
            self.run_amend(True)
        self.chain.pools[fixture_module.PAIR] = copy.deepcopy(self.before["inventory"]["pairs"][0]["pool"])
        self.chain.supply[fixture_module.ALPHA] = "1"
        with self.assertRaisesRegex(RuntimeError, "Token supply changed"):
            self.run_amend(True)

    def test_missing_or_failed_live_receipt_is_not_waived(self):
        tx_hash = next(iter(self.chain.transactions))
        self.chain.transactions[tx_hash]["code"] = 9
        with self.assertRaisesRegex(RuntimeError, "failed on chain"):
            self.run_amend(True)

    def interrupt(self, boundary):
        def stop(where):
            if where == boundary:
                raise RuntimeError("synthetic interruption")
        with self.assertRaisesRegex(RuntimeError, "synthetic interruption"):
            self.run_amend(True, stop)

    def test_recover_after_staging_and_each_file_replacement(self):
        # Independent fixtures test all durable interruption boundaries.
        for boundary in ("staged", "manifest.json-replaced", "journal.json-replaced"):
            with self.subTest(boundary=boundary):
                case = AmendmentTest()
                case.setUp()
                try:
                    case.interrupt(boundary)
                    self.assertTrue((case.root / amendment.PENDING).exists())
                    staged = json.loads((case.root / "journal.json").read_text())
                    engine = amendment.Rollout(case.chain, {}, {}, case.root / "journal.json")
                    engine.state = staged
                    with self.assertRaisesRegex(RuntimeError, "recover it before rollout"):
                        engine._integrity()
                    with self.assertRaisesRegex(RuntimeError, "Interrupted amendment"):
                        case.run_amend(False)
                    result = case.run_amend(True)
                    self.assertEqual("amended", result["status"])
                    self.assertFalse((case.root / amendment.PENDING).exists())
                    case.assert_no_dispatch()
                    state = json.loads((case.root / "journal.json").read_text())
                    self.assertEqual(case.before["operations"], state["operations"])
                    self.assertEqual(1, len(state[amendment.LINKS]))
                finally:
                    case.doCleanups()

    def test_recovery_refuses_unexpected_current_file_edit(self):
        self.interrupt("manifest.json-replaced")
        (self.root / "journal.json").write_bytes(self.original_journal + b" ")
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "changed outside"):
            self.run_amend(True)
        self.assertEqual(before, self.snapshot())

    def test_recovery_refuses_tampered_archive_or_prior_signed_bytes(self):
        self.interrupt("staged")
        signed = Path(next(iter(amendment.frozen_files(self.before))))
        signed.write_bytes(signed.read_bytes() + b" ")
        with self.assertRaisesRegex(RuntimeError, "Prior signed evidence changed"):
            self.run_amend(True)

    def test_recovery_request_cannot_change(self):
        self.interrupt("staged")
        self.replacements.pop("pair")
        with self.assertRaisesRegex(RuntimeError, "Recovery request changed"):
            self.run_amend(True)

    def test_orphan_component_signature_prevents_identity_change(self):
        folder = self.root / "transactions" / "uncheckpointed"
        folder.mkdir(parents=True)
        (folder / "signature0.json").write_text("{}")
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "Uncheckpointed signed"):
            self.run_amend(True)
        self.assertEqual(before, self.snapshot())

    def test_changed_archived_byte_image_is_not_recovered(self):
        self.interrupt("staged")
        pending = json.loads((self.root / amendment.PENDING).read_text())
        image = self.root / "artifact-amendments" / pending["archive"] / "after-journal.json"
        image.write_bytes(image.read_bytes() + b" ")
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "byte image changed"):
            self.run_amend(True)
        self.assertEqual(before, self.snapshot())

    def drift(self):
        self.chain.pools[fixture_module.PAIR]["total_share"] = "12000"
        self.chain.pools[fixture_module.PAIR]["assets"][0]["amount"] = "1500000"
        self.chain.supply[fixture_module.ALPHA] = "999999999900"

    def test_explicit_checkpoint_preserves_stale_inventory_and_original_pauses(self):
        self.drift()
        result = self.run_amend(True, capture=True)
        state = json.loads((self.root / "journal.json").read_text())
        self.assertEqual(self.before["inventory"], state["inventory"])
        self.assertEqual(self.before["operations"], state["operations"])
        cp = state[amendment.CHECKPOINT]
        self.assertEqual(self.chain.pools[fixture_module.PAIR], cp["pairs"][fixture_module.PAIR])
        self.assertEqual(self.chain.supply[fixture_module.ALPHA], cp["token_supplies"][fixture_module.ALPHA])
        self.assertTrue(cp["proofs"]["pairs"][fixture_module.PAIR]["differs_from_original_inventory"])
        self.assertEqual(cp, json.loads(Path(result["receipt"]).read_text())[amendment.CHECKPOINT])
        self.assertEqual([False, True], [p["paused"] for p in state["inventory"]["pairs"]])
        self.assert_no_dispatch()

    def test_checkpoint_check_does_not_write_even_when_planning_inventory_is_stale(self):
        self.drift()
        before = self.snapshot()
        self.assertEqual("eligible", self.run_amend(capture=True)["status"])
        self.assertEqual(before, self.snapshot())

    def test_checkpoint_rejects_unverified_or_wrong_historical_height(self):
        self.chain.historical_pool = lambda _a, height: {"status": "available", "requested_height": str(height),
            "returned_height": "999", "pool": copy.deepcopy(self.chain.pools[fixture_module.PAIR])}
        with self.assertRaisesRegex(RuntimeError, "Historical height differs"):
            self.run_amend(True, capture=True)

    def test_historical_unavailability_is_explicit_without_pre_pause_attribution(self):
        self.drift()
        self.chain.historical_pool = lambda _a, height: {"status": "unavailable", "requested_height": str(height),
            "returned_height": None, "reason": "Synthetic pruned state"}
        self.run_amend(True, capture=True)
        cp = json.loads((self.root / "journal.json").read_text())[amendment.CHECKPOINT]
        self.assertEqual("unavailable", cp["proofs"]["pairs"][fixture_module.PAIR]["historical"]["status"])
        self.assertIn("Current paused legacy state", cp["qualification"])

    def test_available_historical_snapshot_must_match_current_pool(self):
        self.chain.historical_pool = lambda _a, height: {"status": "available", "requested_height": str(height),
            "returned_height": str(height), "pool": {"different": "state"}}
        with self.assertRaisesRegex(RuntimeError, "Pool changed after"):
            self.run_amend(True, capture=True)

    def test_checkpoint_requires_all_original_pair_and_token_code_hashes(self):
        for role, address in (("pair", fixture_module.PAIR), ("token", fixture_module.ALPHA)):
            with self.subTest(role=role):
                old = self.chain.instances[address]["code_id"]
                self.chain.instances[address]["code_id"] = 999
                self.chain.codes[999] = copy.deepcopy(self.chain.codes[old])
                with self.assertRaisesRegex(RuntimeError, "original legacy code"):
                    amendment.capture_checkpoint(self.before, self.chain)
                self.chain.instances[address]["code_id"] = old

    def test_signed_pair_or_token_migration_prevents_new_checkpoint(self):
        for role in ("pair", "token"):
            state = copy.deepcopy(self.before)
            state["operations"]["migrate:" + role + ":address"] = {"tx_hash": "a" * 64}
            with self.assertRaisesRegex(RuntimeError, "migration already signed"):
                amendment.capture_checkpoint(state, self.chain)

    def test_checkpoint_requires_completed_prerequisites_and_unchanged_pair_count(self):
        state = copy.deepcopy(self.before)
        state["operations"].pop("migrate:router")
        with self.assertRaisesRegex(RuntimeError, "migration is incomplete"):
            amendment.capture_checkpoint(state, self.chain)
        self.chain.pair_rows.append(copy.deepcopy(self.chain.pair_rows[0]))
        with self.assertRaisesRegex(RuntimeError, "pair membership"):
            amendment.capture_checkpoint(self.before, self.chain)

    def test_equal_count_cannot_hide_a_replaced_listed_pair(self):
        self.chain.pair_rows[0]["contract_addr"] = "replacement-market"
        before = self.snapshot()
        with self.assertRaisesRegex(RuntimeError, "membership changed"):
            self.run_amend(True, capture=True)
        self.assertEqual(before, self.snapshot())

    def test_equal_address_cannot_hide_changed_listed_assets(self):
        self.chain.pair_rows[0]["asset_infos"][0] = {"token": {"contract_addr": "replacement-asset"}}
        with self.assertRaisesRegex(RuntimeError, "membership changed"):
            self.run_amend(True, capture=True)

    def test_later_pool_or_supply_mutation_cannot_rebase_checkpoint(self):
        self.drift()
        self.run_amend(True, capture=True)
        self.chain.pools[fixture_module.PAIR]["total_share"] = "13000"
        with self.assertRaisesRegex(RuntimeError, "Contained pool changed"):
            self.run_amend(True, capture=True)
        self.chain.pools[fixture_module.PAIR]["total_share"] = "12000"
        self.chain.supply[fixture_module.ALPHA] = "1"
        with self.assertRaisesRegex(RuntimeError, "Token supply changed"):
            self.run_amend(True, capture=True)

    def test_tampered_checkpoint_is_rejected_by_normal_engine_integrity(self):
        self.run_amend(True, capture=True)
        state = json.loads((self.root / "journal.json").read_text())
        state[amendment.CHECKPOINT]["pairs"][fixture_module.PAIR]["total_share"] = "1"
        self.write_journal(state)
        engine = amendment.Rollout(self.chain, amendment.artifacts(state["identity"]), state["identity"]["config"], self.root / "journal.json")
        with self.assertRaisesRegex(RuntimeError, "checkpoint differs"):
            engine.prepare()

    def test_normal_execute_and_verify_use_frozen_checkpoint_then_reject_drift(self):
        self.drift()
        self.run_amend(True, capture=True)
        state = json.loads((self.root / "journal.json").read_text())
        self.chain.transaction = fixture_module.FakeChain.transaction.__get__(self.chain)
        for role in self.replacements:
            self.chain.artifact_metadata[state["identity"]["artifacts"][role]["path"]]["sha256"] = self.replacements[role]
        # Synthetic transport uses external placeholder paths in subsequent ops;
        # its included receipt validation remains exercised by existing suite.
        del self.chain.validate_saved
        engine = amendment.Rollout(self.chain, amendment.artifacts(state["identity"]), state["identity"]["config"], self.root / "journal.json")
        engine.execute()
        engine.verify()
        self.chain.pools[fixture_module.PAIR]["total_share"] = "999"
        with self.assertRaisesRegex(RuntimeError, "pool liquidity changed"):
            engine.verify()

    def test_unlinked_checkpoint_cannot_override_original_inventory(self):
        state = copy.deepcopy(self.before)
        state[amendment.CHECKPOINT] = {"pairs": {fixture_module.PAIR: self.chain.pools[fixture_module.PAIR]}}
        self.write_journal(state)
        engine = amendment.Rollout(self.chain, {}, {}, self.root / "journal.json")
        with self.assertRaisesRegex(RuntimeError, "checkpoint differs"):
            engine._integrity()


class ReadOnlyProofTest(unittest.TestCase):
    def setUp(self):
        self.config = {"transport": {"node": "https://rpc.invalid", "lcd": "https://lcd.invalid",
            "store_key": "store", "multisig_key": "gov", "store_address": "deployer", "max_gas": "1000"},
            "governance": "governance", "eoa_admins": {}}
        self.chain = amendment.ReadOnlyChain(Path("/unused"), self.config)

    def test_transport_has_no_keyring_or_broadcast_surface(self):
        for name in ("unlock", "transaction", "rebroadcast", "command"):
            with self.assertRaises(AttributeError):
                getattr(self.chain, name)

    def test_saved_proof_compares_included_messages_signatures_hash_and_fee_without_commands(self):
        import tempfile
        with tempfile.TemporaryDirectory() as folder:
            action = {"op": "execute", "role": "gov", "contract": "factory", "msg": {"set_pair_paused": {"pair": "pair", "paused": True}}}
            saved = {"body": {"messages": [{"@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
                "sender": "governance", "contract": "factory", "msg": action["msg"], "funds": []}]},
                "auth_info": {"signer_infos": [], "fee": {"amount": [{"denom": "uluna", "amount": "7"}], "gas_limit": "500"}},
                "signatures": ["c2lnbmF0dXJl"]}
            path = Path(folder) / "signed.json"
            path.write_text(json.dumps(saved))
            live = copy.deepcopy(saved)
            live["body"].update(unordered=False, timeout_timestamp=None)
            live["body"]["messages"][0]["msg"] = base64.b64encode(json.dumps(action["msg"]).encode()).decode()
            live["body"]["messages"][0].pop("funds")
            raw = b"synthetic-protobuf-transaction"
            entry = {"signed_path": str(path), "tx_hash": amendment.sha(raw).upper(), "fee_uluna": 7}
            self.chain.client.get = lambda _path, rpc=False: {"result": {"tx": base64.b64encode(raw).decode()}} if rpc else {"tx": live}
            with patch.object(amendment.TerradChain, "command", side_effect=AssertionError("No commands")):
                self.chain.validate_saved(action, entry)
                saved["signatures"] = ["dGFtcGVyZWQ="]
                path.write_text(json.dumps(saved))
                with self.assertRaisesRegex(RuntimeError, "differs from included"):
                    self.chain.validate_saved(action, entry)

    def test_new_sdk_defaults_do_not_allow_unordered_transactions(self):
        tx = {"body": {"messages": [], "unordered": True}, "auth_info": {}, "signatures": []}
        with self.assertRaisesRegex(RuntimeError, "unordered"):
            amendment.canonical_transaction(tx)

    def test_only_immutable_code_and_transaction_reads_are_cached(self):
        calls = []
        def get(_client, path, rpc=False):
            calls.append((path, rpc))
            return {"value": {"nested": "unchanged"}}
        with patch.object(amendment.TerradChain, "get", get):
            chain = amendment.ReadOnlyChain(Path("/unused"), self.config)
            for path in ("/cosmwasm/wasm/v1/code/10", "/cosmos/tx/v1beta1/txs/" + "A" * 64):
                value = chain.get(path)
                value["value"]["nested"] = "mutated"
                self.assertEqual("unchanged", chain.get(path)["value"]["nested"])
                self.assertEqual(1, sum(p == path for p, _rpc in calls))
            for path in ("/cosmwasm/wasm/v1/contract/pair/smart/query", "/cosmwasm/wasm/v1/code/10/contracts"):
                chain.get(path)
                chain.get(path)
                self.assertEqual(2, sum(p == path for p, _rpc in calls))


if __name__ == "__main__":
    unittest.main()
