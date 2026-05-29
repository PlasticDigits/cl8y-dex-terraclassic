//! CoinGecko / CoinMarketCap listing timestamp formats (GitLab **#222**, **#224**).
//! CMC orderbook root array wrapper: **#223** (`GET /cmc/orderbook/*` only).
//!
//! Orderbook endpoints use numeric JSON timestamps (not RFC3339 strings):
//! - CG `/cg/orderbook` → Unix **milliseconds**
//! - CMC `/cmc/orderbook/*` → Unix **seconds** (aligned with `/cmc/trades`)

use chrono::Utc;

/// UTC instant for a single orderbook response, with CG and CMC field shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListingOrderbookTimestamps {
    /// `GET /cg/orderbook` `timestamp` (milliseconds).
    pub cg_ms: i64,
    /// `GET /cmc/orderbook/*` `timestamp` (seconds).
    pub cmc_s: i64,
}

impl ListingOrderbookTimestamps {
    /// Capture one UTC instant for both listing formats.
    pub fn now() -> Self {
        let t = Utc::now();
        Self {
            cg_ms: t.timestamp_millis(),
            cmc_s: t.timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cg_ms_divides_to_cmc_s_within_one_second() {
        let ts = ListingOrderbookTimestamps::now();
        let diff = (ts.cg_ms / 1000 - ts.cmc_s).abs();
        assert!(diff <= 1, "cg_ms/1000 should match cmc_s: {ts:?}");
    }

    #[test]
    fn millisecond_magnitude_sanity() {
        let ts = ListingOrderbookTimestamps::now();
        assert!(ts.cg_ms > 1_700_000_000_000);
        assert!(ts.cmc_s > 1_700_000_000);
    }
}
