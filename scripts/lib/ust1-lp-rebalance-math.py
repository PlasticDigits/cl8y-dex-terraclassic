#!/usr/bin/env python3
"""Plan UST1/cUSTC rebalance + $N LP sizes for UST1/cUSTC and UST1/USTR.

Used by scripts/rebalance-mint-ust1-lp.sh. Constant-product math matches pair
swap (ceil k/new_in, commission taken from gross output).
"""
from __future__ import annotations

import json
import sys
from decimal import Decimal, ROUND_DOWN, getcontext
from typing import Any

getcontext().prec = 80

USTR_PER_USTC = Decimal("2.5")


def _d(x: Any) -> Decimal:
    if isinstance(x, Decimal):
        return x
    return Decimal(str(x))


def _i(x: Any) -> int:
    return int(_d(x))


def human_price(r0: int, r1: int, d0: int, d1: int) -> Decimal:
    if r0 <= 0 or r1 <= 0:
        raise ValueError("empty pool")
    return (_d(r1) / (Decimal(10) ** d1)) / (_d(r0) / (Decimal(10) ** d0))


def rel_error(current: Decimal, target: Decimal) -> Decimal:
    if target <= 0:
        raise ValueError("target must be positive")
    return abs(current - target) / target


def within_tolerance(current: Decimal, target: Decimal, tol: Decimal) -> bool:
    return rel_error(current, target) <= tol


def ceil_div(num: int, den: int) -> int:
    if den <= 0:
        raise ValueError("denom")
    return (num + den - 1) // den


def simulate_pool_swap(r_in: int, r_out: int, offer: int, fee_bps: int) -> tuple[int, int, int]:
    """Return (new_in, new_out, user_return) after a pool-only swap."""
    if offer <= 0:
        return r_in, r_out, 0
    if r_in <= 0 or r_out <= 0:
        raise ValueError("empty reserves")
    k = r_in * r_out
    new_in = r_in + offer
    new_out = ceil_div(k, new_in)
    if new_out > r_out:
        new_out = r_out
    gross = r_out - new_out
    commission = gross * fee_bps // 10000
    ret = gross - commission
    return new_in, new_out, ret


def target_custc_per_ust1(ustc_usd: Decimal) -> Decimal:
    if ustc_usd <= 0:
        raise ValueError("ustc_usd")
    return Decimal(1) / ustc_usd


def target_ustr_per_ust1(ustc_usd: Decimal, ustr_per_ustc: Decimal = USTR_PER_USTC) -> Decimal:
    return Decimal(1) / (ustc_usd * ustr_per_ustc)


def find_rebalance_offer(
    r0: int,
    r1: int,
    d0: int,
    d1: int,
    target: Decimal,
    fee_bps: int,
    tol: Decimal,
    max_frac: Decimal = Decimal("0.35"),
) -> dict[str, Any]:
    """Binary-search a pool-only offer that lands human quote/base within tol of target.

    token0 is UST1 (base), token1 is quote (cUSTC). Price = quote_human / base_human.
    """
    cur = human_price(r0, r1, d0, d1)
    if within_tolerance(cur, target, tol):
        return {
            "needed": False,
            "offer_token": None,
            "offer_amount": "0",
            "expected_return": "0",
            "current_price": str(cur),
            "projected_price": str(cur),
            "rel_error": str(rel_error(cur, target)),
        }

    if cur > target:
        offer_is_0 = True
        r_in, r_out = r0, r1
        hi = max(1, int(_d(r0) * max_frac))
    else:
        offer_is_0 = False
        r_in, r_out = r1, r0
        hi = max(1, int(_d(r1) * max_frac))

    lo = 1
    best: tuple[int, int, Decimal, int, int] | None = None
    while lo <= hi:
        mid = (lo + hi) // 2
        n_in, n_out, ret = simulate_pool_swap(r_in, r_out, mid, fee_bps)
        if offer_is_0:
            n0, n1 = n_in, n_out
        else:
            n0, n1 = n_out, n_in
        if n0 <= 0 or n1 <= 0:
            hi = mid - 1
            continue
        p = human_price(n0, n1, d0, d1)
        best = (mid, ret, p, n0, n1)
        if within_tolerance(p, target, tol):
            break
        # Selling token0 lowers quote/base; selling token1 raises it.
        if offer_is_0:
            if p > target:
                lo = mid + 1
            else:
                hi = mid - 1
        else:
            if p < target:
                lo = mid + 1
            else:
                hi = mid - 1

    if best is None:
        raise RuntimeError("rebalance search failed")
    offer, ret, p, n0, n1 = best
    if not within_tolerance(p, target, tol):
        raise RuntimeError(
            f"cannot reach target {target} within {tol} (best {p}, rel {rel_error(p, target)})"
        )
    return {
        "needed": True,
        "offer_token": "ust1" if offer_is_0 else "custc",
        "offer_amount": str(offer),
        "expected_return": str(ret),
        "current_price": str(cur),
        "projected_price": str(p),
        "rel_error": str(rel_error(p, target)),
        "new_r0": str(n0),
        "new_r1": str(n1),
    }


def lp_raw_for_usd(
    r0: int,
    r1: int,
    d0: int,
    d1: int,
    usd: Decimal,
    px0: Decimal,
    px1: Decimal,
) -> tuple[int, int]:
    """Pro-rata provide sized so oracle USD of both legs ≈ usd. Empty pool uses px ratio."""
    if usd <= 0:
        raise ValueError("usd")
    if r0 > 0 and r1 > 0:
        price = human_price(r0, r1, d0, d1)
    else:
        if px0 <= 0 or px1 <= 0:
            raise ValueError("empty pool needs positive oracle prices")
        price = px0 / px1
    denom = px0 + price * px1
    if denom <= 0:
        raise ValueError("non-positive value denom")
    a0_h = usd / denom
    a1_h = a0_h * price
    a0 = int((a0_h * (Decimal(10) ** d0)).to_integral_value(rounding=ROUND_DOWN))
    if r0 > 0 and r1 > 0:
        a1 = a0 * r1 // r0
    else:
        a1 = int((a1_h * (Decimal(10) ** d1)).to_integral_value(rounding=ROUND_DOWN))
    if a0 <= 0 or a1 <= 0:
        raise RuntimeError("LP legs rounded to zero")
    return a0, a1


def apply_buffer(amount: int, buffer_bps: int) -> int:
    if amount <= 0:
        return 0
    return amount + ceil_div(amount * buffer_bps, 10000)


def mint_need(have: int, need: int) -> int:
    return max(0, need - have)


def build_plan(inp: dict[str, Any]) -> dict[str, Any]:
    ustc_usd = _d(inp["ustc_usd"])
    ustr_per = _d(inp.get("ustr_per_ustc", USTR_PER_USTC))
    usd_each = _d(inp.get("usd_each", 5000))
    tol = _d(inp.get("tolerance", "0.001"))
    fee_bps = int(inp.get("fee_bps", 180))
    buffer_bps = int(inp.get("buffer_bps", 50))
    d_ust1 = int(inp.get("dec_ust1", 6))
    d_custc = int(inp.get("dec_custc", 6))
    d_ustr = int(inp.get("dec_ustr", 18))

    r0 = _i(inp["custc_r0"])
    r1 = _i(inp["custc_r1"])
    u0 = _i(inp["ustr_r0"])
    u1 = _i(inp["ustr_r1"])

    bal_ust1 = _i(inp.get("bal_ust1", 0))
    bal_custc = _i(inp.get("bal_custc", 0))
    bal_ustr = _i(inp.get("bal_ustr", 0))

    target = target_custc_per_ust1(ustc_usd)
    target_ustr = target_ustr_per_ust1(ustc_usd, ustr_per)
    cur = human_price(r0, r1, d_ust1, d_custc) if r0 > 0 and r1 > 0 else None
    cur_ustr = human_price(u0, u1, d_ust1, d_ustr) if u0 > 0 and u1 > 0 else None

    swap = (
        find_rebalance_offer(r0, r1, d_ust1, d_custc, target, fee_bps, tol)
        if r0 > 0 and r1 > 0
        else {
            "needed": False,
            "offer_token": None,
            "offer_amount": "0",
            "expected_return": "0",
            "current_price": None,
            "projected_price": str(target),
            "rel_error": "0",
        }
    )

    post_r0, post_r1 = r0, r1
    if swap.get("needed") and swap.get("new_r0"):
        post_r0, post_r1 = _i(swap["new_r0"]), _i(swap["new_r1"])

    lp_c0, lp_c1 = lp_raw_for_usd(post_r0, post_r1, d_ust1, d_custc, usd_each, Decimal(1), ustc_usd)
    lp_u0, lp_u1 = lp_raw_for_usd(u0, u1, d_ust1, d_ustr, usd_each, Decimal(1), ustc_usd * ustr_per)

    need_ust1 = lp_c0 + lp_u0
    need_custc = lp_c1
    need_ustr = lp_u1
    if swap.get("needed"):
        if swap["offer_token"] == "ust1":
            need_ust1 += _i(swap["offer_amount"])
            need_custc = max(0, need_custc - _i(swap["expected_return"]))
        elif swap["offer_token"] == "custc":
            need_custc += _i(swap["offer_amount"])
            need_ust1 = max(0, need_ust1 - _i(swap["expected_return"]))

    mint_ust1 = apply_buffer(mint_need(bal_ust1, need_ust1), buffer_bps)
    mint_custc = apply_buffer(mint_need(bal_custc, need_custc), buffer_bps)
    mint_ustr = apply_buffer(mint_need(bal_ustr, need_ustr), buffer_bps)

    return {
        "ustc_usd": str(ustc_usd),
        "target_custc_per_ust1": str(target),
        "target_ustr_per_ust1": str(target_ustr),
        "current_custc_per_ust1": None if cur is None else str(cur),
        "current_ustr_per_ust1": None if cur_ustr is None else str(cur_ustr),
        "current_custc_rel_error": None if cur is None else str(rel_error(cur, target)),
        "current_ustr_rel_error": None if cur_ustr is None else str(rel_error(cur_ustr, target_ustr)),
        "already_on_peg": bool(cur is not None and within_tolerance(cur, target, tol)),
        "swap": swap,
        "lp_custc": {"ust1": str(lp_c0), "custc": str(lp_c1)},
        "lp_ustr": {"ust1": str(lp_u0), "ustr": str(lp_u1)},
        "mint": {"ust1": str(mint_ust1), "custc": str(mint_custc), "ustr": str(mint_ustr)},
        "need_before_buffer": {
            "ust1": str(need_ust1),
            "custc": str(need_custc),
            "ustr": str(need_ustr),
        },
    }


def _self_test() -> None:
    # Spot: same-decimal 1:1, no fee, selling token0 should lower price.
    n0, n1, ret = simulate_pool_swap(1_000_000, 1_000_000, 100_000, 0)
    assert n0 == 1_100_000
    assert n1 * n0 >= 1_000_000 * 1_000_000 - 1_100_000  # ceil-div residue
    assert ret == 1_000_000 - n1

    # On-peg: no swap.
    peg = find_rebalance_offer(1_000_000, 200_000_000, 6, 6, Decimal("200"), 180, Decimal("0.001"))
    assert peg["needed"] is False

    # Current mainnet-ish UST1/cUSTC (2026-08-16 snapshot) should find a UST1 sell.
    ustc = Decimal("0.004883")
    target = target_custc_per_ust1(ustc)
    r0, r1 = 966_891_827, 208_979_024_454
    cur = human_price(r0, r1, 6, 6)
    assert cur > target
    plan = find_rebalance_offer(r0, r1, 6, 6, target, 180, Decimal("0.001"))
    assert plan["needed"] is True
    assert plan["offer_token"] == "ust1"
    assert within_tolerance(_d(plan["projected_price"]), target, Decimal("0.001"))
    offer = int(plan["offer_amount"])
    assert 1_000_000 < offer < 80_000_000  # ~26 UST1, not a drain

    # Stale offer after an intervening same-direction swap overshoots the band.
    # (mint txs between plan and execute can race the pool this way.)
    r0_stale, r1_stale = 2_138_282_553, 380_821_016_835
    target_stale = target_custc_per_ust1(Decimal("0.004868"))
    stale = find_rebalance_offer(
        r0_stale, r1_stale, 6, 6, target_stale, 180, Decimal("0.001")
    )
    assert stale["needed"] is True
    assert stale["offer_token"] == "custc"
    n_in, n_out, _ = simulate_pool_swap(r1_stale, r0_stale, 2_352_000_000, 180)
    n_in2, n_out2, _ = simulate_pool_swap(n_in, n_out, int(stale["offer_amount"]), 180)
    p_overshoot = human_price(n_out2, n_in2, 6, 6)
    assert not within_tolerance(p_overshoot, target_stale, Decimal("0.001"))

    # $1000 LP at peg, 6/6, $1 / $0.005 → 500 + 100_000
    a0, a1 = lp_raw_for_usd(1_000_000, 200_000_000, 6, 6, Decimal(1000), Decimal(1), Decimal("0.005"))
    assert a0 == 500_000_000
    assert a1 == 100_000_000_000

    # Mixed 6/18 empty pool uses oracle ratio (USTR = 2.5 * USTC).
    u0, u1 = lp_raw_for_usd(0, 0, 6, 18, Decimal(1000), Decimal(1), Decimal("0.0125"))
    assert u0 == 500_000_000
    assert u1 == 40_000 * 10**18

    # Full plan: mint covers swap + both LP minus balances.
    out = build_plan(
        {
            "ustc_usd": "0.004883",
            "custc_r0": r0,
            "custc_r1": r1,
            "ustr_r0": 1_062_857_683,
            "ustr_r1": "83340713283948346193812",
            "bal_ust1": 2_026_179,
            "bal_custc": 0,
            "bal_ustr": "14826723208204635665175",
            "usd_each": 1000,
            "tolerance": "0.001",
            "fee_bps": 180,
            "buffer_bps": 50,
        }
    )
    assert out["swap"]["needed"] is True
    assert int(out["mint"]["ust1"]) > 0
    assert int(out["mint"]["custc"]) > 0
    assert int(out["lp_custc"]["ust1"]) > 0
    assert int(out["lp_ustr"]["ustr"]) > 0
    print("self-test ok", file=sys.stderr)


def main() -> int:
    if "--self-test" in sys.argv:
        _self_test()
        return 0
    raw = sys.stdin.read()
    if not raw.strip():
        print("usage: ust1-lp-rebalance-math.py [--self-test]  # or JSON on stdin", file=sys.stderr)
        return 2
    print(json.dumps(build_plan(json.loads(raw)), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
