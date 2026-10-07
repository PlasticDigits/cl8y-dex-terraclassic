#!/usr/bin/env python3
"""Synthetic custody boundaries; all SDK/process/network calls are mocked."""
import importlib.util
import json
from pathlib import Path
import os
import subprocess
import sys
import tempfile
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))
import route_tax_rollout_cli as transport
from route_tax_rollout import RolloutError


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts/ops/2026-10-06" / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


custody = module("security_custody", "complete-custody.py")
smoke = module("security_smoke", "route-live-smoke.py")


class SecurityTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def save(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value))

    def route(self, fee=0, pairs=22):
        inventory = [{"address": f"pair{i}", "paused": False} for i in range(pairs)]
        inventory.append({"address": "originally-paused", "paused": True})
        journal = {"identity_hash": "reviewed-release", "identity": {"config": {
            "max_total_fee_uluna": 5_000_000_000, "factory": "reviewed-factory",
            "transport": {"gas_prices": "28.325uluna"}}},
            "inventory": {"pairs": inventory, "affected_pairs": [p["address"] for p in inventory]},
            "operations": {"prior": {"fee_uluna": fee}}}
        self.save("journal.json", journal)
        return journal

    def adapter(self):
        chain = transport.TerradChain(transport.parser().parse_args(["--state-dir", str(self.root)]))
        chain.unlock = Mock()
        chain.network = Mock(return_value="columbus-5")
        chain._account = Mock(return_value=("1", "2"))
        chain.receipt = Mock(return_value=None)
        return chain

    def test_children_receive_only_their_requested_credentials(self):
        source = {"PATH": "/synthetic/bin", "BAO_ADDR": "https://synthetic-vault",
                  "BAO_CLIENT_KEY": "/synthetic/client.key", "BAO_CACERT": "/synthetic/ca.pem",
                  "TERRAD_HOST_KEYRING_PASS": "synthetic-custody",
                  "COOLIFY_WIRE_TOKEN": "synthetic-service", "FORGEJO_PLASTICDIGTS_PAT": "synthetic-forgejo",
                  "BAO_TOKEN": "synthetic-bao"}
        for allowed in [(), ("COOLIFY_WIRE_TOKEN",), ("FORGEJO_PLASTICDIGTS_PAT",), (transport.KEYRING_PASSWORD_ENV,)]:
            def inspect_child(*args, **kwargs):
                self.assertNotIn(source[transport.KEYRING_PASSWORD_ENV], kwargs["env"].values())
                self.assertTrue(kwargs["close_fds"])
                if transport.KEYRING_PASSWORD_ENV in allowed:
                    fd = int(kwargs["env"][transport.KEYRING_FD_ENV])
                    self.assertEqual(kwargs["pass_fds"], (fd,))
                    self.assertEqual(os.read(fd, 4096), b"synthetic-custody\n")
                else:
                    self.assertNotIn("pass_fds", kwargs)
            with patch.object(custody.subprocess, "run", side_effect=inspect_child) as call:
                custody.run(["synthetic-tool"], source, allowed)
                observed = call.call_args.kwargs["env"]
                self.assertEqual(observed["BAO_CLIENT_KEY"], source["BAO_CLIENT_KEY"])
                self.assertEqual(observed["BAO_CACERT"], source["BAO_CACERT"])
                for secret in transport.SENSITIVE_CHILD_ENV:
                    expected = secret in allowed and secret in source and secret != transport.KEYRING_PASSWORD_ENV
                    expected |= secret == transport.KEYRING_FD_ENV and transport.KEYRING_PASSWORD_ENV in allowed
                    self.assertEqual(secret in observed, expected)

    def test_terrad_environment_has_no_password_or_service_secrets_but_stdin_unlock_remains(self):
        chain = self.adapter()
        chain.password = "synthetic-password"
        with patch.dict(os.environ, {"TERRAD_HOST_KEYRING_PASS": chain.password,
                        "TERRAD_HOST_KEYRING_FD": "999", "COOLIFY_WIRE_TOKEN": "synthetic-api",
                        "FORGEJO_PLASTICDIGTS_PAT": "synthetic-fj"}):
            with patch.object(transport.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, '{}', '')) as call:
                chain.command(["keys", "show", "synthetic"])
                self.assertIn(chain.password, call.call_args.kwargs["input"])
                self.assertNotIn(chain.password, str(call.call_args.args[0]))
                for key in transport.SENSITIVE_CHILD_ENV:
                    self.assertNotIn(key, call.call_args.kwargs["env"])
                self.assertTrue(call.call_args.kwargs["close_fds"])
                self.assertNotIn("pass_fds", call.call_args.kwargs)

    def test_password_pipe_is_bounded_consumed_once_and_closed(self):
        with transport.keyring_password_pipe("synthetic-long-passphrase-κ") as fd:
            child_fd = os.dup(fd)
            with patch.dict(os.environ, {transport.KEYRING_FD_ENV: str(child_fd)}):
                self.assertEqual(transport.read_keyring_password_pipe(), "synthetic-long-passphrase-κ")
                self.assertNotIn(transport.KEYRING_FD_ENV, os.environ)
                self.assertEqual(transport.read_keyring_password_pipe(), "")
            with self.assertRaises(OSError):
                os.fstat(child_fd)
        for value in ["", "synthetic\nsecond-line", "x" * 4096]:
            with self.assertRaises(RolloutError):
                with transport.keyring_password_pipe(value):
                    self.fail("Invalid passphrase unexpectedly entered a pipe")

    def test_private_fd_survives_bash_to_python_without_password_in_environment(self):
        # Harmless real local process inheritance, no terrad/wallet/network calls.
        source = os.environ.copy()
        source[transport.KEYRING_PASSWORD_ENV] = "synthetic-custody-only"
        script = ("import os,sys;sys.path.insert(0," + repr(str(ROOT / "scripts/lib")) + ");"
                  "import route_tax_rollout_cli as t;"
                  "assert t.KEYRING_PASSWORD_ENV not in os.environ;"
                  "assert t.read_keyring_password_pipe() == 'synthetic-custody-only';"
                  "assert t.KEYRING_FD_ENV not in os.environ")
        custody.run(["bash", "-c", '"$1" -c "$2"', "synthetic-custody-child", sys.executable, script],
                    source, (transport.KEYRING_PASSWORD_ENV,))

    def test_entire_pause_and_restore_round_is_reserved_and_near_cap_rejected(self):
        self.route(fee=2_500_000_000)
        result = transport.fee_budget_guard(self.root, 5_000_000)
        self.assertEqual(result["reserve"], 2_492_600_000)
        with self.assertRaisesRegex(RolloutError, "protected containment/restoration reserve"):
            transport.fee_budget_guard(self.root, 10_000_000)

    def test_unknown_pending_smoke_hashes_already_count_toward_aggregate_fees(self):
        journal = self.route(fee=500_000_000)
        journal["operations"]["prior"]["tx_hash"] = "A" * 64
        self.save("journal.json", journal)
        self.save("live-smoke/fund-pending.json", {"tx_hash": "B" * 64, "fee_uluna": 11_330_000})
        self.save("live-smoke/swap-pending.json", {"tx_hash": "C" * 64, "fee_uluna": 141_625_000})
        result = transport.fee_budget_guard(self.root / "live-smoke")
        self.assertEqual(result["fees"], 652_955_000)
        self.assertEqual(result["reserve"], 2_492_600_000)

    def test_containment_releases_only_its_checkpointed_slot_without_double_charge(self):
        journal = self.route(fee=50_000_000, pairs=2)
        targets = {p["address"]: p["paused"] for p in journal["inventory"]["pairs"]}
        state = {"identity": {"manifest_identity": journal["identity_hash"], "targets": targets},
                 "operations": {}, "external_pauses": [], "introduced": ["pair0"], "round": 1, "status": "containing"}
        self.save("containment.json", state)
        action = {"op": "execute", "role": "gov", "contract": "reviewed-factory",
                  "msg": {"set_pair_paused": {"pair": "pair0", "paused": True}}}
        before = transport.fee_budget_guard(self.root, 10_000_000, purpose="containment", action=action)
        self.assertEqual(before["reserve"], 3 * 56_650_000)
        state["operations"]["round:1:pause:pair0"] = {"tx_hash": "D" * 64, "fee_uluna": 10_000_000}
        self.save("containment.json", state)
        after = transport.fee_budget_guard(self.root, purpose="containment", action=action)
        self.assertEqual(after["fees"], before["fees"] + before["proposed_fee"])
        self.assertEqual(after["reserve"], before["reserve"])
        self.assertEqual(after["fees"] + after["reserve"], before["fees"] + before["proposed_fee"] + before["reserve"])
        with self.assertRaisesRegex(RolloutError, "reserved fee envelope"):
            transport.fee_budget_guard(self.root, 56_650_001, purpose="containment", action=action)

    def test_bridge_budget_does_not_reserve_dex_containment(self):
        self.save("journal.json", {"identity": {"max_total_fee_uluna": 2_000_000_000},
                                   "operations": {"prior": {"fee_uluna": 500_000_000}}})
        result = transport.fee_budget_guard(self.root, 1_000_000_000)
        self.assertEqual(result["reserve"], 0)
        with self.assertRaises(RolloutError):
            transport.fee_budget_guard(self.root, 1_500_000_001)

    def test_owned_restore_releases_its_slot_but_external_or_unknown_pauses_do_not(self):
        journal = self.route(fee=50_000_000, pairs=2)
        targets = {p["address"]: p["paused"] for p in journal["inventory"]["pairs"]}
        state = {"identity": {"manifest_identity": journal["identity_hash"], "targets": targets},
                 "operations": {"round:1:pause:pair0": {"tx_hash": "D" * 64, "fee_uluna": 10_000_000}},
                 "external_pauses": ["pair1"], "introduced": ["pair0"], "round": 1, "status": "restoring"}
        self.save("containment.json", state)
        action = {"op": "execute", "role": "gov", "contract": "reviewed-factory",
                  "msg": {"set_pair_paused": {"pair": "pair0", "paused": False}}}
        with self.assertRaisesRegex(RolloutError, "included owned pause"):
            transport.fee_budget_guard(self.root, 10_000_000, purpose="containment", action=action)
        state["operations"]["round:1:pause:pair0"]["receipt"] = {"code": 0, "txhash": "D" * 64}
        self.save("containment.json", state)
        result = transport.fee_budget_guard(self.root, 10_000_000, purpose="containment", action=action)
        self.assertEqual(result["reserve"], 0)
        self.assertEqual(result["fees"], 60_000_000)
        outside = {**action, "msg": {"set_pair_paused": {"pair": "pair1", "paused": False}}}
        with self.assertRaises(RolloutError):
            transport.fee_budget_guard(self.root, 10_000_000, purpose="containment", action=outside)

    def test_containment_gas_ceiling_is_enforced_before_signing(self):
        chain = self.adapter()
        chain.command = Mock(return_value=json.dumps({"auth_info": {"fee": {
            "gas_limit": "2000001", "amount": [{"denom": "uluna", "amount": "100"}]}},
            "body": {"messages": []}}))
        with self.assertRaisesRegex(RolloutError, "Containment gas exceeds"):
            chain.transaction({"op": "execute", "role": "gov", "contract": "reviewed-factory", "msg": {}},
                              Mock(), budget_purpose="containment")
        chain.command.assert_called_once()
        self.assertIn("--generate-only", chain.command.call_args.args[0])

    def test_budget_failure_stops_transport_before_signing(self):
        self.route(fee=2_500_000_000)
        chain = self.adapter()
        chain.command = Mock(return_value=json.dumps({"auth_info": {"fee": {
            "gas_limit": "100", "amount": [{"denom": "uluna", "amount": "10000000"}]}},
            "body": {"messages": []}}))
        with self.assertRaisesRegex(RolloutError, "protected containment/restoration reserve"):
            chain.transaction({"op": "execute", "role": "gov", "contract": "reviewed-factory", "msg": {}}, Mock())
        chain.command.assert_called_once()
        self.assertIn("--generate-only", chain.command.call_args.args[0])

    def test_budget_failure_stops_smoke_before_signing_or_broadcasting(self):
        self.route(fee=2_500_000_000)
        chain = self.adapter()
        chain.command = Mock()
        chain.rebroadcast = Mock()
        folder = self.root / "live-smoke"
        with self.assertRaisesRegex(RolloutError, "protected containment/restoration reserve"):
            smoke.raw_transaction(chain, [], 5_000_000, folder, "swap", "synthetic-key")
        chain.command.assert_not_called()
        chain.rebroadcast.assert_not_called()
        self.assertFalse((folder / "swap-pending.json").exists())

    def test_new_checkpoint_is_rechecked_before_broadcast(self):
        self.route(fee=4_000_000_000)
        chain = self.adapter()
        chain._encoded_hash = Mock(return_value="E" * 64)
        chain.command = Mock()
        with self.assertRaisesRegex(RolloutError, "protected containment/restoration reserve"):
            chain.rebroadcast("synthetic-signed.json", "E" * 64)
        chain.command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
