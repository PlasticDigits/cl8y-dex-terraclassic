//! Postgres-backed hybrid simulation for the route solver (GitLab #319 Phase 1c).
//!
//! Prices pool + resting-book legs from `hybrid_pair_reserves` and `resting_limit_orders` using the same
//! math as on-chain `query_hybrid_simulation` / `orderbook::simulate_match_*`.

use bigdecimal::{num_bigint::BigInt, BigDecimal};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::HashMap;

use crate::api::orderbook_sim::{ceil_div, swap_fee_amount};
use crate::db::queries::{pair_reserves, pairs, resting_orders};
use crate::hybrid_limits::MAX_MAKER_FILLS_HARD_CAP;

/// Book walk scan-step cap (matches pair contract).
const MAX_SCAN_STEPS: u32 = 500;

/// CosmWasm `Decimal` atomics: raw token1 per token0, irrespective of asset decimals.
const DECIMAL_SCALE: u128 = 1_000_000_000_000_000_000;

/// Production tier `discount_bps` by `traders.tier_id` (see `docs/reference/fee-discount-tiers.md`).
pub(crate) fn tier_discount_bps(tier_id: i16) -> u16 {
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
    effective_fee_bps_from_discount_bps(base_fee_bps, tier_discount_bps(discount_tier))
}

#[inline]
pub fn effective_fee_bps_from_discount_bps(base_fee_bps: u16, discount_bps: u16) -> u16 {
    let num = (base_fee_bps as u32).saturating_mul(10_000u32 - discount_bps as u32);
    (num / 10_000) as u16
}

#[inline]
fn taker_fee_bps(effective_fee_bps: u16) -> u16 {
    effective_fee_bps - effective_fee_bps / 2
}

pub fn parse_u128(bd: &BigDecimal) -> Option<u128> {
    bd.with_scale(0).to_string().parse().ok()
}

/// True when mirrored pool reserves cannot support a constant-product swap leg (#369).
#[inline]
pub fn pool_reserves_unusable(reserve_0: u128, reserve_1: u128) -> bool {
    reserve_0 == 0 || reserve_1 == 0
}

/// Reject fractional/out-of-range escrow instead of truncating corrupt mirror rows.
fn exact_u128(value: &BigDecimal) -> Option<u128> {
    let amount = parse_u128(value)?;
    (value == &BigDecimal::from(amount)).then_some(amount)
}

/// Decode exactly the 18-place, Uint128-backed price accepted by the pair contract.
fn price_atomics(price: &BigDecimal) -> Option<u128> {
    let atomics = exact_u128(&(price * BigDecimal::from(DECIMAL_SCALE)))?;
    (atomics > 0).then_some(atomics)
}

fn mul_floor_atomics(amount: u128, atomics: u128) -> Option<u128> {
    let product = BigInt::from(amount) * BigInt::from(atomics);
    (product / BigInt::from(DECIMAL_SCALE))
        .to_string()
        .parse()
        .ok()
}

fn inverse_atomics(atomics: u128) -> Option<u128> {
    if atomics == 0 {
        return None;
    }
    let inverse = (BigInt::from(DECIMAL_SCALE) * BigInt::from(DECIMAL_SCALE)
        / BigInt::from(atomics))
    .to_string()
    .parse::<u128>()
    .ok()?;
    (inverse > 0).then_some(inverse)
}

/// #1399: counterpart of `dex_common::limit_placement::is_limit_order_fillable`.
/// Use the same truncated Decimal inverse as execute/sim, rather than BigDecimal division.
/// Full ask notional may exceed Uint128 while a payable partial fill remains possible.
fn economic_order_terms(order: &resting_orders::RestingOrderRow) -> Option<(u128, u128, u128)> {
    let remaining = exact_u128(&order.remaining)?;
    if remaining == 0 {
        return None;
    }
    let price = price_atomics(&order.price)?;
    let inverse = inverse_atomics(price)?;
    let fillable = match order.side.as_str() {
        "bid" => {
            let capacity = mul_floor_atomics(remaining, inverse)?;
            let cost = mul_floor_atomics(capacity, price)?;
            capacity > 0 && cost > 0 && cost <= remaining
        }
        "ask" => BigInt::from(remaining) * BigInt::from(price) >= BigInt::from(DECIMAL_SCALE),
        _ => false,
    };
    fillable.then_some((remaining, price, inverse))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorFreshness {
    Fresh,
    MissingReserves,
    Stale,
    /// Indexed reserves exist but at least one side is zero (unfunded pair).
    EmptyPool,
}

/// True when the mirror row has no constant-product liquidity on either side.
pub fn mirror_pool_is_empty(mirror: &HopMirror) -> bool {
    mirror.reserve_0 == 0 || mirror.reserve_1 == 0
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
    /// Pool-leg sim is not a ranking winner: ask out > ask reserve, or 18-dec offer
    /// scale-mismatched into a 6-dec pool and ~100% drain (#1257).
    #[error("implausible hop simulation")]
    ImplausibleHop,
    #[error("{0}")]
    Db(#[from] sqlx::Error),
}

pub fn snapshot_age_ms(at: DateTime<Utc>) -> u64 {
    let now = Utc::now();
    // A future timestamp is impossible evidence of freshness (clock skew or poison).
    if at > now {
        return u64::MAX;
    }
    now.signed_duration_since(at).num_milliseconds() as u64
}

pub fn is_snapshot_stale(at: DateTime<Utc>, max_staleness_ms: u64) -> bool {
    snapshot_age_ms(at) > max_staleness_ms
}

/// Load mirrored reserves + resting book for one hop pair contract.
/// Retain every row for scan-cap parity; expiry is checked at quote time rather than load time.
/// `_now_secs` is kept for caller compatibility with the earlier filtering API.
pub async fn load_hop_mirror(
    pool: &PgPool,
    _pair_address: &str,
    asset_addrs: &HashMap<i32, String>,
    pair_row: &pairs::PairRow,
    max_staleness_ms: u64,
    _now_secs: u64,
) -> Result<HopMirror, DbSimError> {
    let asset_0_addr = asset_addrs
        .get(&pair_row.asset_0_id)
        .cloned()
        .ok_or(DbSimError::PairNotFound)?;
    let asset_1_addr = asset_addrs
        .get(&pair_row.asset_1_id)
        .cloned()
        .ok_or(DbSimError::PairNotFound)?;

    // Compatible reserves and resting book are written in one transaction (#1453); read them under
    // REPEATABLE READ so a concurrent snapshot cannot mix rows from different commits.
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;

    let reserves = pair_reserves::get_hybrid_pair_reserves(&mut *tx, pair_row.id)
        .await?
        .ok_or(DbSimError::MissingMirror)?;

    if !(0..=10_000).contains(&reserves.fee_bps) {
        return Err(DbSimError::InvalidNumeric);
    }
    let reserve_0 = parse_u128(&reserves.reserve_0).ok_or(DbSimError::InvalidNumeric)?;
    let reserve_1 = parse_u128(&reserves.reserve_1).ok_or(DbSimError::InvalidNumeric)?;

    let stale = is_snapshot_stale(reserves.snapshot_at, max_staleness_ms);
    let freshness = if reserves.block_height.is_none() {
        MirrorFreshness::MissingReserves
    } else if reserve_0 == 0 || reserve_1 == 0 {
        MirrorFreshness::EmptyPool
    } else if stale {
        MirrorFreshness::Stale
    } else if pool_reserves_unusable(reserve_0, reserve_1) {
        MirrorFreshness::MissingReserves
    } else {
        MirrorFreshness::Fresh
    };

    let bids = resting_orders::get_pair_resting_book(&mut *tx, pair_row.id, "bid").await?;
    let asks = resting_orders::get_pair_resting_book(&mut *tx, pair_row.id, "ask").await?;
    tx.commit().await?;

    // Keep expired, zero and economically unfillable rows in DLL order. Contract walks
    // count these against MAX_SCAN_STEPS; pre-filtering would quote unreachable makers.
    Ok(HopMirror {
        pair_id: pair_row.id,
        asset_0_addr,
        asset_1_addr,
        reserve_0,
        reserve_1,
        fee_bps: reserves.fee_bps as u16,
        block_height: reserves.block_height,
        snapshot_at: reserves.snapshot_at,
        freshness,
        bids,
        asks,
    })
}

fn order_is_expired(order: &resting_orders::RestingOrderRow, now_secs: u64) -> bool {
    order
        .expires_at
        .is_some_and(|expiry| expiry <= 0 || now_secs >= expiry as u64)
}

/// Expected matcher side for a taker offering `offer_token` on this pair mirror.
fn match_side_for_offer(mirror: &HopMirror, offer_token: &str) -> &'static str {
    if offer_token.eq_ignore_ascii_case(&mirror.asset_0_addr) {
        "bid"
    } else {
        "ask"
    }
}

/// First live order id on the taker's match side when the mirror snapshot is fresh (GitLab #332).
/// Omits hint for stale/missing mirrors or when no live liquidity exists on that side.
pub fn first_live_book_start_hint(mirror: &HopMirror, offer_token: &str) -> Option<u64> {
    if mirror.freshness != MirrorFreshness::Fresh {
        return None;
    }
    let expected_side = match_side_for_offer(mirror, offer_token);
    let orders = if expected_side == "bid" {
        &mirror.bids
    } else {
        &mirror.asks
    };
    let now_secs = Utc::now().timestamp().max(0) as u64;
    orders
        .iter()
        .find(|o| {
            o.side == expected_side
                && !order_is_expired(o, now_secs)
                && parse_u128(&o.remaining).unwrap_or(0) > 0
        })
        .map(|o| o.order_id as u64)
}

/// Fresh mirror with at least one fillable resting order on the taker match side (GitLab #493).
/// Used to short-circuit the 17-point hybrid grid when the book cannot improve on pool-only.
pub fn mirror_has_fillable_book(mirror: &HopMirror, offer_token: &str) -> bool {
    if mirror.freshness != MirrorFreshness::Fresh {
        return false;
    }
    let side = match_side_for_offer(mirror, offer_token);
    let orders = if side == "bid" {
        &mirror.bids
    } else {
        &mirror.asks
    };
    let Some(hint) = first_live_book_start_hint(mirror, offer_token) else {
        return false;
    };
    let orders = book_orders_from_hint(orders, Some(hint), side);
    let now_secs = Utc::now().timestamp().max(0) as u64;
    orders.iter().take(MAX_SCAN_STEPS as usize).any(|order| {
        order.side == side
            && !order_is_expired(order, now_secs)
            && economic_order_terms(order).is_some()
    })
}

/// Slice book walk order list from a validated same-side hint, else from head (contract parity).
fn book_orders_from_hint<'a>(
    orders: &'a [resting_orders::RestingOrderRow],
    book_start_hint: Option<u64>,
    expected_side: &str,
) -> &'a [resting_orders::RestingOrderRow] {
    if let Some(hint) = book_start_hint {
        if let Some(pos) = orders.iter().position(|o| {
            o.order_id as u64 == hint
                && o.side == expected_side
                && parse_u128(&o.remaining).unwrap_or(0) > 0
        }) {
            return &orders[pos..];
        }
    }
    orders
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
    book_start_hint: Option<u64>,
) -> Result<BookSimResult, DbSimError> {
    let orders = book_orders_from_hint(orders, book_start_hint, "bid");
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token0_left = token0_budget;
    let mut token1_out = 0u128;
    let mut makers_used = 0u32;
    let mut scan_steps = 0u32;
    let now_secs = Utc::now().timestamp().max(0) as u64;

    for order in orders {
        if !book_walk_step(&mut scan_steps) {
            break;
        }
        if makers_used >= cap || token0_left == 0 {
            break;
        }
        if order.side != "bid" || order_is_expired(order, now_secs) {
            continue;
        }
        let Some((remaining, price, inverse)) = economic_order_terms(order) else {
            continue;
        };
        let Some(max_fill_from_bid) = mul_floor_atomics(remaining, inverse) else {
            continue;
        };
        let mut fill = token0_left.min(max_fill_from_bid);
        if fill == 0 {
            continue;
        }
        let mut cost = mul_floor_atomics(fill, price).unwrap_or(0);
        while fill > 0 && cost > remaining {
            fill = fill.saturating_sub(1);
            if fill == 0 {
                break;
            }
            cost = mul_floor_atomics(fill, price).unwrap_or(0);
        }
        if fill == 0 {
            continue;
        }
        if cost == 0 {
            continue;
        }
        makers_used += 1;
        let commission = cost
            .checked_mul(taker_bps as u128)
            .ok_or(DbSimError::InvalidNumeric)?
            / 10_000;
        let net = cost.saturating_sub(commission);
        token0_left = token0_left.saturating_sub(fill);
        token1_out = token1_out
            .checked_add(net)
            .ok_or(DbSimError::InvalidNumeric)?;
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
    book_start_hint: Option<u64>,
) -> Result<BookSimResult, DbSimError> {
    let orders = book_orders_from_hint(orders, book_start_hint, "ask");
    let cap = max_maker_fills.min(MAX_MAKER_FILLS_HARD_CAP);
    let taker_bps = taker_fee_bps(effective_fee_bps);
    let mut token1_left = token1_budget;
    let mut token0_out = 0u128;
    let mut makers_used = 0u32;
    let mut scan_steps = 0u32;
    let now_secs = Utc::now().timestamp().max(0) as u64;

    for order in orders {
        if !book_walk_step(&mut scan_steps) {
            break;
        }
        if makers_used >= cap || token1_left == 0 {
            break;
        }
        if order.side != "ask" || order_is_expired(order, now_secs) {
            continue;
        }
        let Some((remaining, price, inverse)) = economic_order_terms(order) else {
            continue;
        };
        let Some(max_fill_from_budget) = mul_floor_atomics(token1_left, inverse) else {
            continue;
        };
        let mut fill_t0 = remaining.min(max_fill_from_budget);
        if fill_t0 == 0 {
            continue;
        }
        let mut cost = mul_floor_atomics(fill_t0, price).unwrap_or(0);
        while fill_t0 > 0 && cost > token1_left {
            fill_t0 = fill_t0.saturating_sub(1);
            if fill_t0 == 0 {
                break;
            }
            cost = mul_floor_atomics(fill_t0, price).unwrap_or(0);
        }
        if fill_t0 == 0 {
            continue;
        }
        if cost == 0 {
            continue;
        }
        makers_used += 1;
        let commission = fill_t0
            .checked_mul(taker_bps as u128)
            .ok_or(DbSimError::InvalidNumeric)?
            / 10_000;
        let net = fill_t0.saturating_sub(commission);
        token1_left = token1_left.saturating_sub(cost);
        token0_out = token0_out
            .checked_add(net)
            .ok_or(DbSimError::InvalidNumeric)?;
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

/// Drop a hop whose pool sim cannot be the economic output of this offer (#1257).
///
/// - Ask out greater than that hop's ask reserve (always a bug).
/// - Offer ≥ 1000× the offer-side reserve **and** ≥99% of the ask reserve is taken
///   (18-dec raw into a 6-dec pool). Retail sizes that fit in the pool still quote;
///   99% chrome on the dApp covers honest whale impact.
pub fn hop_sim_implausible(
    offer_amount: u128,
    input_reserve: u128,
    output_reserve: u128,
    ask_out: u128,
) -> bool {
    if output_reserve == 0 {
        return ask_out > 0;
    }
    if ask_out > output_reserve {
        return true;
    }
    let scale_mismatch = input_reserve > 0 && offer_amount > input_reserve.saturating_mul(1_000);
    let full_drain = hop_near_full_ask_drain(ask_out, output_reserve);
    scale_mismatch && full_drain
}

/// Same-scale ~100% pool impact: hop takes ≥99% of the ask reserve (#1218 top-K skip).
pub fn hop_near_full_ask_drain(ask_out: u128, output_reserve: u128) -> bool {
    output_reserve > 0 && ask_out.saturating_mul(100) >= output_reserve.saturating_mul(99)
}

/// Cheap pool-only walk used while filling top-K. Skip ~100% spread / implausible hops
/// so a thin 2-hop cannot occupy the slot that would have scored an honest 3-hop (#1218).
/// Missing/stale mirrors are **not** skipped here (evaluate_candidate owns LCD fallback).
pub fn path_unusable_for_top_k(
    mirrors: &HashMap<String, HopMirror>,
    hops: &[(&str, &str)],
    amount_in: u128,
) -> bool {
    if amount_in == 0 || hops.is_empty() {
        return true;
    }
    let mut offer = amount_in;
    for (pair, offer_token) in hops {
        let Some(mirror) = mirrors.get(*pair) else {
            return false;
        };
        match simulate_pool_only_from_mirror(mirror, offer_token, offer, 0) {
            Ok(ask_out) => {
                let offer_is_token0 = offer_token.eq_ignore_ascii_case(&mirror.asset_0_addr);
                let output_reserve = if offer_is_token0 {
                    mirror.reserve_1
                } else {
                    mirror.reserve_0
                };
                let input_reserve = if offer_is_token0 {
                    mirror.reserve_0
                } else {
                    mirror.reserve_1
                };
                if ask_out == 0
                    || hop_sim_implausible(offer, input_reserve, output_reserve, ask_out)
                    || hop_near_full_ask_drain(ask_out, output_reserve)
                {
                    return true;
                }
                offer = ask_out;
            }
            Err(DbSimError::ImplausibleHop)
            | Err(DbSimError::InsufficientLiquidity)
            | Err(DbSimError::MissingMirror) => return true,
            Err(_) => return false,
        }
    }
    false
}

/// Constant-product pool leg with **wide** `k` (pair Uint256 / GitLab #464 analog).
/// `saturating_mul` on mixed 18/6 reserves can print a near-full drain; that is not a quote.
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
    let new_in = input_reserve
        .checked_add(pool_input)
        .ok_or(DbSimError::InvalidNumeric)?;
    if new_in == 0 {
        return Err(DbSimError::InvalidNumeric);
    }
    let new_out = match input_reserve.checked_mul(output_reserve) {
        Some(k) => ceil_div(k, new_in),
        None => ceil_div_wide(input_reserve, output_reserve, new_in)?,
    };
    let gross = output_reserve.saturating_sub(new_out);
    if gross > output_reserve {
        return Err(DbSimError::ImplausibleHop);
    }
    let fee = swap_fee_amount(gross, fee_bps);
    Ok(gross.saturating_sub(fee))
}

/// `ceil((in * out) / new_in)` when `in * out` does not fit in `u128`.
fn ceil_div_wide(
    input_reserve: u128,
    output_reserve: u128,
    new_in: u128,
) -> Result<u128, DbSimError> {
    if new_in == 0 {
        return Err(DbSimError::InvalidNumeric);
    }
    let k = BigDecimal::from(input_reserve) * BigDecimal::from(output_reserve);
    let den = BigDecimal::from(new_in);
    let numer = k + den.clone() - BigDecimal::from(1u32);
    let ceil = numer / den;
    parse_u128(&ceil).ok_or(DbSimError::InvalidNumeric)
}

/// Hybrid simulation from mirrored Postgres state (forward offer → ask output).
pub fn simulate_hybrid_from_mirror(
    mirror: &HopMirror,
    offer_token: &str,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
    discount_bps: u16,
    book_start_hint: Option<u64>,
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
            MirrorFreshness::EmptyPool => DbSimError::InsufficientLiquidity,
            MirrorFreshness::Fresh => unreachable!(),
        });
    }

    let eff_fee = effective_fee_bps_from_discount_bps(mirror.fee_bps, discount_bps);
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
            simulate_match_bids(
                &mirror.bids,
                book_input,
                max_maker_fills,
                eff_fee,
                book_start_hint,
            )?
        } else {
            simulate_match_asks(
                &mirror.asks,
                book_input,
                max_maker_fills,
                eff_fee,
                book_start_hint,
            )?
        };
        book_return = book_sim.return_net;
        offer_consumed_by_book = book_sim.offer_consumed;
    }

    let pool_input_amount =
        pool_input.saturating_add(book_input.saturating_sub(offer_consumed_by_book));
    let pool_out = simulate_pool_leg(input_reserve, output_reserve, pool_input_amount, eff_fee)?;
    if hop_sim_implausible(offer_amount, input_reserve, output_reserve, pool_out) {
        return Err(DbSimError::ImplausibleHop);
    }
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

/// Bounded pool-only mirror loader (#1376). Never loads resting orders: exact-output
/// sizing excludes books, so loading an entire per-pair book would waste unbounded memory.
pub async fn preload_pool_mirrors_for_pairs(
    pool: &PgPool,
    pair_addresses: &[String],
    id_to_addr: &HashMap<i32, String>,
    max_staleness_ms: u64,
) -> Result<HashMap<String, HopMirror>, sqlx::Error> {
    let mut out = HashMap::new();
    for addr in pair_addresses {
        let Some(pair) = pairs::get_pair_by_address(pool, addr).await? else {
            continue;
        };
        let Some(reserves) = pair_reserves::get_pair_reserves(pool, pair.id).await? else {
            continue;
        };
        let (Some(reserve_0), Some(reserve_1)) = (
            parse_u128(&reserves.reserve_0),
            parse_u128(&reserves.reserve_1),
        ) else {
            continue;
        };
        let (Some(asset_0_addr), Some(asset_1_addr)) = (
            id_to_addr.get(&pair.asset_0_id),
            id_to_addr.get(&pair.asset_1_id),
        ) else {
            continue;
        };
        // Both reserves, fee, height and timestamp are one atomic snapshot row.
        let freshness = if pool_reserves_unusable(reserve_0, reserve_1) {
            MirrorFreshness::EmptyPool
        } else if is_snapshot_stale(reserves.snapshot_at, max_staleness_ms) {
            MirrorFreshness::Stale
        } else {
            MirrorFreshness::Fresh
        };
        if !(0..=10_000).contains(&reserves.fee_bps) {
            continue;
        }
        out.insert(
            addr.clone(),
            HopMirror {
                pair_id: pair.id,
                asset_0_addr: asset_0_addr.clone(),
                asset_1_addr: asset_1_addr.clone(),
                reserve_0,
                reserve_1,
                fee_bps: reserves.fee_bps as u16,
                block_height: reserves.block_height,
                snapshot_at: reserves.snapshot_at,
                freshness,
                bids: vec![],
                asks: vec![],
            },
        );
    }
    Ok(out)
}

pub fn simulate_pool_only_from_mirror(
    mirror: &HopMirror,
    offer_token: &str,
    offer_amount: u128,
    discount_bps: u16,
) -> Result<u128, DbSimError> {
    simulate_hybrid_from_mirror(
        mirror,
        offer_token,
        offer_amount,
        offer_amount,
        0,
        1,
        discount_bps,
        None,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn future_snapshot_is_stale_instead_of_age_zero() {
        let future = chrono::Utc::now() + chrono::Duration::minutes(1);
        assert_eq!(super::snapshot_age_ms(future), u64::MAX);
        assert!(super::is_snapshot_stale(future, 20_000));
    }

    use super::*;
    use std::collections::HashMap;
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
    fn economic_fillability_decimal_boundaries_match_shared_predicate() {
        // Raw prices representing supported 6/6, 6/18 and 18/6 combinations.
        for (price, minimum_token0, minimum_token1) in [
            ("0.000000000001", 1_000_000_000_000u128, 1u128),
            ("1", 1, 1),
            ("0.000000000000000001", 1_000_000_000_000_000_000, 1),
            ("1000000000000", 1, 1_000_000_000_000),
        ] {
            let below = resting(1, "ask", price, &(minimum_token0 - 1).to_string());
            let exact = resting(2, "ask", price, &minimum_token0.to_string());
            let above = resting(3, "ask", price, &(minimum_token0 + 1).to_string());
            assert!(economic_order_terms(&below).is_none(), "price={price}");
            assert!(economic_order_terms(&exact).is_some(), "price={price}");
            assert!(economic_order_terms(&above).is_some(), "price={price}");
            let sim = simulate_match_asks(&[exact], minimum_token1, 1, 0, None).unwrap();
            assert_eq!(sim.return_net, minimum_token0);
            assert_eq!(sim.offer_consumed, minimum_token1);

            // The opposite side uses the same raw price and payment threshold.
            let below = resting(4, "bid", price, &(minimum_token1 - 1).to_string());
            let bid = resting(5, "bid", price, &minimum_token1.to_string());
            let above = resting(6, "bid", price, &(minimum_token1 + 1).to_string());
            assert!(economic_order_terms(&below).is_none());
            assert!(economic_order_terms(&bid).is_some());
            assert!(economic_order_terms(&above).is_some());
            let sim = simulate_match_bids(&[bid], minimum_token0, 1, 0, None).unwrap();
            assert_eq!(sim.return_net, minimum_token1);
            assert_eq!(sim.offer_consumed, minimum_token0);
        }
        // Bid capacity must produce a positive cost, including the inverse's truncation.
        for (price, below, exact, above) in [("2", "1", "2", "3"), ("3", "3", "4", "5")] {
            assert!(economic_order_terms(&resting(1, "bid", price, below)).is_none());
            assert!(economic_order_terms(&resting(2, "bid", price, exact)).is_some());
            assert!(economic_order_terms(&resting(3, "bid", price, above)).is_some());
        }
    }

    #[test]
    fn decimal_inverse_truncation_matches_contract_not_exact_division() {
        // Decimal(1)/Decimal(3) truncates to 0.333333333333333333. A budget of 3
        // buys floor(3 * inverse)=0, whereas arbitrary-precision direct division buys 1.
        let ask = resting(1, "ask", "3", "100");
        let below = simulate_match_asks(&[ask.clone()], 3, 1, 0, None).unwrap();
        assert_eq!(below.return_net, 0);
        assert_eq!(below.offer_consumed, 0);
        let payable = simulate_match_asks(&[ask], 4, 1, 0, None).unwrap();
        assert_eq!(payable.return_net, 1);
        assert_eq!(payable.offer_consumed, 3);
    }

    #[test]
    fn economic_fillability_checked_overflow_and_corrupt_rows() {
        let huge = u128::MAX.to_string();
        // A huge ask still supports partial fills; full notional need not fit Uint128.
        let ask = resting(1, "ask", "2", &huge);
        assert!(economic_order_terms(&ask).is_some());
        let sim = simulate_match_asks(&[ask], 4, 1, 0, None).unwrap();
        assert_eq!(sim.return_net, 2);
        assert_eq!(sim.offer_consumed, 4);
        // The bid walker computes full remaining / price before the taker min: overflow
        // of that capacity is unfillable, and must not create a saturated quote.
        assert!(economic_order_terms(&resting(2, "bid", "0.5", &huge)).is_none());
        for side in ["bid", "ask"] {
            for (price, remaining) in [
                ("0", "100"),
                ("-1", "100"),
                ("0.0000000000000000001", "100"), // Below Decimal precision.
                ("340282366920938463464", "100"), // Price atomics overflow.
                ("1000000000000000001", "100"),   // Inverse truncates to zero.
                ("1", "340282366920938463463374607431768211456"),
                ("1", "1.5"),
                ("1", "-1"),
            ] {
                assert!(
                    economic_order_terms(&resting(3, side, price, remaining)).is_none(),
                    "side={side} price={price} remaining={remaining}"
                );
            }
        }
    }

    #[test]
    fn invalid_prefix_consumes_scan_cap_but_not_maker_cap_on_both_sides() {
        assert_eq!(MAX_SCAN_STEPS, 500); // dex_common::pair::MAX_SCAN_STEPS.
        for side in ["bid", "ask"] {
            let bad_price = if side == "bid" { "2" } else { "0.000000000001" };
            let mut orders: Vec<_> = (0..MAX_SCAN_STEPS + 1)
                .map(|id| resting(id as i64 + 1, side, bad_price, "1"))
                .collect();
            orders.push(resting(MAX_SCAN_STEPS as i64 + 2, side, "1", "100"));
            let simulate = |orders: &[resting_orders::RestingOrderRow], hint| {
                if side == "bid" {
                    simulate_match_bids(orders, 10, 1, 0, hint).unwrap()
                } else {
                    simulate_match_asks(orders, 10, 1, 0, hint).unwrap()
                }
            };
            assert_eq!(simulate(&orders, None).return_net, 0);
            // Cleanup makes progress without removing the scan ceiling; two removals
            // make the valid maker reachable at step 500 of the next snapshot.
            assert_eq!(simulate(&orders[1..], None).return_net, 0);
            assert_eq!(simulate(&orders[2..], None).return_net, 10);
            // Hints retain their same-side/nonzero-remaining semantics, including invalid
            // anchors: the first bad anchor does not remove the scan ceiling.
            assert_eq!(simulate(&orders, Some(1)).return_net, 0);
            assert_eq!(
                simulate(&orders, Some(MAX_SCAN_STEPS as u64 + 2)).return_net,
                10
            );
        }
    }

    #[test]
    fn expired_and_zero_prefix_rows_keep_scan_positions() {
        for side in ["bid", "ask"] {
            let mut orders: Vec<_> = (0..MAX_SCAN_STEPS)
                .map(|id| {
                    let mut order = resting(
                        id as i64 + 1,
                        side,
                        "1",
                        if id % 2 == 0 { "0" } else { "100" },
                    );
                    if id % 2 != 0 {
                        order.expires_at = Some(1);
                    }
                    order
                })
                .collect();
            orders.push(resting(MAX_SCAN_STEPS as i64 + 1, side, "1", "100"));
            let sim = if side == "bid" {
                simulate_match_bids(&orders, 10, 1, 0, None)
            } else {
                simulate_match_asks(&orders, 10, 1, 0, None)
            }
            .unwrap();
            assert_eq!(sim.return_net, 0);
            assert_eq!(sim.offer_consumed, 0);
        }
    }

    #[test]
    fn invalid_rows_do_not_consume_makers_or_allow_zero_payout() {
        for side in ["bid", "ask"] {
            let rows = [resting(1, side, "1", "1.5"), resting(2, side, "1", "100")];
            let sim = if side == "bid" {
                simulate_match_bids(&rows, 10, 1, 0, None)
            } else {
                simulate_match_asks(&rows, 10, 1, 0, None)
            }
            .unwrap();
            assert_eq!(sim.return_net, 10);
            assert_eq!(sim.offer_consumed, 10);
        }
        // Globally fillable bid with an insufficient taker budget stays unfilled.
        let bid = resting(1, "bid", "0.000000000001", "1000");
        assert!(economic_order_terms(&bid).is_some());
        let sim = simulate_match_bids(&[bid], 1, 1, 0, None).unwrap();
        assert_eq!(sim.return_net, 0);
        assert_eq!(sim.offer_consumed, 0);
    }

    #[test]
    fn fillable_book_probe_ignores_economically_unfillable_rows() {
        let mirror = HopMirror {
            bids: vec![resting(1, "bid", "2", "1")],
            asks: vec![resting(2, "ask", "0.000000000001", "100")],
            ..mirror_with_book(vec![])
        };
        assert!(!mirror_has_fillable_book(&mirror, "terra1token0"));
        assert!(!mirror_has_fillable_book(&mirror, "terra1token1"));
        // A generated anchor is still a live same-side order, matching contract hint rules.
        assert_eq!(first_live_book_start_hint(&mirror, "terra1token0"), Some(1));
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
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 100_000, 0, 8, 0, None)
                .unwrap();
        let hybrid =
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 50_000, 50_000, 8, 0, None)
                .unwrap();
        assert!(hybrid >= pool_only);
    }

    #[test]
    fn first_live_book_start_hint_picks_match_side() {
        let m = HopMirror {
            bids: vec![resting(10, "bid", "2", "1000")],
            asks: vec![resting(20, "ask", "3", "1000")],
            ..mirror_with_book(vec![])
        };
        assert_eq!(first_live_book_start_hint(&m, "terra1token0"), Some(10));
        assert_eq!(first_live_book_start_hint(&m, "terra1token1"), Some(20));
    }

    #[test]
    fn mirror_has_fillable_book_empty_vs_live() {
        let empty = mirror_with_book(vec![]);
        assert!(!mirror_has_fillable_book(&empty, "terra1token0"));
        assert!(!mirror_has_fillable_book(&empty, "terra1token1"));

        let with_bid = mirror_with_book(vec![resting(1, "bid", "2", "1000")]);
        assert!(mirror_has_fillable_book(&with_bid, "terra1token0"));
        assert!(!mirror_has_fillable_book(&with_bid, "terra1token1"));
    }

    #[test]
    fn first_live_book_start_hint_omits_wrong_side_in_bid_list() {
        // Corrupt mirror row on wrong side in bids vec — skip when resolving hint.
        let m = mirror_with_book(vec![
            resting(99, "ask", "2", "1000"),
            resting(10, "bid", "2", "1000"),
        ]);
        assert_eq!(first_live_book_start_hint(&m, "terra1token0"), Some(10));
    }

    #[test]
    fn book_start_hint_skips_orders_before_hint() {
        let m = mirror_with_book(vec![
            resting(1, "bid", "1", "100"),
            resting(2, "bid", "2", "10000"),
        ]);
        let head_only =
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 0, 100_000, 8, 0, None)
                .unwrap();
        let from_hint =
            simulate_hybrid_from_mirror(&m, "terra1token0", 100_000, 0, 100_000, 8, 0, Some(2))
                .unwrap();
        assert!(from_hint > head_only);
    }

    #[test]
    fn pool_reserves_unusable_detects_empty_pool() {
        assert!(pool_reserves_unusable(0, 0));
        assert!(pool_reserves_unusable(0, 100));
        assert!(pool_reserves_unusable(100, 0));
        assert!(!pool_reserves_unusable(100, 100));
    }

    #[test]
    fn zero_reserve_mirror_returns_no_output() {
        let m = HopMirror {
            reserve_0: 0,
            reserve_1: 0,
            freshness: MirrorFreshness::EmptyPool,
            ..mirror_with_book(vec![])
        };
        let out = simulate_pool_only_from_mirror(&m, "terra1token0", 1_000_000, 0).unwrap_err();
        assert!(matches!(out, DbSimError::InsufficientLiquidity));
    }

    #[test]
    fn effective_fee_tier_reduces_fee_increases_out() {
        let m = mirror_with_book(vec![]);
        let full = simulate_pool_only_from_mirror(&m, "terra1token0", 1_000_000, 0).unwrap();
        let disc = simulate_pool_only_from_mirror(&m, "terra1token0", 1_000_000, 5_000).unwrap();
        assert!(disc > full);
    }

    fn mixed_18_6_mirror() -> HopMirror {
        HopMirror {
            pair_id: 1,
            asset_0_addr: "terra1ustr18dec000000000000000000000000000".into(),
            asset_1_addr: "terra1ust16dec00000000000000000000000000000".into(),
            reserve_0: 1_000 * 10u128.pow(18),
            reserve_1: 1_000 * 10u128.pow(6),
            fee_bps: 30,
            block_height: Some(1),
            snapshot_at: Utc::now(),
            freshness: MirrorFreshness::Fresh,
            bids: vec![],
            asks: vec![],
        }
    }

    #[test]
    fn mixed_18_to_6_hop_offer_1e18_returns_6dec_scale() {
        let m = mixed_18_6_mirror();
        let out = simulate_pool_only_from_mirror(
            &m,
            "terra1ustr18dec000000000000000000000000000",
            10u128.pow(18),
            0,
        )
        .unwrap();
        assert!(out > 900_000, "expected ~1 human UST1 (1e6 raw), got {out}");
        assert!(
            out < 1_100_000,
            "must not treat 10^18 offer as 18-dec UST1 out, got {out}"
        );
    }

    #[test]
    fn mixed_6_to_18_last_hop_returns_18dec_usdt_raw() {
        let m = HopMirror {
            asset_0_addr: "terra1clunc6dec000000000000000000000000000".into(),
            asset_1_addr: "terra1usdt18dec000000000000000000000000000".into(),
            reserve_0: 1_000 * 10u128.pow(6),
            reserve_1: 1_000 * 10u128.pow(18),
            ..mixed_18_6_mirror()
        };
        let out = simulate_pool_only_from_mirror(
            &m,
            "terra1clunc6dec000000000000000000000000000",
            10u128.pow(6),
            0,
        )
        .unwrap();
        assert!(
            out > 9 * 10u128.pow(17),
            "expected ~1 human USDT (1e18 raw), got {out}"
        );
        assert!(out < 11 * 10u128.pow(17));
    }

    #[test]
    fn scale_mismatch_18dec_offer_into_6dec_pool_is_implausible() {
        let m = HopMirror {
            asset_0_addr: "terra1clunc6dec000000000000000000000000000".into(),
            asset_1_addr: "terra1usdt18dec000000000000000000000000000".into(),
            reserve_0: 1_000 * 10u128.pow(6),
            reserve_1: 1_000 * 10u128.pow(18),
            ..mixed_18_6_mirror()
        };
        let err = simulate_pool_only_from_mirror(
            &m,
            "terra1clunc6dec000000000000000000000000000",
            10u128.pow(18),
            0,
        )
        .unwrap_err();
        assert!(matches!(err, DbSimError::ImplausibleHop));
    }

    #[test]
    fn hop_sim_implausible_ask_out_gt_reserve() {
        assert!(hop_sim_implausible(1_000, 1_000_000, 500, 501));
        assert!(!hop_sim_implausible(1_000, 1_000_000, 500, 400));
    }

    #[test]
    fn hop_near_full_ask_drain_same_scale() {
        assert!(hop_near_full_ask_drain(990, 1_000));
        assert!(!hop_near_full_ask_drain(50, 1_000));
        assert!(!hop_near_full_ask_drain(1, 0));
    }

    #[test]
    fn path_unusable_for_top_k_skips_full_drain_hop() {
        // offer ≈ 99× input reserve so constant-product takes ≥99% of the thin ask.
        let drain = HopMirror {
            reserve_0: 100,
            reserve_1: 2_000,
            ..mirror_with_book(vec![])
        };
        let mut mirrors = HashMap::new();
        mirrors.insert("terra1drainpair".into(), drain);
        let hops = [("terra1drainpair", "terra1token0")];
        assert!(path_unusable_for_top_k(&mirrors, &hops, 1_000_000));
    }

    #[test]
    fn path_unusable_for_top_k_skips_zero_out_thin_ask() {
        // Deep offer reserve + tiny ask: ceil_div can print 0 out (not 99% drain).
        let thin = HopMirror {
            reserve_0: 10_000_000_000,
            reserve_1: 2_000,
            ..mirror_with_book(vec![])
        };
        let mut mirrors = HashMap::new();
        mirrors.insert("terra1thinpair".into(), thin);
        let hops = [("terra1thinpair", "terra1token0")];
        assert!(path_unusable_for_top_k(&mirrors, &hops, 1_000_000));
    }

    #[test]
    fn path_unusable_for_top_k_keeps_deep_pool() {
        let deep = HopMirror {
            reserve_0: 10_000_000_000_000,
            reserve_1: 10_000_000_000_000,
            ..mirror_with_book(vec![])
        };
        let mut mirrors = HashMap::new();
        mirrors.insert("terra1deeppair".into(), deep);
        let hops = [("terra1deeppair", "terra1token0")];
        assert!(!path_unusable_for_top_k(&mirrors, &hops, 1_000_000));
    }

    #[test]
    fn wide_k_18dec_reserves_do_not_print_full_drain() {
        let m = HopMirror {
            reserve_0: 10u128.pow(27),
            reserve_1: 10u128.pow(27),
            ..mirror_with_book(vec![])
        };
        let out = simulate_pool_only_from_mirror(&m, "terra1token0", 10u128.pow(18), 0).unwrap();
        assert!(out > 0);
        assert!(
            out < 10u128.pow(27) / 100,
            "wide k must not drain ~100% of a 1e27 reserve on a 1e18 offer, got {out}"
        );
    }

    #[test]
    fn per_unit_size_curve_honest_pool_is_non_increasing() {
        let m = mixed_18_6_mirror();
        let sizes = [1u128, 100, 1_000, 10_000];
        let mut prev_per = u128::MAX;
        for human in sizes {
            let offer = human * 10u128.pow(18);
            let out = simulate_pool_only_from_mirror(
                &m,
                "terra1ustr18dec000000000000000000000000000",
                offer,
                0,
            )
            .unwrap();
            let per = out / human;
            assert!(
                per <= prev_per,
                "human {human}: per-unit {per} > previous {prev_per}"
            );
            prev_per = per;
        }
    }
}

/// Bounded, execution-aligned local hybrid leg details (#1452). Both pool and book
/// use the existing integer matcher; expired/dust rows retain scan-budget costs.
#[derive(Debug, Clone)]
pub struct LocalLegQuote {
    pub return_amount: u128,
    pub pool_return_amount: u128,
    pub commission_amount: u128,
    pub spread_amount: u128,
    pub book_return_net: u128,
    pub pool_input_actual: u128,
}
pub fn quote_local_leg(
    mirror: &HopMirror,
    offer_token: &str,
    offer: u128,
    book_input: u128,
    max_fills: u32,
) -> Result<LocalLegQuote, DbSimError> {
    if book_input > offer
        || mirror.fee_bps > 10_000
        || !matches!(
            mirror.freshness,
            MirrorFreshness::Fresh | MirrorFreshness::EmptyPool
        )
    {
        return Err(DbSimError::InvalidNumeric);
    }
    let (rin, rout) = if offer_token == mirror.asset_0_addr {
        (mirror.reserve_0, mirror.reserve_1)
    } else if offer_token == mirror.asset_1_addr {
        (mirror.reserve_1, mirror.reserve_0)
    } else {
        return Err(DbSimError::InvalidNumeric);
    };
    let book = if book_input == 0 {
        BookSimResult::default()
    } else if offer_token == mirror.asset_0_addr {
        simulate_match_bids(&mirror.bids, book_input, max_fills, mirror.fee_bps, None)?
    } else {
        simulate_match_asks(&mirror.asks, book_input, max_fills, mirror.fee_bps, None)?
    };
    let actual = offer
        .checked_sub(book.offer_consumed)
        .ok_or(DbSimError::InvalidNumeric)?;
    let gross = simulate_pool_leg(rin, rout, actual, 0)?;
    let commission = swap_fee_amount(gross, mirror.fee_bps);
    let pool_net = gross
        .checked_sub(commission)
        .ok_or(DbSimError::InvalidNumeric)?;
    // A fully filled pure-book swap never touches the AMM; the contract also
    // skips its reserve checks and pool-spread division in that case. Any book
    // remainder still passes through simulate_pool_leg and requires both reserves.
    let ideal = if actual == 0 {
        0
    } else {
        (BigInt::from(actual) * BigInt::from(rout) / BigInt::from(rin))
            .to_string()
            .parse::<u128>()
            .map_err(|_| DbSimError::InvalidNumeric)?
    };
    Ok(LocalLegQuote {
        return_amount: pool_net
            .checked_add(book.return_net)
            .ok_or(DbSimError::InvalidNumeric)?,
        pool_return_amount: pool_net,
        commission_amount: commission,
        spread_amount: ideal.saturating_sub(gross),
        book_return_net: book.return_net,
        pool_input_actual: actual,
    })
}

/// Loads only the maximum contract-reachable 500 rows per side, in original DLL
/// order, and reserves in the same database transaction. No request calls this.
pub async fn load_local_mirror(
    pool: &PgPool,
    pair: &pairs::PairRow,
    addresses: &HashMap<i32, String>,
    max_age_ms: u64,
) -> Result<HopMirror, DbSimError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    // Retain a compatible book generation at its original age when the independent
    // pool lane advances. A stale/missing hybrid falls back to current pool-only data.
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&mut *tx, pair.id).await?;
    let use_hybrid = hybrid
        .as_ref()
        .is_some_and(|r| !is_snapshot_stale(r.snapshot_at, max_age_ms));
    let r = if use_hybrid {
        hybrid.unwrap()
    } else {
        pair_reserves::get_pair_reserves(&mut *tx, pair.id)
            .await?
            .ok_or(DbSimError::MissingMirror)?
    };
    let reserve_0 = exact_u128(&r.reserve_0).ok_or(DbSimError::InvalidNumeric)?;
    let reserve_1 = exact_u128(&r.reserve_1).ok_or(DbSimError::InvalidNumeric)?;
    let fee_bps = u16::try_from(r.fee_bps)
        .ok()
        .filter(|b| *b <= 10_000)
        .ok_or(DbSimError::InvalidNumeric)?;
    // A book cannot be relabeled fresh merely because an independent reserve observation advanced.
    let book_heights: (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT MIN(block_height), MAX(block_height) FROM resting_limit_orders WHERE pair_id=$1",
    )
    .bind(pair.id)
    .fetch_one(&mut *tx)
    .await?;
    let mut bids=sqlx::query_as::<_,resting_orders::RestingOrderRow>("SELECT pair_id,order_id,side,price,remaining,owner,expires_at FROM resting_limit_orders WHERE pair_id=$1 AND side='bid' ORDER BY walk_index ASC LIMIT 500").bind(pair.id).fetch_all(&mut *tx).await?;
    let mut asks=sqlx::query_as::<_,resting_orders::RestingOrderRow>("SELECT pair_id,order_id,side,price,remaining,owner,expires_at FROM resting_limit_orders WHERE pair_id=$1 AND side='ask' ORDER BY walk_index ASC LIMIT 500").bind(pair.id).fetch_all(&mut *tx).await?;
    if !use_hybrid
        || (book_heights.0.is_some()
            && (book_heights.0 != r.block_height || book_heights.1 != r.block_height))
    {
        bids.clear();
        asks.clear();
    }
    tx.commit().await?;
    Ok(HopMirror {
        pair_id: pair.id,
        asset_0_addr: addresses
            .get(&pair.asset_0_id)
            .ok_or(DbSimError::InvalidNumeric)?
            .clone(),
        asset_1_addr: addresses
            .get(&pair.asset_1_id)
            .ok_or(DbSimError::InvalidNumeric)?
            .clone(),
        reserve_0,
        reserve_1,
        fee_bps,
        block_height: r.block_height,
        snapshot_at: r.snapshot_at,
        freshness: if reserve_0 == 0 || reserve_1 == 0 {
            MirrorFreshness::EmptyPool
        } else if is_snapshot_stale(r.snapshot_at, max_age_ms) {
            MirrorFreshness::Stale
        } else {
            MirrorFreshness::Fresh
        },
        bids,
        asks,
    })
}
