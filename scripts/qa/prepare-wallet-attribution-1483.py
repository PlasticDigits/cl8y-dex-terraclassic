#!/usr/bin/env python3
"""Read-only deployed checks before an operator verifies #1483 in their wallet.

Coolify credentials are inherited through bao-exec, never printed or put in URLs.
The two anonymous quotes are preparation only; the wallet must obtain a fresh
actor-bound quote and review its principal, fee, expiry and positive floors.
"""
import argparse
import importlib.util
import json
import os
import time
import webbrowser
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import urlencode

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, HERE / filename)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


ATTEST = module("wallet_attester_1483", "attest-trading-quotes-1473.py")
PROBE = module("wallet_probe_1483", "probe-route-cost-attribution-1483.py")
GROSS = "1000000"


def assets():
    first = json.loads((ROOT / "docs/qa/issue-1473/matrix.json").read_text())["assets"]
    first.update(json.loads((ROOT / "docs/qa/issue-1479/matrix.json").read_text())["assets"])
    return {name: first[name] for name in ("LUNC", "cLUNC", "USTR")}


def quote(reader, offer, ask, amount):
    scenario = {"token_in": offer, "token_out": ask, "amount": amount, "spread": "0.05"}
    query = {"token_in": offer, "token_out": ask, "amount_in": amount,
             "max_spread": "0.05", "pool_only": "true", "protection_policy": "quote_relative_v1"}
    body = json.loads(reader.read(ATTEST.BACKEND + "/api/v1/route/solve?" + urlencode(query), 256 * 1024))
    outcome, valid = PROBE.OUTCOMES.classify(200, body, scenario, int(time.time() * 1000))
    if outcome != "executable" or not valid:
        raise ValueError("current_quote_unavailable")
    metadata = body["execution_metadata"]
    if (metadata.get("trader") is not None or metadata.get("sender") is not None
            or body.get("estimated_amount_out") != body["estimated_amount_out_net"]
            or not 1 <= len(metadata["hops"]) <= 4
            or any(operation["terra_swap"].get("hybrid") is not None for operation in body["router_operations"])):
        raise ValueError("anonymous_pool_scope")
    explanation = PROBE.attribution(body)
    if explanation["status"] != "available":
        raise ValueError("attribution_unavailable")
    return body, {"raw_input": amount, "minimum_receive": metadata["minimum_receive"],
                  "generation": metadata["generation"], "observed_at_ms": metadata["observed_at_ms"],
                  "expires_at_ms": metadata["expires_at_ms"], "attribution": explanation}


def link(offer, ask, raw_amount, decimals):
    scale = 10 ** decimals
    amount = int(raw_amount)
    fractional = str(amount % scale).zfill(decimals).rstrip("0")
    human = str(amount // scale) + ("." + fractional if fractional else "")
    return ATTEST.FRONTEND + "/?" + urlencode({"from": offer, "to": ask, "amount": human})


def prepare(revision, reader):
    if not ATTEST.REVISION.fullmatch(revision):
        raise ValueError("exact_revision_required")
    apps = ATTEST.discover(reader, {"frontend": "dex.cl8y.com", "backend": "indexer.dex.cl8y.com"})
    frontend = ATTEST.deployment(reader, apps["frontend"], "dex.cl8y.com")
    backend = ATTEST.deployment(reader, apps["backend"], "indexer.dex.cl8y.com")
    if frontend["source_sha"] != revision or backend["source_sha"] != revision:
        raise ValueError("deployment_revision_mismatch")
    artifact = ATTEST.artifacts(reader)
    health = json.loads(reader.read(ATTEST.BACKEND + "/health", 4096))
    if health.get("git_sha") != revision:
        raise ValueError("health_revision_mismatch")
    identities = assets()
    native, native_summary = quote(reader, identities["LUNC"]["identity"], identities["USTR"]["identity"], GROSS)
    wrapper = native["execution_metadata"]["wrapper"]
    credit = wrapper["route_input_amount"]
    if (wrapper["input_amount"] != GROSS or wrapper["wrap_input"] is not True
            or wrapper["unwrap_output"] is not False or not 0 < PROBE.raw(credit) <= int(GROSS)
            or native["execution_metadata"]["hops"][0]["offer_token"] != identities["cLUNC"]["identity"]
            or native["execution_metadata"]["hops"][0]["offer_amount"] != credit):
        raise ValueError("native_credit_unavailable")
    wrapped, wrapped_summary = quote(reader, identities["cLUNC"]["identity"], identities["USTR"]["identity"], credit)
    if wrapped["execution_metadata"].get("wrapper") is not None:
        raise ValueError("wrapped_control_scope")
    health = json.loads(reader.read(ATTEST.BACKEND + "/health", 4096))
    now = int(time.time() * 1000)
    if health.get("git_sha") != revision or any(now >= sample["expires_at_ms"] for sample in (native_summary, wrapped_summary)):
        raise ValueError("preparation_expired_or_replaced")
    links = {
        "native_1_lunc": link(identities["LUNC"]["identity"], identities["USTR"]["identity"], GROSS, identities["LUNC"]["decimals"]),
        "matched_credit_clunc": link(identities["cLUNC"]["identity"], identities["USTR"]["identity"], credit, identities["cLUNC"]["decimals"]),
    }
    return {"issue": 1483, "status": "PREPARED", "revision": revision,
            "utc": datetime.now(timezone.utc).isoformat(), "read_only": True, "signing": False,
            "frontend": {**frontend, "artifact": artifact}, "backend": backend,
            "samples": {"native_1_lunc": native_summary, "matched_credit_clunc": wrapped_summary},
            "links": links, "read_count": reader.reads,
            "limits": ["Preparation is not settlement or signing permission; obtain a fresh quote in the wallet.",
                       "Anonymous sequential quotes do not prove a common reserve cohort or actor/tier execution.",
                       "Verify selected 5% protection, authorized pay, network fee and minimum receive before one intentional submit.",
                       "After included failure or unknown inclusion, reconcile the saved transaction; never automatically resend."]}


def open_links(links):
    # Browser children have no need for the scoped attestation credentials.
    # webbrowser has no env argument, so expose only the desktop/runtime basics
    # during its synchronous launch and restore this process afterwards.
    allowed = {"PATH", "HOME", "DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY", "DBUS_SESSION_BUS_ADDRESS",
               "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "LANG", "LC_ALL"}
    inherited = dict(os.environ)
    os.environ.clear()
    os.environ.update({key: value for key, value in inherited.items() if key in allowed})
    try:
        return all(webbrowser.open(url, new=2) for url in links.values())
    except Exception:
        return False
    finally:
        os.environ.clear()
        os.environ.update(inherited)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, default=Path.home() / ".local/state/cl8y-dex/issue-1483/wallet-preparation.json")
    parser.add_argument("--open", action="store_true", help="Open both public Swap links only after all read-only checks pass")
    args = parser.parse_args(argv)
    try:
        result = prepare(args.revision, ATTEST.Reader())
    except Exception as error:
        # No exception prose, request URLs, response bodies, credentials or
        # actor fields cross the terminal/private-receipt boundary.
        result = {"issue": 1483, "status": "FAILED", "read_only": True, "signing": False,
                  "failure_class": type(error).__name__}
    args.output.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    with os.fdopen(os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as receipt:
        os.chmod(args.output, 0o600)
        receipt.write(json.dumps(result, indent=2) + "\n")
    if result["status"] == "PREPARED" and args.open:
        open_links(result["links"])
    print(json.dumps({key: result[key] for key in ("status", "links", "failure_class") if key in result}))
    return int(result["status"] != "PREPARED")


if __name__ == "__main__":
    raise SystemExit(main())
