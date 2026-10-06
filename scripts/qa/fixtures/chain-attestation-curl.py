#!/usr/bin/env python3
"""Synthetic chain identity HTTP responses shared by shell regressions."""
import base64
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
from urllib.parse import parse_qs, urlsplit


def varint(value):
    raw = bytearray()
    while value > 127:
        raw.append((value & 127) | 128)
        value >>= 7
    return bytes(raw + bytes([value]))


def blob(field, value):
    return varint((field << 3) | 2) + varint(len(value)) + value


def respond(url):
    parsed = urlsplit(url)
    path = parsed.path
    prefix = os.environ.get("MOCK_RPC_PATH_PREFIX", "")
    if prefix and path.startswith(prefix):
        path = path[len(prefix):]
    if not (path in ("/status", "/block", "/abci_query") or "/blocks/" in path
            or ("/cosmwasm/wasm/v1/contract/" in path and "/smart/" not in path)
            or "/cosmwasm/wasm/v1/code/" in path):
        return False
    mode = os.environ.get("MOCK_CHAIN_MODE", "good")
    if parsed.hostname == "wrong.example":
        mode = "wrong"
    if parsed.hostname == "down.example":
        mode = "unavailable"
    counter = os.environ.get("MOCK_ATTEST_COUNTER")
    if counter and path == "/status":
        file = Path(counter)
        count = int(file.read_text()) + 1 if file.exists() else 1
        file.write_text(str(count))
        if count > int(os.environ.get("MOCK_ATTEST_FAIL_AFTER", "999999")):
            mode = "wrong"
    elif counter and Path(counter).exists() and int(Path(counter).read_text()) > int(os.environ.get("MOCK_ATTEST_FAIL_AFTER", "999999")):
        mode = "wrong"
    if mode == "unavailable":
        raise SystemExit(22)
    if mode == "malformed":
        print("broken JSON")
        return True
    if mode == "oversized":
        print(json.dumps({"padding": "x" * 1048577}))
        return True
    chain = os.environ.get("MOCK_EXPECTED_CHAIN", "columbus-5")
    if mode == "wrong":
        chain = "wrong-chain"
    if mode == "missing":
        chain = None
    age = 900 if mode == "stale" else -900 if mode == "future" else 0
    stamp = (datetime.now(timezone.utc) - timedelta(seconds=age)).isoformat()
    height = 100
    code_id = int(os.environ.get("MOCK_CODE_ID", "1"))
    digest = bytes.fromhex("ab" * 32)
    if path == "/status":
        data = {"result": {"node_info": {"network": chain}, "sync_info": {
            "latest_block_height": str(height), "catching_up": mode == "catching-up"}}}
    elif path == "/abci_query":
        query = parse_qs(parsed.query)
        if "ContractInfo" in query["path"][0]:
            data = blob(1, b"terra1fixture") + blob(2, b"\x08" + varint(code_id))
        else:
            rpc_digest = bytes.fromhex("cd" * 32) if mode == "code-mismatch" else digest
            data = blob(1, b"\x08" + varint(code_id) + blob(2, b"terra1fixture") + blob(3, rpc_digest))
        data = {"result": {"response": {"code": 0, "height": query["height"][0],
                                         "value": base64.b64encode(data).decode()}}}
    elif "/contract/" in path:
        data = {"contract_info": {"code_id": str(code_id)}}
        if mode == "missing-code":
            data = {"contract_info": {}}
    elif "/code/" in path:
        data = {"code_info": {"data_hash": base64.b64encode(digest).decode()}}
    else:
        rpc = path == "/block"
        target = parse_qs(parsed.query).get("height", [str(height)])[0] if rpc else path.rsplit("/", 1)[-1]
        block_height = str(height if target == "latest" else int(target))
        if mode == "height-lag" and not rpc:
            block_height = "90"
        if mode == "zero-height":
            block_height = "0"
        if mode == "invalid-height":
            block_height = "not-a-height"
        digest = "cd" * 32 if mode == "hash-mismatch" and rpc else "ab" * 32
        data = {"block": {"header": {"chain_id": chain, "height": block_height, "time": stamp}},
                "block_id": {"hash": digest}}
        if mode == "header-list":
            data["block"]["header"] = []
        if mode == "missing-time":
            del data["block"]["header"]["time"]
        if mode == "lcd-wrong" and not rpc:
            data["block"]["header"]["chain_id"] = "wrong-chain"
        if rpc:
            data = {"result": data}
    print(json.dumps(data))
    return True
