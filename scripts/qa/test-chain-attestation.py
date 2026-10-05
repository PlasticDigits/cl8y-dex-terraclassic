#!/usr/bin/env python3
"""Treasury network/preparation regressions: synthetic fixtures, no custody."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from types import SimpleNamespace
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "scripts/qa/fixtures/chain-attestation-curl.py"
HELPER = ROOT / "scripts/lib/chain-attestation.py"
FINANCIAL = (
    "mint-swap-custc-ust1.sh", "mint-swap-burn-ust1-clunc.sh", "rebalance-mint-ust1-lp.sh",
    "rebalance-mint-clunc-custc-lp.sh", "rebalance-oracle-mint-swap-burn.sh", "mint-clunc-usdt-lp.sh",
    "rotate-fee-treasury.sh",
)


class TreasuryAttestation(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.work = Path(self.directory.name)
        curl = self.work / "curl"
        curl.write_text(f'''#!/usr/bin/env python3
import runpy, sys
fixture = runpy.run_path({str(FIXTURE)!r})
url = next(arg for arg in sys.argv[1:] if arg.startswith(("http://", "https://")))
if not fixture["respond"](url):
    raise SystemExit("unexpected financial HTTP read before preflight")
''')
        curl.chmod(0o755)
        terrad = self.work / "terrad"
        terrad.write_text('''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
with open(os.environ["MOCK_TX_LOG"], "a") as file:
    file.write(json.dumps(sys.argv[1:]) + "\\n")
if os.environ.get("MOCK_TX_TRANSIENT"):
    print("503 temporarily unavailable", file=sys.stderr)
    raise SystemExit(1)
print('{"txhash":"FIXTURE","code":0}')
''')
        terrad.chmod(0o755)
        self.env = {k: v for k, v in os.environ.items()
                    if not k.startswith(("TERRAD_HOST_", "MOCK_", "UST1_", "CUSTC_", "CLUNC_", "ORACLE_RB_", "ROTATE_TREASURY_"))}
        self.env.update(PATH=f"{self.work}:{self.env['PATH']}",
                        TERRAD_HOST_LCD_URL="https://lcd.example", TERRAD_HOST_NODE="https://rpc.example",
                        TERRAD_HOST_CHAIN_ID="columbus-5", TERRAD_HOST_ATTEST_TIMEOUT_SEC="2",
                        MOCK_TX_LOG=str(self.work / "transactions"), DRY_RUN="1",
                        TERRAD_HOST_TX_RETRY_DELAY_SEC="0", TERRAD_HOST_TX_RETRIES="2")

    def tearDown(self):
        self.directory.cleanup()

    def run_shell(self, code):
        return subprocess.run(["bash", "-c", "set -euo pipefail; source scripts/lib/terrad-host.sh; " + code],
                              cwd=ROOT, env=self.env, text=True, capture_output=True, timeout=30)

    def run_helper(self, *extra):
        return subprocess.run(["python3", str(HELPER), "--lcd", "https://lcd.example", "--rpc", self.env["TERRAD_HOST_NODE"],
                               "--expected-chain", self.env["TERRAD_HOST_CHAIN_ID"], "--timeout", "2", *extra],
                              env=self.env, text=True, capture_output=True, timeout=30)

    def assert_no_tx(self):
        self.assertFalse((self.work / "transactions").exists())

    def test_correct_and_explicit_localterra_with_code_corroboration(self):
        for chain in ("columbus-5", "localterra"):
            self.env.update(TERRAD_HOST_CHAIN_ID=chain, MOCK_EXPECTED_CHAIN=chain)
            result = self.run_helper("--contracts", "terra1fixture")
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads(result.stdout)
            self.assertEqual(receipt["expected_chain_id"], chain)
            self.assertEqual(receipt["observed_chain_id"], chain)
            self.assertEqual(receipt["height"], 100)
            self.assertEqual(receipt["identities"]["terra1fixture"], {"code_id": 1, "checksum": "ab" * 32})
            result = self.run_shell("TERRAD_HOST_IDENTITY_CONTRACTS=terra1fixture; terrad_host_tx wasm execute terra1fixture '{}' ")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("DRY_RUN_TX", result.stdout)
            self.assert_no_tx()

    def test_bad_identity_stale_views_and_bad_protocol_prevent_all_tx_stages(self):
        for mode in ("wrong", "missing", "malformed", "unavailable", "stale", "height-lag", "hash-mismatch", "catching-up", "code-mismatch"):
            for stage in ("wasm execute terra1fixture '{}' --generate-only", "sign unsigned.json", "multisign unsigned.json key", "broadcast signed.json"):
                with self.subTest(mode=mode, stage=stage):
                    self.env["MOCK_CHAIN_MODE"] = mode
                    result = self.run_shell(f"TERRAD_HOST_IDENTITY_CONTRACTS=terra1fixture; terrad_host_exec tx {stage}")
                    self.assertNotEqual(result.returncode, 0)
                    self.assert_no_tx()
            result = self.run_shell("TERRAD_HOST_IDENTITY_CONTRACTS=terra1fixture; terrad_host_tx wasm execute terra1fixture '{}'")
            self.assertNotEqual(result.returncode, 0, mode)
            self.assert_no_tx()

    def test_each_failed_over_endpoint_is_checked_and_selected_used(self):
        self.env.update(TERRAD_HOST_NODE="https://wrong.example", TERRAD_HOST_RPC_FALLBACKS="https://down.example,https://rpc.example")
        result = self.run_shell("terrad_host_exec tx sign fixture.json --node https://wrong.example")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("candidate 1 rejected", result.stderr)
        self.assertIn("candidate 2 rejected", result.stderr)
        args = json.loads((self.work / "transactions").read_text())
        for index, value in enumerate(args):
            if value == "--node":
                self.assertEqual(args[index + 1], "https://rpc.example")

    def test_path_credentials_are_private_and_sparse_fallback_index_is_preserved(self):
        secret = "SYNTHETIC_PROVIDER_CREDENTIAL_1392"
        verified = "https://rpc.example/" + secret
        self.env.update(TERRAD_HOST_NODE="https://wrong.example",
                        TERRAD_HOST_RPC_FALLBACKS=f"https://wrong.example,, {verified} ,{verified}",
                        MOCK_RPC_PATH_PREFIX="/" + secret)
        result = self.run_helper("--fallbacks", self.env["TERRAD_HOST_RPC_FALLBACKS"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["rpc_candidate_index"], 3)
        self.assertNotIn(secret, result.stdout + result.stderr)
        self.assertNotIn('"rpc":', result.stdout)
        result = self.run_shell("terrad_host_exec tx sign fixture.json")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(secret, result.stdout + result.stderr)
        args = json.loads((self.work / "transactions").read_text())
        for index, value in enumerate(args):
            if value == "--node":
                self.assertEqual(args[index + 1], verified)

    def test_failed_override_and_signing_chain_override_stop_before_tx(self):
        for flag in ("--node https://wrong.example", "--node=https://wrong.example", "--chain-id wrong", "--chain-id=wrong"):
            result = self.run_shell(f"terrad_host_exec tx sign fixture.json {flag}")
            self.assertNotEqual(result.returncode, 0)
            self.assert_no_tx()
        self.env["TERRAD_HOST_EXPECTED_CHAIN_ID"] = "localterra"
        result = self.run_shell("terrad_host_tx wasm execute terra1fixture '{}'")
        self.assertNotEqual(result.returncode, 0)
        self.assert_no_tx()

    def test_restart_and_recovery_rechecks_identity(self):
        self.env["MOCK_CHAIN_MODE"] = "wrong"
        self.assertNotEqual(self.run_helper().returncode, 0)
        self.env["MOCK_CHAIN_MODE"] = "good"
        self.assertEqual(self.run_helper().returncode, 0)
        self.env["MOCK_CHAIN_MODE"] = "wrong"
        self.assertNotEqual(self.run_helper().returncode, 0)

    def test_per_run_pin_and_reviewed_pin_reject_changed_code(self):
        result = self.run_shell("TERRAD_HOST_IDENTITY_CONTRACTS=terra1fixture; terrad_host_attest; export MOCK_CODE_ID=2; terrad_host_exec tx sign fixture.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("changed", result.stderr)
        self.assert_no_tx()
        pins = self.work / "pins.json"
        pins.write_text(json.dumps({"terra1fixture": {"code_id": 2, "checksum": "ab" * 32}}))
        result = self.run_helper("--pins", str(pins))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("pinned identity mismatch", result.stderr)
        pins.write_text("{}")
        self.assertEqual(self.run_helper("--pins", str(pins)).returncode, 0)

    def test_retry_reconnect_rejects_rotated_chain_before_second_transaction(self):
        self.env.update(MOCK_ATTEST_COUNTER=str(self.work / "count"), MOCK_ATTEST_FAIL_AFTER="1", MOCK_TX_TRANSIENT="1")
        result = self.run_shell("terrad_host_exec_retry tx sign fixture.json")
        self.assertNotEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len((self.work / "transactions").read_text().splitlines()), 1)
        self.assertIn("RPC network mismatch", result.stderr)

    def test_named_scripts_reject_before_financial_reads_and_prepare_in_dry_and_live(self):
        self.env["MOCK_CHAIN_MODE"] = "wrong"
        self.env["ROTATE_TREASURY_SKIP_STORE"] = "1"
        for name in FINANCIAL:
            for dry in ("0", "1"):
                with self.subTest(script=name, dry=dry):
                    self.env["DRY_RUN"] = dry
                    result = subprocess.run(["bash", str(ROOT / "scripts" / name)], env=self.env,
                                            text=True, capture_output=True, timeout=30)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("chain-attestation", result.stderr)
                    self.assertNotIn("unexpected financial HTTP read", result.stderr)
                    self.assert_no_tx()

    def test_slow_protocol_reads_cannot_outlive_fresh_header(self):
        spec = importlib.util.spec_from_file_location("chain_attestation", HELPER)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        now = datetime.now(timezone.utc)
        block = {"block": {"header": {"chain_id": "columbus-5", "height": "100",
                                      "time": now.isoformat()}},
                 "block_id": {"hash": "ab" * 32}}
        status = {"result": {"node_info": {"network": "columbus-5"},
                             "sync_info": {"catching_up": False, "latest_block_height": "100"}}}
        args = SimpleNamespace(lcd="https://lcd.example", timeout=2,
                               expected_chain="columbus-5", max_age=120, max_lag=5,
                               contracts="terra1fixture", baseline={}, pins={})
        with mock.patch.object(module, "get_json", side_effect=[status, block, {"result": block},
                                                               block, {"result": block}]), \
             mock.patch.object(module, "contract_identity", return_value={"code_id": 1, "checksum": "ab" * 32}) as identity, \
             mock.patch.object(module, "datetime", wraps=datetime) as clock:
            clock.now.side_effect = [now] * 4 + [now + timedelta(seconds=121)]
            with self.assertRaisesRegex(module.AttestationError, "stale"):
                module.attest(args, "https://rpc.example")
            identity.assert_called_once()
        self.assert_no_tx()

    def test_bad_policy_and_empty_chain_fail_closed(self):
        for flag, value in (("--max-age", "0"), ("--max-lag", "-1"), ("--expected-chain", ""),
                            ("--expected-chain", "columbus-5\n"), ("--expected-chain", "x/y"),
                            ("--expected-chain", "x" * 129)):
            result = self.run_helper(flag, value)
            self.assertNotEqual(result.returncode, 0)
        self.assert_no_tx()

    def test_malformed_future_zero_overlarge_headers_and_missing_protocol_fail_closed(self):
        for mode in ("future", "zero-height", "invalid-height", "header-list", "missing-time", "oversized", "lcd-wrong", "missing-code"):
            with self.subTest(mode=mode):
                self.env["MOCK_CHAIN_MODE"] = mode
                result = self.run_shell("TERRAD_HOST_IDENTITY_CONTRACTS=terra1fixture; terrad_host_exec tx sign unsigned.json")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("chain-attestation", result.stderr)
                self.assertNotIn("Traceback", result.stderr)
                self.assert_no_tx()

    def test_same_code_id_changed_checksum_fails_reviewed_pin(self):
        pins = self.work / "pins.json"
        pins.write_text(json.dumps({"terra1fixture": {"code_id": 1, "checksum": "cd" * 32}}))
        self.env["TERRAD_HOST_IDENTITY_PINS"] = str(pins)
        result = self.run_shell("terrad_host_exec tx sign unsigned.json")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("pinned identity mismatch", result.stderr)
        self.assert_no_tx()


if __name__ == "__main__":
    unittest.main()
