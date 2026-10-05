#!/usr/bin/env python3
"""Hermetic negative checks for the mandatory runner, without a chain or secrets."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


RUNNER = Path(__file__).with_name("functional-run.sh").resolve()
JOBS = ("gitleaks", "contracts", "frontend", "indexer-lib", "indexer-postgres")


class FunctionalRunnerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name in ("smartcontracts", "frontend-dapp", "indexer", "bin"):
            (self.root / name).mkdir()
        (self.root / "scripts/ci").mkdir(parents=True)
        for asset in (".woodpecker.yaml", ".gitleaks.toml", "scripts/ci/functional-run.sh",
                      "scripts/ci/gitleaks-scan-tracked.sh"):
            (self.root / asset).write_text("reviewed policy source mirror\n")
        self.env = {
            **os.environ,
            "CI_WORKSPACE": str(self.root),
            "CI_COMMIT_SHA": "a" * 40,
            "PATH": str(self.root / "bin") + ":" + os.environ["PATH"],
            "RUSTUP_HOME": str(self.root / "no-toolchain"),
            "TEST_DATABASE_URL": "postgres://cl8y_ci:cl8y_ci@postgres:5432/dex_indexer_ci_test",
            "GITLEAKS_SCAN_SCRIPT": str(self.root / "bin" / "scanner"),
        }
        self.stub("cargo", '[ "${CARGO_FAIL:-0}" != 1 ]')
        self.stub("npm", '[ "${NPM_FAIL:-0}" != 1 ] && mkdir -p dist')
        app_bins = self.root / "frontend-dapp" / "node_modules" / ".bin"
        app_bins.mkdir(parents=True)
        self.stub("eslint", 'exit 0', directory=app_bins)
        self.stub("vitest", '[ "${VITEST_FAIL:-0}" != 1 ]', directory=app_bins)
        self.stub("tsc", 'exit 0', directory=app_bins)
        self.stub("vite", 'mkdir -p dist', directory=app_bins)
        self.stub("scanner", '[ "${SCANNER_FAIL:-0}" != 1 ]')
        self.stub("pg_isready", "exit 0")
        self.run_job("init")

    def stub(self, name, command, *, directory=None):
        path = (directory or self.root / "bin") / name
        path.write_text("#!/bin/sh\nset -eu\n" + command + "\n")
        path.chmod(0o755)

    def run_job(self, name, *, success=True, **env):
        result = subprocess.run(
            ["bash", str(RUNNER), name], cwd=self.root,
            env={**self.env, **env}, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return result

    def fill(self):
        for job in JOBS:
            self.run_job(job)

    def test_complete_matrix_passes(self):
        self.fill()
        self.run_job("aggregate")

    def test_deleting_policy_mirrors_fails_before_source_execution(self):
        for asset in (".woodpecker.yaml", ".gitleaks.toml", "scripts/ci/functional-run.sh",
                      "scripts/ci/gitleaks-scan-tracked.sh"):
            with self.subTest(asset=asset):
                path = self.root / asset
                path.unlink()
                self.run_job("init", success=False)
                path.write_text("reviewed policy source mirror\n")

    def test_every_missing_job_fails(self):
        self.fill()
        for job in JOBS:
            with self.subTest(job=job):
                path = self.root / ".ci-functional" / job
                receipt = path.read_text()
                path.unlink()
                self.run_job("aggregate", success=False)
                path.write_text(receipt)

    def test_deliberate_contract_frontend_indexer_failures_do_not_write_success(self):
        self.fill()
        for job, failure in (("contracts", "CARGO_FAIL"), ("frontend", "VITEST_FAIL"),
                             ("indexer-lib", "CARGO_FAIL"), ("indexer-postgres", "CARGO_FAIL"),
                             ("gitleaks", "SCANNER_FAIL")):
            with self.subTest(job=job):
                self.run_job(job, success=False, **{failure: "1"})
                self.assertFalse((self.root / ".ci-functional" / job).exists())
                self.run_job("aggregate", success=False)
                self.run_job(job)

    def test_npm_script_aliases_cannot_bypass_failing_frontend_assertions(self):
        (self.root / "frontend-dapp" / "package.json").write_text(
            '{"scripts":{"lint":"true","test:run":"true","build":"true"}}\n'
        )
        self.run_job("frontend", success=False, VITEST_FAIL="1")
        self.assertFalse((self.root / ".ci-functional" / "frontend").exists())
        self.run_job("aggregate", success=False)

    def test_revision_and_prior_run_receipts_rejected(self):
        self.fill()
        path = self.root / ".ci-functional" / "frontend"
        old = path.read_text()
        self.run_job("aggregate", success=False, CI_COMMIT_SHA="b" * 40)
        self.run_job("init")
        self.fill()
        path.write_text(old)
        self.run_job("aggregate", success=False)

    def test_missing_initialization_and_unknown_jobs_fail(self):
        self.run_job("unknown", success=False)
        (self.root / ".ci-functional" / "run").unlink()
        self.run_job("contracts", success=False)
        self.run_job("aggregate", success=False)

    def test_host_database_and_dotenv_override_rejected(self):
        self.run_job("indexer-postgres", success=False, TEST_DATABASE_URL="postgres://localhost/prod")
        (self.root / "indexer" / ".env").write_text("TEST_DATABASE_URL=postgres://localhost/prod\n")
        self.run_job("indexer-postgres", success=False)

    def test_symlinked_receipt_rejected(self):
        self.fill()
        path = self.root / ".ci-functional" / "contracts"
        receipt = path.read_text()
        path.unlink()
        outside = self.root / "copied-receipt"
        outside.write_text(receipt)
        path.symlink_to(outside)
        self.run_job("aggregate", success=False)

    def test_executable_targets_are_fresh_and_outside_checkout(self):
        poison = self.root / "smartcontracts" / "target" / "synthetic-poison"
        poison.parent.mkdir()
        poison.write_text("UNTRUSTED SYNTHETIC MARKER")
        self.stub("cargo", 'printf "%s %s\\n" "$CARGO_HOME" "$CARGO_TARGET_DIR" >> "$CI_WORKSPACE/paths"')
        self.run_job("contracts")
        self.run_job("contracts")
        paths = (self.root / "paths").read_text().splitlines()
        self.assertNotEqual(paths[0], paths[2], "another invocation reused executable state")
        for line in paths:
            for value in line.split():
                self.assertFalse(Path(value).is_relative_to(self.root))
                self.assertFalse(Path(value).exists(), "scratch target/cache left behind")
        self.assertEqual(poison.read_text(), "UNTRUSTED SYNTHETIC MARKER")


if __name__ == "__main__":
    unittest.main()
