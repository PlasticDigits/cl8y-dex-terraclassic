#!/usr/bin/env python3
"""Exercise release-source qualification against isolated, real Git histories."""
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SOURCE = Path(__file__).with_name("application-source.py")
RUNTIME = ("frontend-dapp/src", "frontend-dapp/package.json", "indexer/src", "tokenlist")


class ApplicationSourceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="cl8y-source-gate-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "fixture"
        self.root.mkdir()
        self.git("init", "--quiet", "--initial-branch=main")
        # Fixture-only human identity. No commits or identity changes in the DEX checkout.
        self.git("config", "user.name", "Fixture Developer")
        self.git("config", "user.email", "fixture-developer@example.test")
        self.files = {
            "frontend-dapp/src/app.ts": "baseline app\n",
            "frontend-dapp/package.json": '{"name":"fixture"}\n',
            "indexer/src/main.rs": "fn main() {}\n",
            "tokenlist/tokens.json": "[]\n",
            "docker/indexer/Dockerfile": "FROM fixture\n",
            "smartcontracts/contracts/token/src/lib.rs": "pub fn instantiate() {}\n",
            "smartcontracts/scripts/optimize.sh": "#!/bin/sh\nexit 0\n",
            "smartcontracts/Cargo.lock": "version = 3\n",
            "smartcontracts/artifacts/token.wasm": "original artifact\n",
            "smartcontracts/artifacts/checksums.txt": "original artifact digest\n",
            "smartcontracts/wasm-checksums.txt": "original workspace digest\n",
            "scripts/ops/operator.py": "print('baseline operations')\n",
        }
        for path, content in self.files.items():
            self.write(path, content)
        self.baseline = self.commit("Initial release inputs")
        self.write("frontend-dapp/src/app.ts", "qualified application\n")
        self.application = self.commit("Prepare application revision")
        spec = importlib.util.spec_from_file_location("application_source_fixture", SOURCE)
        self.gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.gate)
        self.gate.BASELINE = self.baseline
        self.gate.RUNTIME_INPUTS = RUNTIME
        self.gate.RECORD = Path(self.temporary.name) / "application-source.json"
        self.record = {
            "baseline": self.baseline,
            "revision": self.application,
            "runtime_inputs": self.identities(self.application),
        }
        self.gate.RECORD.write_text(json.dumps(self.record))

    def git(self, *arguments):
        return subprocess.check_output(
            ["git", *arguments], cwd=self.root, text=True, stderr=subprocess.PIPE
        ).strip()

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)

    def commit(self, message):
        self.assertEqual(self.git("config", "user.name"), "Fixture Developer")
        self.assertEqual(self.git("config", "user.email"), "fixture-developer@example.test")
        self.git("add", "--all")
        self.git("commit", "--quiet", "-m", message)
        return self.git("rev-parse", "HEAD")

    def identities(self, revision):
        return {path: self.git("rev-parse", "--verify", revision + ":" + path) for path in RUNTIME}

    def assert_rejected(self):
        with self.assertRaises((RuntimeError, subprocess.CalledProcessError)):
            self.gate.revision(self.root)

    def test_qualified_application_and_operations_descendant_return_application_sha(self):
        self.assertEqual(self.gate.revision(self.root), self.application)
        self.write("scripts/ops/operator.py", "print('operations repair')\n")
        descendant = self.commit("Repair operations workflow")
        self.assertNotEqual(descendant, self.application)
        self.assertEqual(self.gate.revision(self.root), self.application)

    def test_sibling_with_identical_application_inputs_is_rejected(self):
        self.git("checkout", "--quiet", "-b", "sibling", self.baseline)
        self.write("frontend-dapp/src/app.ts", "qualified application\n")
        self.write("scripts/ops/operator.py", "print('sibling operations')\n")
        sibling = self.commit("Prepare sibling revision")
        self.assertEqual(self.identities(sibling), self.record["runtime_inputs"])
        self.assert_rejected()

    def test_unrelated_history_with_identical_application_inputs_is_rejected(self):
        self.git("checkout", "--quiet", "--orphan", "unrelated")
        unrelated = self.commit("Independent release inputs")
        self.assertEqual(self.identities(unrelated), self.record["runtime_inputs"])
        self.assert_rejected()

    def test_committed_application_change_is_rejected(self):
        self.write("frontend-dapp/src/app.ts", "different application\n")
        self.commit("Change application input")
        self.assert_rejected()

    def test_dirty_application_change_is_rejected(self):
        self.write("indexer/src/main.rs", "fn main() { panic!(); }\n")
        self.assertEqual(self.identities("HEAD"), self.record["runtime_inputs"])
        self.assert_rejected()

    def test_untracked_application_input_is_rejected(self):
        self.write("frontend-dapp/src/new-input.ts", "new untracked application input\n")
        self.assertTrue(self.git("status", "--porcelain", "--", "frontend-dapp/src/new-input.ts").startswith("??"))
        self.assert_rejected()

    def test_committed_contract_source_change_is_rejected(self):
        self.write("smartcontracts/contracts/token/src/lib.rs", "pub fn changed() {}\n")
        self.commit("Change contract source")
        self.assert_rejected()

    def test_dirty_contract_source_change_is_rejected(self):
        self.write("smartcontracts/contracts/token/src/lib.rs", "pub fn changed() {}\n")
        self.assert_rejected()

    def test_untracked_contract_source_is_rejected(self):
        path = "smartcontracts/contracts/token/src/new-input.rs"
        self.write(path, "pub fn untracked_input() {}\n")
        self.assertTrue(self.git("status", "--porcelain", "--", path).startswith("??"))
        self.assert_rejected()

    def test_committed_contract_build_recipe_change_is_rejected(self):
        self.write("smartcontracts/scripts/optimize.sh", "#!/bin/sh\nexit 1\n")
        self.commit("Change contract build recipe")
        self.assert_rejected()

    def test_dirty_contract_build_recipe_change_is_rejected(self):
        self.write("smartcontracts/scripts/optimize.sh", "#!/bin/sh\nexit 1\n")
        self.assert_rejected()

    def test_prepared_application_cannot_include_a_contract_source_change(self):
        self.write("smartcontracts/contracts/token/src/lib.rs", "pub fn changed() {}\n")
        self.record["revision"] = self.commit("Include contract source change")
        self.gate.RECORD.write_text(json.dumps(self.record))
        self.assert_rejected()

    def test_prepared_application_cannot_include_a_contract_build_recipe_change(self):
        self.write("smartcontracts/scripts/optimize.sh", "#!/bin/sh\nexit 1\n")
        self.record["revision"] = self.commit("Include contract build recipe change")
        self.gate.RECORD.write_text(json.dumps(self.record))
        self.assert_rejected()

    def test_indexer_container_recipe_change_is_rejected(self):
        self.write("docker/indexer/Dockerfile", "FROM different-fixture\n")
        self.commit("Change indexer container input")
        self.assert_rejected()

    def test_artifact_and_checksum_updates_are_allowed_by_application_gate(self):
        # Their byte/hash checks belong to ROUTE_COMMON, outside this source gate.
        for path in ("smartcontracts/artifacts/token.wasm", "smartcontracts/artifacts/checksums.txt",
                     "smartcontracts/wasm-checksums.txt"):
            self.write(path, "reviewed replacement artifact identity\n")
        self.commit("Update reviewed artifact identities")
        self.assertEqual(self.gate.revision(self.root), self.application)
        self.write("smartcontracts/artifacts/token.wasm", "pending replacement artifact\n")
        self.write("smartcontracts/wasm-checksums.txt", "pending replacement digest\n")
        self.assertEqual(self.gate.revision(self.root), self.application)

    def test_untracked_compiled_artifact_is_allowed_by_application_gate(self):
        path = "smartcontracts/artifacts/new-token.wasm"
        self.write(path, "new reviewed artifact; hash validation is separate\n")
        self.assertTrue(self.git("status", "--porcelain", "--", path).startswith("??"))
        self.assertEqual(self.gate.revision(self.root), self.application)

    def test_missing_runtime_identity_is_rejected(self):
        self.record["runtime_inputs"].pop("tokenlist")
        self.gate.RECORD.write_text(json.dumps(self.record))
        self.assert_rejected()


if __name__ == "__main__":
    unittest.main()
