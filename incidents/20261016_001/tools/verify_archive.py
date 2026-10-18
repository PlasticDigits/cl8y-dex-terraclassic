#!/usr/bin/env python3
"""Verify this fixed incident archive offline, without signing or network access."""

import base64
import csv
import gzip
import hashlib
import json
import re
from collections import Counter
from decimal import Decimal, getcontext
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parents[1]
getcontext().prec = 100
RECIPIENT = "0x46b2ebf5ec9bbbca91fb826a6c13aa7ccdb66f8d"
CL8Y = "0x8f452a1fdd388a45e1080992eff051b4dd9048d2"
TRANSFER = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
COUNTS = {
    "source-chain-transaction-ledger.csv": 68,
    "source-chain-asset-flow-ledger.csv": 239,
    "bridge-flow-ledger.csv": 9,
    "bsc-token-flow-ledger.csv": 8,
    "bsc-earlier-swap-ledger.csv": 20,
    "bsc-swap-token-routing-ledger.csv": 388,
    "bsc-native-call-flow-ledger.csv": 128,
    "bsc-transaction-ledger.csv": 56,
    "bsc-native-funding-ledger.csv": 3,
    "osmosis-asset-flow-ledger.csv": 11,
}


def require(condition, description):
    if not condition:
        raise ValueError(description)


def read(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ledger(name):
    with (ROOT / "ledgers" / name).open(encoding="utf-8", newline="") as stream:
        return list(csv.DictReader(stream))


def integrity():
    manifest = read("manifest.json")
    require(manifest["case_id"] == "20261016_001", "case identifier")
    files = {str(p.relative_to(ROOT)): p for p in ROOT.rglob("*") if p.is_file()}
    require(not any(p.is_symlink() for p in ROOT.rglob("*")), "no symlinks")
    expected = {r["path"] for r in manifest["files"]}
    require(len(expected) == len(manifest["files"]), "unique manifest paths")
    require(expected == set(files) - {"manifest.json", "SHA256SUMS"}, "complete manifest coverage")
    for record in manifest["files"]:
        path = (ROOT / record["path"]).resolve()
        require(path.is_relative_to(ROOT), "manifest path remains inside archive")
        require(path.stat().st_size == record["bytes"], f"byte size: {record['path']}")
        require(digest(path) == record["sha256"], f"SHA256: {record['path']}")
    checksums = {}
    for line in (ROOT / "SHA256SUMS").read_text().splitlines():
        value, name = line.split("  ", 1)
        require(name not in checksums, "unique checksum path")
        checksums[name] = value
    require(set(checksums) == set(files) - {"SHA256SUMS"}, "complete SHA256SUMS coverage")
    for name, value in checksums.items():
        require(digest(files[name]) == value, f"SHA256SUMS: {name}")
    for path in ROOT.rglob("*.json"):
        json.loads(path.read_text(encoding="utf-8"))
    for path in ROOT.rglob("*.md"):
        for target in re.findall(r"\]\(([^)]+)\)", path.read_text()):
            target = target.split("#", 1)[0]
            if not target or "://" in target or target.startswith("mailto:"):
                continue
            require(not target.startswith("/"), f"portable Markdown link: {path.name}")
            require((path.parent / unquote(target)).exists(), f"local link: {target}")
    for record in read("evidence-catalog.json")["records"]:
        require((ROOT / record["path"]).is_file(), f"catalog target: {record['path']}")
    for record in read("sources/original-document-digests.json")["originals"]:
        require(digest(ROOT / record["published_derivative"]) == record["derivative_sha256"], "source derivative digest")
    return len(files)


def verify_source():
    for row in ledger("source-chain-transaction-ledger.csv"):
        value = read(f"evidence/terra-transactions/{row['tx_hash']}.json")
        tx, response = value["tx"], value["tx_response"]
        require(response["txhash"] == row["tx_hash"], "source transaction hash")
        require(str(response["height"]) == row["block_height"], "source height")
        require(int(response["code"]) == int(row["result_code"]), "source result")
        fee = sum(int(c["amount"]) for c in tx["auth_info"]["fee"]["amount"] if c["denom"] == "uluna")
        require(Decimal(row["network_fee_LUNC"]) == Decimal(fee) / 10**6, "source declared fee")


def verify_bridge():
    deposits = ledger("bridge-flow-ledger.csv")
    require({int(r["nonce"]) for r in deposits} == {150, *range(152, 160)}, "source deposit coverage")
    releases = {int(r["source_bridge_nonce"]): r for r in ledger("bsc-token-flow-ledger.csv")}
    for row in deposits:
        gross, fee, net = (int(row[k]) for k in ("gross_raw", "fee_raw", "net_raw"))
        require(gross - fee == net, "bridge gross/fee/net")
        source = read(f"evidence/terra-transactions/{row['source_tx']}.json")
        message = source["tx"]["body"]["messages"][0]["msg"]
        if isinstance(message, str):
            message = json.loads(base64.b64decode(message))
        require(int(message["send"]["amount"]) == gross, "funded source send")
        matched = False
        for event in source["tx_response"]["events"]:
            attributes = {a["key"]: a["value"] for a in event.get("attributes", [])}
            if attributes.get("xchain_hash_id") == row["xchain_hash_id"]:
                require(int(attributes["amount"]) == net and int(attributes["fee"]) == fee, "source event economics")
                matched = True
        require(matched, "source cross-chain event")
        nonce = int(row["nonce"])
        if nonce == 153:
            require(not row["bsc_execution_tx"] and nonce not in releases, "unsubmitted vFDUSD is not a release")
            continue
        release = releases[nonce]
        require(int(release["amount_raw"]) == net, "release matches net source amount")
        raw = read(f"evidence/bsc-releases/{row['bsc_execution_tx']}.json")
        require(int(raw["receipt"]["status"], 16) == 1, "successful release receipt")
        require(any(row["xchain_hash_id"].lower() in [t.lower() for t in event["topics"]] for event in raw["receipt"]["logs"]), "destination cross-chain event")
        credits = [event for event in raw["receipt"]["logs"] if event["topics"] and event["topics"][0].lower() == TRANSFER and len(event["topics"]) == 3 and ("0x" + event["topics"][2][-40:]).lower() == RECIPIENT]
        require(any(int(e["data"], 16) == net for e in credits), "actual destination token credit")
    require(sum(int(r["net_raw"]) for r in deposits if r["asset"] == "CL8Y") == 32052503948354102807429, "net CL8Y deposits")
    require(sum(int(r["net_raw"]) for r in deposits if r["asset"] == "USDT") == 578336355470164688681, "net USDT deposits")


def verify_bsc():
    transactions = ledger("bsc-transaction-ledger.csv")
    require([int(r["account_nonce"]) for r in transactions] == list(range(56)), "complete nonce interval")
    total_gas = 0
    for row in transactions:
        raw = read(f"evidence/bsc-transactions/{row['tx_hash']}.json")
        tx, receipt = raw["transaction"], raw["receipt"]
        require(tx["from"].lower() == RECIPIENT, "outgoing sender")
        require(int(tx["nonce"], 16) == int(row["account_nonce"]), "outgoing nonce")
        require(int(tx["value"], 16) == int(row["native_value_raw"]) == 0, "zero outer native value")
        require(int(receipt["status"], 16) == int(row["status"]), "outgoing status")
        gas = int(receipt["gasUsed"], 16) * int(receipt.get("effectiveGasPrice", tx["gasPrice"]), 16)
        require(gas == int(row["gas_paid_wei"]), "actual receipt gas")
        total_gas += gas
    require(total_gas == 6202503618620000, "all56 total gas")
    require(Counter(int(r["status"]) for r in transactions) == {1: 50, 0: 6}, "success/failure counts")
    funding = 0
    for row in ledger("bsc-native-funding-ledger.csv"):
        raw = read(f"evidence/bsc-native-funding/{row['tx_hash']}.json")
        tx, receipt = raw["tx"], raw["receipt"]
        require(tx["to"].lower() == RECIPIENT and tx["from"].lower() == row["from_address"].lower(), "funding endpoints")
        require(int(receipt["status"], 16) == 1, "funding success")
        value = int(tx["value"], 16)
        require(value == int(row["amount_raw_wei"]), "funding amount")
        funding += value
    require(funding == 62918448329107, "funding total")
    return total_gas, funding


def verify_sales():
    sales = ledger("bsc-earlier-swap-ledger.csv")
    actual_edges = []
    payout = 0

    def walk(call, tx_hash, path="0"):
        if call.get("error"):
            return
        value = int(call.get("value", "0x0"), 16)
        if value and call.get("type") in {"CALL", "CREATE", "CREATE2", "SELFDESTRUCT"}:
            actual_edges.append((tx_hash, path, call.get("from", "").lower(), call.get("to", "").lower(), value))
        for i, child in enumerate(call.get("calls", [])):
            walk(child, tx_hash, f"{path}.{i}")

    for row in sales:
        tree = read(f"evidence/bsc-call-traces/{row['tx_hash']}.json")
        require(tree["from"].lower() == RECIPIENT, "call-tree sender")
        walk(tree, row["tx_hash"])
        edges = [e for e in actual_edges if e[0] == row["tx_hash"]]
        incoming = sum(e[4] for e in edges if e[3] == RECIPIENT)
        outgoing = sum(e[4] for e in edges if e[2] == RECIPIENT)
        require(incoming == int(row["BNB_received_raw"]) and outgoing == int(row["BNB_sent_raw"]) == 0, "sale native credits")
        raw = read(f"evidence/bsc-transactions/{row['tx_hash']}.json")
        debit = 0
        for event in raw["receipt"]["logs"]:
            topics = event["topics"]
            if event["address"].lower() == CL8Y and len(topics) == 3 and topics[0].lower() == TRANSFER and ("0x" + topics[1][-40:]).lower() == RECIPIENT:
                debit += int(event["data"], 16)
        require(debit == int(row["CL8Y_debited_raw"]), "sale CL8Y debit")
        payout += incoming
    published = [(r["tx_hash"], r["call_path"], r["from_address"].lower(), r["to_address"].lower(), int(r["raw_amount_wei"])) for r in ledger("bsc-native-call-flow-ledger.csv")]
    require(Counter(actual_edges) == Counter(published), "all128 native call edges")
    require(sum(int(r["CL8Y_debited_raw"]) for r in sales) == 6600 * 10**18, "CL8Y sold/debited")
    require(sum(int(r["CL8Y_routing_fee_raw"]) for r in sales) == 165 * 10**17, "included routing fee")
    require(payout == 3943439176513535698, "actual twenty-sale BNB payout")
    return payout


def verify_other_evidence():
    equation = Decimal("1250950") + Decimal("5082600") + Decimal("4324206.487715") - Decimal("4.284242")
    bank = read("evidence/terra-state/relay_bank.json")
    require(int(next(c["amount"] for c in bank["balances"] if c["denom"] == "uluna")) == int(equation * 10**6), "relay LUNC bank reconciliation")
    require(Decimal("44029.5") - Decimal("44.0295") == Decimal("43985.4705"), "Osmosis input split")
    require(Decimal("4353371.354935") - Decimal("22165.299087") - Decimal("6999.568133") == Decimal("4324206.487715"), "Osmosis output split")
    notice = read("evidence/communications/terra-notice-included.json")
    require(int(notice["tx_response"]["code"]) == 0, "later notice included success")
    require(notice["tx_response"]["txhash"] == "1092C2E48B2A1B4A5B8FECE1B42F686B26456227C1A4B4045DADC7142A670183", "notice hash")
    require(len(notice["tx"]["body"]["memo"].encode()) == 224, "notice memo bytes")
    require(notice["tx"]["body"]["messages"][0]["amount"] == [{"amount": "1", "denom": "uluna"}], "notice separate one-uluna amount")
    value = read("evidence/valuation/usd_theft_valuation.json")
    total = sum(Decimal(r["amount"]) * Decimal(r["price_usd"]) for r in value["positions"])
    require(total == Decimal(value["net_usd_total"]) == Decimal("3907.189701326823390163267437"), "dated spot valuation")
    for item in read("evidence/runtime-artifacts/index.json")["artifacts"]:
        file = ROOT / "evidence/runtime-artifacts" / item["artifact"]
        raw = gzip.decompress(file.read_bytes()) if item.get("chain") == "columbus-5" else bytes.fromhex(file.read_text().strip().removeprefix("0x"))
        expected = item.get("uncompressed_sha256", item.get("runtime_sha256"))
        require(hashlib.sha256(raw).hexdigest() == expected, "runtime artifact checksum")


def main():
    files = integrity()
    for name, count in COUNTS.items():
        require(len(ledger(name)) == count, f"row count: {name}")
    verify_source()
    verify_bridge()
    gas, funding = verify_bsc()
    proceeds = verify_sales()
    require(proceeds + funding - gas == 3937299591343244805, "native terminal balance")
    require(32052503948354102807429 - 6600 * 10**18 == 25452503948354102807429, "CL8Y terminal balance")
    verify_other_evidence()
    print(f"PASS: {files} archived files; 10 ledgers; 68 source transactions; 9 deposits; 8 releases; 56 nonces; 20 sale call trees; native/CL8Y/relay/USD reconciliations; runtime checksums.")


if __name__ == "__main__":
    main()
