#!/usr/bin/env python3
"""Read-only, bounded LCD/RPC corroboration for treasury transaction preflight.

An agreed chain/header is evidence of network identity, not endpoint honesty.
Contract identities are pinned for this process run; optional reviewed pins also
constrain the first observation. No signing command or custody material is used.
"""
import argparse
import base64
from datetime import datetime, timezone
import json
import re
import subprocess
import sys
from urllib.parse import urlencode, urlsplit


class AttestationError(Exception):
    pass


def get_json(url, timeout, height=None, max_bytes=1048576):
    # curl preserves the operator's TLS defaults and existing shell mock tooling.
    args = ["curl", "--fail", "--silent", "--show-error", "--connect-timeout", str(timeout),
            "--max-time", str(timeout), "--max-filesize", str(max_bytes)]
    if height is not None:
        args += ["-H", f"x-cosmos-block-height: {height}"]
    try:
        result = subprocess.run(args + [url], capture_output=True, timeout=timeout + 2, check=True)
        if len(result.stdout) > max_bytes:
            raise AttestationError("oversized attestation response")
        data = json.loads(result.stdout)
        if not isinstance(data, dict):
            raise ValueError("expected JSON object")
        return data
    except (subprocess.SubprocessError, ValueError, OSError) as exc:
        # Do not echo endpoint URLs/query params or provider response bodies.
        raise AttestationError("identity endpoint unavailable or malformed") from exc


def positive(value, label):
    if isinstance(value, bool) or not re.fullmatch(r"[1-9][0-9]*", str(value)):
        raise AttestationError(f"invalid {label}")
    return int(value)


def checksum(value):
    if not isinstance(value, str):
        raise AttestationError("missing code checksum")
    if re.fullmatch(r"[0-9A-Fa-f]{64}", value):
        return value.lower()
    try:
        raw = base64.b64decode(value, validate=True)
    except ValueError as exc:
        raise AttestationError("malformed code checksum") from exc
    if len(raw) != 32:
        raise AttestationError("malformed code checksum")
    return raw.hex()


def header(data, rpc=False):
    try:
        block = data["result"]["block"] if rpc else data.get("block", data.get("sdk_block"))
        result = block["header"]
        block_id = data["result"]["block_id"] if rpc else data["block_id"]
        return result, checksum(block_id["hash"])
    except (KeyError, TypeError) as exc:
        raise AttestationError("missing block header") from exc


def check_header(h, expected, max_age):
    if not isinstance(h, dict):
        raise AttestationError("malformed block header")
    if not isinstance(h.get("chain_id"), str) or h["chain_id"] != expected:
        raise AttestationError("block chain identity mismatch or missing")
    height = positive(h.get("height"), "block height")
    try:
        stamp = datetime.fromisoformat(h["time"].replace("Z", "+00:00"))
        if stamp.tzinfo is None:
            raise ValueError("missing time zone")
        age = (datetime.now(timezone.utc) - stamp).total_seconds()
        if age > max_age or age < -30:
            raise AttestationError("stale or future block header")
    except (ValueError, KeyError, AttributeError) as exc:
        raise AttestationError("missing or malformed block time") from exc
    return height


def varint(value):
    out = bytearray()
    while value > 127:
        out.append((value & 127) | 128)
        value >>= 7
    out.append(value)
    return bytes(out)


def protobuf_fields(raw):
    """Decode only the varint and byte fields used by CosmWasm identity queries."""
    pos, fields = 0, {}

    def read_varint():
        nonlocal pos
        value = 0
        for shift in range(0, 70, 7):
            if pos >= len(raw):
                raise AttestationError("truncated identity protobuf")
            byte = raw[pos]
            pos += 1
            value |= (byte & 127) << shift
            if byte < 128:
                return value
        raise AttestationError("invalid identity protobuf varint")

    while pos < len(raw):
        key = read_varint()
        number, wire = key >> 3, key & 7
        if not number:
            raise AttestationError("invalid identity protobuf field")
        if wire == 0:
            value = read_varint()
        elif wire == 2:
            size = read_varint()
            if size > len(raw) - pos:
                raise AttestationError("truncated identity protobuf bytes")
            value, pos = raw[pos:pos + size], pos + size
        else:
            raise AttestationError("unsupported identity protobuf field")
        fields[number] = value
    return fields


def rpc_query(rpc, method, request, height, timeout):
    params = urlencode({"path": f'"/cosmwasm.wasm.v1.Query/{method}"',
                        "data": "0x" + request.hex(), "height": height, "prove": "false"})
    # Query/Code includes wasm bytes as well as metadata. Keep a finite cap
    # large enough for optimized production wasm; headers/info remain 1 MiB.
    data = get_json(rpc + "/abci_query?" + params, timeout,
                    max_bytes=8388608 if method == "Code" else 1048576)
    try:
        response = data["result"]["response"]
        if positive(response["height"], "query height") != height or response.get("code", 0) != 0:
            raise AttestationError("RPC identity query failed or wrong height")
        return protobuf_fields(base64.b64decode(response["value"], validate=True))
    except (KeyError, ValueError, TypeError) as exc:
        raise AttestationError("malformed RPC contract identity") from exc


def contract_identity(lcd, rpc, address, height, timeout, code_checksums):
    info = get_json(lcd + "/cosmwasm/wasm/v1/contract/" + address, timeout, height)
    try:
        code_id = positive(info["contract_info"]["code_id"], "contract code id")
        request = b"\x0a" + varint(len(address.encode())) + address.encode()
        rpc_info = rpc_query(rpc, "ContractInfo", request, height, timeout)
        rpc_code = positive(protobuf_fields(rpc_info[2])[1], "RPC contract code id")
        if code_id != rpc_code:
            raise AttestationError("LCD/RPC contract identity mismatch")
        # Immutable code bytes are shared by contracts using the same code ID.
        # This cache lasts one attestation at one common height, never a retry.
        if code_id in code_checksums:
            return {"code_id": code_id, "checksum": code_checksums[code_id]}
        code = get_json(lcd + "/cosmwasm/wasm/v1/code/" + str(code_id), timeout, height,
                        max_bytes=8388608)
        digest = checksum(code["code_info"]["data_hash"])
        rpc_code_info = rpc_query(rpc, "Code", b"\x08" + varint(code_id), height, timeout)
        rpc_digest = protobuf_fields(rpc_code_info[1])[3]
        if not isinstance(rpc_digest, bytes) or rpc_digest.hex() != digest:
            raise AttestationError("LCD/RPC code checksum mismatch")
        code_checksums[code_id] = digest
        return {"code_id": code_id, "checksum": digest}
    except (KeyError, TypeError) as exc:
        raise AttestationError("missing protocol contract identity") from exc


def endpoint(url):
    url = url.replace("tcp://", "http://", 1).rstrip("/")
    parsed = urlsplit(url)
    if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password:
        raise AttestationError("invalid identity endpoint configuration")
    return url


def attest(args, rpc):
    lcd = endpoint(args.lcd)
    rpc_http = endpoint(rpc)
    status = get_json(rpc_http + "/status", args.timeout)
    try:
        result = status["result"]
        if result["node_info"]["network"] != args.expected_chain:
            raise AttestationError("RPC network mismatch or missing")
        if result["sync_info"]["catching_up"] is not False:
            raise AttestationError("RPC catching up or missing sync status")
        rpc_height = positive(result["sync_info"]["latest_block_height"], "RPC height")
    except (KeyError, TypeError) as exc:
        raise AttestationError("missing RPC identity") from exc
    lcd_header, _ = header(get_json(lcd + "/cosmos/base/tendermint/v1beta1/blocks/latest", args.timeout))
    lcd_height = check_header(lcd_header, args.expected_chain, args.max_age)
    latest_rpc_header, _ = header(get_json(rpc_http + "/block", args.timeout), True)
    latest_rpc_height = check_header(latest_rpc_header, args.expected_chain, args.max_age)
    if max(lcd_height, rpc_height, latest_rpc_height) - min(lcd_height, rpc_height, latest_rpc_height) > args.max_lag:
        raise AttestationError("LCD/RPC block height disagreement")
    common = min(lcd_height, rpc_height, latest_rpc_height)
    lh, lhash = header(get_json(lcd + f"/cosmos/base/tendermint/v1beta1/blocks/{common}", args.timeout))
    rh, rhash = header(get_json(rpc_http + f"/block?height={common}", args.timeout), True)
    if check_header(lh, args.expected_chain, args.max_age) != common or check_header(rh, args.expected_chain, args.max_age) != common or lhash != rhash:
        raise AttestationError("LCD/RPC common block hash disagreement")
    identities = dict(args.baseline)
    code_checksums = {}
    for address in sorted(set(args.contracts.split()) | set(args.baseline) | set(args.pins)):
        if not re.fullmatch(r"terra1[a-z0-9]+", address):
            raise AttestationError("invalid protocol contract address")
        identity = contract_identity(lcd, rpc_http, address, common, args.timeout, code_checksums)
        for trusted in (args.baseline.get(address), args.pins.get(address)):
            if trusted is not None and identity != trusted:
                raise AttestationError("protocol code id/checksum changed or pinned identity mismatch")
        identities[address] = identity
    # Slow identity reads must not turn an initially fresh anchor into stale trust.
    check_header(lh, args.expected_chain, args.max_age)
    check_header(rh, args.expected_chain, args.max_age)
    return {"expected_chain_id": args.expected_chain, "observed_chain_id": args.expected_chain,
            "height": common, "block_hash": lhash, "identities": identities}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lcd", required=True)
    parser.add_argument("--rpc", required=True)
    parser.add_argument("--fallbacks", default="")
    parser.add_argument("--expected-chain", required=True)
    parser.add_argument("--max-age", type=int, default=120)
    parser.add_argument("--max-lag", type=int, default=5)
    parser.add_argument("--timeout", type=int, default=10)
    parser.add_argument("--contracts", default="")
    parser.add_argument("--baseline", default="{}")
    parser.add_argument("--pins")
    args = parser.parse_args()
    try:
        if not re.fullmatch(r"[A-Za-z0-9_-]{1,128}", args.expected_chain) or args.max_age < 1 or args.max_lag < 0 or args.timeout < 1:
            raise AttestationError("invalid expected chain or freshness policy")
        args.baseline = json.loads(args.baseline)
        pins_path = args.pins
        args.pins = {}
        if pins_path:
            with open(pins_path, encoding="utf-8") as pins_file:
                args.pins = json.load(pins_file)
        for identities in (args.baseline, args.pins):
            if not isinstance(identities, dict):
                raise AttestationError("identity pins must be an object")
            for address, pin in identities.items():
                if not isinstance(pin, dict) or set(pin) != {"code_id", "checksum"}:
                    raise AttestationError("identity pin requires code_id and checksum")
                pin["code_id"] = positive(pin["code_id"], "pinned code id")
                pin["checksum"] = checksum(pin["checksum"])
        # Preserve original list indices so the caller can select privately without
        # publishing a provider URL that may embed API credentials in its path.
        candidates = [args.rpc] + [value.strip() for value in args.fallbacks.split(",")]
        seen = set()
        for index, rpc in enumerate(candidates):
            if not rpc or rpc in seen:
                continue
            seen.add(rpc)
            try:
                receipt = attest(args, rpc)
                receipt["rpc_candidate_index"] = index
                print(json.dumps(receipt, separators=(",", ":")))
                return 0
            except AttestationError as exc:
                print(f"[chain-attestation] RPC candidate {index + 1} rejected: {exc}", file=sys.stderr)
        raise AttestationError("no verified LCD/RPC endpoint pair")
    except (AttestationError, ValueError, OSError) as exc:
        print(f"[chain-attestation] FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
