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
// `Decimal` scaled to 18 digits. The running integral is a `Uint256`
// (zero-extended from the historical `Uint128` decimal strings).
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
// 4. **Width** — `price × dt` and the cumulative add are `Uint256`
//    ([#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224),
//    [#1322](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1322)).
//    A `Decimal` atomic (`u128`) times a `u64` dt always fits in 192 bits,
//    so one sample is not truncated and does not abort execute. Sums past
//    `2^128` stay the full integer. A window whose integral itself does not
//    fit in `Uint256` is not a realistic pair age. Spot `Decimal::from_ratio`
//    that cannot fit `Decimal::MAX` is skipped (execute `#465`, Observe
//    `#1231`) rather than panicking or clamping.
//
// JSON decimal strings that used to fit in `Uint128` deserialize as the
// same integer (`Uint256` zero-extend). Migrate must not rewrite them.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, StdResult, Uint128, Uint256};

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
    pub price_a_cumulative: Uint256,
    /// Cumulative `∫ (reserve_a / reserve_b) dt`, scaled by 1e18.
    pub price_b_cumulative: Uint256,
}

/// Response for the `Observe` query — returns cumulative price values
/// at each requested time offset.
#[cw_serde]
pub struct ObserveResponse {
    pub price_a_cumulatives: Vec<Uint256>,
    pub price_b_cumulatives: Vec<Uint256>,
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

/// `floor(price × dt × 1e18)` as `Uint256`.
///
/// `Decimal` atomics are `u128` and `dt` is `u64`, so the product always
/// fits in 192 bits and therefore in `Uint256`. Checked mul does not
/// truncate to the low 128 bits. [#1224](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1224)
pub fn price_times_dt(price: Decimal, dt: u64) -> StdResult<Uint256> {
    let price_scaled = Uint256::from(price.atomics());
    price_scaled
        .checked_mul(Uint256::from(dt))
        .map_err(|e| StdError::generic_err(format!("oracle: price × dt overflow: {}", e)))
}

/// Arithmetic-mean TWAP from two cumulative snapshots.
///
/// `(cum_end - cum_start) / time_elapsed` as a `Decimal`. `cum_end < cum_start`
/// is treated as a corrupt pair of snapshots. Cumulatives are monotonic
/// `Uint256` values, not modulo `2^128`.
pub fn compute_twap_price(
    cum_start: Uint256,
    cum_end: Uint256,
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
    let diff = cum_end.checked_sub(cum_start)?;
    let avg_scaled = diff.checked_div(Uint256::from(time_elapsed))?;
    let avg_u128 = Uint128::try_from(avg_scaled)
        .map_err(|e| StdError::generic_err(format!("oracle: twap exceeds Decimal range: {e}")))?;
    Decimal::from_atomics(avg_u128, 18)
        .map_err(|e| StdError::generic_err(format!("oracle: decimal conversion error: {}", e)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{from_json, to_json_vec};

    /// Live pair cumulative cited on #1322 (~99.96% of `u128::MAX`).
    const LIVE_CUMULATIVE: &str = "340144359629112943994362291128760055446";

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
            Uint256::from(200u128 * DECIMAL_SCALE),
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
            Uint256::from(90u128 * DECIMAL_SCALE),
            "1.5 × 60 = 90 (scaled)"
        );
    }

    #[test]
    fn price_times_dt_above_u128_is_full_product() {
        let price = Decimal::from_atomics(Uint128::MAX, 18).unwrap();
        let dt = 2u64;
        let result = price_times_dt(price, dt).unwrap();
        let expected = Uint256::from(Uint128::MAX) * Uint256::from(dt);
        assert!(expected > Uint256::from(Uint128::MAX));
        assert_eq!(result, expected);
        let low_128 = Uint256::from(Uint128::try_from(expected).unwrap_or(Uint128::MAX));
        assert_ne!(
            result, low_128,
            "must not store a truncated or saturated u128 product"
        );
    }

    #[test]
    fn price_times_dt_max_decimal_times_max_dt_fits() {
        let price = Decimal::from_atomics(Uint128::MAX, 18).unwrap();
        let result = price_times_dt(price, u64::MAX).unwrap();
        assert_eq!(
            result,
            Uint256::from(Uint128::MAX) * Uint256::from(u64::MAX)
        );
    }

    #[test]
    fn compute_twap_constant_price() {
        let price = Decimal::from_ratio(2u128, 1u128);
        let dt = 3600u64;
        let cum_start = Uint256::zero();
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
        let twap = compute_twap_price(Uint256::zero(), cum_end, 200).unwrap();
        assert_decimal_close(twap, 2.0, 0.01, "avg of 1.0 and 3.0");
    }

    #[test]
    fn compute_twap_rejects_zero_elapsed() {
        assert!(compute_twap_price(Uint256::zero(), Uint256::from(100u128), 0).is_err());
    }

    #[test]
    fn compute_twap_rejects_end_lt_start() {
        assert!(compute_twap_price(Uint256::from(100u128), Uint256::from(50u128), 10).is_err());
    }

    #[test]
    fn compute_twap_across_u128_ceiling_is_the_window_integral() {
        let dt = 10u64;
        let start = Uint256::from(Uint128::MAX) - Uint256::from(5u128 * DECIMAL_SCALE);
        let end = start + price_times_dt(Decimal::one(), dt).unwrap();
        assert!(end > Uint256::from(Uint128::MAX));
        let twap = compute_twap_price(start, end, dt).unwrap();
        assert_eq!(twap, Decimal::one());
    }

    #[test]
    fn legacy_u128_observation_json_zero_extends() {
        #[cw_serde]
        struct LegacyObservation {
            timestamp: u64,
            price_a_cumulative: Uint128,
            price_b_cumulative: Uint128,
        }
        let legacy = LegacyObservation {
            timestamp: 1_700_000_000,
            price_a_cumulative: LIVE_CUMULATIVE.parse().unwrap(),
            price_b_cumulative: Uint128::new(11),
        };
        let obs: Observation = from_json(to_json_vec(&legacy).unwrap()).unwrap();
        assert_eq!(obs.timestamp, legacy.timestamp);
        assert_eq!(
            obs.price_a_cumulative,
            Uint256::from(legacy.price_a_cumulative)
        );
        assert_eq!(obs.price_b_cumulative, Uint256::from(11u128));
        let again = cosmwasm_std::to_json_string(&obs).unwrap();
        assert!(again.contains(LIVE_CUMULATIVE));
    }
}
