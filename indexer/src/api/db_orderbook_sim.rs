//! Postgres-backed hybrid simulation for the route solver (GitLab #319 Phase 1c).
//!
//! Prices pool + resting-book legs from `pair_reserves` and `resting_limit_orders` using the same
//! math as on-chain `query_hybrid_simulation` / `orderbook::simulate_match_*`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::api::orderbook_sim::{ceil_div, swap_fee_amount};
use crate::db::queries::{pair_reserves, pairs, resting_orders};

/// Max maker fills per hop (on-chain hard cap).
pub const MAX_MAKER_FILLS_HARD_CAP: u32 = 30;

/// Book walk scan-step cap (matches pair contract).
const MAX_SCAN_STEPS: u32 = 256;

/// Production tier `discount_bps` by `traders.tier_id` (see `docs/reference/fee-discount-tiers.md`).
fn tier_discount_bps(tier_id: i16) -> u16 {
    match tier_id {
        // Sentinel from `resolve_discount_tier` when no discount subject is known (#283).
        -1 => 0,
        0 => 10_000,
        1 => 250,
        2 => 1_000,
        3 => 2_000,
        4 => 3_500,
        5 => 5_000,
        6 => 6_000,
        7 => 7_500,
        8 => 8_500,
        9 => 9_500,
        255 => 0,
        _ => 0,
    }
}

#[inline]
pub fn effective_fee_bps(base_fee_bps: u16, discount_tier: i16) -> u16 {
    let discount = tier_discount_bps(discount_tier);
    let num = (base_fee_bps as u32).saturating_mul(10_000u32 - discount as u32);
    (num / 10_000) as u16
}

#[inline]
fn taker_fee_bps(effective_fee_bps: u16) -> u16 {
    effective_fee_bps - effective_fee_bps / 2
}

pub fn parse_u128(bd: &BigDecimal) -> Option<u128> {
    bd.with_scale(0).to_string().parse().ok()
}

fn mul_floor_u128(a: u128, price: &BigDecimal) -> Option<u128> {
    if price <= &BigDecimal::from(0u32) {
        return None;
    }
    let prod = BigDecimal::from(a) * price;
    prod.with_scale(0).to_string().parse().ok()
}

fn div_floor_u128(a: u128, price: &BigDecimal) -> Option<u128> {
    if price <= &BigDecimal::from(0u32) {
        return None;
    }
    let q = BigDecimal::from(a) / price;
    q.with_scale(0).to_string().parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorFreshness {
    Fresh,
    MissingReserves,
    Stale,
}

#[derive(Debug, Clone)]
pub struct HopMirror {
    pub pair_id: i32,
    pub asset_0_addr: String,
    pub asset_1_addr: String,
    pub reserve_0: u128,
    pub reserve_1: u128,
    pub fee_bps: u16,
    pub block_height: Option<i64>,
    pub snapshot_at: DateTime<Utc>,
    pub freshness: MirrorFreshness,
    pub bids: Vec<resting_orders::RestingOrderRow>,
    pub asks: Vec<resting_orders::RestingOrderRow>,
}

#[derive(Debug, Clone, Default)]
pub struct MirrorLoadMeta {
    pub db_hybrid_queries: u32,
    pub lcd_fallback_queries: u32,
    pub mirror_stale_hops: u32,
    pub mirror_missing_hops: u32,
    pub max_snapshot_age_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DbSimError {
    #[error("pair not indexed")]
    PairNotFound,
    #[error("mirror missing reserves")]
    MissingMirror,
    #[error("mirror stale")]
    StaleMirror,
    #[error("invalid numeric in mirror row")]
    InvalidNumeric,
    #[error("insufficient pool liquidity")]
    InsufficientLiquidity,
    #[error("{0}")]
    Db(#[from] sqlx::Error),
}

pub fn snapshot_age_ms(at: DateTime<Utc>) -> u64 {
    let now = Utc::now();
    now.signed_duration_since(at).num_milliseconds().max(0) as u64
}

pub fn is_snapshot_stale(at: DateTime<Utc>, max_staleness_ms: u64) -> bool {
    snapshot_age_ms(at) > max_staleness_ms
}

/// Load mirrored reserves + resting book for one hop pair contract.
pub async fn load_hop_mirror(
    pool: &PgPool,
    _pair_address: &str,
    asset_addrs: &HashMap<i32, String>,
    pair_row: &pairs::PairRow,
    max_staleness_ms: u64,
    now_secs: u64,
) -> Result<HopMirror, DbSimError> {
    let asset_0_addr = asset_addrs
        .get(&pair_row.asset_0_id)
        .cloned()
        .ok_or(DbSimError::PairNotFound)?;
    let asset_1_addr = asset_addrs
        .get(&pair_row.asset_1_id)
        .cloned()
        .ok_or(DbSimError::PairNotFound)?;

    let reserves = pair_reserves::get_pair_reserves(pool, pair_row.id)
        .await?
        .ok_or(DbSimError::MissingMirror)?;

    let stale = is_snapshot_stale(reserves.snapshot_at, max_staleness_ms);
    let freshness = if stale {
        MirrorFreshness::Stale
    } else {
        MirrorFreshness::Fresh
    };

    let reserve_0 = parse_u128(&reserves.reserve_0).ok_or(DbSimError::InvalidNumeric)?;
    let reserve_1 = parse_u128(&reserves.reserve_1).ok_or(DbSimError::InvalidNumeric)?;

    let bids = resting_orders::get_pair_resting_book(pool, pair_row.id, "bid").await?;
    let asks = resting_orders::get_pair_resting_book(pool, pair_row.id, "ask").await?;

    let bids = filter_live_orders(bids, now_secs);
    let asks = filter_live_orders(asks, now_secs);

    Ok(HopMirror {
        pair_id: pair_row.id,
        asset_0_addr,
        asset_1_addr,
        reserve_0,
        reserve_1,
        fee_bps: reserves.fee_bps.max(0) as u16,
        block_height: reserves.block_height,
        snapshot_at: reserves.snapshot_at,
        freshness,
        bids,
        asks,
    })
}

fn filter_live_orders(
    rows: Vec<resting_orders::RestingOrderRow>,
    now_secs: u64,
) -> Vec<resting_orders::RestingOrderRow> {
    rows.into_iter()
        .filter(|o| o.expires_at.map(|e| now_secs < e as u64).unwrap_or(true))
        .filter(|o| parse_u128(&o.remaining).unwrap_or(0) > 0)
        .collect()
}

#[derive(Debug, Clone, Copy, Default)]
struct BookSimResult {
    return_net: u128,
    offer_consumed: u128,
}

fn simulate_match_bids(
    orders: &[resting_orders::RestingOrderRow],
    token0_budget: u128,
    max_maker_fills: u32,
    effective_fee_bps: u16,
) -> Result<BookSimResult, DbSimError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token0_left = token0_budget;
    let mut token1_out = 0u128;
    let mut makers_used = 0u32;
    let mut scan_steps = 0u32;

    for order in orders {
        if !book_walk_step(&mut scan_steps) {
            break;
        }
        if makers_used >= cap || token0_left == 0 {
            break;
        }
        let price = &order.price;
        if price <= &BigDecimal::from(0u32) {
            continue;
        }
        let remaining = parse_u128(&order.remaining).ok_or(DbSimError::InvalidNumeric)?;
        if remaining == 0 {
            continue;
        }
        let max_fill_from_bid = div_floor_u128(remaining, price).unwrap_or(0);
        let mut fill = token0_left.min(max_fill_from_bid);
        if fill == 0 {
            continue;
        }
        let mut cost = mul_floor_u128(fill, price).unwrap_or(0);
        while fill > 0 && cost > remaining {
            fill = fill.saturating_sub(1);
            if fill == 0 {
                break;
            }
            cost = mul_floor_u128(fill, price).unwrap_or(0);
        }
        if fill == 0 {
            continue;
        }
        makers_used += 1;
        let commission = cost.saturating_mul(taker_bps as u128) / 10_000;
        let net = cost.saturating_sub(commission);
        token0_left = token0_left.saturating_sub(fill);
        token1_out = token1_out.saturating_add(net);
    }

    Ok(BookSimResult {
        return_net: token1_out,
        offer_consumed: token0_budget.saturating_sub(token0_left),
    })
}

fn simulate_match_asks(
    orders: &[resting_orders::RestingOrderRow],
    token1_budget: u128,
    max_maker_fills: u32,
    effective_fee_bps: u16,
) -> Result<BookSimResult, DbSimError> {
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token1_left = token1_budget;
    let mut token0_out = 0u128;
    let mut makers_used = 0u32;
    let mut scan_steps = 0u32;

    for order in orders {
        if !book_walk_step(&mut scan_steps) {
            break;
        }
        if makers_used >= cap || token1_left == 0 {
            break;
        }
        let price = &order.price;
        if price <= &BigDecimal::from(0u32) {
            continue;
        }
        let remaining = parse_u128(&order.remaining).ok_or(DbSimError::InvalidNumeric)?;
        if remaining == 0 {
            continue;
        }
        let max_fill_from_budget = div_floor_u128(token1_left, price).unwrap_or(0);
        let mut fill_t0 = remaining.min(max_fill_from_budget);
        if fill_t0 == 0 {
            continue;
        }
        let mut cost = mul_floor_u128(fill_t0, price).unwrap_or(0);
        while fill_t0 > 0 && cost > token1_left {
            fill_t0 = fill_t0.saturating_sub(1);
            if fill_t0 == 0 {
                break;
            }
            cost = mul_floor_u128(fill_t0, price).unwrap_or(0);
        }
        if fill_t0 == 0 {
            continue;
        }
        makers_used += 1;
        let commission = fill_t0.saturating_mul(taker_bps as u128) / 10_000;
        let net = fill_t0.saturating_sub(commission);
        token1_left = token1_left.saturating_sub(cost);
        token0_out = token0_out.saturating_add(net);
    }

    Ok(BookSimResult {
        return_net: token0_out,
        offer_consumed: token1_budget.saturating_sub(token1_left),
    })
}

fn book_walk_step(scan_steps: &mut u32) -> bool {
    *scan_steps += 1;
    *scan_steps <= MAX_SCAN_STEPS
}

fn simulate_pool_leg(
    input_reserve: u128,
    output_reserve: u128,
    pool_input: u128,
    fee_bps: u16,
) -> Result<u128, DbSimError> {
    if pool_input == 0 {
        return Ok(0);
    }
    if input_reserve == 0 || output_reserve == 0 {
        return Err(DbSimError::InsufficientLiquidity);
    }
    let k = input_reserve.saturating_mul(output_reserve);
    let new_in = input_reserve.saturating_add(pool_input);
    let new_out = ceil_div(k, new_in);
    let gross = output_reserve.saturating_sub(new_out);
    let fee = swap_fee_amount(gross, fee_bps);
    Ok(gross.saturating_sub(fee))
}

/// Hybrid simulation from mirrored Postgres state (forward offer → ask output).
pub fn simulate_hybrid_from_mirror(
    mirror: &HopMirror,
    offer_token: &str,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
    discount_tier: i16,
) -> Result<u128, DbSimError> {
    if offer_amount == 0 {
        return Ok(0);
    }
    if pool_input.saturating_add(book_input) != offer_amount {
        return Err(DbSimError::InvalidNumeric);
    }
    if mirror.freshness != MirrorFreshness::Fresh {
        return Err(match mirror.freshness {
            MirrorFreshness::Stale => DbSimError::StaleMirror,
            MirrorFreshness::MissingReserves => DbSimError::MissingMirror,
            MirrorFreshness::Fresh => unreachable!(),
        });
    }

    let eff_fee = effective_fee_bps(mirror.fee_bps, discount_tier);
    let offer_is_token0 = offer_token.eq_ignore_ascii_case(&mirror.asset_0_addr);

    let (input_reserve, output_reserve) = if offer_is_token0 {
        (mirror.reserve_0, mirror.reserve_1)
    } else {
        (mirror.reserve_1, mirror.reserve_0)
    };

    let mut book_return = 0u128;
    let mut offer_consumed_by_book = 0u128;

    if book_input > 0 {
        let book_sim = if offer_is_token0 {
            simulate_match_bids(&mirror.bids, book_input, max_maker_fills, eff_fee)?
        } else {
            simulate_match_asks(&mirror.asks, book_input, max_maker_fills, eff_fee)?
        };
        book_return = book_sim.return_net;
        offer_consumed_by_book = book_sim.offer_consumed;
    }

    let pool_input_amount =
        pool_input.saturating_add(book_input.saturating_sub(offer_consumed_by_book));
    let pool_out = simulate_pool_leg(input_reserve, output_reserve, pool_input_amount, eff_fee)?;
    Ok(book_return.saturating_add(pool_out))
}

/// Pool-only hybrid (`book_input = 0`) when mirror is fresh.
/// Preload mirrored state for a set of pair contract addresses (route solver).
pub async fn preload_mirrors_for_pairs(
    pool: &PgPool,
    pair_addresses: &[String],
    id_to_addr: &HashMap<i32, String>,
    max_staleness_ms: u64,
) -> Result<HashMap<String, HopMirror>, sqlx::Error> {
    let now_secs = Utc::now().timestamp().max(0) as u64;
    let mut out = HashMap::new();
    for addr in pair_addresses {
        let Some(pair_row) = pairs::get_pair_by_address(pool, addr).await? else {
            continue;
        };
        match load_hop_mirror(
            pool,
            addr,
            id_to_addr,
            &pair_row,
            max_staleness_ms,
            now_secs,
        )
        .await
        {
            Ok(m) => {
                out.insert(addr.clone(), m);
            }
            Err(DbSimError::MissingMirror) => {
                out.insert(
                    addr.clone(),
                    HopMirror {
                        pair_id: pair_row.id,
                        asset_0_addr: id_to_addr
                            .get(&pair_row.asset_0_id)
                            .cloned()
                            .unwrap_or_default(),
                        asset_1_addr: id_to_addr
                            .get(&pair_row.asset_1_id)
                            .cloned()
                            .unwrap_or_default(),
                        reserve_0: 0,
                        reserve_1: 0,
                        fee_bps: pair_row.fee_bps.unwrap_or(30) as u16,
                        block_height: None,
                        snapshot_at: Utc::now(),
                        freshness: MirrorFreshness::MissingReserves,
                        bids: vec![],
                        asks: vec![],
                    },
                );
            }
            Err(e) => {
                tracing::warn!(pair = %addr, "preload mirror failed: {}", e);
            }
        }
    }
    Ok(out)
}

pub fn simulate_pool_only_from_mirror(
    mirror: &HopMirror,
    offer_token: &str,
    offer_amount: u128,
    discount_tier: i16,
) -> Result<u128, DbSimError> {
    simulate_hybrid_from_mirror(
        mirror,
        offer_token,
        offer_amount,
        offer_amount,
        0,
        1,
        discount_tier,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bd(s: &str) -> BigDecimal {
        s.parse().unwrap()
    }

    fn resting(
        order_id: i64,
        side: &str,
        price: &str,
        remaining: &str,
    ) -> resting_orders::RestingOrderRow {
        resting_orders::RestingOrderRow {
            pair_id: 1,
            order_id,
            side: side.to_string(),
            price: bd(price),
            remaining: bd(remaining),
            owner: None,
            expires_at: None,
        }
    }

    fn mirror_with_book(bids: Vec<resting_orders::RestingOrderRow>) -> HopMirror {
        HopMirror {
            pair_id: 1,
            asset_0_addr: "terra1token0".into(),
            asset_1_addr: "terra1token1".into(),
            reserve_0: 1_000_000_000,
            reserve_1: 2_000_000_000,
            fee_bps: 30,
            block_height: Some(1),
            snapshot_at: Utc::now(),
            freshness: MirrorFreshness::Fresh,
            bids,
            asks: vec![],
        }
    }

    #[test]
    fn pool_only_positive_output() {
        let m = mirror_with_book(vec![]);
        let out = simulate_pool_only_from_mirror(&m, "terra1token0", 100_000, 0).unwrap();
        assert!(out > 0);
    }

    #[test]
    fn bid_book_adds_return() {
        let m = mirror_with_book(vec![resting(1, "bid", "2", "1000")]);
        let pool_only =
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 100_000, 0, 8, 0).unwrap();
        let hybrid =
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 50_000, 50_000, 8, 0).unwrap();
        assert!(hybrid >= pool_only);
    }

    #[test]
    fn effective_fee_tier_reduces_fee_increases_out() {
        let m = mirror_with_book(vec![]);
        let full = simulate_pool_only_from_mirror(&m, "terra1token0", 1_000_000, 255).unwrap();
        let disc = simulate_pool_only_from_mirror(&m, "terra1token0", 1_000_000, 5).unwrap();
        assert!(disc > full);
    }
}
