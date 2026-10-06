#!/usr/bin/env python3
"""Bounded, read-only endpoint observations for legacy AutoLP recovery (#1429).

No wallet, environment secrets, transactions, or migration are used. Chain IDs
and freshness reject obvious misconfiguration; they do not attest an endpoint.
"""

import argparse
import base64
import datetime as dt
import json
import sys
import urllib.error
import urllib.parse
import urllib.request


DEFAULT_LAUNCHER = "terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
HISTORICAL_AUTOLP = (11613, 11621, 11633)
HISTORICAL_TOKENS = (11611, 11619, 11626, 11630)


class ObservationError(Exception):
    """The endpoint observation is unavailable, malformed or exceeds bounds."""


class Client:
    def __init__(self, endpoint, timeout, page_size, max_pages, max_contracts):
        self.endpoint = endpoint.rstrip("/")
        self.timeout = timeout
        self.page_size = page_size
        self.max_pages = max_pages
        self.max_contracts = max_contracts
        self.requests = 0

    def get(self, path):
        self.requests += 1
        request = urllib.request.Request(
            self.endpoint + path,
            headers={"User-Agent": "cl8y-dex-issue-1429-inventory/1.0"},
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                # Avoid an unbounded response even from a faulty provider.
                raw = response.read(2_000_001)
                if len(raw) > 2_000_000:
                    raise ObservationError("response exceeds 2 MB")
                body = json.loads(raw)
        except urllib.error.HTTPError as error:
            # Preserve the public chain error for unsupported smart queries.
            detail = error.read(8192).decode("utf-8", errors="replace")
            raise ObservationError(f"HTTP {error.code}: {detail}") from error
        except (urllib.error.URLError, TimeoutError, ValueError) as error:
            raise ObservationError(str(error)) from error
        if not isinstance(body, dict):
            raise ObservationError("response must be an object")
        if body.get("code") not in (None, 0):
            raise ObservationError(f"chain query error: {body}")
        return body

    def smart(self, address, query):
        encoded = base64.b64encode(json.dumps(query, separators=(",", ":")).encode()).decode()
        body = self.get(f"/cosmwasm/wasm/v1/contract/{address}/smart/{encoded}")
        if "data" not in body:
            raise ObservationError("smart-query response has no data")
        data = body["data"]
        if isinstance(data, str):
            try:
                return json.loads(base64.b64decode(data, validate=True))
            except ValueError as error:
                raise ObservationError("invalid encoded smart-query data") from error
        return data

    def contracts(self, code_id):
        addresses, seen_keys, next_key = [], set(), None
        for page in range(1, self.max_pages + 1):
            params = {"pagination.limit": self.page_size}
            if next_key:
                params["pagination.key"] = next_key
            body = self.get(
                f"/cosmwasm/wasm/v1/code/{code_id}/contracts?"
                + urllib.parse.urlencode(params)
            )
            batch = body.get("contracts")
            pagination = body.get("pagination")
            if not isinstance(batch, list) or not isinstance(pagination, dict):
                raise ObservationError("malformed contracts pagination")
            if any(not isinstance(address, str) or not address.startswith("terra1") for address in batch):
                raise ObservationError("malformed contract address")
            if len(batch) > self.page_size:
                raise ObservationError("provider exceeded requested page size")
            addresses.extend(batch)
            if len(addresses) > self.max_contracts or len(set(addresses)) != len(addresses):
                raise ObservationError("contract count bound exceeded or duplicate address")
            next_key = pagination.get("next_key")
            if next_key is None or next_key == "":
                return {"addresses": addresses, "pages": page, "next_key": None}
            if not isinstance(next_key, str) or next_key in seen_keys:
                raise ObservationError("invalid or repeated pagination key")
            seen_keys.add(next_key)
        raise ObservationError("pagination page bound exceeded")


def policy_observation(client, address):
    try:
        policy = client.smart(address, {"get_lp_policy": {}})
        if policy is not None and not isinstance(policy, dict):
            raise ObservationError("LP policy must be an object or null")
        return {"status": "missing" if policy is None else "present", "value": policy}
    except ObservationError as error:
        detail = str(error)
        unsupported = "unknown variant" in detail.lower() and "get_lp_policy" in detail.lower()
        return {"status": "unsupported" if unsupported else "unavailable", "error": detail}


def observe(client, launcher, chain_id, max_age):
    now = dt.datetime.now(dt.timezone.utc)
    node = client.get("/cosmos/base/tendermint/v1beta1/node_info")
    header = client.get("/cosmos/base/tendermint/v1beta1/blocks/latest")["block"]["header"]
    if node["default_node_info"]["network"] != chain_id or header["chain_id"] != chain_id:
        raise ObservationError("endpoint chain ID differs from requested chain")
    block_time = dt.datetime.fromisoformat(header["time"].replace("Z", "+00:00"))
    age = (now - block_time).total_seconds()
    if age > max_age or age < -120:
        raise ObservationError(f"latest block freshness failed: age {age:.1f}s")
    launcher_info = client.get(f"/cosmwasm/wasm/v1/contract/{launcher}")
    launcher_config = client.smart(launcher, {"get_config": {}})
    autolp_codes = sorted(set(HISTORICAL_AUTOLP + (int(launcher_config["autolp_code_id"]),)))
    token_codes = sorted(set(HISTORICAL_TOKENS + (int(launcher_config["token_code_id"]),)))
    autolp_inventory = {str(code): client.contracts(code) for code in autolp_codes}
    token_inventory = {str(code): client.contracts(code) for code in token_codes}
    sisters = {address for entry in autolp_inventory.values() for address in entry["addresses"]}
    tokens = {}
    for address in sorted({address for entry in token_inventory.values() for address in entry["addresses"]}):
        config = client.smart(address, {"get_config": {}})
        tokens[address] = {"config": config}
        if config.get("autolp"):
            sisters.add(config["autolp"])
    sister_inventory = {}
    for address in sorted(sisters):
        config = client.smart(address, {"get_config": {}})
        sister_inventory[address] = {
            "contract_info": client.get(f"/cosmwasm/wasm/v1/contract/{address}"),
            "config": config,
            "lp_policy": policy_observation(client, address),
            "manager_kind": "token-owned" if config.get("manager") == config.get("token") else "other",
        }
    return {
        "schema": 1,
        "issue": "https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1429",
        "observed_at_utc": now.isoformat(),
        "completed_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "endpoint": client.endpoint,
        "scope": "Documented historical and current launcher pins plus discovered token bindings; endpoint observation, not chain attestation or deployment acceptance.",
        "chain_id": chain_id,
        "block": {key: header[key] for key in ("height", "time", "chain_id")},
        "block_age_seconds": round(age, 3),
        "bounds": {"timeout_seconds": client.timeout, "page_size": client.page_size, "max_pages_per_code": client.max_pages, "max_contracts_per_code": client.max_contracts, "max_block_age_seconds": max_age},
        "launcher": {"address": launcher, "contract_info": launcher_info, "config": launcher_config},
        "autolp_contracts_by_code": autolp_inventory,
        "token_contracts_by_code": token_inventory,
        "tokens": tokens,
        "sisters": sister_inventory,
        "request_count": client.requests,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", default="https://lcd.terra-classic.hexxagon.io")
    parser.add_argument("--launcher", default=DEFAULT_LAUNCHER)
    parser.add_argument("--chain-id", default="columbus-5")
    parser.add_argument("--timeout", type=float, default=25)
    parser.add_argument("--page-size", type=int, default=100)
    parser.add_argument("--max-pages", type=int, default=10)
    parser.add_argument("--max-contracts", type=int, default=1000)
    parser.add_argument("--max-block-age", type=int, default=180)
    args = parser.parse_args()
    parsed = urllib.parse.urlsplit(args.endpoint)
    if parsed.scheme not in ("http", "https") or not parsed.netloc or parsed.username or parsed.password or parsed.query or parsed.fragment:
        parser.error("endpoint must be an HTTP(S) URL without credentials, query or fragment")
    if not args.launcher.startswith("terra1") or not args.launcher.isalnum():
        parser.error("launcher must be a Terra contract address")
    if not 0 < args.timeout <= 60 or not 0 < args.page_size <= 1000 or not 0 < args.max_pages <= 100 or not 0 < args.max_contracts <= 10000 or args.max_block_age <= 0:
        parser.error("query timeout/pagination/freshness bounds are invalid")
    client = Client(args.endpoint, args.timeout, args.page_size, args.max_pages, args.max_contracts)
    try:
        receipt = observe(client, args.launcher, args.chain_id, args.max_block_age)
    except (ObservationError, KeyError, TypeError, ValueError) as error:
        print(f"inventory unavailable or incomplete: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
