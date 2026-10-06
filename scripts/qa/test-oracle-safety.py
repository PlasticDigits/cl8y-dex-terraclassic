#!/usr/bin/env python3
"""Deterministic #1391 financial-consumer abuse controls (no real transactions)."""
from copy import deepcopy
from datetime import datetime, timedelta, timezone
from decimal import Decimal
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("oracle_safety", ROOT / "scripts/lib/oracle_safety.py")
safety = importlib.util.module_from_spec(spec)
spec.loader.exec_module(safety)
NOW = datetime(2026, 10, 5, 1, 0, 0, tzinfo=timezone.utc)


def quote():
    return dict(ticker="ustc", quote_asset="USTC", price_usd="0.005", quality="fresh",
                source_policy="cex-quorum-v1", as_of=NOW.isoformat(), max_age_seconds=300,
                source_count=2, quorum=2, accepted_sources=["kucoin", "mexc"])


class OracleSafety(unittest.TestCase):
    def test_response_shape_fails_cleanly(self):
        for raw in (None, [], "invalid"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                safety.validate_quote(raw,"ustc",NOW)
            with self.subTest(venus=raw), self.assertRaises(ValueError):
                safety.validate_venus(raw,NOW)

    def test_fresh_exact_identity_and_precision(self):
        self.assertEqual(safety.validate_quote(quote(), "ustc", NOW), Decimal("0.005"))
        for value in ("NaN", "Infinity", "-1", "0", "1e999", "1e-999", "1e7"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                safety.positive_decimal(value)

    def test_fail_closed_quality_identity_policy_and_quorum(self):
        for key, value in [("quality", q) for q in (None,"degraded","stale","unavailable","spot")]+[
            ("ticker", "lunc"), ("quote_asset", "cUSTC"), ("source_policy", "legacy-average"),
            ("source_count", 1), ("quorum", 1), ("quorum", True),
            ("accepted_sources", ["kucoin", "kucoin"]), ("accepted_sources", ["kucoin", "unknown"]),
            ("max_age_seconds", 0), ("max_age_seconds", "300")]:
            raw = quote(); raw[key] = value
            with self.subTest(key=key,value=value), self.assertRaises(ValueError):
                safety.validate_quote(raw,"ustc",NOW)

    def test_missing_future_expired_and_malformed_timestamp(self):
        for stamp in (None,"bad","2026-10-05T01:00:00",(NOW+timedelta(seconds=1)).isoformat(),
                      (NOW-timedelta(seconds=301)).isoformat()):
            raw=quote();raw["as_of"]=stamp
            with self.subTest(stamp=stamp), self.assertRaises(ValueError):
                safety.validate_quote(raw,"ustc",NOW)
        raw=quote();raw["as_of"]=(NOW-timedelta(seconds=300)).isoformat()
        self.assertEqual(safety.validate_quote(raw,"ustc",NOW),Decimal("0.005"))
        with self.assertRaises(ValueError):
            safety.validate_quote(raw,"ustc",NOW,max_age_seconds=60)

    def test_fdusd_requires_two_listed_sources(self):
        raw = quote(); raw.update(ticker="vfdusd", quote_asset="FDUSD", price_usd="1")
        with self.assertRaises(ValueError): safety.validate_quote(raw,"vfdusd",NOW)
        raw["accepted_sources"] = ["mexc","coingecko"]
        self.assertEqual(safety.validate_quote(raw,"vfdusd",NOW),Decimal("1"))

    def test_venus_exact_source_pin_and_freshness(self):
        raw=dict(quality="fresh",source="venus_bsc",vtoken=safety.VENUS_VTOKEN,fetched_at=NOW.isoformat(),fdusd_per_vfdusd="0.02")
        self.assertEqual(safety.validate_venus(raw,NOW),Decimal("0.02"))
        for key,value in [("quality","unavailable"),("source","cex"),("vtoken","0xspoof"),("fdusd_per_vfdusd","NaN"),
                          ("fetched_at",(NOW-timedelta(seconds=301)).isoformat())]:
            altered=deepcopy(raw);altered[key]=value
            with self.subTest(key=key),self.assertRaises(ValueError):
                safety.validate_venus(altered,NOW)

    def test_plan_movement_requires_recompute(self):
        safety.check_movement("1.01","1")
        with self.assertRaises(ValueError): safety.check_movement("1.010001","1")


if __name__ == "__main__": unittest.main()
