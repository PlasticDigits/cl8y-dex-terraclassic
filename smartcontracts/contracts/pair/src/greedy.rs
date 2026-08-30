//! Greedy book-first vs residual pool comparison (GitLab #708 / **G3**).
//!
//! **Beats (pinned):** a live maker is taken only when the taker's **net ask per 1 offer
//! (Decimal rate)** at that maker's executable price (after book taker commission) is **strictly
//! greater** than the **pool spot** `output_reserve / input_reserve` after pair `effective_fee_bps`.
//! Integer 1-raw-unit CP dumps floor to **0** on large reserves, which would make every priceable
//! maker "beat" the pool — rates avoid that artifact while matching the 1-unit *marginal* intent
//! (**G3**). Unpriceable makers (`price == 0`, no inverse) are **skipped** (**L18** / **L20**),
//! not treated as a stop.

use cosmwasm_std::{Decimal, Uint128, Uint256};
use dex_common::pair::GreedyStopReason;

use crate::error::ContractError;
use crate::orderbook::taker_fee_bps;

/// Pool snapshot used for the greedy 1-unit marginal (**G3**).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GreedyPoolRef {
    pub input_reserve: Uint128,
    pub output_reserve: Uint128,
    pub pool_fee_bps: u16,
}

/// Whether the next live maker should fill, stop the book, or be skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GreedyBeats {
    Yes,
    No,
    Skip,
}

fn u256(x: Uint128) -> Uint256 {
    Uint256::from(x)
}

fn ceil_div_u256(numerator: Uint256, denominator: Uint256) -> Uint256 {
    let d = numerator / denominator;
    if d * denominator < numerator {
        d + Uint256::one()
    } else {
        d
    }
}

fn try_price_inverse(price: Decimal) -> Option<Decimal> {
    Decimal::one().checked_div(price).ok()
}

/// Net ask from dumping `offer` into the constant-product pool (same rounding as execute).
pub fn constant_product_net_out(
    input_reserve: Uint128,
    output_reserve: Uint128,
    offer: Uint128,
    fee_bps: u16,
) -> Result<Uint128, ContractError> {
    if offer.is_zero() || input_reserve.is_zero() || output_reserve.is_zero() {
        return Ok(Uint128::zero());
    }
    let k = u256(input_reserve).checked_mul(u256(output_reserve))?;
    let new_input = input_reserve.checked_add(offer)?;
    let new_output = Uint128::try_from(ceil_div_u256(k, u256(new_input))).map_err(|_| {
        ContractError::InvariantViolation {
            reason: format!("greedy pool new_output exceeds u128: {k} / {new_input}"),
        }
    })?;
    let gross = output_reserve.saturating_sub(new_output);
    let fee_numerator = gross.checked_mul(Uint128::new(fee_bps as u128))?;
    let commission = fee_numerator.checked_div(Uint128::new(10000))?;
    Ok(gross.checked_sub(commission)?)
}

fn pool_spot_net(pool: &GreedyPoolRef) -> Option<Decimal> {
    if pool.input_reserve.is_zero() || pool.output_reserve.is_zero() {
        return None;
    }
    let spot = Decimal::from_ratio(pool.output_reserve, pool.input_reserve);
    let keep = Decimal::from_ratio(
        (10_000u16.saturating_sub(pool.pool_fee_bps)) as u128,
        10_000u128,
    );
    spot.checked_mul(keep).ok()
}

fn fee_keep(bps: u16) -> Decimal {
    Decimal::from_ratio((10_000u16.saturating_sub(bps)) as u128, 10_000u128)
}

/// Bid (taker sells token0): maker pays `price` token1 per token0 after taker commission.
fn bid_maker_net(price: Decimal, taker_bps: u16) -> Option<Decimal> {
    if price.is_zero() {
        return None;
    }
    price.checked_mul(fee_keep(taker_bps)).ok()
}

/// Ask (taker sells token1): maker yields `1/price` token0 per token1 after taker commission.
fn ask_maker_net(price: Decimal, taker_bps: u16) -> Option<Decimal> {
    if price.is_zero() {
        return None;
    }
    let inv = try_price_inverse(price)?;
    inv.checked_mul(fee_keep(taker_bps)).ok()
}

fn compare_rate(
    maker_net: Option<Decimal>,
    pool: &GreedyPoolRef,
) -> Result<GreedyBeats, ContractError> {
    let Some(maker_net) = maker_net else {
        return Ok(GreedyBeats::Skip);
    };
    let Some(pool_net) = pool_spot_net(pool) else {
        return Ok(GreedyBeats::Skip);
    };
    if maker_net > pool_net {
        Ok(GreedyBeats::Yes)
    } else {
        Ok(GreedyBeats::No)
    }
}

/// Bid (taker sells token0) vs residual pool spot after `effective_fee_bps`.
pub fn bid_beats_residual_pool(
    price: Decimal,
    taker_bps: u16,
    pool: &GreedyPoolRef,
) -> Result<GreedyBeats, ContractError> {
    compare_rate(bid_maker_net(price, taker_bps), pool)
}

/// Ask (taker sells token1) vs residual pool spot after `effective_fee_bps`.
pub fn ask_beats_residual_pool(
    price: Decimal,
    taker_bps: u16,
    pool: &GreedyPoolRef,
) -> Result<GreedyBeats, ContractError> {
    compare_rate(ask_maker_net(price, taker_bps), pool)
}

/// Taker bps used on the book leg for a greedy walk (same as match_*).
pub fn greedy_taker_bps(pool: &GreedyPoolRef) -> u16 {
    taker_fee_bps(pool.pool_fee_bps)
}

/// Classify why a greedy walk ended (**G3** stop reasons).
pub fn greedy_stop_after_walk(
    greedy: bool,
    stopped_worse: bool,
    scan_capped: bool,
    makers_used: u32,
    cap: u32,
    offer_left: Uint128,
) -> Option<GreedyStopReason> {
    if !greedy {
        return None;
    }
    if stopped_worse {
        return Some(GreedyStopReason::WorseThanPool);
    }
    if scan_capped {
        return Some(GreedyStopReason::ScanCap);
    }
    if offer_left.is_zero() && makers_used > 0 {
        return Some(GreedyStopReason::Filled);
    }
    if makers_used == 0 {
        return Some(GreedyStopReason::Empty);
    }
    if makers_used >= cap {
        return Some(GreedyStopReason::MaxMakers);
    }
    Some(GreedyStopReason::Empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(in_r: u128, out_r: u128, fee: u16) -> GreedyPoolRef {
        GreedyPoolRef {
            input_reserve: Uint128::new(in_r),
            output_reserve: Uint128::new(out_r),
            pool_fee_bps: fee,
        }
    }

    #[test]
    fn better_bid_beats_flat_pool() {
        // 1:1 pool, 30 bps: 1-unit net is 0 after floor (gross 0 or 1 minus fee).
        // A bid at price 2 token1/token0 pays 2 token1 per token0, net after 15 bps taker.
        let p = pool(1_000_000, 1_000_000, 30);
        let taker = greedy_taker_bps(&p);
        let beats = bid_beats_residual_pool(Decimal::from_ratio(2u128, 1u128), taker, &p).unwrap();
        assert_eq!(beats, GreedyBeats::Yes);
    }

    #[test]
    fn worse_bid_does_not_beat_pool() {
        let p = pool(1_000_000, 1_000_000, 30);
        let taker = greedy_taker_bps(&p);
        // Price 0.5 token1 per token0 is worse than ~0.997 pool spot after 30 bps.
        let beats = bid_beats_residual_pool(Decimal::percent(50), taker, &p).unwrap();
        assert_eq!(beats, GreedyBeats::No);
    }

    #[test]
    fn equal_is_not_strictly_better() {
        let p = pool(1_000_000, 1_000_000, 0);
        // Zero fee, 1:1 pool unit net = 0 (ceil_div k/(r+1) → gross 0) or 1.
        // Use a bid whose unit net equals pool unit net when both are 0 → No or Skip.
        let beats = bid_beats_residual_pool(Decimal::zero(), 0, &p).unwrap();
        assert_eq!(beats, GreedyBeats::Skip);
    }

    #[test]
    fn stop_reason_priority() {
        assert_eq!(
            greedy_stop_after_walk(true, true, true, 3, 8, Uint128::new(1)),
            Some(GreedyStopReason::WorseThanPool)
        );
        assert_eq!(
            greedy_stop_after_walk(true, false, true, 0, 8, Uint128::new(1)),
            Some(GreedyStopReason::ScanCap)
        );
        assert_eq!(
            greedy_stop_after_walk(true, false, false, 2, 8, Uint128::zero()),
            Some(GreedyStopReason::Filled)
        );
        assert_eq!(
            greedy_stop_after_walk(true, false, false, 8, 8, Uint128::new(1)),
            Some(GreedyStopReason::MaxMakers)
        );
        assert_eq!(
            greedy_stop_after_walk(true, false, false, 0, 8, Uint128::new(1)),
            Some(GreedyStopReason::Empty)
        );
        assert_eq!(
            greedy_stop_after_walk(false, true, false, 0, 8, Uint128::new(1)),
            None
        );
    }
}
