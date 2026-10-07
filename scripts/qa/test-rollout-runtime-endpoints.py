#!/usr/bin/env python3
"""Synthetic endpoint/proof-cache regressions; no network, keys or broadcasts."""
import base64
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError, URLError

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/lib"))
import route_tax_rollout_cli as cli


class Response(io.BytesIO):
    def __init__(self, value):
        super().__init__(json.dumps(value).encode())


class RuntimeEndpointTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        with patch.dict(cli.os.environ, {cli.RUNTIME_READ_NODE: "", cli.RUNTIME_READ_LCD: "", cli.RUNTIME_EXEC_NODE: ""}):
            self.args = cli.parser().parse_args(["--state-dir", str(self.root), "--node", "https://anchor-rpc.invalid", "--lcd", "https://anchor-lcd.invalid"])
        self.chain = cli.TerradChain(self.args)

    def status(self, chain="columbus-5", catching_up=False):
        return {"result": {"node_info": {"network": chain}, "sync_info": {"catching_up": catching_up}}}

    def test_runtime_urls_do_not_mutate_manifest_anchor_arguments(self):
        args = copy.deepcopy(self.args)
        args.read_node = "https://read-rpc.invalid"
        args.read_lcd = "https://read-lcd.invalid"
        args.execution_node = "https://exec-rpc.invalid"
        before = copy.deepcopy(vars(args))
        chain = cli.TerradChain(args)
        with patch.object(chain, "_request", return_value=self.status()):
            flags = chain.flags()
        self.assertEqual("https://exec-rpc.invalid", flags[flags.index("--node") + 1])
        self.assertEqual(before, vars(args))

    def test_wrong_chain_runtime_rpc_rejected_before_data_query(self):
        self.args.read_node = "https://alternative.invalid"
        chain = cli.TerradChain(self.args)
        with patch.object(chain, "_request", return_value=self.status("wrong-chain")) as request:
            with self.assertRaisesRegex(RuntimeError, "chain identity mismatch"):
                chain.get("/sensitive-data", rpc=True)
        request.assert_called_once_with("https://alternative.invalid", "/status")

    def test_wrong_chain_or_syncing_execution_rpc_cannot_be_used(self):
        self.args.execution_node = "https://exec.invalid"
        for status in (self.status("wrong-chain"), self.status(catching_up=True)):
            chain = cli.TerradChain(self.args)
            with patch.object(chain, "_request", return_value=status):
                with self.assertRaises(RuntimeError):
                    chain.flags()

    def test_read_failover_requires_same_chain_and_does_not_change_execution(self):
        self.args.read_lcd_fallback = ["https://fallback.invalid"]
        chain = cli.TerradChain(self.args)
        calls = []
        def request(endpoint, path):
            calls.append((endpoint, path))
            if endpoint == self.args.lcd:
                raise cli.EndpointReadError("unavailable", endpoint, retryable=True)
            if path.endswith("/latest"):
                return {"block": {"header": {"chain_id": "columbus-5"}}}
            return {"live": "data"}
        with patch.object(chain, "_request", side_effect=request):
            self.assertEqual({"live": "data"}, chain.get("/data"))
        self.assertEqual("https://fallback.invalid", chain.selected_read[False])
        self.assertEqual(self.args.node, chain.execution_node)
        self.assertEqual(("https://fallback.invalid", "/cosmos/base/tendermint/v1beta1/blocks/latest"), calls[1])

    def test_wrong_chain_fallback_rejected_without_application_read(self):
        self.args.read_lcd_fallback = ["https://fallback.invalid"]
        chain = cli.TerradChain(self.args)
        def request(endpoint, _path):
            if endpoint == self.args.lcd:
                raise cli.EndpointReadError("unavailable", endpoint, retryable=True)
            return {"block": {"header": {"chain_id": "other"}}}
        with patch.object(chain, "_request", side_effect=request) as calls:
            with self.assertRaisesRegex(RuntimeError, "chain identity mismatch"):
                chain.get("/data")
            self.assertEqual(2, calls.call_count)

    def test_quote_timeout_never_retries_or_fans_out(self):
        self.args.read_lcd_fallback = ["https://fallback.invalid"]
        chain = cli.TerradChain(self.args)
        with patch.object(chain, "_request", side_effect=cli.EndpointReadError("timeout", self.args.lcd, retryable=True)) as request:
            with self.assertRaises(RuntimeError):
                chain.smart("router", {"reverse_simulate_swap_operations": {"ask_amount": "1"}})
            self.assertEqual(1, request.call_count)

    def test_deterministic_gas_error_is_retained_without_failover(self):
        self.args.read_lcd_fallback = ["https://fallback.invalid"]
        chain = cli.TerradChain(self.args)
        body = b'{"code":11,"message":"out of gas"}'
        error = HTTPError("https://anchor-lcd.invalid", 503, "error", {}, io.BytesIO(body))
        with patch.object(cli, "urlopen", side_effect=error) as request:
            with self.assertRaises(cli.EndpointReadError) as caught:
                chain.get("/data")
            self.assertEqual(1, request.call_count)
            self.assertEqual(body.decode(), caught.exception.http_error_body)
            self.assertFalse(caught.exception.retryable)

    def test_endpoint_credentials_and_remote_plaintext_are_rejected(self):
        for endpoint in ("https://name:password@host.invalid", "http://remote.invalid", "https://host.invalid?token=x"):
            self.args.execution_node = endpoint
            with self.assertRaises(RuntimeError):
                cli.TerradChain(self.args)

    def test_confirmed_receipt_and_code_hash_reads_are_cached_by_bytes_and_context(self):
        path = "/cosmos/tx/v1beta1/txs/" + "A" * 64
        tx = {"tx_response": {"txhash": "A" * 64, "height": "42", "code": 0}}
        with patch.object(self.chain, "_request", return_value=tx) as request:
            first = self.chain.get(path)
            first["tx_response"]["height"] = "999"
            self.assertEqual("42", self.chain.get(path)["tx_response"]["height"])
            self.assertEqual(1, request.call_count)
        with patch.object(self.chain, "_request", return_value={"code_info": {"data_hash": "b" * 64}}) as request:
            self.assertEqual(self.chain.code_hash(1), self.chain.code_hash(1))
            self.assertEqual(1, request.call_count)

    def test_pending_receipts_and_dynamic_state_are_never_cached(self):
        txpath = "/cosmos/tx/v1beta1/txs/" + "C" * 64
        for path, value in ((txpath, {"tx_response": {"txhash": "C" * 64, "height": "0"}}),
                            ("/cosmwasm/wasm/v1/contract/token", {"contract_info": {"code_id": "1"}}),
                            ("/cosmwasm/wasm/v1/contract/pair/smart/pool", {"data": {"total_share": "1"}})):
            with patch.object(self.chain, "_request", return_value=value) as request:
                self.chain.get(path)
                self.chain.get(path)
                self.assertEqual(2, request.call_count)

    def test_sdk_encode_cache_reuses_only_exact_current_file_bytes(self):
        path = self.root / "signed.json"
        path.write_bytes(b"first signed bytes")
        def encode(_parts):
            return base64.b64encode(path.read_bytes()).decode()
        with patch.object(self.chain, "command", side_effect=encode) as command:
            expected = self.chain._encoded_hash(path)
            self.assertEqual(expected, self.chain._encoded_hash(path))
            self.assertEqual(1, command.call_count)
            path.write_bytes(b"changed signed bytes")
            changed = self.chain._encoded_hash(path)
            self.assertNotEqual(expected, changed)
            self.assertEqual(2, command.call_count)

    def test_edit_during_sdk_encode_cannot_seed_cache(self):
        path = self.root / "signed.json"
        path.write_bytes(b"original")
        def encode(_parts):
            path.write_bytes(b"concurrent edit")
            return base64.b64encode(b"original encoded bytes").decode()
        with patch.object(self.chain, "command", side_effect=encode):
            with self.assertRaisesRegex(RuntimeError, "changed while being encoded"):
                self.chain._encoded_hash(path)
        self.assertEqual({}, self.chain.encoded_transactions)


if __name__ == "__main__":
    unittest.main()
