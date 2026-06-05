#!/usr/bin/env python3
"""
One-shot liquidity top-up before launch-swarm workers (GitLab #293).

Brings thin factory pairs back toward deploy-depth so swap QA (OE-1) sees
near-inverse quotes on direct routes instead of lopsided multi-hop arbs on
drained pools. Idempotent — skips pairs already above the floor.
"""

from __future__ import annotations

import asyncio
import os
import sys

# Allow `from swarm import …` when run as scripts/bots/bootstrap-swarm-liquidity.py
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPT_DIR not in sys.path:
    sys.path.insert(0, _SCRIPT_DIR)

from swarm import (  # noqa: E402
    PairMeta,
    _collect_pair_metas,
    _docker_localterra_id,
    _env,
    _factory_addr,
    _lcd_base,
    provide_liquidity_pair,
)
from swarm_liquidity import (  # noqa: E402
    MIN_PROVIDE_LIQUIDITY_LEG,
    bootstrap_top_up_amounts,
    pick_scaled_provide_amounts,
)


def _int_env(name: str, default: int) -> int:
    raw = _env(name)
    if not raw:
        return default
    try:
        return int(raw)
    except ValueError:
        return default


# Hub pairs used in OE-1 swap checklist — always get an extra proportional deepen.
HUB_SYMBOL_PAIRS = (
    ("EMBER", "CORAL"),
    ("TOPAZ", "ONYX"),
    ("ONYX", "CORAL"),
)


def _is_hub(meta: PairMeta) -> bool:
    syms = {meta.sym0, meta.sym1}
    for a, b in HUB_SYMBOL_PAIRS:
        if a in syms and b in syms:
            return True
    return False


async def _bootstrap_pair(
    container: str,
    meta: PairMeta,
    *,
    floor_per_side: int,
    target_per_side: int,
    hub_fraction_ppm: int,
    dry: bool,
) -> str | None:
    top = bootstrap_top_up_amounts(
        meta.reserve0,
        meta.reserve1,
        floor_per_side=floor_per_side,
        target_per_side=target_per_side,
    )
    if top:
        a0, a1 = top
        await provide_liquidity_pair(container, meta.token0, meta.token1, meta.pair_addr, a0, a1, dry)
        return f"top-up {meta.sym0}/{meta.sym1} +{a0}/+{a1}"

    if _is_hub(meta):
        scaled = pick_scaled_provide_amounts(meta.reserve0, meta.reserve1, hub_fraction_ppm)
        if scaled:
            a0, a1 = scaled
            if a0 >= MIN_PROVIDE_LIQUIDITY_LEG and a1 >= MIN_PROVIDE_LIQUIDITY_LEG:
                await provide_liquidity_pair(
                    container, meta.token0, meta.token1, meta.pair_addr, a0, a1, dry
                )
                return f"hub deepen {meta.sym0}/{meta.sym1} +{a0}/+{a1}"
    return None


async def main_async() -> int:
    lcd = _lcd_base()
    factory = _factory_addr()
    container = _docker_localterra_id()
    dry = (_env("BOTS_DRY_RUN", "0") or "0") == "1"
    floor = _int_env("BOTS_BOOTSTRAP_FLOOR_PER_SIDE", 5_000_000_000)
    target = _int_env("BOTS_BOOTSTRAP_TARGET_PER_SIDE", 50_000_000_000)
    hub_ppm = _int_env("BOTS_BOOTSTRAP_HUB_FRACTION_PPM", 5_000)  # 0.5%

    metas = _collect_pair_metas(lcd, factory)
    if not metas:
        print("bootstrap-swarm-liquidity: no pairs found", file=sys.stderr)
        return 1

    print(
        f"bootstrap-swarm-liquidity: {len(metas)} pairs floor={floor} target={target} "
        f"hub_ppm={hub_ppm} dry_run={dry}",
        flush=True,
    )

    actions = 0
    for meta in metas:
        note = await _bootstrap_pair(
            container,
            meta,
            floor_per_side=floor,
            target_per_side=target,
            hub_fraction_ppm=hub_ppm,
            dry=dry,
        )
        if note:
            actions += 1
            print(f"  {note}", flush=True)

    print(f"bootstrap-swarm-liquidity: done ({actions} provide_liquidity action(s))", flush=True)
    return 0


def main() -> None:
    raise SystemExit(asyncio.run(main_async()))


if __name__ == "__main__":
    main()
