#!/usr/bin/env python3
"""Exercise buyback failure paths with mocked HTTP and terrad; never broadcast."""
import json
import os
from decimal import Decimal
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
USDT = "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4"

CURL = r'''#!/usr/bin/env python3
import base64, json, os, runpy, sys
from urllib.parse import urlsplit
url = next(arg for arg in sys.argv[1:] if arg.startswith("http"))
if runpy.run_path(os.environ["MOCK_CHAIN_FIXTURE"])["respond"](url):
    sys.exit(0)
mode = os.environ["MOCK_QUOTE_MODE"]
if "/route/solve/best" in url:
    if mode == "http-error":
        sys.exit(22)
    if mode == "empty":
        sys.exit(0)
    if mode == "invalid-json":
        print("upstream timeout")
        sys.exit(0)
    if mode == "missing-ops":
        print('{"error":"no route"}')
        sys.exit(0)
    ts = {
        "offer_asset_info":{"token":{"contract_addr":"terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch"}},
        "ask_asset_info":{"token":{"contract_addr":os.environ["CUSTC_UST1_UST1"]}},
    }
    if mode == "wrong-token":
        ts["ask_asset_info"]["token"]["contract_addr"] = "terra1wrong"
    if mode == "invalid-hook":
        ts["greedy"] = {"max_maker_fills":8}
    print(json.dumps({"router_operations":[{"terra_swap":ts}],
        "estimated_amount_out":"50000000000000000000", "hops":[{"pair":"terra1pair"}]}))
elif "/smart/" in url:
    query = json.loads(base64.b64decode(urlsplit(url).path.rsplit("/",1)[-1]))
    if "token_info" in query:
        custc = "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch" in url
        data = {"decimals":6 if custc else 18,"symbol":"cUSTC" if custc else "USDT","total_supply":"0"}
    elif "minter" in query:
        data = {"minter":"terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7"}
    elif "minters" in query:
        data = {"minters":[]}
    elif "hybrid_simulation" in query:
        data = {"return_amount":"50000000000000000000"}
    else:
        treasury = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2"
        balance = os.environ.get("MOCK_CMM_BALANCE","0") if query["balance"]["address"] == treasury else "0"
        data = {"balance":balance}
    print(json.dumps({"data":data}))
elif "/balances/" in url:
    print('{"balance":{"amount":"100000000000000"}}')
else:
    print('{"price_usd":"0.00573"}')
'''

TERRAD = r'''#!/usr/bin/env python3
import os, sys
if sys.argv[1:3] == ["keys","show"]:
    print("terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv")
    sys.exit(0)
with open(os.environ["MOCK_TX_LOG"],"a") as f:
    f.write("unexpected transaction command\n")
sys.exit(99)
'''

PREPARE_TERRAD = r'''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
args = sys.argv[1:]
if args[:2] == ["keys", "show"]:
    print("terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv")
    sys.exit(0)
stage = "generate" if "--generate-only" in args else args[1]
if stage == "sign":
    stage += ":" + args[args.index("--from") + 1]
node = args[args.index("--node") + 1]
counter = Path(os.environ["MOCK_PREPARE_CALLS"])
calls = json.loads(counter.read_text()) if counter.exists() else []
attempt = 1 + sum(call["stage"] == stage for call in calls)
calls.append({"stage": stage, "attempt": attempt, "node": node})
counter.write_text(json.dumps(calls))
if stage == "broadcast":
    print("mock broadcast deliberately disabled", file=sys.stderr)
    sys.exit(99)
mode = os.environ["MOCK_PREPARE_MODE"]
if mode == "permanent":
    print("unauthorized minter", file=sys.stderr)
    sys.exit(1)
if mode == "exhaust" or attempt <= (2 if stage == "generate" else 1):
    print("PARTIAL FAILED STDOUT")
    print("503 Service Unavailable: upstream temporarily unavailable", file=sys.stderr)
    sys.exit(1)
# Parse previous-stage files to catch partial failed stdout leaking into JSON.
for arg in args:
    if arg.endswith(".json"):
        json.loads(Path(arg).read_text())
print(json.dumps({"body": {"messages": [{}]}}))
'''


class BuybackFailures(unittest.TestCase):
    def run_tick(self, mode, *, live=False, loop=False, target_met=False, randomized=False, ticks=1,
                 prepare_mode=None, node_override=None):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            # End a failed loop after its first tick, without waiting an hour.
            sleep = '''#!/usr/bin/env python3
import os, sys
from pathlib import Path
print("mock sleep seconds=" + sys.argv[1], file=sys.stderr)
if int(sys.argv[1]) < 2700:
    raise SystemExit(0)
counter = Path(os.environ["MOCK_SLEEP_COUNT"])
count = int(counter.read_text()) + 1 if counter.exists() else 1
counter.write_text(str(count))
raise SystemExit(77 if count >= int(os.environ["MOCK_TICKS"]) else 0)
'''
            terrad = PREPARE_TERRAD if prepare_mode else TERRAD
            for name, body in (("curl", CURL), ("terrad", terrad), ("sleep", sleep)):
                executable = work / name
                executable.write_text(body)
                executable.chmod(0o755)
            env = {k: v for k, v in os.environ.items()
                   if not k.startswith(("CUSTC_UST1_", "TERRAD_HOST_")) and k != "DRY_RUN"}
            env.update(
                PATH=f"{work}:{env['PATH']}",
                DRY_RUN="0" if live else "1",
                CUSTC_UST1_LOOP="1" if loop else "0", CUSTC_UST1_YES="1",
                CUSTC_UST1_UST1=USDT, CUSTC_UST1_VFDUSD=USDT,
                CUSTC_UST1_VFDUSD_USD="1", CUSTC_UST1_MINT_USD="50",
                CUSTC_UST1_AMOUNT_JITTER_PERCENT="20" if randomized else "0",
                CUSTC_UST1_TARGET_USD="4000", CUSTC_UST1_DEST="cmm",
                CUSTC_UST1_STATE_FILE=str(work / "state.json"),
                TERRAD_HOST_KEYRING_BACKEND="test", TERRAD_HOST_KEYRING_PASS="fixture-only",
                MOCK_CHAIN_FIXTURE=str(ROOT / "scripts/qa/fixtures/chain-attestation-curl.py"),
                MOCK_QUOTE_MODE=mode, MOCK_TX_LOG=str(work / "transactions"),
                MOCK_SLEEP_COUNT=str(work / "sleeps"), MOCK_TICKS=str(ticks),
                MOCK_PREPARE_CALLS=str(work / "prepare.json"), MOCK_PREPARE_MODE=prepare_mode or "",
                TMPDIR=str(work),
                MOCK_CMM_BALANCE="4000000000000000000000" if target_met else "0",
            )
            if node_override:
                env["TERRAD_HOST_NODE"] = node_override
            result = subprocess.run(
                ["bash", str(ROOT / "scripts/mint-swap-custc-ust1.sh")],
                env=env, capture_output=True, text=True, timeout=240,
            )
            self.assertFalse((work / "transactions").exists(), result.stdout + result.stderr)
            self.prepare_calls = json.loads((work / "prepare.json").read_text()) if (work / "prepare.json").exists() else []
            return result

    def test_bad_quotes_stop_before_mint_in_dry_and_live_ticks(self):
        for live in (False, True):
            for loop in (False, True):
                for mode in ("http-error", "empty", "invalid-json", "missing-ops", "wrong-token", "invalid-hook"):
                    with self.subTest(live=live, loop=loop, mode=mode):
                        result = self.run_tick(mode, live=live, loop=loop)
                        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                        self.assertNotIn("would swap", result.stdout)
                        self.assertNotIn("→ msig", result.stderr)
                        if loop:
                            self.assertIn("tick failed", result.stderr)

    def test_valid_preview_builds_floor_and_quotes_before_mint(self):
        result = self.run_tick("valid")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("Slippage:   0.25%", result.stdout)
        self.assertIn("min_receive=49875000000000000000", result.stdout)
        self.assertIn("would transfer received UST1 to CMM", result.stdout)
        self.assertLess(result.stderr.index("GET "), result.stderr.index("→ msig"))

    def test_existing_cmm_target_exits_without_mint_or_quote(self):
        result = self.run_tick("http-error", target_met=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("target reached", result.stdout)
        self.assertNotIn("GET ", result.stderr)

    def test_each_tick_uses_random_budget_and_relative_wait(self):
        result = self.run_tick("valid", loop=True, randomized=True, ticks=2)
        self.assertEqual(result.returncode, 77, result.stdout + result.stderr)
        batches = re.findall(r"\[mint\] \$([0-9.]+) → ([0-9]+) cUSTC raw", result.stdout)
        self.assertEqual(len(batches), 2, result.stdout)
        for usd_text, raw_text in batches:
            usd, raw = Decimal(usd_text), int(raw_text)
            self.assertTrue(Decimal("40") <= usd <= Decimal("60"))
            # Raw mint/quote amounts must match this tick's randomized USD budget.
            self.assertLessEqual(Decimal(raw) * Decimal("0.00573") / 10**6, usd)
            self.assertGreater(Decimal(raw + 1) * Decimal("0.00573") / 10**6, usd)
            self.assertIn(f"amount_in={raw}&", result.stderr)
        waits = re.findall(r"mock sleep seconds=(\d+)", result.stderr)
        logged_waits = re.findall(r"sleep (\d+)s until next tick", result.stdout)
        self.assertEqual(len(waits), 2, result.stderr)
        self.assertEqual(waits, logged_waits)
        self.assertTrue(all(2700 <= int(wait) <= 4500 for wait in waits))

    def test_mint_preparation_retries_without_retrying_broadcast(self):
        result = self.run_tick("valid", live=True, prepare_mode="transient")
        self.assertEqual(result.returncode, 99, result.stdout + result.stderr)
        counts = {stage: sum(call["stage"] == stage for call in self.prepare_calls)
                  for stage in ("generate", "sign:multisig1", "sign:multisig2", "multisign", "broadcast")}
        self.assertEqual(counts, {"generate": 3, "sign:multisig1": 2, "sign:multisig2": 2,
                                  "multisign": 2, "broadcast": 1})
        self.assertTrue(all(call["node"] == "https://rpc.terra-classic.hexxagon.io:443" for call in self.prepare_calls))
        self.assertEqual(re.findall(r"mock sleep seconds=(\d+)", result.stderr), ["4", "6", "4", "4", "4"])
        self.assertNotIn("PARTIAL FAILED STDOUT", result.stdout)

    def test_exhausted_rpc_retries_stop_before_signing_and_broadcast(self):
        result = self.run_tick("valid", live=True, prepare_mode="exhaust")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual([call["stage"] for call in self.prepare_calls], ["generate"] * 6)
        self.assertEqual(re.findall(r"mock sleep seconds=(\d+)", result.stderr), ["4", "6", "8", "10", "12"])
        self.assertIn("exhausted 6 RPC retries", result.stderr)

    def test_permanent_mint_errors_are_not_retried_and_node_override_wins(self):
        result = self.run_tick("valid", live=True, prepare_mode="permanent", node_override="https://operator-rpc.test:443")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.prepare_calls, [{"stage": "generate", "attempt": 1, "node": "https://operator-rpc.test:443"}])
        self.assertNotIn("mock sleep seconds=", result.stderr)


if __name__ == "__main__":
    unittest.main()
