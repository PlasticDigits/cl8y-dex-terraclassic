#!/usr/bin/env python3
"""Measure isolated synthetic reverse smart queries inside LocalTerra transactions.

Never resets the shared chain or queries production. Query-probe receipts include
transaction/wrapper overhead; actual smart-query outcomes are recorded separately.
"""
import argparse
import base64
import hashlib
import importlib.util
import json
import re
import subprocess
import time
import tomllib
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path

HELPER = Path(__file__).with_name("localterra-route-tax-1381.py")
SPEC = importlib.util.spec_from_file_location("localterra_tax", HELPER)
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)


GREEDY_REJECTION = "GREEDY_REVERSE_ROUTE_UNSUPPORTED"


def provider_gas_failure(error, limit):
    """Require the actual provider meter, rather than a transaction OOG."""
    text = error.lower()
    return ("out of gas" in text and "gasused" in text and
            re.search(r"gaswanted\s*:\s*" + str(limit) + r"\b", text) is not None)


def observe_smart_query(chain, contract, query, row, save):
    """Persist the real provider outcome before any independent probe attempt."""
    response = None
    try:
        response = chain.smart(contract, query)
        row.update(smart_status="PASS", response=response)
    except RuntimeError as error:
        row.update(smart_status="FAIL", error=str(error))
    save()
    return response


def node_metadata(chain, node_home, query_limit):
    config = tomllib.loads(chain.command("cat", node_home + "/config/app.toml"))
    actual = int(config["wasm"]["query_gas_limit"])
    assert actual == query_limit, f"Expected existing {query_limit} query envelope, got {actual}"
    inspected = json.loads(subprocess.check_output(["docker", "inspect", chain.container]))[0]
    compose = Path(__file__).resolve().parents[2] / "docker-compose.yml"
    pinned = re.search(r"image:\s*(\S*/localterra-cl8y@sha256:[0-9a-f]+)", compose.read_text())
    assert pinned and inspected["Config"]["Image"] == pinned.group(1), "Use the pinned SDK53 image"
    version = chain.command("terrad", "version", "--long")
    assert "cosmos_sdk_version: v0.53." in version, "Expected SDK53 node"
    fields = {}
    for line in version.splitlines():
        name, separator, value = line.partition(":")
        if separator and name in {"app_name", "commit", "cosmos_sdk_version", "go",
                                  "name", "version", "wasmvm_version", "build_tags"}:
            fields[name] = value.strip()
    return {"container": chain.container, "image": inspected["Config"]["Image"],
            "image_id": inspected["Image"], "terrad_version": fields,
            "terrad_version_dump_sha256": hashlib.sha256(version.encode()).hexdigest(),
            "wasm_query_gas_limit": actual}


class EvidenceLocalTerra(helper.LocalTerra):
    """Retain full smart errors and independently failed measurement receipts."""
    capture_probe = False

    def command(self, *parts, stdin=None):
        result = subprocess.run(["docker", "exec", "-i", self.container, *parts],
                                input=stdin, capture_output=True, timeout=90)
        if result.returncode:
            raise RuntimeError(result.stderr.decode())
        return result.stdout.decode()

    def wait(self, response, expect_failure=False):
        if not self.capture_probe:
            return super().wait(response, expect_failure)
        if response.get("code", 0):
            raise RuntimeError("LocalTerra broadcast rejected: " + response.get("raw_log", ""))
        for _ in range(45):
            try:
                receipt = self.query("tx", response["txhash"])
            except RuntimeError:
                time.sleep(1)
                continue
            self.receipts.append({"txhash": receipt["txhash"], "height": receipt["height"],
                "code": receipt.get("code", 0), "gas_used": receipt["gas_used"],
                "raw_log": receipt.get("raw_log", "")})
            return receipt
        raise TimeoutError("LocalTerra probe inclusion timed out")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", default="cl8y-dex-terraclassic-localterra-1")
    parser.add_argument("--artifacts", type=Path, default=Path(__file__).resolve().parents[2] / "smartcontracts/artifacts")
    parser.add_argument("--probe", type=Path, default=Path(__file__).with_name("fixtures") / "reverse-query-probe/target/wasm32-unknown-unknown/release/reverse_query_probe.wasm")
    parser.add_argument("--router-version", default="1.4.0", help="Expected reviewed router cw2 version (historical comparison only)")
    expectation = parser.add_mutually_exclusive_group()
    expectation.add_argument("--expect-populated-gas-failure", action="store_true",
                             help="Reproduce #1428 on frozen integrated artifacts; retain raw FAIL evidence")
    expectation.add_argument("--expect-four-hop-greedy-rejection", action="store_true",
                             help="Verify #1428 admission denial and supported populated reverse routes")
    parser.add_argument("--node-home", default="/home/terra/.terra")
    parser.add_argument("--query-gas-limit", type=int, default=15_000_000,
                        help="Historical book fixture envelope; never raise this to pass the fixture")
    parser.add_argument("--code-cache", type=Path)
    parser.add_argument("--evidence-file", required=True, type=Path)
    args = parser.parse_args()
    if ((args.expect_populated_gas_failure or args.expect_four_hop_greedy_rejection)
            and args.query_gas_limit != 15_000_000):
        parser.error("#1428 requires the unchanged historical 15,000,000 book-query envelope")
    chain = EvidenceLocalTerra(args.container, args.code_cache)
    node = (node_metadata(chain, args.node_home, args.query_gas_limit)
            if args.expect_populated_gas_failure or args.expect_four_hop_greedy_rejection else None)
    # Only bootstrap funding uses the shared genesis account. All subsequent
    # transactions use a unique test signer, avoiding concurrent QA nonce races.
    key = "issue1389-" + str(time.time_ns())
    chain.command("terrad", "keys", "add", key, "--keyring-backend", "test", "--output", "json")
    owner = chain.address(key)
    for attempt in range(5):
        try:
            chain.tx("bank", "send", "test1", owner, "100000000000uluna", key="test1")
            break
        except RuntimeError as error:
            # Only retry a rejected bootstrap send; never retry accepted txs or
            # arbitrary failures, and never use this logic outside LocalTerra.
            if "LocalTerra broadcast rejected: account sequence mismatch" not in str(error) or attempt == 4:
                raise
            time.sleep(1)
    chain.tx_key = key
    codes = {name: chain.store(args.artifacts / (name + ".wasm")) for name in (
        "cl8y_dex_factory", "cl8y_dex_pair", "cl8y_dex_router", "cw20_mintable")}
    probe_code = chain.store(args.probe)
    _, treasury = chain.fresh_trader()
    factory = chain.instantiate(codes["cl8y_dex_factory"]["code_id"], {
        "governance": owner, "treasury": treasury, "default_fee_bps": 30,
        "pair_code_id": codes["cl8y_dex_pair"]["code_id"],
        "lp_token_code_id": codes["cw20_mintable"]["code_id"],
        "whitelisted_code_ids": [codes["cw20_mintable"]["code_id"]],
        "pair_creation_fee_uluna": "0"}, "issue1389-factory")
    router = chain.instantiate(codes["cl8y_dex_router"]["code_id"], {"factory": factory}, "issue1389-router")
    probe = chain.instantiate(probe_code["code_id"], {}, "issue1389-test-query-probe")
    decimals = [6, 18, 6, 18, 6]
    tokens = [chain.instantiate(codes["cw20_mintable"]["code_id"], {
        "name": "Synthetic reverse " + str(i), "symbol": "REV", "decimals": d,
        "initial_balances": [{"address": owner, "amount": str(10 ** (d + 9))}],
        "mint": {"minter": owner}}, "issue1389-token-" + str(i)) for i, d in enumerate(decimals)]
    pairs = []
    for i in range(4):
        infos = [helper.asset(token) for token in tokens[i:i+2]]
        chain.execute(factory, {"create_pair": {"asset_infos": infos}})
        pair = chain.smart(factory, {"pair": {"asset_infos": infos}})["pair"]["contract_addr"]
        pairs.append(pair)
        for j in (i, i+1):
            chain.execute(tokens[j], {"increase_allowance": {
                "spender": pair, "amount": str(10 ** (decimals[j] + 6))}})
        chain.execute(pair, {"provide_liquidity": {"assets": [
            {"info": helper.asset(tokens[j]), "amount": str(10 ** (decimals[j] + 6))}
            for j in (i, i+1)]}})

    checks = []
    def version(address):
        raw = chain.query("wasm", "contract-state", "raw", address, "contract_info", "--ascii")
        return json.loads(base64.b64decode(raw["data"]))

    pair_versions = {pair: version(pair) for pair in pairs}
    router_version = version(router)
    assert all(value == {"contract": "cl8y-dex-pair", "version": "1.21.0"}
               for value in pair_versions.values()), pair_versions
    assert router_version == {"contract": "cl8y-dex-router", "version": args.router_version}, router_version
    evidence = {"utc": datetime.now(timezone.utc).isoformat(), "chain_id": "localterra",
        "synthetic_only": True, "gas_scope": "transaction including test-wrapper overhead",
        "codes": codes, "probe_code": probe_code, "factory": factory, "router": router,
        "pairs": pairs, "pair_versions": pair_versions, "router_version": router_version,
        "expected_router_version": args.router_version,
        "tokens": tokens, "decimals": decimals, "checks": checks,
        "forward_validations": [], "book_setup": [], "maker_walks": [], "status": "PARTIAL"}
    if node:
        evidence["node"] = node
        evidence["issue"] = 1428
        evidence["expectation"] = ("frozen populated gas failure" if args.expect_populated_gas_failure
                                   else "deterministic unsupported-route denial")
    if args.expect_four_hop_greedy_rejection:
        evidence["supported_scope"] = {
            "empty_books": "existing matrix plus three-hop greedy boundary in both directions",
            "populated_books": [
                {"hops": 1, "directions": [False, True], "ask_raw": "1", "max_maker_fills": 8},
                {"hops": 1, "directions": [False], "ask_human": "1", "max_maker_fills": 8}],
            "unsupported_scope": "four-hop routes containing greedy; original 100-human/100-maker requests preserved",
            "provider_guarantee": False,
            "note": "Each case is an explicit independent request; these are not retries or automatic fallback quotes."}
    evidence["gas_scope"] = "probe transaction including transaction/wrapper overhead; actual smart-query outcomes separately recorded"
    for stored in {**codes, "probe": probe_code}.values():
        info = chain.query("wasm", "code-info", str(stored["code_id"]))
        assert info["checksum"].lower() == stored["sha256"], info
        stored["onchain_sha256"] = info["checksum"].lower()

    def save():
        evidence["receipts"] = chain.receipts
        args.evidence_file.parent.mkdir(parents=True, exist_ok=True)
        args.evidence_file.write_text(json.dumps(evidence, indent=2) + "\n")

    def measure(label, contract, query, expected_error=None, expected_gas_failure=False):
        row = {"label": label, "contract": contract, "query": query, "smart_status": "PENDING"}
        checks.append(row)
        save()
        quote = observe_smart_query(chain, contract, query, row, save)
        chain.capture_probe = True
        try:
            receipt = chain.execute(probe, {"contract": contract,
                "query": base64.b64encode(json.dumps(query).encode()).decode()})
            status = ("transaction_error" if int(receipt.get("code", 0)) else
                      chain.event(receipt, "wasm", "query_status"))
            error = (receipt.get("raw_log", "") if status == "transaction_error" else
                     chain.event(receipt, "wasm", "query_error") if status == "error" else None)
            row["probe"] = {"gas_used": int(receipt["gas_used"]), "height": receipt["height"],
                            "txhash": receipt["txhash"], "query_status": status, "error": error,
                            "code": receipt.get("code", 0)}
            # Retain historical field names for existing receipt consumers.
            row.update(gas_used=int(receipt["gas_used"]), height=receipt["height"],
                       txhash=receipt["txhash"], query_status=status)
        except RuntimeError as error:
            row["probe"] = {"query_status": "measurement_error", "error": str(error)}
        finally:
            chain.capture_probe = False
            save()

        def require(condition):
            if not condition:
                row["status"] = "FAIL"
                evidence.update(status="FAIL", failure="Unexpected query outcome: " + label)
                save()
                raise AssertionError(row)

        if expected_gas_failure:
            require(row["smart_status"] == "FAIL" and provider_gas_failure(
                row["error"], args.query_gas_limit))
            row.update(status="FAIL", baseline_regression="REPRODUCED")
            save()
            print(json.dumps(row), flush=True)
            return None
        if expected_error:
            require(row["smart_status"] == "FAIL" and expected_error in row["error"])
            require(row["probe"]["query_status"] == "error" and expected_error in row["probe"]["error"])
        else:
            require(row["smart_status"] == "PASS" and row["probe"]["query_status"] == "ok")
        # A fixed test envelope, never a production gas guarantee.
        require(row["gas_used"] < 12_000_000)
        row["status"] = "PASS"
        save()
        print(json.dumps(row), flush=True)
        return quote

    def route(names, mode, max_maker_fills=100):
        ops = helper.operations(names)
        for op in ops:
            if mode == "greedy":
                op["terra_swap"]["greedy"] = {"max_maker_fills": max_maker_fills}
            elif mode == "declared":
                op["terra_swap"]["hybrid"] = {"pool_input": "1", "book_input": "1", "max_maker_fills": 100}
        return ops

    def router_query(names, target, mode, max_maker_fills=100):
        return {"reverse_simulate_swap_operations": {
            "ask_amount": str(target), "operations": route(names, mode, max_maker_fills)}}

    def forward_query(names, mode, offer, max_maker_fills=100):
        ops = route(names, mode, max_maker_fills)
        if mode == "declared":
            # Reverse accepts ratio templates; executable forward operations
            # require each split to sum to that hop's actual economic offer.
            current = int(offer)
            for op, a, b in zip(ops, names, names[1:]):
                split = {"pool_input": str(current - current // 2),
                         "book_input": str(current // 2), "max_maker_fills": 100}
                op["terra_swap"]["hybrid"] = split
                pair_index = min(tokens.index(a), tokens.index(b))
                hop = chain.smart(pairs[pair_index], {"hybrid_simulation": {
                    "offer_asset": {"info": helper.asset(a), "amount": str(current)},
                    "hybrid": split}})
                current = int(hop["return_amount"])
        return {"simulate_swap_operations": {"offer_amount": offer, "operations": ops}}

    def validate_forward(label, names, mode, quote, target, max_maker_fills=100):
        """Retain the independent fresh target-covering smart query and payload."""
        row = {"label": label, "contract": router, "original_target": str(target),
               "reverse_offer": quote["amount"],
               "query": forward_query(names, mode, quote["amount"], max_maker_fills),
               "smart_status": "PENDING"}
        evidence["forward_validations"].append(row)
        save()
        try:
            response = chain.smart(router, row["query"])
            row.update(response=response, smart_status="PASS")
            assert int(response["amount"]) >= target, (quote, response)
            row["target_covered"] = True
        except (RuntimeError, AssertionError) as error:
            row.update(smart_status="FAIL", error=str(error))
            evidence.update(status="FAIL", failure=str(error))
            raise
        finally:
            save()

    for direction in (False, True):
        names = tokens[::-1] if direction else tokens
        for mode in ("pool", "greedy", "declared"):
            hop_counts = ((1, 2) if mode == "declared" else
                          (1, 3, 4) if mode == "greedy" and args.expect_four_hop_greedy_rejection else
                          (1, 4))
            for hops in hop_counts:
                target_decimals = decimals[::-1][hops] if direction else decimals[hops]
                target = 100 * 10 ** target_decimals
                query = router_query(names[:hops+1], target, mode)
                rejection = (GREEDY_REJECTION if args.expect_four_hop_greedy_rejection and
                             mode == "greedy" and hops == 4 else None)
                quote = measure(f"empty {mode} {hops} hops reverse={direction}", router, query, rejection)
                if rejection:
                    continue
                validate_forward(f"empty {mode} {hops} hops reverse={direction}",
                                 names[:hops+1], mode, quote, target)
        # Three opaque 32-call reservations cannot fit alongside fresh validations.
        measure(f"declared whole-request denial reverse={direction}", router,
                router_query(names, 100 * 10 ** 6, "declared"), "REVERSE_QUERY_BUDGET_EXCEEDED")

    # Two opposing 20-order books per pair exercise maker walks in four capped hops.
    for pair in pairs:
        assets = chain.smart(pair, {"pool": {}})["assets"]
        ordered_tokens = [asset["info"]["token"]["contract_addr"] for asset in assets]
        ds = [decimals[tokens.index(token)] for token in ordered_tokens]
        for side, index in (("ask", 0), ("bid", 1)):
            # Strictly beat the pool in each orientation so greedy visits makers.
            price = format(Decimal(10) ** (ds[1] - ds[0]) *
                           (Decimal("0.99") if side == "ask" else Decimal("1.01")), "f")
            amount = 10 ** (ds[index] + 1)
            hook = {"place_limit_order_batch": {"side": side, "orders": [
                {"price": price, "amount": str(amount), "max_adjust_steps": 256}
                for _ in range(20)]}}
            message = {"send": {"contract": pair, "amount": str(amount * 20),
                                "msg": base64.b64encode(json.dumps(hook).encode()).decode()}}
            receipt = chain.execute(ordered_tokens[index], message)
            evidence["book_setup"].append({"pair": pair, "side": side, "price": price,
                "orders": 20, "order_amount": str(amount), "contract": ordered_tokens[index],
                "message": message, "txhash": receipt["txhash"], "height": receipt["height"]})
            save()
        for index in (0, 1):
            query = {"hybrid_simulation": {
                "offer_asset": {"info": helper.asset(ordered_tokens[index]),
                                "amount": str(100 * 10 ** ds[index])},
                "greedy": {"max_maker_fills": 100}}}
            walk = chain.smart(pair, query)
            evidence["maker_walks"].append({"pair": pair, "direction": index,
                                           "query": query, "response": walk})
            save()
            assert int(walk["limit_book_offer_consumed"]) > 0, (pair, index, walk)
    for direction in (False, True):
        names = tokens[::-1] if direction else tokens
        if args.expect_four_hop_greedy_rejection:
            target_decimals = decimals[::-1][1] if direction else decimals[1]
            scoped_targets = [("one raw ask unit", 1)]
            if not direction:
                scoped_targets.append(("one human ask unit", 10 ** target_decimals))
            for scope, target in scoped_targets:
                query = router_query(names[:2], target, "greedy", max_maker_fills=8)
                label = f"full greedy one-hop {scope} max-makers=8 reverse={direction}"
                quote = measure(label, router, query)
                validate_forward(label + " fresh forward", names[:2], "greedy", quote, target,
                                 max_maker_fills=8)
        # Greedy full-book precision may intentionally exhaust the small budget.
        query = router_query(names, 100 * 10 ** 6, "greedy")
        if args.expect_populated_gas_failure:
            measure(f"full greedy four-hop provider gas failure reverse={direction}", router, query,
                    expected_gas_failure=True)
            continue
        if args.expect_four_hop_greedy_rejection:
            measure(f"full greedy four-hop admission denial reverse={direction}", router, query,
                    GREEDY_REJECTION)
            continue
        try:
            quote = chain.smart(router, query)
        except RuntimeError as error:
            assert "REVERSE_QUERY_BUDGET_EXCEEDED" in str(error), str(error)
            measure(f"full greedy four-hop budget reverse={direction}", router, query,
                    "REVERSE_QUERY_BUDGET_EXCEEDED")
        else:
            measure(f"full greedy four-hop sufficient reverse={direction}", router, query)
            validate_forward(f"full greedy four-hop sufficient reverse={direction}",
                             names, "greedy", quote, 100 * 10 ** 6)
    # Initial liquidity starts from zero reserves and cannot seed the oracle.
    # Seed only after completing the reverse matrix, so its books/reserves remain
    # identical to the historical regression throughout all quote measurements.
    seed_message = {"send": {"contract": pairs[0], "amount": str(10 ** decimals[0]),
        "msg": base64.b64encode(json.dumps({"swap": {"min_return": "1"}}).encode()).decode()}}
    seed_receipt = chain.execute(tokens[0], seed_message)
    evidence["oracle_seed"] = {"contract": tokens[0], "message": seed_message,
                               "txhash": seed_receipt["txhash"], "height": seed_receipt["height"]}
    save()
    measure("invalid Observe horizon", pairs[0], {"observe": {"seconds_ago": [2 ** 32 - 1]}},
            "seconds_ago")
    evidence["status"] = "FAIL" if args.expect_populated_gas_failure else "PASS"
    if args.expect_populated_gas_failure:
        evidence["baseline_regression"] = "REPRODUCED"
    save()


if __name__ == "__main__":
    main()
