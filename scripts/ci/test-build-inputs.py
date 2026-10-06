#!/usr/bin/env python3
"""Synthetic contamination/cleanup tests; never use host Cargo targets or vaults."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("provenance", ROOT / "scripts/ci/build-provenance.py")
PROVENANCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROVENANCE)
PIN_SPEC = importlib.util.spec_from_file_location("pins", ROOT / "scripts/ci/verify-image-pins.py")
PINS = importlib.util.module_from_spec(PIN_SPEC)
PIN_SPEC.loader.exec_module(PINS)
FIXTURE_SPEC = importlib.util.spec_from_file_location("fixtures", ROOT / "scripts/ci/build-locked-fixtures.py")
FIXTURES = importlib.util.module_from_spec(FIXTURE_SPEC)
FIXTURE_SPEC.loader.exec_module(FIXTURES)


class BuildInputs(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        (self.root / "smartcontracts").mkdir()
        (self.root / "smartcontracts/Cargo.lock").write_text("locked input\n")
        (self.root / "smartcontracts/source.rs").write_text("trusted source\n")
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=Test Developer",
                        "-c", "user.email=fixture@example.invalid", "commit", "-qm", "fixture"], check=True)
        self.artifact = self.root / "output.wasm"
        self.artifact.write_bytes(b"fresh wasm")

    def receipt(self):
        return PROVENANCE.receipt(self.root, "smartcontracts", "smartcontracts/Cargo.lock",
                                  "image:fixture@sha256:" + "1" * 64, "rustc fixture", [self.artifact])

    def test_source_lock_and_output_tampering_are_detectable(self):
        original = self.receipt()
        (self.root / "smartcontracts/source.rs").write_text("modified source\n")
        changed = self.receipt()
        self.assertNotEqual(original["source_sha256"], changed["source_sha256"])
        self.assertTrue(changed["source_dirty"])
        (self.root / "smartcontracts/Cargo.lock").write_text("changed lock\n")
        self.assertNotEqual(original["lockfile"], self.receipt()["lockfile"])
        self.artifact.write_bytes(b"poisoned output")
        self.assertNotEqual(original["artifacts"], self.receipt()["artifacts"])

    def test_verifier_and_independent_comparison_fail_closed_on_tampering(self):
        original = self.receipt()
        PROVENANCE.verify(self.root, original, original["builder_image"])
        PROVENANCE.compare(original, self.receipt())
        self.artifact.write_bytes(b"tampered executable")
        with self.assertRaisesRegex(ValueError, "artifacts"):
            PROVENANCE.verify(self.root, original)
        with self.assertRaisesRegex(ValueError, "artifacts"):
            PROVENANCE.compare(original, self.receipt())
        with self.assertRaisesRegex(ValueError, "builder image"):
            PROVENANCE.verify(self.root, original, "different@sha256:" + "2" * 64)
        original["artifacts"] = {}
        with self.assertRaisesRegex(ValueError, "no artifact"):
            PROVENANCE.verify(self.root, original)

    def test_floating_images_cannot_disappear_from_pin_validation(self):
        path = self.root / "Dockerfile"
        path.write_text("FROM node:24-bookworm AS builder\n")
        references = PINS.declared_images(path)
        self.assertEqual(references, ["node:24-bookworm"])
        with self.assertRaisesRegex(ValueError, "floating"):
            PINS.validate_reference(references[0], {})
        path = self.root / ".gitlab-ci.yml"
        path.write_text("variables:\n  GITLEAKS_IMAGE: gitleaks:latest\njob:\n  image:\n"
                        "    name: $GITLEAKS_IMAGE\n  services:\n    - name: postgres:16\n")
        references = PINS.declared_images(path)
        self.assertEqual(references, ["gitleaks:latest", "postgres:16"])
        for reference in references:
            with self.assertRaisesRegex(ValueError, "floating"):
                PINS.validate_reference(reference, {})
        optimizer = self.root / "optimize.sh"
        optimizer.write_text("docker run unpinned:latest\n")
        with self.assertRaisesRegex(ValueError, "exactly one"):
            PINS.declared_images(optimizer)

    def test_external_fixture_sources_are_exact_locked_commits(self):
        for name in ["cw20-mintable", "treasury"]:
            url, revision = FIXTURES.locked_source(ROOT / "smartcontracts/Cargo.lock", name)
            self.assertTrue(url.startswith("https://"))
            self.assertRegex(revision, r"^[0-9a-f]{40}$")
        lock = self.root / "floating.lock"
        lock.write_text('[[package]]\nname = "fixture"\nsource = "git+https://example.invalid/project#main"\n')
        with self.assertRaisesRegex(ValueError, "locked Git commit"):
            FIXTURES.locked_source(lock, "fixture")
        lock.write_text('version = 4\n[[package]]\nname = "fixture"\nversion = "1.0.0"\n')
        self.assertEqual(FIXTURES.prepare_legacy_lock(lock), "4-to-3")
        self.assertTrue(lock.read_text().startswith("version = 3\n"))
        self.assertIn('version = "1.0.0"', lock.read_text())

    def test_external_optimizer_failure_cleans_disposable_volumes(self):
        binary = self.root / "bin"
        binary.mkdir()
        logfile = self.root / "docker-calls.jsonl"
        fake = binary / "docker"
        fake.write_text("#!/usr/bin/env python3\nimport json,os,sys\n"
                        "with open(os.environ['DOCKER_TEST_LOG'], 'a') as f: "
                        "f.write(json.dumps(sys.argv[1:])+'\\n')\n"
                        "sys.exit(17 if sys.argv[1]=='run' else 0)\n")
        fake.chmod(0o755)
        result = subprocess.run(["bash", str(ROOT / "scripts/ci/optimize-project.sh"),
                                 str(self.root / "smartcontracts")],
                                env={**os.environ, "PATH": str(binary) + ":" + os.environ["PATH"],
                                     "DOCKER_TEST_LOG": str(logfile)})
        self.assertEqual(result.returncode, 17)
        calls = [json.loads(line) for line in logfile.read_text().splitlines()]
        run = next(call for call in calls if call[0] == "run")
        self.assertTrue(any("target=/target" in argument for argument in run))
        self.assertTrue(any("target=/code,readonly" in argument for argument in run))
        self.assertTrue(any("@sha256:" in argument for argument in run))
        self.assertTrue(any(call[:2] == ["volume", "rm"] for call in calls))

    def test_untracked_source_is_included_and_executables_are_excluded(self):
        original = self.receipt()
        (self.root / "smartcontracts/target").mkdir()
        (self.root / "smartcontracts/target/poisoned").write_text("cached executable")
        self.assertEqual(original["source_sha256"], self.receipt()["source_sha256"])
        (self.root / "smartcontracts/new.rs").write_text("untracked source")
        self.assertNotEqual(original["source_sha256"], self.receipt()["source_sha256"])

    def test_optimizer_failure_cleans_fresh_volumes_and_overlays_actual_target(self):
        scripts = self.root / "smartcontracts/scripts"
        scripts.mkdir()
        shutil.copy(ROOT / "smartcontracts/scripts/optimize.sh", scripts / "optimize.sh")
        shared = self.root / "scripts/ci"
        shared.mkdir(parents=True)
        shutil.copy(ROOT / "scripts/ci/build-provenance.py", shared / "build-provenance.py")
        binary = self.root / "bin"
        binary.mkdir()
        logfile = self.root / "docker-calls.jsonl"
        fake = binary / "docker"
        fake.write_text("#!/usr/bin/env python3\nimport json,os,sys\n"
                        "with open(os.environ['DOCKER_TEST_LOG'], 'a') as f: "
                        "f.write(json.dumps(sys.argv[1:])+'\\n')\n"
                        "sys.exit(17 if sys.argv[1]=='run' else 0)\n")
        fake.chmod(0o755)
        env = {**os.environ, "PATH": str(binary) + ":" + os.environ["PATH"],
               "DOCKER_TEST_LOG": str(logfile)}
        for _ in range(2):
            result = subprocess.run(["bash", str(scripts / "optimize.sh")], env=env,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            self.assertEqual(result.returncode, 17)
        calls = [json.loads(line) for line in logfile.read_text().splitlines()]
        runs = [call for call in calls if call[0] == "run"]
        self.assertEqual(len(runs), 2)
        self.assertNotEqual(runs[0][runs[0].index("--name") + 1],
                            runs[1][runs[1].index("--name") + 1])
        for run in runs:
            self.assertTrue(any("target=/target" in arg for arg in run))
            self.assertTrue(any("target=/code/target" in arg for arg in run))
            self.assertTrue(any("target=/code,readonly" in arg for arg in run))
            self.assertTrue(any("@sha256:" in arg for arg in run))
        self.assertEqual(len([call for call in calls if call[:2] == ["volume", "rm"]]), 2)

    def test_fresh_indexer_build_replaces_poisoned_binary_and_cleans_target(self):
        indexer = self.root / "indexer"
        (indexer / "target/release").mkdir(parents=True)
        (indexer / "Cargo.lock").write_text("locked input")
        (indexer / "target/release/cl8y-dex-indexer").write_bytes(b"poison")
        scripts = self.root / "scripts/ci"
        scripts.mkdir(parents=True)
        for name in ["build-indexer-release.sh", "build-provenance.py"]:
            shutil.copy(ROOT / "scripts/ci" / name, scripts / name)
        binary = self.root / "bin"
        binary.mkdir()
        build_log = self.root / "target-path"
        (binary / "cargo").write_text(
            "#!/usr/bin/env python3\nimport os,pathlib,sys\n"
            "if sys.argv[1]=='--version': print('cargo fixture');sys.exit()\n"
            "assert '--locked' in sys.argv and '--release' in sys.argv\n"
            "p=pathlib.Path(sys.argv[sys.argv.index('--target-dir')+1])\n"
            "assert [f.name for f in p.iterdir()]==['inputs.json']\n"
            "pathlib.Path(os.environ['TARGET_TEST_LOG']).write_text(str(p))\n"
            "(p/'release').mkdir();(p/'release/cl8y-dex-indexer').write_bytes(b'fresh output')\n")
        (binary / "cargo").chmod(0o755)
        (binary / "rustc").write_text("#!/bin/sh\necho 'rustc fixture'\n")
        (binary / "rustc").chmod(0o755)
        subprocess.run(["bash", str(scripts / "build-indexer-release.sh")], check=True,
                       env={**os.environ, "PATH": str(binary) + ":" + os.environ["PATH"],
                            "TARGET_TEST_LOG": str(build_log)})
        self.assertEqual((indexer / "target/release/cl8y-dex-indexer").read_bytes(), b"fresh output")
        self.assertFalse(Path(build_log.read_text()).exists())
        receipt = json.loads((indexer / "target/release/build-provenance.json").read_text())
        self.assertEqual(receipt["cache_policy"], "fresh-executable-target")


if __name__ == "__main__":
    unittest.main()
