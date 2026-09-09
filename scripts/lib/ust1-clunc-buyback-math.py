#!/usr/bin/env python3
"""Helpers for hourly UST1 → cLUNC best-solver buyback.

Used by scripts/mint-swap-burn-ust1-clunc.sh.
"""
from __future__ import annotations

import argparse
import json
import sys
from decimal import Decimal, ROUND_DOWN, getcontext
from typing import Any

getcontext().prec = 80

BPS_SCALE = 10000


def _d(x: Any) -> Decimal:
    if isinstance(x, Decimal):
        return x
    return Decimal(str(x))


def mint_raw(human: str, decimals: int) -> int:
    if decimals < 0 or decimals > 18:
        raise ValueError("decimals")
    h = _d(human)
    if h <= 0:
        raise ValueError("human mint must be positive")
    scale = Decimal(10) ** decimals
    raw = (h * scale).to_integral_value(rounding=ROUND_DOWN)
    if raw <= 0:
        raise ValueError("mint raw underflow")
    return int(raw)


def human_amount(raw: int, decimals: int) -> Decimal:
    if decimals < 0:
        raise ValueError("decimals")
    return _d(raw) / (Decimal(10) ** decimals)


def usd_from_raw(raw: int, decimals: int, price: str) -> Decimal:
    px = _d(price)
    if px <= 0:
        raise ValueError("price must be positive")
    return (human_amount(raw, decimals) * px).quantize(Decimal("0.00000001"), rounding=ROUND_DOWN)


def slippage_percent_to_bps(percent: str) -> int:
    p = _d(percent)
    if p < 0 or p > 100:
        raise ValueError("slippage percent out of range")
    return min(10000, max(0, int((p * 100).to_integral_value(rounding=ROUND_DOWN))))


def min_receive(estimated_out: int, slip_percent: str) -> int:
    if estimated_out <= 0:
        raise ValueError("estimated_out")
    bps = slippage_percent_to_bps(slip_percent)
    return (estimated_out * (BPS_SCALE - bps)) // BPS_SCALE


def delta_raw(before: int, after: int) -> int:
    if after < before:
        raise ValueError("balance decreased")
    return after - before


def progress_met(current_usd: str, target_usd: str) -> bool:
    return _d(current_usd) >= _d(target_usd)


def remaining_usd(current_usd: str, target_usd: str) -> Decimal:
    rem = _d(target_usd) - _d(current_usd)
    return rem if rem > 0 else Decimal(0)


def _hybrid_nonzero(h: Any) -> bool:
    if not isinstance(h, dict):
        return False
    pool = str(h.get("pool_input") or "0")
    book = str(h.get("book_input") or "0")
    return pool != "0" or book != "0"


def normalize_router_operations(ops: list[Any]) -> list[dict[str, Any]]:
    if not isinstance(ops, list) or not ops:
        raise ValueError("router_operations must be a non-empty list")
    if len(ops) > 4:
        raise ValueError("router_operations longer than 4 hops")
    out: list[dict[str, Any]] = []
    for i, op in enumerate(ops):
        if not isinstance(op, dict) or "terra_swap" not in op:
            raise ValueError(f"hop {i}: expected terra_swap")
        ts = op["terra_swap"]
        if not isinstance(ts, dict):
            raise ValueError(f"hop {i}: terra_swap object")
        if ts.get("greedy"):
            raise ValueError(
                f"hop {i}: greedy is not a solver quote; this buyback executes GET /route/solve/best"
            )
        offer = ts.get("offer_asset_info")
        ask = ts.get("ask_asset_info")
        if not isinstance(offer, dict) or not isinstance(ask, dict):
            raise ValueError(f"hop {i}: missing asset info")
        entry: dict[str, Any] = {
            "offer_asset_info": offer,
            "ask_asset_info": ask,
        }
        hybrid = ts.get("hybrid")
        if _hybrid_nonzero(hybrid):
            h = hybrid
            entry["hybrid"] = {
                "pool_input": str(h.get("pool_input") or "0"),
                "book_input": str(h.get("book_input") or "0"),
                "max_maker_fills": int(h.get("max_maker_fills") or 8),
            }
            hint = h.get("book_start_hint")
            if hint is not None:
                entry["hybrid"]["book_start_hint"] = int(hint)
        min_ret = ts.get("min_return")
        if min_ret not in (None, ""):
            entry["min_return"] = str(min_ret)
        out.append({"terra_swap": entry})
    return out


def build_execute_hook(
    ops: list[Any],
    max_spread: str,
    minimum_receive: str,
    *,
    to: str | None = None,
) -> dict[str, Any]:
    if int(minimum_receive) <= 0:
        raise ValueError("minimum_receive")
    inner: dict[str, Any] = {
        "execute_swap_operations": {
            "operations": normalize_router_operations(ops),
            "max_spread": str(max_spread),
            "minimum_receive": str(minimum_receive),
            "to": to,
            "deadline": None,
        }
    }
    return inner


def build_cw20_send(router: str, amount: str, hook: dict[str, Any]) -> dict[str, Any]:
    if int(amount) <= 0:
        raise ValueError("send amount")
    if not router.startswith("terra1"):
        raise ValueError("router")
    return {
        "send": {
            "contract": router,
            "amount": str(amount),
            "msg": hook,
        }
    }


def self_test() -> None:
    assert mint_raw("50", 6) == 50_000_000
    assert mint_raw("50.5", 6) == 50_500_000
    assert human_amount(1_000_000, 6) == Decimal("1")
    assert usd_from_raw(1_000_000, 6, "0.00008") == Decimal("0.00008000")
    assert slippage_percent_to_bps("5") == 500
    assert min_receive(1_000_000, "5") == 950_000
    assert min_receive(100, "5") == 95
    assert delta_raw(10, 25) == 15
    assert progress_met("2500", "2500")
    assert progress_met("2500.01", "2500")
    assert not progress_met("2499.99", "2500")
    assert remaining_usd("100", "2500") == Decimal("2400")
    assert remaining_usd("3000", "2500") == Decimal("0")

    pool_only = [
        {
            "terra_swap": {
                "offer_asset_info": {"token": {"contract_addr": "terra1in"}},
                "ask_asset_info": {"token": {"contract_addr": "terra1out"}},
                "hybrid": None,
            }
        }
    ]
    norm = normalize_router_operations(pool_only)
    assert "hybrid" not in norm[0]["terra_swap"]

    hybrid = [
        {
            "terra_swap": {
                "offer_asset_info": {"token": {"contract_addr": "terra1in"}},
                "ask_asset_info": {"token": {"contract_addr": "terra1out"}},
                "hybrid": {
                    "pool_input": "900",
                    "book_input": "100",
                    "max_maker_fills": 8,
                    "book_start_hint": 12,
                },
            }
        }
    ]
    hnorm = normalize_router_operations(hybrid)
    assert hnorm[0]["terra_swap"]["hybrid"]["book_input"] == "100"
    assert hnorm[0]["terra_swap"]["hybrid"]["book_start_hint"] == 12

    try:
        normalize_router_operations(
            [
                {
                    "terra_swap": {
                        "offer_asset_info": {"token": {"contract_addr": "terra1in"}},
                        "ask_asset_info": {"token": {"contract_addr": "terra1out"}},
                        "greedy": {"max_maker_fills": 8},
                    }
                }
            ]
        )
        raise AssertionError("greedy should reject")
    except ValueError as exc:
        assert "greedy" in str(exc)

    hook = build_execute_hook(pool_only, "0.20", "950000")
    send = build_cw20_send("terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw", "50000000", hook)
    assert send["send"]["amount"] == "50000000"
    assert send["send"]["msg"]["execute_swap_operations"]["minimum_receive"] == "950000"
    print("self-test ok")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    t = sub.add_parser("self-test")
    t.set_defaults(cmd="self-test")

    p = sub.add_parser("mint-raw")
    p.add_argument("--human", required=True)
    p.add_argument("--decimals", type=int, required=True)

    p = sub.add_parser("min-receive")
    p.add_argument("--out", required=True)
    p.add_argument("--slip", required=True)

    p = sub.add_parser("usd")
    p.add_argument("--raw", required=True)
    p.add_argument("--decimals", type=int, required=True)
    p.add_argument("--price", required=True)

    p = sub.add_parser("delta")
    p.add_argument("--before", required=True)
    p.add_argument("--after", required=True)

    p = sub.add_parser("progress")
    p.add_argument("--current", required=True)
    p.add_argument("--target", required=True)

    p = sub.add_parser("remaining")
    p.add_argument("--current", required=True)
    p.add_argument("--target", required=True)

    p = sub.add_parser("send-msg")
    p.add_argument("--quote-file", required=True)
    p.add_argument("--amount", required=True)
    p.add_argument("--router", required=True)
    p.add_argument("--max-spread", required=True)
    p.add_argument("--min-receive", required=True)
    p.add_argument("--to", default="")

    args = parser.parse_args()
    if args.cmd == "self-test":
        self_test()
        return 0
    if args.cmd == "mint-raw":
        print(mint_raw(args.human, args.decimals))
        return 0
    if args.cmd == "min-receive":
        print(min_receive(int(args.out), args.slip))
        return 0
    if args.cmd == "usd":
        print(usd_from_raw(int(args.raw), args.decimals, args.price))
        return 0
    if args.cmd == "delta":
        print(delta_raw(int(args.before), int(args.after)))
        return 0
    if args.cmd == "progress":
        sys.exit(0 if progress_met(args.current, args.target) else 1)
    if args.cmd == "remaining":
        print(remaining_usd(args.current, args.target))
        return 0
    if args.cmd == "send-msg":
        with open(args.quote_file, encoding="utf-8") as fh:
            quote = json.load(fh)
        ops = quote.get("router_operations")
        hook = build_execute_hook(
            ops,
            args.max_spread,
            args.min_receive,
            to=args.to or None,
        )
        # CosmWasm send.msg is binary; the shell base64-encodes this JSON.
        json.dump(hook, sys.stdout, separators=(",", ":"))
        print()
        return 0
    raise SystemExit(f"unknown cmd {args.cmd}")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 — operator CLI
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
