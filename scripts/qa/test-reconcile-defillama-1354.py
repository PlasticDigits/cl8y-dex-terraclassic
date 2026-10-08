import importlib.util
from datetime import date
from decimal import Decimal
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("reconciliation", Path(__file__).with_name("reconcile-defillama-1354.py"))
report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(report)


class ReceiptContract(unittest.TestCase):
    def test_historical_corrections_preserve_null_and_decimal_precision(self):
        previous = {"daily": [{"date": "2026-09-24", "volume_usd": None,
                               "daily_fees_usd": "7.2500", "llama_fees": 115}]}
        current = {"observed_at": "now", "daily": [{"date": "2026-09-24",
                   "volume_usd": "558.461005", "daily_fees_usd": "7.25", "llama_fees": 167}]}
        changes = report.historical_corrections(previous, current)
        self.assertEqual(changes[0]["fields"]["volume_usd"]["previous"], None)
        self.assertEqual(changes[0]["fields"]["llama_fees"]["current"], 167)
        self.assertNotIn("daily_fees_usd", changes[0]["fields"])

    def test_missing_null_and_display_rounding_are_distinct(self):
        self.assertEqual(report.compare(None, 0), "feed_null")
        self.assertEqual(report.compare("12", None), "llama_day_missing")
        self.assertEqual(report.compare("558.461005", 558), "within_rounding")
        self.assertEqual(report.compare("166.95", 115), "drift_over_rounding")

    def test_exact_decimal_rounding_boundary(self):
        # Floating conversion would erase this fraction and hide a real drift.
        self.assertEqual(report.compare("10000000000000000001.000000000000000001",
                                        Decimal("10000000000000000000")), "drift_over_rounding")

    def test_unavailable_transport_never_fabricates_accounting_failure(self):
        with patch.object(report, "fetch", side_effect=lambda url: {
                "url": url, "status": 503, "reason": "HTTPError"}):
            receipt = report.collect(date(2026, 9, 24), date(2026, 9, 24))
        self.assertEqual(receipt["result"], "INCOMPLETE")
        self.assertEqual(receipt["failures"], [])
        self.assertEqual(receipt["daily"], [])
        self.assertEqual(len(receipt["unavailable"]), 9)

    def test_completeness_counts_reject_null_bool_and_inconsistent_totals(self):
        self.assertTrue(report.has_completeness({"trade_count": 28, "unpriced_trade_count": 1}))
        self.assertTrue(report.has_completeness({"trade_count": 28, "unpriced_trade_count": 1,
                                               "priced_trade_count": 27}))
        for fields in ({"unpriced_trade_count": None}, {"unpriced_trade_count": True},
                       {"unpriced_trade_count": 29}, {"unpriced_trade_count": 1, "priced_trade_count": 29},
                       {"unpriced_trade_count": 1, "priced_trade_count": 26}):
            self.assertFalse(report.has_completeness({"trade_count": 28, **fields}))


if __name__ == "__main__":
    unittest.main()
