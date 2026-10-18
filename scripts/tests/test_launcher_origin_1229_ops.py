"""Exercise origin-release adapter with real artifact validation and mocked chain CLI."""
import hashlib
import contextlib
import io
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))
spec = importlib.util.spec_from_file_location("origin_ops", ROOT / "scripts/ops/launcher-origin-1229.py")
ops = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ops)


class OriginOpsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.artifacts = self.root / "smartcontracts/artifacts"
        self.artifacts.mkdir(parents=True)
        self.filename = "cl8y_community_token_launcher.wasm"
        # Real loader verifies header, release identity/version and manifest hash.
        self.binary = b"\0asm\x01\0\0\0crates.io:cl8y-community-token-launcher 1.1.0"
        (self.artifacts / self.filename).write_bytes(self.binary)
        (self.artifacts / "checksums.txt").write_text(
            hashlib.sha256(self.binary).hexdigest() + "  " + self.filename + "\n"
        )

    def run_adapter(self, mode, inventory):
        seen = {}

        def cli(args):
            seen["args"] = args
            self.assertEqual(ops.rollout.SPECS["token"][1], "1.5.0")
            self.assertEqual(ops.rollout.SPECS["launcher"][1], "1.1.0")
            actual = ops.rollout.Artifact.load(
                "launcher", self.artifacts / self.filename,
                hashlib.sha256(self.binary).hexdigest(),
            )
            self.assertEqual(actual.version, "1.1.0")
            return ops.rollout.Rollout.inventory(object())

        with patch.object(ops, "ROOT", self.root), patch.dict(ops.rollout.SPECS), \
                patch.object(ops.rollout.Rollout, "inventory", return_value=inventory), \
                patch.object(ops.cli, "main", side_effect=cli), \
                patch.object(sys, "argv", ["origin.py", *mode]):
            result = ops.main()
        return result, seen["args"]

    def test_exact_release_versions_hash_and_plan_execute_verify_finalize_modes(self):
        inventory = {"instances": {"factory": {"needs_migrate": False},
                                   "router": {"needs_migrate": False},
                                   "launcher": {"needs_migrate": True}},
                     "pairs": [{"needs_migrate": False}]}
        for mode in [[], ["--execute"], ["--verify"], ["--finalize"]]:
            with self.subTest(mode=mode):
                result, args = self.run_adapter(mode, inventory)
                self.assertIs(result, inventory)
                self.assertIn("--launcher-wasm", args)
                self.assertIn(str(self.artifacts / self.filename), args)
                self.assertEqual(args[-1:] if mode else [], mode)
                self.assertIn("--admin-key", args)
                self.assertIn("cl8y2_admin", args)

    def test_unrelated_factory_and_pair_upgrades_fail_before_transport(self):
        for inventory in [
            {"instances": {"factory": {"needs_migrate": True}}, "pairs": []},
            {"instances": {"launcher": {"needs_migrate": True}},
             "pairs": [{"needs_migrate": True}]},
        ]:
            with self.subTest(inventory=inventory), self.assertRaisesRegex(
                ops.rollout.RolloutError, "Unrelated contract upgrade"
            ):
                self.run_adapter(["--execute"], inventory)

    def test_modified_launcher_artifact_rejected_before_cli(self):
        (self.artifacts / self.filename).write_bytes(self.binary + b"modified")
        with patch.object(ops, "ROOT", self.root), patch.dict(ops.rollout.SPECS), \
                patch.object(ops.cli, "main") as cli, \
                patch.object(sys, "argv", ["origin.py", "--execute"]), \
                self.assertRaisesRegex(SystemExit, "differs from reviewed"):
            ops.main()
        cli.assert_not_called()

    def test_local_artifact_preflight_validates_all_release_inputs_without_chain(self):
        entries = []
        for role in ("factory", "pair", "router", "token", "launcher"):
            contract, version, name = ops.rollout.SPECS[role]
            version = {"token": "1.5.0", "launcher": "1.1.0"}.get(role, version)
            binary = b"\0asm\x01\0\0\0" + contract.encode() + b" " + version.encode()
            (self.artifacts / name).write_bytes(binary)
            entries.append(hashlib.sha256(binary).hexdigest() + "  " + name + "\n")
        (self.artifacts / "checksums.txt").write_text("".join(entries))
        with patch.object(ops, "ROOT", self.root), patch.dict(ops.rollout.SPECS), \
                patch.object(ops.cli, "main") as cli, \
                patch.object(sys, "argv", ["origin.py", "--check-artifacts"]), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(ops.main(), 0)
        self.assertIn('"status": "PASS"', output.getvalue())
        cli.assert_not_called()

    def test_unknown_or_multiple_modes_rejected_before_files(self):
        for mode in [["--force"], ["--execute", "--finalize"]]:
            with patch.object(sys, "argv", ["origin.py", *mode]), \
                    self.assertRaisesRegex(SystemExit, "Usage"):
                ops.main()


if __name__ == "__main__":
    unittest.main()
