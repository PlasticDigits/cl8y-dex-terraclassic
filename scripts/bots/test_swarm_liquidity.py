#!/usr/bin/env python3
"""Unit tests for scripts/bots/swarm_liquidity.py (stdlib unittest)."""

from __future__ import annotations

import unittest

from swarm_liquidity import (
    MIN_PROVIDE_LIQUIDITY_LEG,
    MIN_RESERVE_PER_SIDE_FOR_SWAP,
    bootstrap_top_up_amounts,
    pick_scaled_provide_amounts,
    pool_reserves_ok,
)


class SwarmLiquidityTests(unittest.TestCase):
    def test_pool_reserves_ok(self) -> None:
        self.assertFalse(pool_reserves_ok(1_000, 1_000))
        self.assertTrue(pool_reserves_ok(10_000_000, 10_000_000))

    def test_pick_scaled_provide_amounts(self) -> None:
        got = pick_scaled_provide_amounts(100_000_000_000, 100_000_000_000)
        self.assertIsNotNone(got)
        a0, a1 = got
        self.assertEqual(a0, 300_000_000)
        self.assertEqual(a1, 300_000_000)
        self.assertGreaterEqual(a0, MIN_PROVIDE_LIQUIDITY_LEG)

    def test_pick_scaled_returns_none_when_too_small(self) -> None:
        self.assertIsNone(pick_scaled_provide_amounts(5_000_000, 5_000_000))

    def test_bootstrap_top_up_when_below_floor(self) -> None:
        got = bootstrap_top_up_amounts(
            500_000_000,
            500_000_000,
            floor_per_side=5_000_000_000,
            target_per_side=50_000_000_000,
        )
        self.assertIsNotNone(got)
        a0, a1 = got
        self.assertGreaterEqual(a0, MIN_PROVIDE_LIQUIDITY_LEG)
        self.assertGreaterEqual(a1, MIN_PROVIDE_LIQUIDITY_LEG)

    def test_bootstrap_skips_when_healthy(self) -> None:
        self.assertIsNone(
            bootstrap_top_up_amounts(
                100_000_000_000,
                100_000_000_000,
                floor_per_side=5_000_000_000,
                target_per_side=50_000_000_000,
            )
        )


if __name__ == "__main__":
    unittest.main()
