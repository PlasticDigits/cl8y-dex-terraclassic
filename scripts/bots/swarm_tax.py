"""Community-tax helpers for the Python LocalTerra swarm (GitLab #621).

No docker / LCD I/O — unit-testable without a chain.
"""

from __future__ import annotations

import os
from typing import Any, Iterable

BPS_DENOM = 10_000
DEFAULT_SELL_BPS = 500


def tax_workers_enabled(env: dict[str, str] | None = None) -> bool:
    src = env if env is not None else os.environ
    raw = (src.get("SWARM_TAX_WORKERS") or "1").strip().lower()
    return raw not in {"0", "false", "off", "no"}


def tax_token_from_mapping(env: dict[str, str]) -> str | None:
    pinned = (env.get("VITE_TOKEN_COMMUNITY_TAX_ADDRESS") or "").strip()
    return pinned if pinned.startswith("terra1") else None


def parse_vite_env_file(path: str) -> dict[str, str]:
    out: dict[str, str] = {}
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith("#") or "=" not in line:
                    continue
                key, _, val = line.partition("=")
                key = key.strip()
                if not key.startswith("VITE_"):
                    continue
                val = val.strip().strip('"').strip("'")
                if val:
                    out[key] = val
    except OSError:
        return {}
    return out


def collect_tax_addrs(
    env: dict[str, str] | None = None,
    vite_pins: dict[str, str] | None = None,
) -> set[str]:
    src: dict[str, str] = {}
    if vite_pins:
        src.update(vite_pins)
    if env is not None:
        src.update({k: v for k, v in env.items() if v})
    else:
        src.update({k: v for k, v in os.environ.items() if v})
    tok = tax_token_from_mapping(src)
    return {tok} if tok else set()


def pair_touches_tax(token0: str, token1: str, tax_addrs: set[str]) -> bool:
    return token0 in tax_addrs or token1 in tax_addrs


def filter_gem_pairs(metas: Iterable[Any], tax_addrs: set[str]) -> list[Any]:
    if not tax_addrs:
        return list(metas)
    return [m for m in metas if not pair_touches_tax(m.token0, m.token1, tax_addrs)]


def filter_tax_pairs(metas: Iterable[Any], tax_addrs: set[str]) -> list[Any]:
    if not tax_addrs:
        return []
    return [m for m in metas if pair_touches_tax(m.token0, m.token1, tax_addrs)]


def fail_closed_sell_debit(amount: int, sell_bps: int = DEFAULT_SELL_BPS) -> int:
    if amount <= 0:
        return 0
    bps = max(0, int(sell_bps))
    return amount + (amount * bps) // BPS_DENOM


def _as_int(raw: Any) -> int:
    try:
        return int(raw)
    except (TypeError, ValueError):
        return 0


def required_wallet_debit(
    preview: dict[str, Any] | None,
    amount: int,
    path: str,
    sell_bps: int = DEFAULT_SELL_BPS,
) -> int:
    if not preview:
        return fail_closed_sell_debit(amount, sell_bps)
    debit = _as_int(preview.get("debit"))
    hop = _as_int(preview.get("hop_trader_debit"))
    if path == "router":
        extra = hop if hop > 0 else fail_closed_sell_debit(amount, sell_bps) - amount
        return debit + extra
    return debit if debit > 0 else fail_closed_sell_debit(amount, sell_bps)


def balance_covers_debit(balance: int, required: int) -> bool:
    return balance >= required > 0


def pair_direct_swap_hook(max_spread: str = "0.50") -> dict[str, Any]:
    return {
        "swap": {
            "belief_price": None,
            "max_spread": max_spread,
            "to": None,
            "deadline": None,
            "trader": None,
        }
    }


def pair_direct_sets_trader(hook: dict[str, Any]) -> bool:
    trader = (hook.get("swap") or {}).get("trader")
    return trader not in (None, "", False)


def router_execute_swap_operations(operations: list[dict[str, Any]], max_spread: str = "0.50") -> dict[str, Any]:
    return {
        "execute_swap_operations": {
            "operations": operations,
            "max_spread": max_spread,
            "minimum_receive": None,
            "to": None,
            "deadline": None,
        }
    }


def router_hop_swap_preview_hook(trader: str, max_spread: str = "0.50") -> dict[str, Any]:
    return {
        "swap": {
            "belief_price": None,
            "max_spread": max_spread,
            "to": None,
            "deadline": None,
            "trader": trader,
            "hybrid": None,
        }
    }


def terra_swap_op(offer: str, ask: str) -> dict[str, Any]:
    return {
        "terra_swap": {
            "offer_asset_info": {"token": {"contract_addr": offer}},
            "ask_asset_info": {"token": {"contract_addr": ask}},
        }
    }


def find_tax_multihop(
    metas: Iterable[Any],
    tax_addrs: set[str],
    *,
    prefer_sell: bool = True,
) -> tuple[str, str, list[dict[str, Any]]] | None:
    """Return (offer, dest, ≥2hop ops) TAX→hub→other or the reverse buy path."""
    tax_pair = None
    others: list[Any] = []
    for m in metas:
        if pair_touches_tax(m.token0, m.token1, tax_addrs):
            tax_pair = m
        else:
            others.append(m)
    if tax_pair is None or not others:
        return None
    tax = tax_pair.token0 if tax_pair.token0 in tax_addrs else tax_pair.token1
    hub = tax_pair.token1 if tax_pair.token0 in tax_addrs else tax_pair.token0
    for m in others:
        if m.token0 == hub:
            dest = m.token1
        elif m.token1 == hub:
            dest = m.token0
        else:
            continue
        if dest == tax:
            continue
        if prefer_sell:
            ops = [terra_swap_op(tax, hub), terra_swap_op(hub, dest)]
            return tax, dest, ops
        ops = [terra_swap_op(dest, hub), terra_swap_op(hub, tax)]
        return dest, tax, ops
    return None
