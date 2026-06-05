"""Shared liquidity sizing helpers for scripts/bots/swarm.py (GitLab #293)."""

from __future__ import annotations

# Mirrors packages/localnet-trading-swarm/src/liquidityGuards.ts
MIN_RESERVE_PER_SIDE_FOR_SWAP = 10_000_000
MIN_PROVIDE_LIQUIDITY_LEG = 5_000_000
LP_FRACTION_PPM = 3_000  # 0.3% of pool reserves per provide_liquidity action


def pool_reserves_ok(reserve0: int, reserve1: int) -> bool:
    return (
        reserve0 >= MIN_RESERVE_PER_SIDE_FOR_SWAP
        and reserve1 >= MIN_RESERVE_PER_SIDE_FOR_SWAP
    )


def pick_scaled_provide_amounts(
    reserve0: int, reserve1: int, fraction_ppm: int = LP_FRACTION_PPM
) -> tuple[int, int] | None:
    """Proportional add amounts for an existing pool (keeps spot price)."""
    if not pool_reserves_ok(reserve0, reserve1):
        return None
    amount0 = (reserve0 * fraction_ppm) // 1_000_000
    amount1 = (reserve1 * fraction_ppm) // 1_000_000
    if amount0 < MIN_PROVIDE_LIQUIDITY_LEG or amount1 < MIN_PROVIDE_LIQUIDITY_LEG:
        return None
    return amount0, amount1


def bootstrap_top_up_amounts(
    reserve0: int,
    reserve1: int,
    *,
    floor_per_side: int,
    target_per_side: int,
) -> tuple[int, int] | None:
    """One-shot top-up when either reserve is below *floor_per_side*."""
    if reserve0 >= floor_per_side and reserve1 >= floor_per_side:
        return None
    # Bring the thinner side up toward target while preserving the pool ratio.
    if reserve0 <= 0 or reserve1 <= 0:
        return None
    scale0 = (target_per_side * 1_000_000) // reserve0 if reserve0 < floor_per_side else 0
    scale1 = (target_per_side * 1_000_000) // reserve1 if reserve1 < floor_per_side else 0
    scale = max(scale0, scale1)
    if scale <= 1_000_000:
        return None
    add0 = (reserve0 * (scale - 1_000_000)) // 1_000_000
    add1 = (reserve1 * (scale - 1_000_000)) // 1_000_000
    if add0 < MIN_PROVIDE_LIQUIDITY_LEG or add1 < MIN_PROVIDE_LIQUIDITY_LEG:
        return None
    return add0, add1
