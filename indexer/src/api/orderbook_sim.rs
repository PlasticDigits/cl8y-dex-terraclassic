//! **AMM-simulated orderbook** for CoinGecko / CoinMarketCap depth endpoints.
//!
//! [`walk_amm_book`] walks the **constant-product pool curve** with pair-style [`ceil_div`]
//! and pool swap fees — **not** the on-chain **FIFO limit order book** (see [`limit-book`](../../docs/limit-orders.md)).
//!
//! LCD integration: live `pool` reserves + `get_fee_config`; indexer `pairs.fee_bps` is used only when
//! the fee query fails (logged). Wiremock LCD stubs in `tests/common/lcd_mock.rs` mock HTTP only.
//!
//! Normative algorithm: [`docs/CG_CMC_COMPLIANCE.md`](../../docs/CG_CMC_COMPLIANCE.md) § AMM Orderbook Simulation.
//! Invariants: [`docs/indexer-invariants.md`](../../docs/indexer-invariants.md). Closed in GitLab **#210**; Openware total-depth split in **#221**.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bigdecimal::BigDecimal;
use sqlx::PgPool;
use tokio::sync::RwLock;
use tracing::warn;

use crate::db::queries::pairs;
use crate::lcd::{FeeConfigResponse, LcdClient, PoolResponse};

const ORDERBOOK_CACHE_TTL: Duration = Duration::from_secs(30);

/// Default / max for CG/CMC `depth` query: **total levels across the book** (Openware / CMC exchange integration).
pub const ORDERBOOK_DEPTH_DEFAULT: usize = 20;
pub const ORDERBOOK_DEPTH_MAX: usize = 100;

/// Cap `depth` query param: default [`ORDERBOOK_DEPTH_DEFAULT`], max [`ORDERBOOK_DEPTH_MAX`].
#[must_use]
pub fn cap_orderbook_depth(requested: Option<usize>) -> usize {
    requested
        .unwrap_or(ORDERBOOK_DEPTH_DEFAULT)
        .clamp(1, ORDERBOOK_DEPTH_MAX)
}

/// Ladder steps **per side** from total requested depth (`depth=100` → 50 bids + 50 asks).
///
/// Uses integer floor (`total / 2`); odd totals drop the remainder (e.g. `21` → 10+10).
/// `depth=1` yields one bid and one ask (`max(1, 0)`).
#[must_use]
pub fn levels_per_side(total_depth: usize) -> usize {
    (total_depth / 2).max(1)
}

/// Maximum fraction of `reserve_0` (base) per level: 10% (`CG_CMC_COMPLIANCE.md`).
const MAX_STEP_NUMERATOR: u128 = 10;
const MAX_STEP_DENOMINATOR: u128 = 100;

#[derive(Clone, Debug)]
pub struct OrderbookData {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

/// In-memory cache of simulated orderbooks to limit LCD amplification (per pair + depth + fee).
#[derive(Clone, Default)]
pub struct OrderbookCache {
    inner: Arc<RwLock<HashMap<String, (OrderbookData, Instant)>>>,
}

/// Ceiling division matching pair `ceil_div` (pool-favorable rounding).
#[must_use]
pub fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    assert!(denominator > 0, "ceil_div denominator must be non-zero");
    let q = numerator / denominator;
    if q.saturating_mul(denominator) < numerator {
        q + 1
    } else {
        q
    }
}

/// Pool swap fee on gross output: `gross * fee_bps / 10000` (pair `execute_swap` pool leg).
#[must_use]
pub fn swap_fee_amount(gross_output: u128, fee_bps: u16) -> u128 {
    if fee_bps == 0 || gross_output == 0 {
        return 0;
    }
    gross_output
        .saturating_mul(fee_bps as u128)
        .saturating_div(10_000)
}

/// Base amount for step `i` (1-based) per CG/CMC spec: `R0 * (i / depth) * 0.10`.
#[must_use]
pub fn step_base_amount(reserve_0: u128, step_index: usize, depth: usize) -> u128 {
    let depth = depth.max(1);
    let i = step_index.max(1) as u128;
    let d = depth as u128;
    reserve_0
        .saturating_mul(i)
        .saturating_mul(MAX_STEP_NUMERATOR)
        .saturating_div(d.saturating_mul(MAX_STEP_DENOMINATOR))
}

fn format_amount(amount: u128) -> String {
    amount.to_string()
}

/// Price as quote-per-base decimal string (`asset_1` / `asset_0`).
fn format_price(quote_amount: u128, base_amount: u128) -> String {
    if base_amount == 0 {
        return "0".to_string();
    }
    let price = BigDecimal::from(quote_amount) / BigDecimal::from(base_amount);
    price.to_string()
}

/// Pure constant-product walk — no I/O. `asset_0` = base, `asset_1` = quote.
#[must_use]
pub fn walk_amm_book(
    reserve_0: u128,
    reserve_1: u128,
    depth: usize,
    fee_bps: u16,
) -> OrderbookData {
    if reserve_0 == 0 || reserve_1 == 0 {
        return OrderbookData {
            bids: Vec::new(),
            asks: Vec::new(),
        };
    }

    let depth = depth.max(1);
    let k = reserve_0.saturating_mul(reserve_1);
    let mut bids = Vec::with_capacity(depth);
    let mut asks = Vec::with_capacity(depth);

    for step in 1..=depth {
        let dx = step_base_amount(reserve_0, step, depth);
        if dx == 0 {
            continue;
        }

        // Bid: sell `dx` base into pool → receive quote (fee on quote output).
        if let Some(level) = simulate_bid_level(reserve_0, reserve_1, k, dx, fee_bps) {
            bids.push(level);
        }

        // Ask: buy `dx` base from pool → pay quote (fee on base output).
        if let Some(level) = simulate_ask_level(reserve_0, reserve_1, k, dx, fee_bps) {
            asks.push(level);
        }
    }

    OrderbookData { bids, asks }
}

fn simulate_bid_level(
    reserve_0: u128,
    reserve_1: u128,
    k: u128,
    dx: u128,
    fee_bps: u16,
) -> Option<[String; 2]> {
    let new_reserve_0 = reserve_0.checked_add(dx)?;
    let new_reserve_1 = ceil_div(k, new_reserve_0);
    let gross_quote = reserve_1.checked_sub(new_reserve_1)?;
    if gross_quote == 0 {
        return None;
    }
    let fee = swap_fee_amount(gross_quote, fee_bps);
    let net_quote = gross_quote.checked_sub(fee)?;
    if net_quote == 0 {
        return None;
    }
    Some([
        format_price(net_quote, dx),
        format_amount(dx),
    ])
}

fn simulate_ask_level(
    reserve_0: u128,
    reserve_1: u128,
    k: u128,
    dx: u128,
    fee_bps: u16,
) -> Option<[String; 2]> {
    if dx >= reserve_0 {
        return None;
    }
    let new_reserve_0 = reserve_0.checked_sub(dx)?;
    let new_reserve_1 = ceil_div(k, new_reserve_0);
    let quote_cost = new_reserve_1.checked_sub(reserve_1)?;
    if quote_cost == 0 {
        return None;
    }
    let fee = swap_fee_amount(dx, fee_bps);
    let net_base = dx.checked_sub(fee)?;
    if net_base == 0 {
        return None;
    }
    Some([
        format_price(quote_cost, net_base),
        format_amount(dx),
    ])
}

async fn fee_bps_from_db(pool: &PgPool, pair_addr: &str) -> Option<u16> {
    match pairs::get_pair_by_address(pool, pair_addr).await {
        Ok(Some(row)) => row.fee_bps.map(|f| f.max(0) as u16),
        Ok(None) => None,
        Err(e) => {
            warn!(pair = pair_addr, error = %e, "pairs.fee_bps lookup failed");
            None
        }
    }
}

async fn resolve_fee_bps_lcd(lcd: &LcdClient, pair_addr: &str) -> Option<u16> {
    match lcd
        .query_contract::<FeeConfigResponse>(
            pair_addr,
            &serde_json::json!({"get_fee_config": {}}),
        )
        .await
    {
        Ok(resp) => Some(resp.fee_config.fee_bps),
        Err(e) => {
            warn!(
                pair = pair_addr,
                error = %e,
                "LCD get_fee_config failed"
            );
            None
        }
    }
}

/// Pool fee for simulation: indexer `pairs.fee_bps` when indexed, else LCD `get_fee_config`.
async fn resolve_fee_bps(pool: &PgPool, lcd: &LcdClient, pair_addr: &str) -> u16 {
    if let Some(fee) = fee_bps_from_db(pool, pair_addr).await {
        return fee;
    }
    if let Some(fee) = resolve_fee_bps_lcd(lcd, pair_addr).await {
        return fee;
    }
    0
}

fn parse_reserve(amount: &str) -> Option<u128> {
    amount.parse().ok().filter(|&v| v > 0)
}

async fn fetch_reserves(
    lcd: &LcdClient,
    pair_addr: &str,
) -> Result<(u128, u128), Box<dyn std::error::Error + Send + Sync>> {
    let pool_resp: PoolResponse = lcd
        .query_contract(pair_addr, &serde_json::json!({"pool": {}}))
        .await?;

    match (
        parse_reserve(&pool_resp.assets[0].amount),
        parse_reserve(&pool_resp.assets[1].amount),
    ) {
        (Some(r0), Some(r1)) => Ok((r0, r1)),
        _ => Ok((0, 0)),
    }
}

/// Simulate an AMM orderbook from LCD pool reserves and on-chain fee config.
pub async fn simulate_orderbook(
    pool: &PgPool,
    lcd: &LcdClient,
    pair_addr: &str,
    depth: usize,
) -> Result<OrderbookData, Box<dyn std::error::Error + Send + Sync>> {
    let fee_bps = resolve_fee_bps(pool, lcd, pair_addr).await;
    let (r0, r1) = fetch_reserves(lcd, pair_addr).await?;
    if r0 == 0 || r1 == 0 {
        return Ok(OrderbookData {
            bids: Vec::new(),
            asks: Vec::new(),
        });
    }
    let per_side = levels_per_side(depth);
    Ok(walk_amm_book(r0, r1, per_side, fee_bps))
}

/// Like [`simulate_orderbook`] but caches results per `(pair_addr, depth, fee_bps)` for [`ORDERBOOK_CACHE_TTL`].
///
/// `requested_depth` is the **query** `depth` (total across book); cache keys use this value, not per-side count.
pub async fn simulate_orderbook_cached(
    cache: &OrderbookCache,
    pool: &PgPool,
    lcd: &LcdClient,
    pair_addr: &str,
    requested_depth: usize,
) -> Result<OrderbookData, Box<dyn std::error::Error + Send + Sync>> {
    let now = Instant::now();
    let prefix = format!("{pair_addr}:{requested_depth}:");
    {
        let guard = cache.inner.read().await;
        for (key, (data, exp)) in guard.iter() {
            if key.starts_with(&prefix) && now < *exp {
                return Ok(data.clone());
            }
        }
    }

    let fee_bps = resolve_fee_bps(pool, lcd, pair_addr).await;
    let key = format!("{prefix}{fee_bps}");

    let (r0, r1) = fetch_reserves(lcd, pair_addr).await?;
    let data = if r0 == 0 || r1 == 0 {
        OrderbookData {
            bids: Vec::new(),
            asks: Vec::new(),
        }
    } else {
        walk_amm_book(r0, r1, levels_per_side(requested_depth), fee_bps)
    };
    let mut guard = cache.inner.write().await;
    guard.insert(key, (data.clone(), now + ORDERBOOK_CACHE_TTL));
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_orderbook_depth_defaults_and_caps() {
        assert_eq!(cap_orderbook_depth(None), 20);
        assert_eq!(cap_orderbook_depth(Some(9999)), 100);
        assert_eq!(cap_orderbook_depth(Some(0)), 1);
    }

    #[test]
    fn levels_per_side_openware_mapping() {
        assert_eq!(levels_per_side(100), 50);
        assert_eq!(levels_per_side(50), 25);
        assert_eq!(levels_per_side(20), 10);
        assert_eq!(levels_per_side(21), 10);
        assert_eq!(levels_per_side(1), 1);
        assert_eq!(levels_per_side(2), 1);
    }

    #[test]
    fn ceil_div_matches_pair_semantics() {
        assert_eq!(ceil_div(10, 3), 4);
        assert_eq!(ceil_div(9, 3), 3);
        assert_eq!(ceil_div(1, 3), 1);
    }

    #[test]
    fn step_base_amount_follows_cg_spec() {
        let r0 = 10_000u128;
        assert_eq!(step_base_amount(r0, 1, 20), 50); // 0.5%
        assert_eq!(step_base_amount(r0, 20, 20), 1000); // 10%
    }

    #[test]
    fn empty_reserves_yield_empty_book() {
        let book = walk_amm_book(0, 100, 20, 30);
        assert!(book.bids.is_empty());
        assert!(book.asks.is_empty());
    }

    #[test]
    fn depth_one_produces_single_levels() {
        let book = walk_amm_book(1_000_000, 2_000_000, 1, 0);
        assert_eq!(book.bids.len(), 1);
        assert_eq!(book.asks.len(), 1);
    }

    #[test]
    fn depth_100_produces_full_ladder() {
        let book = walk_amm_book(1_000_000_000_000, 500_000_000_000_000, 100, 30);
        assert_eq!(book.bids.len(), 100);
        assert_eq!(book.asks.len(), 100);
    }

    #[test]
    fn bid_prices_decrease_with_size() {
        let book = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 20, 30);
        let prices: Vec<f64> = book
            .bids
            .iter()
            .map(|l| l[0].parse().unwrap())
            .collect();
        for w in prices.windows(2) {
            assert!(
                w[0] > w[1],
                "bid prices should decrease: {:?}",
                prices
            );
        }
    }

    #[test]
    fn ask_prices_increase_with_size() {
        let book = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 20, 30);
        let prices: Vec<f64> = book
            .asks
            .iter()
            .map(|l| l[0].parse().unwrap())
            .collect();
        for w in prices.windows(2) {
            assert!(
                w[0] < w[1],
                "ask prices should increase: {:?}",
                prices
            );
        }
    }

    #[test]
    fn fee_worsens_bid_and_ask_vs_zero_fee() {
        let r0 = 10_000_000_000u128;
        let r1 = 5_000_000_000_000_000u128;
        let depth = 20;
        let zero = walk_amm_book(r0, r1, depth, 0);
        let fee = walk_amm_book(r0, r1, depth, 30);
        for i in 0..depth {
            let z_bid: f64 = zero.bids[i][0].parse().unwrap();
            let f_bid: f64 = fee.bids[i][0].parse().unwrap();
            assert!(f_bid < z_bid, "fee should lower bid price at level {i}");
            let z_ask: f64 = zero.asks[i][0].parse().unwrap();
            let f_ask: f64 = fee.asks[i][0].parse().unwrap();
            assert!(f_ask > z_ask, "fee should raise ask price at level {i}");
        }
    }

    #[test]
    fn manual_bid_level_matches_pair_math() {
        let r0 = 1_000_000u128;
        let r1 = 2_000_000u128;
        let k = r0 * r1;
        let dx = step_base_amount(r0, 1, 1);
        assert_eq!(dx, 100_000);
        let new_r0 = r0 + dx;
        let new_r1 = ceil_div(k, new_r0);
        let gross = r1 - new_r1;
        let fee = swap_fee_amount(gross, 30);
        let net = gross - fee;
        let book = walk_amm_book(r0, r1, 1, 30);
        assert_eq!(book.bids[0][0], format_price(net, dx));
        assert_eq!(book.bids[0][1], dx.to_string());
    }

    #[test]
    fn ask_levels_never_skipped_below_ten_percent_cap() {
        let book = walk_amm_book(10_000_000_000, 5_000_000_000_000_000, 20, 0);
        assert_eq!(book.asks.len(), 20);
    }
}
