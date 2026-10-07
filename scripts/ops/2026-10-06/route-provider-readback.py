#!/usr/bin/env python3
"""Read-only funded quote parity for the migrated ALPHA/UST1 market."""
import argparse
import datetime
import hashlib
import json
from pathlib import Path
import sys
from urllib.error import HTTPError

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts/lib"))
from route_tax_rollout_cli import TerradChain, parser as transport_parser


def synthetic_address():
    # A valid public query identity with no key or funded account.
    alphabet = "qpzry9x8gf2tvdw0s3jn54khce6mua7l"
    payload = hashlib.sha256(b"cl8y-release-20261006-readonly-query").digest()[:20]
    bits = "".join(f"{byte:08b}" for byte in payload)
    words = [int(bits[i:i+5], 2) for i in range(0, len(bits), 5)]
    hrp = "terra"
    values = [ord(c) >> 5 for c in hrp] + [0] + [ord(c) & 31 for c in hrp] + words + [0]*6
    chk = 1
    generators = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3]
    for value in values:
        top = chk >> 25
        chk = ((chk & 0x1ffffff) << 5) ^ value
        for i, generator in enumerate(generators):
            if (top >> i) & 1:
                chk ^= generator
    chk ^= 1
    checksum = [(chk >> (5*(5-i))) & 31 for i in range(6)]
    return hrp + "1" + "".join(alphabet[w] for w in words + checksum)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--state-dir", type=Path, default=Path.home()/".local/state/cl8y-dex/release-20261006-route")
    p.add_argument("--provider-smart-query-limit", default="UNKNOWN: obtain the provider's actual configured limit")
    args = p.parse_args()
    journal = json.loads((args.state_dir/"journal.json").read_text())
    if journal["status"] not in ("contracts_ready_paused", "complete"):
        raise SystemExit("Run the migrated fleet verification before provider qualification")
    config = journal["identity"]["config"]
    transport = config["transport"]
    chain = TerradChain(transport_parser().parse_args([
        "--state-dir", str(args.state_dir), "--lcd", transport["lcd"], "--node", transport["node"]]))
    trader = synthetic_address()
    alpha = config["alpha"]
    ust1 = journal["inventory"]["launcher_config"]["ust1"]
    evidence = {"time_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                "provider": transport["lcd"], "provider_smart_query_limit": args.provider_smart_query_limit,
                "chain_id": chain.network(), "manifest_identity": journal["identity_hash"],
                "query_trader": trader, "checks": [], "status": "PASS",
                "limitations": ["Two one-hop pool routes only; populated greedy routes and broader provider envelopes remain separate.",
                                "No signing or execution; unknown provider limits cannot establish a 3M envelope."]}
    if evidence["chain_id"] != config["chain_id"]:
        raise SystemExit("Provider chain differs from the reviewed rollout manifest")
    for offer, ask, target in [(alpha, ust1, "10"), (ust1, alpha, "1000000")]:
        operations = [{"terra_swap": {"offer_asset_info": {"token": {"contract_addr": offer}},
                                      "ask_asset_info": {"token": {"contract_addr": ask}}}}]
        query = {"reverse_simulate_swap_operations": {"ask_amount": target, "operations": operations, "trader": trader}}
        check = {"reverse_request": query}
        try:
            result = chain.smart(config["router"], query)
            check["reverse_response"] = result
            forward = {"simulate_swap_operations": {"offer_amount": result["amount"], "operations": operations, "trader": trader}}
            check["fresh_forward_request"] = forward
            check["fresh_forward_response"] = chain.smart(config["router"], forward)
            assert int(check["fresh_forward_response"]["amount"]) >= int(target), "Fresh forward is insufficient"
            check["status"] = "PASS"
        except Exception as error:
            check["status"] = evidence["status"] = "FAIL"
            check["error"] = str(error)
            if isinstance(error.__cause__, HTTPError):
                check["http_error_body"] = error.__cause__.read().decode(errors="replace")
        evidence["checks"].append(check)
    print(json.dumps(evidence, indent=2))
    return 0 if evidence["status"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
