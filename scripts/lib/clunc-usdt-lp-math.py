#!/usr/bin/env python3
"""Size $N cLUNC (LUNC/USD oracle) + $N USDT for an empty cLUNC/USDT seed.

Used by scripts/mint-clunc-usdt-lp.sh. USDT is 18 decimals — never compare
raw amounts in bash arithmetic (1e21 overflows signed 64-bit).
"""
from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from decimal import ROUND_HALF_UP, Decimal, getcontext
from typing import Any

getcontext().prec = 80

# Live LUNC is ~5e-5. Far outside this band is a ticker/oracle mixup.
LUNC_USD_MIN = Decimal("1e-8")
LUNC_USD_MAX = Decimal("0.01")


def _d(x: Any) -> Decimal:
    if isinstance(x, Decimal):
        return x
    return Decimal(str(x))


def _i(x: Any) -> int:
    return int(str(x).strip())


def usd_to_raw(usd: Any, price: Any, decimals: int) -> int:
    px = _d(price)
    if px <= 0:
        raise ValueError(f"price must be positive (got {price})")
    human = _d(usd) / px
    raw = (human * (Decimal(10) ** int(decimals))).to_integral_value(rounding=ROUND_HALF_UP)
    return int(raw)


def human_from_raw(raw: int, decimals: int) -> Decimal:
    return _d(raw) / (Decimal(10) ** int(decimals))


def parse_oracle(payload: dict[str, Any], max_age_sec: int) -> dict[str, Any]:
    errors: list[str] = []
    if payload.get("ticker") != "lunc":
        errors.append(f"indexer ticker {payload.get('ticker')!r} != 'lunc'")
    px = payload.get("price_usd")
    if px in (None, ""):
        errors.append("indexer lunc price_usd empty")
        return {"ok": False, "errors": errors}

    try:
        lunc_usd = _d(px)
    except Exception as exc:
        return {"ok": False, "errors": [f"bad price_usd {px!r}: {exc}"]}

    if lunc_usd <= 0:
        errors.append(f"LUNC/USD {lunc_usd} is not positive")
    elif lunc_usd < LUNC_USD_MIN or lunc_usd > LUNC_USD_MAX:
        errors.append(
            f"LUNC/USD {lunc_usd} outside sanity band [{LUNC_USD_MIN}, {LUNC_USD_MAX}]"
        )

    avg_at = None
    for src in payload.get("sources") or []:
        if src.get("source") == "average":
            avg_at = src.get("fetched_at")
            break
    age_sec: float | None = None
    if avg_at:
        raw_ts = str(avg_at).replace("Z", "+00:00")
        try:
            dt = datetime.fromisoformat(raw_ts)
        except ValueError:
            errors.append(f"unparseable fetched_at {avg_at!r}")
        else:
            if dt.tzinfo is None:
                dt = dt.replace(tzinfo=timezone.utc)
            age_sec = (datetime.now(timezone.utc) - dt.astimezone(timezone.utc)).total_seconds()
            if age_sec > max_age_sec:
                errors.append(f"indexer lunc average is {int(age_sec)}s old (max {max_age_sec})")

    return {
        "ok": not errors,
        "lunc_usd": str(lunc_usd),
        "fetched_at": avg_at,
        "age_sec": None if age_sec is None else int(age_sec),
        "errors": errors,
    }


def size(inp: dict[str, Any]) -> dict[str, Any]:
    errors: list[str] = []
    try:
        usd_each = _d(inp.get("usd_each") or "1000")
        lunc_usd = _d(inp["lunc_usd"])
        usdt_usd = _d(inp.get("usdt_usd") or "1")
        dec_clunc = int(inp.get("dec_clunc") or 6)
        dec_usdt = int(inp.get("dec_usdt") or 18)
        bal_clunc = _i(inp.get("bal_clunc") or 0)
        bal_usdt = _i(inp.get("bal_usdt") or 0)
    except Exception as exc:
        return {"ok": False, "errors": [str(exc)]}

    if usd_each <= 0:
        errors.append(f"usd_each must be positive (got {usd_each})")
    if usdt_usd <= 0:
        errors.append(f"usdt_usd must be positive (got {usdt_usd})")
    if lunc_usd <= 0:
        errors.append(f"lunc_usd must be positive (got {lunc_usd})")
    elif lunc_usd < LUNC_USD_MIN or lunc_usd > LUNC_USD_MAX:
        errors.append(
            f"LUNC/USD {lunc_usd} outside sanity band [{LUNC_USD_MIN}, {LUNC_USD_MAX}]"
        )
    if errors:
        return {"ok": False, "errors": errors}

    clunc_raw = usd_to_raw(usd_each, lunc_usd, dec_clunc)
    usdt_raw = usd_to_raw(usd_each, usdt_usd, dec_usdt)
    if clunc_raw <= 0 or usdt_raw <= 0:
        return {"ok": False, "errors": ["sized raw amount is 0"]}

    mint_clunc = max(0, clunc_raw - bal_clunc)
    usdt_short = max(0, usdt_raw - bal_usdt)
    if usdt_short > 0:
        errors.append(
            f"USDT balance {bal_usdt} < need {usdt_raw} "
            f"(short {usdt_short} raw; do not mint USDT)"
        )

    clunc_human = human_from_raw(clunc_raw, dec_clunc)
    usdt_human = human_from_raw(usdt_raw, dec_usdt)
    return {
        "ok": not errors,
        "errors": errors,
        "usd_each": str(usd_each),
        "lunc_usd": str(lunc_usd),
        "usdt_usd": str(usdt_usd),
        "tvl_usd": str(usd_each * 2),
        "dec_clunc": dec_clunc,
        "dec_usdt": dec_usdt,
        "clunc_raw": str(clunc_raw),
        "usdt_raw": str(usdt_raw),
        "clunc_human": format(clunc_human, "f"),
        "usdt_human": format(usdt_human, "f"),
        "mint_clunc": str(mint_clunc),
        "bal_clunc": str(bal_clunc),
        "bal_usdt": str(bal_usdt),
        "usdt_short": str(usdt_short),
    }


def price_moved(old: Any, new: Any, tolerance: Any) -> bool:
    a, b, tol = _d(old), _d(new), _d(tolerance)
    if a <= 0 or b <= 0:
        return True
    return abs(a - b) / a > tol


def self_test() -> None:
    px = Decimal("0.000050815")
    clunc = usd_to_raw("1000", px, 6)
    usdt = usd_to_raw("1000", "1", 18)
    assert usdt == 10**21, usdt
    expect_clunc = int(
        (Decimal("1000") / px * (Decimal(10) ** 6)).to_integral_value(rounding=ROUND_HALF_UP)
    )
    assert clunc == expect_clunc == 19679228574240, clunc
    roundtrip = human_from_raw(clunc, 6) * px
    assert abs(roundtrip - Decimal("1000")) < Decimal("0.000001"), roundtrip

    sized = size(
        {
            "usd_each": "1000",
            "lunc_usd": str(px),
            "usdt_usd": "1",
            "dec_clunc": 6,
            "dec_usdt": 18,
            "bal_clunc": "580000000",
            "bal_usdt": "1019875133330000000000",
        }
    )
    assert sized["ok"] is True, sized
    assert sized["clunc_raw"] == "19679228574240"
    assert sized["usdt_raw"] == "1000000000000000000000"
    assert sized["mint_clunc"] == str(19679228574240 - 580000000)

    short = size(
        {
            "usd_each": "1000",
            "lunc_usd": str(px),
            "usdt_usd": "1",
            "bal_clunc": "0",
            "bal_usdt": "1",
        }
    )
    assert short["ok"] is False
    assert any("do not mint USDT" in e for e in short["errors"])

    bad_px = size({"usd_each": "1000", "lunc_usd": "1", "usdt_usd": "1"})
    assert bad_px["ok"] is False

    oracle = parse_oracle(
        {
            "ticker": "lunc",
            "price_usd": str(px),
            "sources": [
                {
                    "source": "average",
                    "price_usd": str(px),
                    "fetched_at": datetime.now(timezone.utc).isoformat(),
                }
            ],
        },
        1800,
    )
    assert oracle["ok"] is True, oracle
    assert not price_moved(px, px, "0.01")
    assert price_moved(px, px * Decimal("1.02"), "0.01")
    print("clunc-usdt-lp-math self-test OK")


def main() -> int:
    if "--self-test" in sys.argv:
        self_test()
        return 0
    if "--parse-oracle" in sys.argv:
        max_age = 1800
        for i, arg in enumerate(sys.argv):
            if arg == "--max-age" and i + 1 < len(sys.argv):
                max_age = int(sys.argv[i + 1])
        payload = json.loads(sys.stdin.read() or "{}")
        json.dump(parse_oracle(payload, max_age), sys.stdout)
        sys.stdout.write("\n")
        return 0
    raw = sys.stdin.read()
    inp = json.loads(raw) if raw.strip() else {}
    mode = inp.get("mode") or "size"
    if mode == "size":
        json.dump(size(inp), sys.stdout)
        sys.stdout.write("\n")
        return 0
    if mode == "parse-oracle":
        json.dump(
            parse_oracle(inp.get("payload") or {}, int(inp.get("max_age_sec") or 1800)),
            sys.stdout,
        )
        sys.stdout.write("\n")
        return 0
    if mode == "price-moved":
        moved = price_moved(inp["old"], inp["new"], inp.get("tolerance") or "0.01")
        json.dump({"moved": moved}, sys.stdout)
        sys.stdout.write("\n")
        return 0
    print("unknown mode", mode, file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
