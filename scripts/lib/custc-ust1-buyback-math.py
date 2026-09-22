#!/usr/bin/env python3
"""Helpers for hourly $200 cUSTC → UST1 best-solver buyback.

Used by scripts/mint-swap-custc-ust1.sh.
Router send-hook / min-receive live in ust1-clunc-buyback-math.py (shared).
"""
from __future__ import annotations

import argparse
import sys
from decimal import Decimal, ROUND_DOWN, getcontext
from typing import Any

getcontext().prec = 80

# Live USTC is ~5e-3. Far outside this band is a ticker/oracle mixup.
USTC_USD_MIN = Decimal("1e-5")
USTC_USD_MAX = Decimal("0.1")
# CEX FDUSD ~1. Venus redeem is typically 1–2 FDUSD per vFDUSD.
FDUSD_USD_MIN = Decimal("0.5")
FDUSD_USD_MAX = Decimal("2")
VENUS_MIN = Decimal("0.5")
VENUS_MAX = Decimal("5")


def _d(x: Any) -> Decimal:
    if isinstance(x, Decimal):
        return x
    return Decimal(str(x))


def check_ustc_usd(price: str) -> Decimal:
    px = _d(price)
    if px <= 0:
        raise ValueError("USTC/USD must be positive")
    if px < USTC_USD_MIN or px > USTC_USD_MAX:
        raise ValueError(f"USTC/USD {px} outside {USTC_USD_MIN}..{USTC_USD_MAX}")
    return px


def check_fdusd_usd(price: str) -> Decimal:
    px = _d(price)
    if px <= 0:
        raise ValueError("FDUSD/USD must be positive")
    if px < FDUSD_USD_MIN or px > FDUSD_USD_MAX:
        raise ValueError(f"FDUSD/USD {px} outside {FDUSD_USD_MIN}..{FDUSD_USD_MAX}")
    return px


def check_venus(rate: str) -> Decimal:
    r = _d(rate)
    if r <= 0:
        raise ValueError("fdusd_per_vfdusd must be positive")
    if r < VENUS_MIN or r > VENUS_MAX:
        raise ValueError(f"fdusd_per_vfdusd {r} outside {VENUS_MIN}..{VENUS_MAX}")
    return r


def human_amount(raw: int, decimals: int) -> Decimal:
    if decimals < 0 or decimals > 18:
        raise ValueError("decimals")
    return _d(raw) / (Decimal(10) ** decimals)


def usd_to_raw(usd: str, price: str, decimals: int) -> int:
    """Raw tokens worth `usd` at `price` USD/token. Round down so a tick stays ≤ budget."""
    if decimals < 0 or decimals > 18:
        raise ValueError("decimals")
    px = check_ustc_usd(price)
    budget = _d(usd)
    if budget <= 0:
        raise ValueError("usd mint must be positive")
    human = budget / px
    raw = (human * (Decimal(10) ** decimals)).to_integral_value(rounding=ROUND_DOWN)
    if raw <= 0:
        raise ValueError("mint raw underflow")
    return int(raw)


def usd_from_raw(raw: int, decimals: int, price: str) -> Decimal:
    px = _d(price)
    if px <= 0:
        raise ValueError("price must be positive")
    return (human_amount(raw, decimals) * px).quantize(Decimal("0.00000001"), rounding=ROUND_DOWN)


def vfdusd_usd_per_token(fdusd_usd: str, fdusd_per_vfdusd: str) -> Decimal:
    """USD of 1 human vFDUSD = Venus FDUSD-per-vFDUSD × CEX FDUSD/USD. Not $1."""
    return (check_venus(fdusd_per_vfdusd) * check_fdusd_usd(fdusd_usd)).quantize(
        Decimal("0.00000001"), rounding=ROUND_DOWN
    )


def vfdusd_holding_usd(raw: int, decimals: int, fdusd_usd: str, fdusd_per_vfdusd: str) -> Decimal:
    px = vfdusd_usd_per_token(fdusd_usd, fdusd_per_vfdusd)
    return usd_from_raw(raw, decimals, str(px))


def progress_met(current_usd: str, target_usd: str) -> bool:
    return _d(current_usd) >= _d(target_usd)


def remaining_usd(current_usd: str, target_usd: str) -> Decimal:
    rem = _d(target_usd) - _d(current_usd)
    return rem if rem > 0 else Decimal(0)


def self_test() -> None:
    assert usd_to_raw("50", "0.008", 6) == 6_250_000_000
    assert usd_to_raw("200", "0.008", 6) == 25_000_000_000
    assert usd_to_raw("50", "0.01", 6) == 5_000_000_000
    # Round down: 50 / 0.007 does not overshoot $50.
    raw = usd_to_raw("50", "0.007", 6)
    assert usd_from_raw(raw, 6, "0.007") <= Decimal("50")
    raw200 = usd_to_raw("200", "0.007", 6)
    assert usd_from_raw(raw200, 6, "0.007") <= Decimal("200")
    try:
        usd_to_raw("50", "1.00", 6)
        raise AssertionError("USTC=$1 should reject")
    except ValueError as exc:
        assert "USTC/USD" in str(exc)

    assert vfdusd_usd_per_token("1.00", "1.08") == Decimal("1.08000000")
    # 500 human vFDUSD × 1.08 FDUSD × $1 = $540
    assert vfdusd_holding_usd(500_000_000, 6, "1", "1.08") == Decimal("540.00000000")
    assert vfdusd_holding_usd(400_000_000, 6, "1", "1") == Decimal("400.00000000")
    try:
        vfdusd_usd_per_token("1", "1")
    except ValueError:
        raise AssertionError("1.0 Venus × $1 FDUSD must be in band") from None

    assert progress_met("500", "500")
    assert progress_met("500.01", "500")
    assert not progress_met("499.99", "500")
    assert remaining_usd("100", "500") == Decimal("400")
    assert remaining_usd("600", "500") == Decimal("0")
    print("self-test ok")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    t = sub.add_parser("self-test")
    t.set_defaults(cmd="self-test")

    p = sub.add_parser("mint-raw-usd")
    p.add_argument("--usd", required=True)
    p.add_argument("--price", required=True)
    p.add_argument("--decimals", type=int, required=True)

    p = sub.add_parser("usd")
    p.add_argument("--raw", required=True)
    p.add_argument("--decimals", type=int, required=True)
    p.add_argument("--price", required=True)

    p = sub.add_parser("vfdusd-usd")
    p.add_argument("--raw", required=True)
    p.add_argument("--decimals", type=int, required=True)
    p.add_argument("--fdusd-usd", required=True)
    p.add_argument("--venus", required=True)

    p = sub.add_parser("vfdusd-px")
    p.add_argument("--fdusd-usd", required=True)
    p.add_argument("--venus", required=True)

    p = sub.add_parser("progress")
    p.add_argument("--current", required=True)
    p.add_argument("--target", required=True)

    p = sub.add_parser("remaining")
    p.add_argument("--current", required=True)
    p.add_argument("--target", required=True)

    args = parser.parse_args()
    if args.cmd == "self-test":
        self_test()
        return 0
    if args.cmd == "mint-raw-usd":
        print(usd_to_raw(args.usd, args.price, args.decimals))
        return 0
    if args.cmd == "usd":
        print(usd_from_raw(int(args.raw), args.decimals, args.price))
        return 0
    if args.cmd == "vfdusd-usd":
        print(vfdusd_holding_usd(int(args.raw), args.decimals, args.fdusd_usd, args.venus))
        return 0
    if args.cmd == "vfdusd-px":
        print(vfdusd_usd_per_token(args.fdusd_usd, args.venus))
        return 0
    if args.cmd == "progress":
        sys.exit(0 if progress_met(args.current, args.target) else 1)
    if args.cmd == "remaining":
        print(remaining_usd(args.current, args.target))
        return 0
    raise SystemExit(f"unknown cmd {args.cmd}")


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 — operator CLI
        print(f"ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)
