//! CG/CMC ticker bid/ask without `f64 * 0.999` (GitLab #685).
//!
//! List path is SQL stamps only — no LCD N+1, no hybrid book sim per row.
//! When `pair_reserves` are usable: human quote-per-base mid ± `fee_bps`.
//! Otherwise omit-equivalent (`last_price` both sides). `/cg/orderbook` remains
//! the hybrid book.

use bigdecimal::{BigDecimal, Zero};

use crate::indexer::pair_price_usd::human_quote_per_base_from_reserves;

/// AMM v2 TVL stamp as the CoinGecko `liquidity_in_usd` string. Unpriced → `"0"`.
/// Never 24h `volume_usd`, never `$1` UST1, never `2.5×` USTR.
pub fn listing_liquidity_in_usd(stamp: Option<&BigDecimal>) -> String {
    stamp
        .filter(|v| **v > BigDecimal::zero())
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0".to_string())
}

/// `(bid, ask)` in the same human quote-per-base units as `last_price`.
pub fn listing_bid_ask(
    last_price: Option<&BigDecimal>,
    reserve_0: Option<&BigDecimal>,
    reserve_1: Option<&BigDecimal>,
    decimals_0: i16,
    decimals_1: i16,
    fee_bps: i16,
) -> (String, String) {
    let last = last_price.filter(|p| **p > BigDecimal::zero());
    let mid = match (reserve_0, reserve_1) {
        (Some(r0), Some(r1)) => human_quote_per_base_from_reserves(r0, r1, decimals_0, decimals_1),
        _ => None,
    };

    if let Some(center) = mid {
        let bps = i64::from(fee_bps.max(0)).min(10_000);
        let ten_k = BigDecimal::from(10_000);
        let mut bid = &center * BigDecimal::from(10_000 - bps) / &ten_k;
        let mut ask = &center * BigDecimal::from(10_000 + bps) / &ten_k;
        if let Some(last) = last {
            if last < &bid {
                bid = last.clone();
            }
            if last > &ask {
                ask = last.clone();
            }
        }
        (bid.to_string(), ask.to_string())
    } else if let Some(last) = last {
        let s = last.to_string();
        (s.clone(), s)
    } else {
        ("0".to_string(), "0".to_string())
    }
}

pub fn listing_cmc_unified_id(cmc_id: Option<i32>) -> i32 {
    cmc_id.filter(|id| *id > 0).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn liquidity_zero_when_unpriced() {
        assert_eq!(listing_liquidity_in_usd(None), "0");
        assert_eq!(listing_liquidity_in_usd(Some(&bd("0"))), "0");
        assert_eq!(listing_liquidity_in_usd(Some(&bd("123.45"))), "123.45");
    }

    #[test]
    fn bid_ask_from_reserves_not_f64_toy() {
        // 1:1 human mid, 30 bps → 0.997 / 1.003
        let last = bd("1");
        let r0 = bd("1000000");
        let r1 = bd("1000000");
        let (bid, ask) = listing_bid_ask(Some(&last), Some(&r0), Some(&r1), 6, 6, 30);
        let bid_n = BigDecimal::from_str(&bid).unwrap();
        let ask_n = BigDecimal::from_str(&ask).unwrap();
        assert!(bid_n <= last && last <= ask_n, "{bid} {ask}");
        assert_ne!(bid, format!("{:.18}", 1.0 * 0.999));
        assert!(ask_n - bid_n > BigDecimal::zero());
    }

    #[test]
    fn omit_equivalent_when_no_reserves() {
        let last = bd("0.95");
        let (bid, ask) = listing_bid_ask(Some(&last), None, None, 6, 6, 30);
        assert_eq!(bid, "0.95");
        assert_eq!(ask, "0.95");
    }

    #[test]
    fn last_price_inside_spread_when_stale_vs_reserves() {
        let last = bd("0.5");
        let r0 = bd("1000000");
        let r1 = bd("1000000");
        let (bid, ask) = listing_bid_ask(Some(&last), Some(&r0), Some(&r1), 6, 6, 30);
        let bid_n = BigDecimal::from_str(&bid).unwrap();
        let ask_n = BigDecimal::from_str(&ask).unwrap();
        assert!(bid_n <= last && last <= ask_n);
    }

    #[test]
    fn cmc_id_zero_when_unset() {
        assert_eq!(listing_cmc_unified_id(None), 0);
        assert_eq!(listing_cmc_unified_id(Some(0)), 0);
        assert_eq!(listing_cmc_unified_id(Some(1027)), 1027);
    }
}
