//! **Hybrid-simulated orderbook** for CoinGecko / CoinMarketCap depth (GitLab **#220**).
//!
//! Merges **AMM curve-walk** levels ([`orderbook_sim::walk_amm_book`]) with **resting FIFO limit**
//! levels from LCD. This is a **listing simulation**, not a live CEX L2 feed or guaranteed fill quote.
//!
//! Normative merge: [`docs/CG_CMC_COMPLIANCE.md`](../../docs/CG_CMC_COMPLIANCE.md) § Hybrid Orderbook Simulation.
//! Limit semantics: [`docs/limit-orders.md`](../../docs/limit-orders.md). Parent AMM walk: **#210**.

use bigdecimal::BigDecimal;

use super::limit_book_lcd::LimitBookOrderItem;
use super::orderbook_sim::OrderbookData;

/// Convert a resting limit order to a CG/CMC `[price, quantity]` level.
///
/// `price` is token1 per token0 (on-chain `LimitOrderResponse::price`).
/// Quantity is **base (token0)** units: asks use `remaining` directly; bids convert quote escrow.
#[must_use]
pub fn limit_order_to_level(side: &str, price: &str, remaining: &str) -> Option<[String; 2]> {
    let rem = remaining.parse::<u128>().ok().filter(|&v| v > 0)?;
    let price_bd = price.parse::<BigDecimal>().ok().filter(|p| p > &BigDecimal::from(0u32))?;

    let qty = if side.eq_ignore_ascii_case("bid") {
        let rem_bd = BigDecimal::from(rem);
        let base = rem_bd / &price_bd;
        // Floor toward zero — escrow is quote (token1); listing qty is base (token0).
        base.with_scale(0)
            .to_string()
            .parse::<u128>()
            .ok()
            .filter(|&v| v > 0)?
    } else {
        rem
    };

    Some([price.to_string(), qty.to_string()])
}

fn parse_level_price(level: &[String; 2]) -> Option<BigDecimal> {
    level[0].parse().ok()
}

/// Price-sorted merge of pool-sim and limit levels, truncated to `depth` per side.
///
/// - **Bids:** descending price (best bid first).
/// - **Asks:** ascending price (best ask first).
/// - **Same price:** quantities are **summed** into one level (pool + limits at identical price).
#[must_use]
pub fn merge_orderbook_sides(
    pool_levels: Vec<[String; 2]>,
    limit_levels: Vec<[String; 2]>,
    depth: usize,
    bids: bool,
) -> Vec<[String; 2]> {
    let depth = depth.max(1);
    let mut combined: Vec<[String; 2]> = pool_levels;
    combined.extend(limit_levels);

    if combined.is_empty() {
        return Vec::new();
    }

    combined.sort_by(|a, b| {
        let pa = parse_level_price(a).unwrap_or_default();
        let pb = parse_level_price(b).unwrap_or_default();
        if bids {
            pb.cmp(&pa)
        } else {
            pa.cmp(&pb)
        }
    });

    let mut merged: Vec<[String; 2]> = Vec::with_capacity(depth.min(combined.len()));
    for level in combined {
        if let Some(last) = merged.last_mut() {
            if last[0] == level[0] {
                let a = last[1].parse::<u128>().unwrap_or(0);
                let b = level[1].parse::<u128>().unwrap_or(0);
                last[1] = a.saturating_add(b).to_string();
                continue;
            }
        }
        merged.push(level);
        if merged.len() >= depth {
            break;
        }
    }
    merged
}

/// Merge pool-sim and limit books into listing depth.
#[must_use]
pub fn build_hybrid_book(
    pool: OrderbookData,
    bid_limits: &[LimitBookOrderItem],
    ask_limits: &[LimitBookOrderItem],
    depth: usize,
) -> OrderbookData {
    let bid_limit_levels: Vec<[String; 2]> = bid_limits
        .iter()
        .filter_map(|o| limit_order_to_level(&o.side, &o.price, &o.remaining))
        .collect();
    let ask_limit_levels: Vec<[String; 2]> = ask_limits
        .iter()
        .filter_map(|o| limit_order_to_level(&o.side, &o.price, &o.remaining))
        .collect();

    OrderbookData {
        bids: merge_orderbook_sides(pool.bids, bid_limit_levels, depth, true),
        asks: merge_orderbook_sides(pool.asks, ask_limit_levels, depth, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::limit_book_lcd::LimitBookOrderItem;
    use crate::api::orderbook_sim::walk_amm_book;

    fn limit_item(side: &str, price: &str, remaining: &str) -> LimitBookOrderItem {
        LimitBookOrderItem {
            order_id: 1,
            owner: "terra1x".to_string(),
            side: side.to_string(),
            price: price.to_string(),
            remaining: remaining.to_string(),
            expires_at: None,
        }
    }

    #[test]
    fn bid_limit_converts_quote_remaining_to_base_qty() {
        let level = limit_order_to_level("bid", "2", "1000").unwrap();
        assert_eq!(level[0], "2");
        assert_eq!(level[1], "500");
    }

    #[test]
    fn ask_limit_uses_base_remaining() {
        let level = limit_order_to_level("ask", "1.5", "900").unwrap();
        assert_eq!(level[1], "900");
    }

    #[test]
    fn merge_includes_limit_price_and_sorts_bids_desc() {
        let pool = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 3, 0);
        let limits = vec![limit_item("bid", "999999999999", "2000000000000")];
        let book = build_hybrid_book(pool, &limits, &[], 10);
        assert!(
            book.bids.iter().any(|l| l[0] == "999999999999"),
            "synthetic limit bid should appear: {:?}",
            book.bids
        );
        let prices: Vec<f64> = book
            .bids
            .iter()
            .map(|l| l[0].parse().unwrap())
            .collect();
        for w in prices.windows(2) {
            assert!(w[0] >= w[1], "bids must be non-increasing: {prices:?}");
        }
    }

    #[test]
    fn merge_asks_ascending() {
        let pool = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 2, 0);
        let limits = vec![limit_item("ask", "0.00001", "100")];
        let book = build_hybrid_book(pool, &[], &limits, 10);
        assert!(book.asks.iter().any(|l| l[0] == "0.00001"));
        let prices: Vec<f64> = book.asks.iter().map(|l| l[0].parse().unwrap()).collect();
        for w in prices.windows(2) {
            assert!(w[0] <= w[1], "asks must be non-decreasing: {prices:?}");
        }
    }

    #[test]
    fn merge_truncates_to_depth() {
        let pool = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 5, 0);
        let limits = vec![
            limit_item("bid", "100", "1"),
            limit_item("bid", "99", "1"),
            limit_item("bid", "98", "1"),
        ];
        let book = build_hybrid_book(pool, &limits, &[], 3);
        assert_eq!(book.bids.len(), 3);
    }

    #[test]
    fn same_price_levels_aggregate_quantity() {
        let pool = vec![["1.5".to_string(), "100".to_string()]];
        let limits = vec![["1.5".to_string(), "50".to_string()]];
        let merged = merge_orderbook_sides(pool, limits, 10, true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0][1], "150");
    }

    #[test]
    fn empty_limits_returns_pool_only_shape() {
        let pool = walk_amm_book(1_000_000, 2_000_000, 2, 0);
        let book = build_hybrid_book(pool.clone(), &[], &[], 2);
        assert_eq!(book.bids.len(), pool.bids.len());
        assert_eq!(book.asks.len(), pool.asks.len());
    }
}
