#!/usr/bin/env python3
"""Offline safety/idempotency checks of the actual deployment script sections."""
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = (ROOT / "scripts/deploy-usdt-ust1-lp.sh").read_text()
SETUP = SOURCE[SOURCE.index('echo "[multisig]'):SOURCE.index('admin_usdt=')]
PREFLIGHT = SOURCE[SOURCE.index('USDT_CODE='):SOURCE.index('for id in "$USDT_CODE"')]
CAP = "10000000000000000000000"
LIMITS = dict(max_per_issue=CAP, max_outstanding=CAP, epoch_seconds=86400, max_per_epoch=CAP)


class DeploymentTests(unittest.TestCase):
    def run_setup(self, market=None, new_market=False, query_error="", legacy_config=False, **changes):
        market = market or dict(debt_token="debt", pins=dict(underlying_code_id=10184, debt_code_id=12),
                                limits=LIMITS, paused=False, pending_upgrade=None)
        market.update(changes)
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "transactions"
            env = f"""set -euo pipefail
LIVE=1 ISSUER=issuer DEBT=debt DEBT_CODE_ID=12 USDT=usdt USDT_CODE=10184 REVIEWED_USDT_CODE_ID=10184
ADMIN_ADDR=admin USDT_RAW={CAP}
MARKET='{json.dumps(market)}'
REGISTERED={0 if new_market else 1}
QUERY_ERROR='{query_error}'
CONFIG_JSON='{json.dumps(dict(debt_code_id=None) if legacy_config else dict(debt_code_id=None, pending_owner=None))}'
die() {{ echo "$*" >&2; exit 1; }}
lcd_smart() {{ jq '.data' <<<"$1"; }}
lcd_smart_raw() {{
  case "$2" in
    *'"market"'*)
      if [[ -n "$QUERY_ERROR" ]]; then echo "$QUERY_ERROR"; return 1; fi
      if [[ "$REGISTERED" == 0 ]]; then echo 'unknown market'; return 1; fi
      jq -nc --argjson m "$MARKET" '{{data:$m}}';;
    *'"minters"'*) echo '{{"data":{{"minters":[]}}}}';;
    *'"minter"'*) echo '{{"data":{{"minter":"primary","cap":null}}}}';;
    *'"config"'*) jq -nc --argjson c "$CONFIG_JSON" '{{data:$c}}';;
    *) return 1;;
  esac
}}
msig_exec() {{
  printf '%s\\n' "$2" >> '{log}'
  if [[ "$2" == *'"register_market"'* ]]; then REGISTERED=1; fi
}}
"""
            result = subprocess.run(["bash", "-c", env + SETUP], capture_output=True, text=True)
            txs = log.read_text().splitlines() if log.exists() else []
            return result, [json.loads(tx) for tx in txs]

    def test_existing_market_is_read_only(self):
        result, txs = self.run_setup()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(txs, [])

    def test_paused_pending_or_drifted_market_never_mutates(self):
        for changes in ({"paused": True, "limits": None}, {"pending_upgrade": {}},
                        {"pins": {"underlying_code_id": 7, "debt_code_id": 12}},
                        {"debt_token": "different"}, {"limits": {**LIMITS, "max_per_issue": "1"}}):
            with self.subTest(changes=changes):
                result, txs = self.run_setup(**changes)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(txs, [])

    def test_unconfigured_market_gets_exact_finite_limits(self):
        result, txs = self.run_setup(limits=None)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(txs, [{"configure_market": {"underlying": "usdt", "limits": LIMITS, "paused": False}}])

    def test_new_market_approval_and_limits_are_ordered(self):
        result, txs = self.run_setup(new_market=True, limits=None)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([next(iter(tx)) for tx in txs], [
            "add_minter", "set_debt_code_id", "approve_underlying_code", "register_market", "configure_market"])
        self.assertEqual(txs[2], {"approve_underlying_code": {"code_id": 10184, "multi_minter": True}})

    def test_transport_or_contract_missing_error_cannot_trigger_setup(self):
        for error in ("contract not found", "404 page not found", "503 unavailable"):
            result, txs = self.run_setup(query_error=error)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(txs, [])

    def test_legacy_issuer_cannot_start_new_market_setup_mutations(self):
        result, txs = self.run_setup(new_market=True, legacy_config=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(txs, [])

    def test_live_code_requires_independent_review_input(self):
        for observed in (10184, 99999):
            env = f"""set -euo pipefail
LCD=http://offline USDT=usdt UST1=ust1 REVIEWED_USDT_CODE_ID=10184
lcd_get() {{ echo '{{"contract_info":{{"code_id":"{observed}"}}}}'; }}
die() {{ echo "$*" >&2; exit 1; }}
"""
            result = subprocess.run(["bash", "-c", env + PREFLIGHT], capture_output=True, text=True)
            self.assertEqual(result.returncode == 0, observed == 10184, result.stderr)


if __name__ == "__main__":
    unittest.main()
