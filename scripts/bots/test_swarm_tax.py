#!/usr/bin/env python3
"""Unit tests for scripts/bots/swarm_tax.py (stdlib unittest, no chain)."""

from __future__ import annotations

import tempfile
import unittest
from types import SimpleNamespace

from swarm_tax import (
    balance_covers_debit,
    collect_tax_addrs,
    fail_closed_sell_debit,
    filter_gem_pairs,
    filter_tax_pairs,
    find_tax_multihop,
    pair_direct_sets_trader,
    pair_direct_swap_hook,
    parse_vite_env_file,
    required_wallet_debit,
    router_execute_swap_operations,
    tax_token_from_mapping,
    tax_workers_enabled,
)


class TaxDetectTests(unittest.TestCase):
    def test_workers_default_on(self) -> None:
        self.assertTrue(tax_workers_enabled({}))

    def test_workers_escape_hatch(self) -> None:
        self.assertFalse(tax_workers_enabled({"SWARM_TAX_WORKERS": "0"}))
        self.assertFalse(tax_workers_enabled({"SWARM_TAX_WORKERS": "false"}))

    def test_tax_token_from_mapping(self) -> None:
        self.assertEqual(
            tax_token_from_mapping({"VITE_TOKEN_COMMUNITY_TAX_ADDRESS": "terra1tax"}),
            "terra1tax",
        )
        self.assertIsNone(tax_token_from_mapping({}))

    def test_parse_vite_env_file(self) -> None:
        with tempfile.NamedTemporaryFile("w", suffix=".env", delete=False) as fh:
            fh.write("VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1tax\n# comment\nFACTORY=no\n")
            path = fh.name
        pins = parse_vite_env_file(path)
        self.assertEqual(pins["VITE_TOKEN_COMMUNITY_TAX_ADDRESS"], "terra1tax")
        self.assertEqual(collect_tax_addrs({}, pins), {"terra1tax"})


class PreviewMathTests(unittest.TestCase):
    def test_fail_closed_500_bps(self) -> None:
        self.assertEqual(fail_closed_sell_debit(10_000, 500), 10_500)

    def test_pair_preview_debit(self) -> None:
        preview = {"declared": "10000", "debit": "10500", "credit": "10000", "tax": "500"}
        self.assertEqual(required_wallet_debit(preview, 10_000, "pair", 500), 10_500)

    def test_router_adds_hop_trader_debit(self) -> None:
        preview = {
            "declared": "10000",
            "debit": "10000",
            "credit": "10000",
            "tax": "500",
            "hop_trader_debit": "500",
        }
        self.assertEqual(required_wallet_debit(preview, 10_000, "router", 500), 10_500)

    def test_router_missing_hop_fail_closes(self) -> None:
        preview = {"declared": "10000", "debit": "10000", "credit": "10000", "tax": "0"}
        self.assertEqual(required_wallet_debit(preview, 10_000, "router", 500), 10_500)

    def test_no_preview_fail_closes(self) -> None:
        self.assertEqual(required_wallet_debit(None, 20_000, "pair", 500), 21_000)

    def test_balance_gate_refuses_100_percent_send(self) -> None:
        need = fail_closed_sell_debit(10_000, 500)
        self.assertFalse(balance_covers_debit(10_000, need))
        self.assertTrue(balance_covers_debit(10_500, need))


class ExcludeFilterTests(unittest.TestCase):
    def test_gem_workers_drop_tax_pair(self) -> None:
        tax = SimpleNamespace(token0="terra1tax", token1="terra1ember", pair_addr="p-tax")
        gem = SimpleNamespace(token0="terra1ember", token1="terra1coral", pair_addr="p-gem")
        gems = filter_gem_pairs([tax, gem], {"terra1tax"})
        self.assertEqual([m.pair_addr for m in gems], ["p-gem"])
        self.assertEqual([m.pair_addr for m in filter_tax_pairs([tax, gem], {"terra1tax"})], ["p-tax"])


class HookTests(unittest.TestCase):
    def test_pair_direct_does_not_set_trader(self) -> None:
        hook = pair_direct_swap_hook()
        self.assertIsNone(hook["swap"]["trader"])
        self.assertFalse(pair_direct_sets_trader(hook))
        self.assertTrue(pair_direct_sets_trader({"swap": {"trader": "terra1victim"}}))

    def test_router_user_send_has_no_trader_field(self) -> None:
        inner = router_execute_swap_operations([{"terra_swap": {}}])
        self.assertNotIn("trader", inner["execute_swap_operations"])


class MultihopTests(unittest.TestCase):
    def test_tax_ember_coral_two_hop(self) -> None:
        tax_p = SimpleNamespace(token0="terra1tax", token1="terra1ember")
        gem = SimpleNamespace(token0="terra1ember", token1="terra1coral")
        got = find_tax_multihop([tax_p, gem], {"terra1tax"}, prefer_sell=True)
        self.assertIsNotNone(got)
        offer, dest, ops = got
        self.assertEqual(offer, "terra1tax")
        self.assertEqual(dest, "terra1coral")
        self.assertEqual(len(ops), 2)

    def test_buy_path_ends_at_tax(self) -> None:
        tax_p = SimpleNamespace(token0="terra1tax", token1="terra1ember")
        gem = SimpleNamespace(token0="terra1ember", token1="terra1coral")
        got = find_tax_multihop([tax_p, gem], {"terra1tax"}, prefer_sell=False)
        self.assertIsNotNone(got)
        offer, dest, ops = got
        self.assertEqual(dest, "terra1tax")
        self.assertEqual(offer, "terra1coral")
        self.assertEqual(len(ops), 2)


if __name__ == "__main__":
    unittest.main()
