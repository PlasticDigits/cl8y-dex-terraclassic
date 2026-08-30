//! Greedy book-first vs residual pool comparison (GitLab #708 / **G3**, leftovers #709).
//!
//! **Beats (pinned):** a live maker is taken only when the taker's **net ask per 1 offer
//! (Decimal rate)** at that maker's executable price (after book taker commission) is **strictly
//! greater** than the **pool spot** `output_reserve / input_reserve` after pair `effective_fee_bps`.
//! Integer 1-raw-unit CP dumps floor to **0** on large reserves, which would make every priceable
//! maker "beat" the pool — rates avoid that artifact while matching the 1-unit *marginal* intent
//! (**G3**). Unpriceable makers (`price == 0`, no inverse) are **skipped** (**L18** / **L20**),
//! not treated as a stop.
//!
//! Pool-spot overflow (`Decimal::checked_from_ratio` fails when `output/input` ≳ 3.4×10²⁰) is
//! **Skip**, not a VM panic (**A7** / #709). Equal Decimal rates are a stop (`No`), not Skip.

use cosmwasm_std::{Decimal, Uint128};
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

fn try_price_inverse(price: Decimal) -> Option<Decimal> {
    Decimal::one().checked_div(price).ok()
}

/// Pool net rate after `effective_fee_bps`. Zero reserves or an unrepresentable
/// `output/input` Decimal (overflow of `from_ratio`) → `None` so the caller **Skips**
/// rather than panicking (**A7** / **L18** / **L20**).
fn pool_spot_net(pool: &GreedyPoolRef) -> Option<Decimal> {
    if pool.input_reserve.is_zero() || pool.output_reserve.is_zero() {
        return None;
    }
    let spot = Decimal::checked_from_ratio(pool.output_reserve, pool.input_reserve).ok()?;
    let keep = Decimal::checked_from_ratio(
        (10_000u16.saturating_sub(pool.pool_fee_bps)) as u128,
        10_000u128,
    )
    .ok()?;
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

/// Classify why a greedy walk ended (**G3** stop reasons, #709 remainder).
///
/// Priority: worse → scan_cap → filled (offer gone, makers>0) → empty (makers==0)
/// → max_makers (at cap with offer left) → remainder_to_pool (makers>0, offer left).
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
    Some(GreedyStopReason::RemainderToPool)
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
    fn zero_price_bid_is_skip_not_stop() {
        let p = pool(1_000_000, 1_000_000, 0);
        let beats = bid_beats_residual_pool(Decimal::zero(), 0, &p).unwrap();
        assert_eq!(beats, GreedyBeats::Skip);
    }

    #[test]
    fn equal_bid_rate_is_stop_not_skip() {
        let p = pool(1_000_000, 1_000_000, 0);
        let beats = bid_beats_residual_pool(Decimal::one(), 0, &p).unwrap();
        assert_eq!(beats, GreedyBeats::No);
    }

    #[test]
    fn equal_ask_rate_is_stop_not_skip() {
        let p = pool(1_000_000, 1_000_000, 0);
        let beats = ask_beats_residual_pool(Decimal::one(), 0, &p).unwrap();
        assert_eq!(beats, GreedyBeats::No);
    }

    #[test]
    fn better_ask_beats_flat_pool() {
        let p = pool(1_000_000, 1_000_000, 30);
        let taker = greedy_taker_bps(&p);
        // Ask price 0.5 token1/token0 → 2 token0 per token1, beats ~0.997 pool.
        let beats = ask_beats_residual_pool(Decimal::percent(50), taker, &p).unwrap();
        assert_eq!(beats, GreedyBeats::Yes);
    }

    #[test]
    fn zero_reserves_skip() {
        let empty_in = pool(0, 1_000_000, 30);
        let empty_out = pool(1_000_000, 0, 30);
        assert_eq!(
            bid_beats_residual_pool(Decimal::from_ratio(2u128, 1u128), 0, &empty_in).unwrap(),
            GreedyBeats::Skip
        );
        assert_eq!(
            bid_beats_residual_pool(Decimal::from_ratio(2u128, 1u128), 0, &empty_out).unwrap(),
            GreedyBeats::Skip
        );
    }

    #[test]
    fn pool_spot_overflow_is_skip_not_panic() {
        // `Decimal::from_ratio` panics when numerator * 10^18 overflows Uint128
        // (~output/input ≳ 3.4×10²⁰). checked_from_ratio → Skip (**A7**).
        let p = GreedyPoolRef {
            input_reserve: Uint128::new(1),
            output_reserve: Uint128::MAX,
            pool_fee_bps: 0,
        };
        assert!(pool_spot_net(&p).is_none());
        let beats = bid_beats_residual_pool(Decimal::from_ratio(2u128, 1u128), 0, &p).unwrap();
        assert_eq!(beats, GreedyBeats::Skip);
        let ask = ask_beats_residual_pool(Decimal::percent(50), 0, &p).unwrap();
        assert_eq!(ask, GreedyBeats::Skip);
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
            greedy_stop_after_walk(true, false, false, 2, 8, Uint128::new(1)),
            Some(GreedyStopReason::RemainderToPool)
        );
        assert_eq!(
            greedy_stop_after_walk(false, true, false, 0, 8, Uint128::new(1)),
            None
        );
    }
}
