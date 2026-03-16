// # TWAP Oracle — Arithmetic Mean
//
// ## How it works
//
// Each pair accumulates `price_cumulative` — the integral of `price × dt`
// over time — on every state-changing action (swap, provide/withdraw
// liquidity). A ring buffer of `Observation` snapshots lets consumers
// query any historical window and compute the arithmetic-mean TWAP:
//
//   twap = (cum_end − cum_start) / (t_end − t_start)
//
// Price is stored as `reserve_b / reserve_a` (and vice versa) using
// `Decimal` scaled to 18 digits, accumulated in `Uint128`.
//
// ## Security model & known risks
//
// This oracle resists single-transaction manipulation: the price is
// sampled from the *previous* reserves before the current action mutates
// them.
//
// **Risks:**
//
// 1. **Multi-block manipulation** — A validator controlling consecutive
//    blocks can skew the TWAP over short windows. Use windows ≥30 minutes
//    and cross-check against secondary feeds.
//
// 2. **Low-liquidity pairs** — Manipulation cost is proportional to pool
//    depth. Do not rely on this TWAP for pairs with < $100k TVL.
//
// 3. **Stale observations** — If no one interacts with the pair for an
//    extended period, observations stop being written. The observe() query
//    linearly interpolates between known points.
//
// 4. **Overflow** — Cumulative values use Uint128 with checked arithmetic.
//    At extreme prices or very long windows (years), overflow is possible
//    but handled gracefully with errors.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128};

/// Maximum number of observations the ring buffer can hold.
/// ~65 535 × 6s blocks ≈ 109 hours of history at one observation per block.
pub const MAX_OBSERVATION_CARDINALITY: u16 = 65_000;

/// Default observation cardinality for new pairs.
/// 360 × 6s blocks ≈ 36 minutes — enough for a 30-minute TWAP window.
pub const DEFAULT_OBSERVATION_CARDINALITY: u16 = 360;

// ---------------------------------------------------------------------------
// Observation types
// ---------------------------------------------------------------------------

/// A single TWAP observation recorded in the ring buffer.
#[cw_serde]
pub struct Observation {
    /// Block timestamp (seconds) when this observation was recorded.
    pub timestamp: u64,
    /// Cumulative `∫ (reserve_b / reserve_a) dt`, scaled by 1e18.
    pub price_a_cumulative: Uint128,
    /// Cumulative `∫ (reserve_a / reserve_b) dt`, scaled by 1e18.
    pub price_b_cumulative: Uint128,
}

/// Response for the `Observe` query — returns cumulative price values
/// at each requested time offset.
#[cw_serde]
pub struct ObserveResponse {
    pub price_a_cumulatives: Vec<Uint128>,
    pub price_b_cumulatives: Vec<Uint128>,
}

/// Response for the `OracleInfo` query.
#[cw_serde]
pub struct OracleInfoResponse {
    pub observation_cardinality: u16,
    pub observation_index: u16,
    pub observations_stored: u16,
    pub oldest_observation_timestamp: u64,
    pub newest_observation_timestamp: u64,
}

// ---------------------------------------------------------------------------
// Price accumulation helpers
// ---------------------------------------------------------------------------

/// 1e18 — Decimal's internal scale factor.
#[cfg(test)]
const DECIMAL_SCALE: u128 = 1_000_000_000_000_000_000;

/// Compute price × dt as a Uint128, where price = Decimal (18 digits).
/// Returns `floor(price * dt * 1e18)`.
pub fn price_times_dt(price: Decimal, dt: u64) -> StdResult<Uint128> {
    let price_scaled = Uint128::new(price.atomics().u128());
    price_scaled
        .checked_mul(Uint128::new(dt as u128))
        .map_err(|e| StdError::generic_err(format!("oracle: price × dt overflow: {}", e)))
}

/// Compute the arithmetic-mean TWAP from two cumulative snapshots.
/// Returns `(cum_end - cum_start) / time_elapsed`, as a `Decimal`.
pub fn compute_twap_price(
    cum_start: Uint128,
    cum_end: Uint128,
    time_elapsed: u64,
) -> StdResult<Decimal> {
    if time_elapsed == 0 {
        return Err(StdError::generic_err("oracle: time_elapsed must be > 0"));
    }
    if cum_end < cum_start {
        return Err(StdError::generic_err(
            "oracle: cumulative end < start (possible data corruption)",
        ));
    }
    let diff = cum_end - cum_start;
    let avg_scaled = diff.checked_div(Uint128::new(time_elapsed as u128))?;
    Decimal::from_atomics(avg_scaled, 18)
        .map_err(|e| StdError::generic_err(format!("oracle: decimal conversion error: {}", e)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_decimal_close(actual: Decimal, expected_f: f64, tolerance_pct: f64, label: &str) {
        let actual_str = actual.to_string();
        let actual_f: f64 = actual_str.parse().unwrap();
        let pct_err = if expected_f == 0.0 {
            actual_f.abs()
        } else {
            ((actual_f - expected_f) / expected_f).abs() * 100.0
        };
        assert!(
            pct_err < tolerance_pct,
            "{}: expected ~{}, got {} (err {:.4}%)",
            label,
            expected_f,
            actual_f,
            pct_err
        );
    }

    #[test]
    fn price_times_dt_basic() {
        let price = Decimal::from_ratio(2u128, 1u128);
        let dt = 100u64;
        let result = price_times_dt(price, dt).unwrap();
        assert_eq!(
            result,
            Uint128::new(200 * DECIMAL_SCALE),
            "2.0 × 100 = 200 (scaled)"
        );
    }

    #[test]
    fn price_times_dt_fractional() {
        let price = Decimal::from_ratio(3u128, 2u128); // 1.5
        let dt = 60u64;
        let result = price_times_dt(price, dt).unwrap();
        assert_eq!(
            result,
            Uint128::new(90 * DECIMAL_SCALE),
            "1.5 × 60 = 90 (scaled)"
        );
    }

    #[test]
    fn compute_twap_constant_price() {
        let price = Decimal::from_ratio(2u128, 1u128);
        let dt = 3600u64;
        let cum_start = Uint128::zero();
        let cum_end = price_times_dt(price, dt).unwrap();
        let twap = compute_twap_price(cum_start, cum_end, dt).unwrap();
        assert_decimal_close(twap, 2.0, 0.01, "constant price 2.0");
    }

    #[test]
    fn compute_twap_varying_price() {
        let p1 = Decimal::from_ratio(1u128, 1u128);
        let p2 = Decimal::from_ratio(3u128, 1u128);
        let dt = 100u64;
        let cum_mid = price_times_dt(p1, dt).unwrap();
        let cum_end = cum_mid + price_times_dt(p2, dt).unwrap();
        let twap = compute_twap_price(Uint128::zero(), cum_end, 200).unwrap();
        assert_decimal_close(twap, 2.0, 0.01, "avg of 1.0 and 3.0");
    }

    #[test]
    fn compute_twap_rejects_zero_elapsed() {
        assert!(compute_twap_price(Uint128::zero(), Uint128::new(100), 0).is_err());
    }

    #[test]
    fn compute_twap_rejects_end_lt_start() {
        assert!(compute_twap_price(Uint128::new(100), Uint128::new(50), 10).is_err());
    }
}
