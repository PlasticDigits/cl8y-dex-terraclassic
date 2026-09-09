#!/usr/bin/env python3
"""Plan cLUNC/cUSTC oracle rebalance + stepwise USD TVL LP.

Used by scripts/rebalance-mint-clunc-custc-lp.sh.
Price is human cUSTC per cLUNC = LUNC_USD / USTC_USD (both 6dp wraps).
"""
from __future__ import annotations

import json
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from decimal import Decimal, ROUND_DOWN, getcontext
from typing import Any

getcontext().prec = 80

UA = "cl8y-dex-ops/clunc-custc-lp (+https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic)"
DEFAULT_RUNGS = (Decimal("200"), Decimal("500"), Decimal("2000"), Decimal("5000"), Decimal("10000"))


def _d(x: Any) -> Decimal:
    if isinstance(x, Decimal):
        return x
    return Decimal(str(x))


def _i(x: Any) -> int:
    return int(_d(x))


def human_from_raw(raw: int, decimals: int) -> Decimal:
    return _d(raw) / (Decimal(10) ** int(decimals))


def human_price(r0: int, r1: int, d0: int, d1: int) -> Decimal:
    if r0 <= 0 or r1 <= 0:
        raise ValueError("empty pool")
    return human_from_raw(r1, d1) / human_from_raw(r0, d0)


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


def target_custc_per_clunc(lunc_usd: Decimal, ustc_usd: Decimal) -> Decimal:
    if lunc_usd <= 0 or ustc_usd <= 0:
        raise ValueError("oracle prices must be positive")
    return lunc_usd / ustc_usd


def tvl_usd(r0: int, r1: int, d0: int, d1: int, lunc_usd: Decimal, ustc_usd: Decimal) -> Decimal:
    return human_from_raw(r0, d0) * lunc_usd + human_from_raw(r1, d1) * ustc_usd


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
    """Binary-search a pool-only offer that lands cUSTC-per-cLUNC within tol of target.

    token0 is cLUNC (base), token1 is cUSTC (quote).
    """
    idle = {
        "needed": False,
        "offer_token": None,
        "offer_amount": "0",
        "expected_return": "0",
        "current_price": None,
        "projected_price": str(target),
        "rel_error": "0",
    }
    if r0 <= 0 or r1 <= 0:
        return idle
    cur = human_price(r0, r1, d0, d1)
    idle["current_price"] = str(cur)
    idle["rel_error"] = str(rel_error(cur, target))
    idle["projected_price"] = str(cur)
    if within_tolerance(cur, target, tol):
        return idle

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
        "offer_token": "clunc" if offer_is_0 else "custc",
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
    """Pro-rata provide sized so oracle USD of both legs ≈ usd."""
    if usd <= 0:
        return 0, 0
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


def wrap_native_for_cw20(cw20_out: int, fee_wrap_bps: int) -> int:
    """Gross native needed so wrap fee leaves at least cw20_out minted."""
    if cw20_out <= 0:
        return 0
    keep_bps = 10000 - int(fee_wrap_bps)
    if keep_bps <= 0 or keep_bps > 10000:
        raise ValueError("fee_wrap_bps")
    return ceil_div(cw20_out * 10000, keep_bps)


def parse_rungs(raw: Any) -> list[Decimal]:
    if raw is None or raw == "":
        return list(DEFAULT_RUNGS)
    if isinstance(raw, (list, tuple)):
        vals = [_d(x) for x in raw]
    else:
        vals = [_d(p.strip()) for p in str(raw).split(",") if p.strip()]
    if not vals:
        raise ValueError("empty rungs")
    prev = Decimal(0)
    out: list[Decimal] = []
    for v in vals:
        if v <= prev:
            raise ValueError(f"rungs must be strictly increasing (got {v} after {prev})")
        out.append(v)
        prev = v
    return out


def apply_swap_to_state(
    r0: int,
    r1: int,
    bal0: int,
    bal1: int,
    swap: dict[str, Any],
) -> tuple[int, int, int, int]:
    if not swap.get("needed"):
        return r0, r1, bal0, bal1
    offer = _i(swap["offer_amount"])
    ret = _i(swap.get("expected_return") or 0)
    if swap["offer_token"] == "clunc":
        r0, r1 = _i(swap["new_r0"]), _i(swap["new_r1"])
        bal0 -= offer
        bal1 += ret
    elif swap["offer_token"] == "custc":
        r0, r1 = _i(swap["new_r0"]), _i(swap["new_r1"])
        bal1 -= offer
        bal0 += ret
    else:
        raise ValueError(f"unknown offer_token {swap.get('offer_token')}")
    if bal0 < 0 or bal1 < 0:
        # Caller mints before swap; clamp for planning only.
        bal0 = max(0, bal0)
        bal1 = max(0, bal1)
    return r0, r1, bal0, bal1


def plan_step(inp: dict[str, Any]) -> dict[str, Any]:
    lunc_usd = _d(inp["lunc_usd"])
    ustc_usd = _d(inp["ustc_usd"])
    target_usd = _d(inp["target_usd"])
    tol = _d(inp.get("tolerance", "0.001"))
    fee_bps = int(inp.get("fee_bps", 180))
    buffer_bps = int(inp.get("buffer_bps", 50))
    fee_wrap_bps = int(inp.get("fee_wrap_bps", 200))
    skip_swap = bool(inp.get("skip_swap", False))
    d0 = int(inp.get("dec_clunc", 6))
    d1 = int(inp.get("dec_custc", 6))
    r0 = _i(inp["r0"])
    r1 = _i(inp["r1"])
    bal0 = _i(inp.get("bal_clunc", 0))
    bal1 = _i(inp.get("bal_custc", 0))

    target = target_custc_per_clunc(lunc_usd, ustc_usd)
    cur = human_price(r0, r1, d0, d1) if r0 > 0 and r1 > 0 else None
    swap = {
        "needed": False,
        "offer_token": None,
        "offer_amount": "0",
        "expected_return": "0",
        "current_price": None if cur is None else str(cur),
        "projected_price": str(target),
        "rel_error": "0" if cur is None else str(rel_error(cur, target)),
    }
    if not skip_swap and r0 > 0 and r1 > 0:
        swap = find_rebalance_offer(r0, r1, d0, d1, target, fee_bps, tol)

    post_r0, post_r1 = r0, r1
    if swap.get("needed") and swap.get("new_r0"):
        post_r0, post_r1 = _i(swap["new_r0"]), _i(swap["new_r1"])

    post_tvl = (
        tvl_usd(post_r0, post_r1, d0, d1, lunc_usd, ustc_usd)
        if post_r0 > 0 and post_r1 > 0
        else Decimal(0)
    )
    add_usd = target_usd - post_tvl
    if add_usd < 0:
        add_usd = Decimal(0)

    lp0, lp1 = (
        lp_raw_for_usd(post_r0, post_r1, d0, d1, add_usd, lunc_usd, ustc_usd)
        if add_usd > 0
        else (0, 0)
    )

    need0 = lp0
    need1 = lp1
    if swap.get("needed"):
        if swap["offer_token"] == "clunc":
            need0 += _i(swap["offer_amount"])
            need1 = max(0, need1 - _i(swap["expected_return"]))
        elif swap["offer_token"] == "custc":
            need1 += _i(swap["offer_amount"])
            need0 = max(0, need0 - _i(swap["expected_return"]))

    mint0 = apply_buffer(mint_need(bal0, need0), buffer_bps)
    mint1 = apply_buffer(mint_need(bal1, need1), buffer_bps)
    wrap0 = wrap_native_for_cw20(mint0, fee_wrap_bps)
    wrap1 = wrap_native_for_cw20(mint1, fee_wrap_bps)

    side = target_usd / Decimal(2)
    return {
        "lunc_usd": str(lunc_usd),
        "ustc_usd": str(ustc_usd),
        "target_custc_per_clunc": str(target),
        "current_custc_per_clunc": None if cur is None else str(cur),
        "current_rel_error": None if cur is None else str(rel_error(cur, target)),
        "already_on_peg": bool(cur is not None and within_tolerance(cur, target, tol)),
        "target_usd": str(target_usd),
        "each_side_usd": str(side),
        "post_swap_tvl_usd": str(post_tvl),
        "add_usd": str(add_usd),
        "swap": swap,
        "lp": {
            "clunc": str(lp0),
            "custc": str(lp1),
            "clunc_human": str(human_from_raw(lp0, d0)),
            "custc_human": str(human_from_raw(lp1, d1)),
        },
        "mint": {"clunc": str(mint0), "custc": str(mint1)},
        "wrap_native": {"uluna": str(wrap0), "uusd": str(wrap1)},
        "need_before_buffer": {"clunc": str(need0), "custc": str(need1)},
        "post_r0": str(post_r0),
        "post_r1": str(post_r1),
    }


def plan_rungs(inp: dict[str, Any]) -> dict[str, Any]:
    rungs = parse_rungs(inp.get("rungs"))
    r0 = _i(inp["r0"])
    r1 = _i(inp["r1"])
    bal0 = _i(inp.get("bal_clunc", 0))
    bal1 = _i(inp.get("bal_custc", 0))
    steps: list[dict[str, Any]] = []
    total_mint0 = 0
    total_mint1 = 0
    total_uluna = 0
    total_uusd = 0
    for target in rungs:
        step_in = dict(inp)
        step_in["target_usd"] = str(target)
        step_in["r0"] = str(r0)
        step_in["r1"] = str(r1)
        step_in["bal_clunc"] = str(bal0)
        step_in["bal_custc"] = str(bal1)
        step = plan_step(step_in)
        steps.append(step)
        mint0 = _i(step["mint"]["clunc"])
        mint1 = _i(step["mint"]["custc"])
        total_mint0 += mint0
        total_mint1 += mint1
        total_uluna += _i(step["wrap_native"]["uluna"])
        total_uusd += _i(step["wrap_native"]["uusd"])
        bal0 += mint0
        bal1 += mint1
        r0, r1, bal0, bal1 = apply_swap_to_state(r0, r1, bal0, bal1, step["swap"])
        r0 += _i(step["lp"]["clunc"])
        r1 += _i(step["lp"]["custc"])
        bal0 -= _i(step["lp"]["clunc"])
        bal1 -= _i(step["lp"]["custc"])
        bal0 = max(0, bal0)
        bal1 = max(0, bal1)
    d0 = int(inp.get("dec_clunc", 6))
    d1 = int(inp.get("dec_custc", 6))
    lunc_usd = _d(inp["lunc_usd"])
    ustc_usd = _d(inp["ustc_usd"])
    final_tvl = tvl_usd(r0, r1, d0, d1, lunc_usd, ustc_usd) if r0 > 0 and r1 > 0 else Decimal(0)
    return {
        "rungs": [str(x) for x in rungs],
        "steps": steps,
        "totals": {
            "mint_clunc": str(total_mint0),
            "mint_custc": str(total_mint1),
            "uluna": str(total_uluna),
            "uusd": str(total_uusd),
            "clunc_human": str(human_from_raw(total_mint0, d0)),
            "custc_human": str(human_from_raw(total_mint1, d1)),
        },
        "final": {
            "r0": str(r0),
            "r1": str(r1),
            "tvl_usd": str(final_tvl),
            "clunc_usd": str(human_from_raw(r0, d0) * lunc_usd),
            "custc_usd": str(human_from_raw(r1, d1) * ustc_usd),
        },
    }


def _http_json(url: str, timeout: int = 20) -> Any:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as exc:
        body = b""
        try:
            body = exc.read()[:240]
        except Exception:
            pass
        raise RuntimeError(f"HTTP {exc.code} {url}: {body!r}") from exc


def _oracle_fail(errors: list[str], warnings: list[str] | None = None) -> dict[str, Any]:
    return {
        "ok": False,
        "lunc_usd": None,
        "ustc_usd": None,
        "target_custc_per_clunc": None,
        "indexer": {},
        "live": {"lunc": {}, "ustc": {}},
        "live_median": {"lunc": None, "ustc": None},
        "hub": {},
        "errors": errors,
        "warnings": warnings or [],
    }


def _parse_iso(ts: str) -> datetime | None:
    try:
        return datetime.fromisoformat(ts.replace("Z", "+00:00"))
    except (TypeError, ValueError):
        return None


def fetch_oracles(inp: dict[str, Any]) -> dict[str, Any]:
    try:
        return _fetch_oracles_inner(inp)
    except Exception as exc:
        return _oracle_fail([f"{type(exc).__name__}: {exc}"])


def _fetch_oracles_inner(inp: dict[str, Any]) -> dict[str, Any]:
    indexer = str(inp.get("indexer") or "https://indexer.dex.cl8y.com").rstrip("/")
    max_age = int(inp.get("max_age_sec", 1800))
    cross_tol = _d(inp.get("cross_tolerance", "0.03"))
    skip_cross = bool(inp.get("skip_cross_check", False))
    errors: list[str] = []
    warnings: list[str] = []

    ustc_idx = _http_json(f"{indexer}/api/v1/oracle/price/ustc")
    lunc_idx = _http_json(f"{indexer}/api/v1/oracle/price/lunc")
    hub = _http_json(f"{indexer}/api/v1/hub-prices")

    def idx_price(payload: dict[str, Any], ticker: str) -> tuple[Decimal, dict[str, Any]]:
        if payload.get("ticker") != ticker:
            raise ValueError(f"indexer ticker {payload.get('ticker')} != {ticker}")
        px = payload.get("price_usd")
        if px in (None, ""):
            raise ValueError(f"indexer {ticker} price_usd empty")
        avg_at = None
        for src in payload.get("sources") or []:
            if src.get("source") == "average":
                avg_at = src.get("fetched_at")
                break
        if avg_at:
            dt = _parse_iso(str(avg_at))
            if dt is not None:
                age = (datetime.now(timezone.utc) - dt.astimezone(timezone.utc)).total_seconds()
                if age > max_age:
                    raise ValueError(f"indexer {ticker} average is {int(age)}s old (max {max_age})")
        return _d(px), payload

    ustc_usd, ustc_raw = idx_price(ustc_idx, "ustc")
    lunc_usd, lunc_raw = idx_price(lunc_idx, "lunc")
    if lunc_usd >= ustc_usd:
        errors.append(f"LUNC/USD {lunc_usd} is not below USTC/USD {ustc_usd} — likely ticker mixup")

    hub_map = {p.get("ticker"): p for p in (hub.get("prices") or []) if isinstance(p, dict)}
    for ticker, idx_px, addr_key in (
        ("custc", ustc_usd, inp.get("custc_addr")),
        ("lunc", lunc_usd, inp.get("clunc_addr")),
    ):
        row = hub_map.get(ticker) or {}
        hub_px = row.get("price_usd")
        if hub_px in (None, ""):
            warnings.append(f"hub-prices {ticker} missing")
            continue
        # Hub and /oracle/price refresh on different loops — 0.1% is too tight
        # mid-run. Warn inside 3%; only fail when they actually disagree.
        if not within_tolerance(_d(hub_px), idx_px, Decimal("0.001")):
            msg = f"hub-prices {ticker} {hub_px} vs indexer {idx_px}"
            if not within_tolerance(_d(hub_px), idx_px, cross_tol):
                errors.append(msg)
            else:
                warnings.append(msg)
        want = (addr_key or "").strip()
        got = (row.get("asset_address") or "").strip()
        if want and got and got != want:
            errors.append(f"hub-prices {ticker} asset_address {got} != {want}")

    live: dict[str, dict[str, str]] = {"lunc": {}, "ustc": {}}

    def add_live(bucket: str, source: str, px: Any) -> None:
        try:
            val = _d(px)
        except Exception:
            return
        if val <= 0:
            return
        live[bucket][source] = str(val)

    fetches = [
        (
            "lunc",
            "coingecko",
            "https://api.coingecko.com/api/v3/simple/price?ids=terra-luna&vs_currencies=usd",
            lambda j: j["terra-luna"]["usd"],
        ),
        (
            "ustc",
            "coingecko",
            "https://api.coingecko.com/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            lambda j: j["terrausd"]["usd"],
        ),
        (
            "lunc",
            "binance",
            "https://api.binance.com/api/v3/ticker/price?symbol=LUNCUSDT",
            lambda j: j["price"],
        ),
        (
            "ustc",
            "binance",
            "https://api.binance.com/api/v3/ticker/price?symbol=USTCUSDT",
            lambda j: j["price"],
        ),
        (
            "lunc",
            "kucoin",
            "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=LUNC-USDT",
            lambda j: j["data"]["price"],
        ),
        (
            "ustc",
            "kucoin",
            "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol=USTC-USDT",
            lambda j: j["data"]["price"],
        ),
        (
            "lunc",
            "mexc",
            "https://api.mexc.com/api/v3/ticker/price?symbol=LUNCUSDT",
            lambda j: j["price"],
        ),
        (
            "ustc",
            "mexc",
            "https://api.mexc.com/api/v3/ticker/price?symbol=USTCUSDT",
            lambda j: j["price"],
        ),
    ]
    for bucket, source, url, pick in fetches:
        try:
            add_live(bucket, source, pick(_http_json(url, timeout=15)))
        except Exception as exc:
            warnings.append(f"{source}/{bucket}: {type(exc).__name__}: {exc}")

    def median(vals: list[Decimal]) -> Decimal:
        vals = sorted(vals)
        n = len(vals)
        if n == 0:
            raise ValueError("no live prices")
        if n % 2:
            return vals[n // 2]
        return (vals[n // 2 - 1] + vals[n // 2]) / 2

    live_med: dict[str, str | None] = {"lunc": None, "ustc": None}
    for bucket, idx_px in (("lunc", lunc_usd), ("ustc", ustc_usd)):
        vals = [_d(v) for v in live[bucket].values()]
        if len(vals) < 2:
            msg = f"need ≥2 live {bucket} sources, got {len(vals)}"
            if skip_cross:
                warnings.append(msg)
            else:
                errors.append(msg)
            continue
        med = median(vals)
        live_med[bucket] = str(med)
        if not within_tolerance(idx_px, med, cross_tol):
            msg = f"indexer {bucket} {idx_px} vs live median {med} (tol {cross_tol})"
            if skip_cross:
                warnings.append(msg)
            else:
                errors.append(msg)

    ok = not errors
    return {
        "ok": ok,
        "lunc_usd": str(lunc_usd),
        "ustc_usd": str(ustc_usd),
        "target_custc_per_clunc": str(target_custc_per_clunc(lunc_usd, ustc_usd)),
        "indexer": {"lunc": lunc_raw, "ustc": ustc_raw},
        "live": live,
        "live_median": live_med,
        "hub": hub_map,
        "errors": errors,
        "warnings": warnings,
    }


def _self_test() -> None:
    n0, n1, ret = simulate_pool_swap(1_000_000, 1_000_000, 100_000, 0)
    assert n0 == 1_100_000
    assert ret == 1_000_000 - n1

    peg = find_rebalance_offer(1_000_000, 9_794, 6, 6, Decimal("0.009794"), 180, Decimal("0.001"))
    assert peg["needed"] is False

    # 2026-09-04 columbus-5 cLUNC/cUSTC snapshot: slightly cheap cLUNC → sell cUSTC.
    lunc = Decimal("0.000052086666666667")
    ustc = Decimal("0.005318")
    target = target_custc_per_clunc(lunc, ustc)
    r0, r1 = 4_823_672_118, 47_040_118
    cur = human_price(r0, r1, 6, 6)
    assert cur < target
    plan = find_rebalance_offer(r0, r1, 6, 6, target, 180, Decimal("0.001"))
    assert plan["needed"] is True
    assert plan["offer_token"] == "custc"
    assert within_tolerance(_d(plan["projected_price"]), target, Decimal("0.001"))

    tvl = tvl_usd(r0, r1, 6, 6, lunc, ustc)
    assert Decimal("0.4") < tvl < Decimal("0.7")

    a0, a1 = lp_raw_for_usd(r0, r1, 6, 6, Decimal(200) - tvl, lunc, ustc)
    assert a0 > 0 and a1 > 0
    added = tvl_usd(a0, a1, 6, 6, lunc, ustc)
    assert abs(added - (Decimal(200) - tvl)) / Decimal(200) < Decimal("0.02")

    assert wrap_native_for_cw20(9_800_000, 200) == 10_000_000
    assert wrap_native_for_cw20(1, 200) >= 2

    rungs = parse_rungs("200,500,2000,5000,10000")
    assert rungs[-1] == Decimal(10000)
    try:
        parse_rungs("200,200")
        raise AssertionError("duplicate rungs")
    except ValueError:
        pass

    full = plan_rungs(
        {
            "lunc_usd": str(lunc),
            "ustc_usd": str(ustc),
            "r0": r0,
            "r1": r1,
            "bal_clunc": 0,
            "bal_custc": 0,
            "fee_bps": 180,
            "buffer_bps": 50,
            "tolerance": "0.001",
            "rungs": "200,500,2000,5000,10000",
        }
    )
    assert len(full["steps"]) == 5
    assert _i(full["totals"]["mint_clunc"]) > 0
    assert _i(full["totals"]["mint_custc"]) > 0
    final = _d(full["final"]["tvl_usd"])
    assert Decimal("9800") <= final <= Decimal("10200")
    # Last rung is $10k, $5k each side.
    assert abs(_d(full["final"]["clunc_usd"]) - Decimal(5000)) < Decimal(150)
    assert abs(_d(full["final"]["custc_usd"]) - Decimal(5000)) < Decimal(150)

    crashed = fetch_oracles({"indexer": "http://127.0.0.1:1", "max_age_sec": 1})
    assert crashed["ok"] is False
    assert crashed["errors"]
    print("self-test ok", file=sys.stderr)


def main() -> int:
    if "--self-test" in sys.argv:
        _self_test()
        return 0
    raw = sys.stdin.read()
    if not raw.strip():
        print(
            "usage: clunc-custc-lp-math.py [--self-test]  # JSON on stdin; "
            "set mode=fetch-oracles | plan-step | plan-rungs",
            file=sys.stderr,
        )
        return 2
    inp = json.loads(raw)
    mode = inp.get("mode") or ("fetch-oracles" if "--fetch-oracles" in sys.argv else "plan-rungs")
    if mode == "fetch-oracles":
        out = fetch_oracles(inp)
        print(json.dumps(out, indent=2))
        return 0 if out.get("ok") else 1
    if mode == "plan-step":
        print(json.dumps(plan_step(inp), indent=2))
        return 0
    print(json.dumps(plan_rungs(inp), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
