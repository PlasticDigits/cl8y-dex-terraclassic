#!/usr/bin/env python3
"""Synthetic #1391 LP-consumer receipts: unhealthy input never reaches tx preparation."""
from datetime import datetime, timedelta, timezone
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))


def module(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / f"scripts/lib/{name}.py")
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


CLUNC = module("clunc-usdt-lp-math")
LP = module("clunc-custc-lp-math")


def quote(ticker):
    return {
        "ticker": ticker, "quote_asset": "FDUSD" if ticker == "vfdusd" else ticker.upper(),
        "price_usd": "0.00005" if ticker == "lunc" else "1" if ticker == "vfdusd" else "0.005",
        "quality": "fresh", "source_policy": "cex-quorum-v1", "quorum": 2,
        "source_count": 2, "accepted_sources": ["mexc", "coingecko"] if ticker == "vfdusd" else ["mexc", "kucoin"],
        "as_of": datetime.now(timezone.utc).isoformat(), "max_age_seconds": 300,
    }


def hub():
    return {"prices": [
        {"ticker": ticker, "price_usd": px, "quality": "external_quorum",
         "policy": "live-reserve-spot-v1", "as_of": datetime.now(timezone.utc).isoformat(),
         "max_age_seconds": 300, "asset_address": address}
        for ticker, px, address in (("custc", "0.005", "synthetic-custc"),
                                    ("lunc", "0.00005", "synthetic-clunc"))
    ]}


def unhealthy(ticker="lunc"):
    for key, value in (
        ("quality", "stale"), ("quality", "unavailable"), ("quality", "degraded"),
        ("as_of", (datetime.now(timezone.utc) - timedelta(seconds=301)).isoformat()),
        ("as_of", (datetime.now(timezone.utc) + timedelta(days=1)).isoformat()),
        ("as_of", None), ("as_of", "malformed"), ("source_count", 1),
        ("accepted_sources", ["binance", "spoofed"]), ("quote_asset", "SPOOF"),
        ("price_usd", "NaN"), ("price_usd", "Infinity"), ("price_usd", "0"),
        ("price_usd", "-1"), ("price_usd", "1e999"),
    ):
        bad = quote(ticker)
        bad[key] = value
        yield key, value, bad


class ConsumerTests(unittest.TestCase):
    def test_python_consumers_reject_unhealthy_quotes(self):
        self.assertTrue(CLUNC.parse_oracle(quote("lunc"), 300)["ok"])
        original = LP._http_json
        try:
            for key, value, bad in unhealthy():
                with self.subTest(key=key, value=value):
                    self.assertFalse(CLUNC.parse_oracle(bad, 1800)["ok"])
                    def fetch(url):
                        if url.endswith("/lunc"):
                            return bad
                        if url.endswith("/ustc"):
                            return quote("ustc")
                        return hub()
                    LP._http_json = fetch
                    self.assertFalse(LP.fetch_oracles({"skip_cross_check": True})["ok"])
        finally:
            LP._http_json = original

    def test_hub_freshness_and_canonical_identity(self):
        original = LP._http_json
        try:
            for key, value in (("asset_address", "spoof"), ("quality", "stale"),
                               ("as_of", "malformed"), ("max_age_seconds", None)):
                fixture = hub()
                fixture["prices"][0][key] = value
                LP._http_json = lambda url: fixture if url.endswith("hub-prices") else quote(url.rsplit("/", 1)[1])
                self.assertFalse(LP.fetch_oracles({"skip_cross_check": True,
                    "custc_addr": "synthetic-custc", "clunc_addr": "synthetic-clunc"})["ok"])
        finally:
            LP._http_json = original

    def test_real_shell_guards_never_prepare_unhealthy_transactions(self):
        fixture = {"lunc": quote("lunc"), "ustc": quote("ustc"), "vfdusd": quote("vfdusd"), "hub-prices": hub()}
        fixture["vfdusd"]["venus"] = {
            "quality": "fresh", "source": "venus_bsc", "vtoken": "0xc4ef4229fec74ccfe17b2bdef7715fac740ba0ba",
            "fetched_at": datetime.now(timezone.utc).isoformat(), "fdusd_per_vfdusd": "0.02",
        }
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(json.dumps(fixture[self.path.rsplit("/", 1)[1]]).encode())
            def log_message(self, *args):
                pass
        server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            cases = [
                ("mint-clunc-usdt-lp.sh", ("fetch_lunc_oracle", "broadcast_admin"), 'broadcast_admin test wasm execute synthetic-token "{}"', "lunc"),
                ("rebalance-mint-clunc-custc-lp.sh", ("fetch_oracles", "fetch_oracles_ok", "broadcast_from"), 'broadcast_from synthetic-key synthetic-wallet test wasm execute synthetic-token "{}"', "lunc"),
                ("rebalance-oracle-mint-swap-burn.sh", ("fetch_oracles", "fetch_oracles_ok", "oracle_prepare_guard", "broadcast_from"), 'broadcast_from synthetic-key synthetic-wallet test wasm execute synthetic-token "{}"', "lunc"),
                ("mint-swap-custc-ust1.sh", ("fetch_ustc_usd", "fetch_vfdusd_marks", "validate_financial_marks", "broadcast_admin"), 'broadcast_admin test wasm execute synthetic-token "{}"', "ustc"),
                ("mint-swap-burn-ust1-clunc.sh", ("fetch_lunc_usd", "validate_financial_marks", "broadcast_admin"), 'broadcast_admin test wasm execute synthetic-token "{}"', "lunc"),
                ("rebalance-mint-ust1-lp.sh", ("fetch_ustc_usd", "validate_financial_marks", "broadcast_admin"), 'broadcast_admin test wasm execute synthetic-token "{}"', "ustc"),
            ]
            for file, functions, invocation, ticker in list(cases):
                if file in ("mint-swap-custc-ust1.sh", "mint-swap-burn-ust1-clunc.sh", "rebalance-mint-ust1-lp.sh"):
                    cases.append((file, (*functions[:-1], "broadcast_msig"), 'broadcast_msig test synthetic-token "{}"', ticker))
                if file == "rebalance-oracle-mint-swap-burn.sh":
                    for name, call in (("broadcast_msig", 'broadcast_msig test synthetic-token "{}"'),
                                       ("generate_admin_unsigned", 'generate_admin_unsigned synthetic-token "{}" "$TEST_UNSIGNED"'),
                                       ("broadcast_admin_unsigned", 'broadcast_admin_unsigned test "$TEST_UNSIGNED"')):
                        cases.append((file, (*functions[:-1], name), call, ticker))
            for file, functions, invocation, ticker in cases:
                source = (ROOT / "scripts" / file).read_text()
                definitions = "\n".join(re.search(rf"^{name}\(\) \{{\n.*?^\}}\n\n", source, re.M | re.S)[0] for name in functions)
                with tempfile.TemporaryDirectory() as directory:
                    marker = Path(directory) / "prepared"
                    body = f'''set -euo pipefail
SCRIPT_DIR={ROOT / 'scripts'}
MATH_PY="$SCRIPT_DIR/lib/{'clunc-usdt' if file == 'mint-clunc-usdt-lp.sh' else 'clunc-custc'}-lp-math.py"
CLUNC_MATH="$SCRIPT_DIR/lib/clunc-custc-lp-math.py"
INDEXER=http://127.0.0.1:{server.server_port}
INDEXER_ORACLE="$INDEXER/api/v1/oracle/price/lunc"
source "$SCRIPT_DIR/lib/oracle-safety.sh"
CUSTC_UST1_INDEXER_USTC="$INDEXER/api/v1/oracle/price/ustc"
CUSTC_UST1_INDEXER_VFDUSD="$INDEXER/api/v1/oracle/price/vfdusd"
UST1_CLUNC_INDEXER_ORACLE="$INDEXER_ORACLE"
UST1_LP_INDEXER_ORACLE="$CUSTC_UST1_INDEXER_USTC"
MAX_AGE=300; HTTP_UA=synthetic-test; CROSS_TOL=0.03
CLUNC=synthetic-clunc; CUSTC=synthetic-custc
ADMIN_KEY=synthetic-key; ADMIN_ADDR=synthetic-wallet; MSIG_KEY=synthetic-msig; SIGNER1=synthetic-1; SIGNER2=synthetic-2; SWAP_GAS=5000000
LUNC_USD=0.00005; USTC_USD=0.005; lunc_usd=0.00005; ustc_usd=0.005; fdusd_usd=1; venus=0.02
CLUNC_USDT_ORACLE_RETRIES=1; CLUNC_LP_ORACLE_RETRIES=1; CLUNC_LP_SKIP_CROSS_CHECK=1; ORACLE_RB_SKIP_CROSS_CHECK=1; ORACLE_RB_ORACLE_RETRIES=1
# Chain identity is exercised by test-chain-attestation.py; accept it here
# so this fixture specifically proves independent oracle rejection.
terrad_host_attest() {{ :; }}
terrad_host_tx() {{ touch '{marker}'; echo '{{"txhash":"synthetic"}}'; }}
terrad_host_exec() {{ touch '{marker}'; echo '{{"txhash":"synthetic","code":0,"body":{{"messages":[{{"@type":"synthetic","contract":"synthetic-token"}}]}}}}'; }}
terrad_host_exec_retry() {{ terrad_host_exec "$@"; }}
terrad_host_common_flags() {{ :; }}
terrad_host_gas_flags() {{ :; }}
terrad_host_fee_flags() {{ :; }}
terrad_host_wait_tx_inclusion() {{ :; }}
TEST_UNSIGNED='{Path(directory) / "unsigned.json"}'
echo '{{"body":{{"messages":[{{"@type":"synthetic","contract":"synthetic-token"}}]}}}}' > "$TEST_UNSIGNED"
die() {{ echo "$*" >&2; exit 1; }}
{definitions}
{invocation}
'''
                    # Start from an accepted healthy planning quote; the next read degrades.
                    self.assertTrue(CLUNC.parse_oracle(quote("lunc"), 300)["ok"])
                    for key, value, bad in unhealthy(ticker):
                        fixture["lunc"] = quote("lunc")
                        fixture["ustc"] = quote("ustc")
                        fixture["hub-prices"] = hub()
                        fixture["vfdusd"]["as_of"] = datetime.now(timezone.utc).isoformat()
                        fixture["vfdusd"]["venus"]["fetched_at"] = datetime.now(timezone.utc).isoformat()
                        fixture[ticker] = bad
                        with self.subTest(file=file, key=key, value=value):
                            outcome = subprocess.run(["bash", "-c", body], capture_output=True, text=True)
                            self.assertNotEqual(outcome.returncode, 0, outcome.stdout + outcome.stderr)
                            self.assertFalse(marker.exists(), "unhealthy quote prepared a transaction")
                    fixture["lunc"] = quote("lunc")
                    fixture["ustc"] = quote("ustc")
                    fixture["hub-prices"] = hub()
                    fixture["vfdusd"]["as_of"] = datetime.now(timezone.utc).isoformat()
                    fixture["vfdusd"]["venus"]["fetched_at"] = datetime.now(timezone.utc).isoformat()
                    outcome = subprocess.run(["bash", "-c", body], capture_output=True, text=True)
                    self.assertEqual(outcome.returncode, 0, outcome.stdout + outcome.stderr)
                    self.assertTrue(marker.exists(), "healthy control did not reach mocked preparation")
        finally:
            server.shutdown()
            server.server_close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
