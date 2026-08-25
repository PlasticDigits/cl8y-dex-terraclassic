#!/usr/bin/env python3
"""Paginate LCD contracts for a code id and probe wasm-admin + cw2 (GitLab #627).

Usage:
  census-code-instances.py 3
  census-code-instances.py 3 --limit-pages 20   # smoke / CI

Writes codeids/<id>/census.json. No mnemonics. Fail closed on LCD errors.
"""
from __future__ import annotations

import argparse
import base64
import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

LCD_DEFAULT = "https://terra-classic-lcd.publicnode.com"
FACTORY_C5 = "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
KNOWN = {
    "MIR": "terra15gwkyepfc6xgca5t5zefzwy42uts8l2m4g40k6",
    "KUJI": "terra1xfsdgcemqwxp4hhnyk4rle6wr22sseq7j07dnn",
    "sKUJI": "terra188w26t95tf4dz77raftme8p75rggatxjxfeknw",
    "VKR": "terra1dy9kmlm4anr92e42mrkjwzyvfqwz66un00rwr5",
    "WHALE": "terra1php5m8a6qd68z02t3zpw4jv2pj4vgw4wz0t8mz",
    "TWD": "terra19djkaepjjswucys4npd5ltaxgsntl7jf0xz7w6",
}


def get_json(url: str, timeout: int = 45) -> dict:
    req = urllib.request.Request(url, headers={"User-Agent": "cl8y-cw20-census/627"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode())


def smart_query(lcd: str, contract: str, msg: dict) -> dict:
    raw = json.dumps(msg, separators=(",", ":")).encode()
    b64 = base64.b64encode(raw).decode()
    url = f"{lcd}/cosmwasm/wasm/v1/contract/{contract}/smart/{b64}"
    return get_json(url)


def contract_info(lcd: str, contract: str) -> dict:
    url = f"{lcd}/cosmwasm/wasm/v1/contract/{contract}"
    return get_json(url)


def paginate_contracts(lcd: str, code_id: int, page_limit: int, max_pages: int | None) -> dict:
    contracts: list[str] = []
    next_key = None
    pages = 0
    while True:
        if max_pages is not None and pages >= max_pages:
            break
        qs = {"pagination.limit": str(page_limit)}
        if next_key:
            qs["pagination.key"] = next_key
        url = f"{lcd}/cosmwasm/wasm/v1/code/{code_id}/contracts?{urllib.parse.urlencode(qs)}"
        body = get_json(url)
        batch = body.get("contracts") or []
        contracts.extend(batch)
        pages += 1
        nxt = (body.get("pagination") or {}).get("next_key") or ""
        print(f"page {pages}: +{len(batch)} (running {len(contracts)})", flush=True)
        if not nxt or not batch:
            next_key = None
            break
        next_key = nxt
        time.sleep(0.05)
    return {
        "count": len(contracts),
        "pages": pages,
        "truncated": bool(next_key),
        "sample_head": contracts[:15],
        "sample_tail": contracts[-5:] if contracts else [],
        "contracts": contracts,
    }


def probe_instance(lcd: str, addr: str) -> dict:
    out: dict = {"address": addr}
    try:
        info = contract_info(lcd, addr)
        ci = info.get("contract_info") or {}
        out["code_id"] = str(ci.get("code_id") or "")
        out["admin"] = ci.get("admin") or ""
        out["label"] = ci.get("label") or ""
        out["ibc_port_id"] = ci.get("ibc_port_id") or ""
    except (urllib.error.HTTPError, urllib.error.URLError, json.JSONDecodeError, TimeoutError) as e:
        out["contract_info_error"] = str(e)
        return out
    for name, msg in (
        ("token_info", {"token_info": {}}),
        ("minter", {"minter": {}}),
        ("tax_map", {"tax_map": {}}),
    ):
        try:
            out[name] = smart_query(lcd, addr, msg).get("data")
        except urllib.error.HTTPError as e:
            err_body = e.read().decode(errors="replace")[:240]
            out[f"{name}_error"] = f"HTTP {e.code}: {err_body}"
        except (urllib.error.URLError, json.JSONDecodeError, TimeoutError) as e:
            out[f"{name}_error"] = str(e)
    try:
        raw_key = base64.b64encode(b"\x00\x0dcontract_info").decode()
        raw = get_json(f"{lcd}/cosmwasm/wasm/v1/contract/{addr}/raw/{raw_key}")
        data = raw.get("data")
        if data:
            out["cw2_raw"] = json.loads(base64.b64decode(data))
    except Exception as e:  # noqa: BLE001 — census residual
        out["cw2_raw_error"] = str(e)[:200]
    return out


def factory_whitelist(lcd: str) -> dict:
    try:
        data = smart_query(lcd, FACTORY_C5, {"get_whitelisted_code_ids": {}})
        return {"ok": True, "data": data.get("data")}
    except Exception as e:  # noqa: BLE001
        return {"ok": False, "error": str(e)[:300]}


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("code_id", type=int)
    p.add_argument("--lcd", default=LCD_DEFAULT)
    p.add_argument("--limit-pages", type=int, default=None)
    p.add_argument("--page-size", type=int, default=100)
    p.add_argument("--skip-list", action="store_true", help="probe known tokens only")
    args = p.parse_args()
    lcd = args.lcd.rstrip("/")
    root = Path(__file__).resolve().parents[1]
    dest_dir = root / "codeids" / str(args.code_id)
    dest_dir.mkdir(parents=True, exist_ok=True)

    listing = {"count": 0, "pages": 0, "truncated": False, "sample_head": [], "sample_tail": []}
    if not args.skip_list:
        listing = paginate_contracts(lcd, args.code_id, args.page_size, args.limit_pages)

    known_probes = {}
    for label, addr in KNOWN.items():
        print(f"probe {label} {addr}", flush=True)
        known_probes[label] = probe_instance(lcd, addr)
        time.sleep(0.05)

    admin_sample = []
    for addr in listing.get("sample_head", [])[:12]:
        print(f"admin-sample {addr}", flush=True)
        admin_sample.append(probe_instance(lcd, addr))
        time.sleep(0.05)

    census = {
        "code_id": args.code_id,
        "lcd": lcd,
        "factory": FACTORY_C5,
        "factory_whitelist": factory_whitelist(lcd),
        "instance_count": listing["count"],
        "pages": listing["pages"],
        "truncated": listing["truncated"],
        "sample_head": listing["sample_head"],
        "sample_tail": listing["sample_tail"],
        "known": known_probes,
        "admin_sample": admin_sample,
        "note": (
            "Approving this code id admits every instantiate (C589-4 / B13). "
            "Do not treat this file as a license to whitelist or append migrate env."
        ),
    }
    # Do not commit the full address list (huge). Count + samples only.
    out = dest_dir / "census.json"
    out.write_text(json.dumps(census, indent=2) + "\n", encoding="utf-8")
    print(f"PASS: wrote {out} count={listing['count']} truncated={listing['truncated']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
