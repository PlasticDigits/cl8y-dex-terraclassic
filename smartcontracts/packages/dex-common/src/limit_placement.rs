//! Batch / ladder limit order placement types and ladder expansion (GitLab #206).

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, StdError, Uint128};

use crate::pair::LimitOrderSide;

/// Absolute ceiling (gas safety); governance cannot exceed this on-chain.
/// Raised to 100 per LocalTerra gas benchmarks (GitLab #263).
pub const MAX_LIMIT_BATCH_RUNGS_HARD_CAP: u32 = 100;

/// Default for new pairs / migrated pairs when config was absent.
pub const DEFAULT_LIMIT_BATCH_MAX_RUNGS: u32 = 10;

/// Suggested factory default for localnet (governance may change via `UpdateConfig`).
pub const SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS: u32 = 20;

/// Minimum **human-scale** limit price (token1 per token0) at placement, ladder expansion,
/// and price update.
///
/// Applied to `price_raw × 10^(decimals0 − decimals1)` so a 6-vs-18 pair at ~79 human
/// (raw ~7.9e13) is in-band (GitLab #529). Equal-decimal pairs keep the #467 dust floor:
/// `Decimal::raw(1)` = 1e-18 is rejected; at human 1e-9 the `1/price` mul_floor overflow
/// threshold is ~3.4e29 raw units on the operand.
pub const MIN_LIMIT_PRICE: Decimal = Decimal::raw(1_000_000_000);

/// Maximum **human-scale** limit price — symmetric reciprocal bound (#467 / #529).
///
/// Execution still uses the raw `Decimal` (token1 base units per token0 base unit).
pub const MAX_LIMIT_PRICE: Decimal = Decimal::raw(1_000_000_000_000_000_000_000_000_000);

/// Human token1-per-token0: `price_raw × 10^(decimals0 − decimals1)` (GitLab #529).
///
/// Pair bootstrap caps each asset at 18 decimals, so the scale factor is at most `10^18`.
pub fn human_scale_limit_price(
    price: Decimal,
    decimals0: u8,
    decimals1: u8,
) -> Result<Decimal, &'static str> {
    if decimals0 > 18 || decimals1 > 18 {
        return Err("asset decimals exceed bootstrap cap");
    }
    if decimals0 == decimals1 {
        return Ok(price);
    }
    if decimals0 > decimals1 {
        let factor = 10u128.pow((decimals0 - decimals1) as u32);
        price
            .checked_mul(Decimal::from_ratio(factor, 1u128))
            .map_err(|_| "limit price human-scale overflow")
    } else {
        let factor = 10u128.pow((decimals1 - decimals0) as u32);
        price
            .checked_div(Decimal::from_ratio(factor, 1u128))
            .map_err(|_| "limit price human-scale overflow")
    }
}

/// Placement / price-update gate for limit order prices (GitLab #467, decimals-normalized #529).
///
/// `price` is the on-chain raw ratio (token1 base units / token0 base units). Bounds apply to
/// the human-scale value so economically ordinary mismatched-decimal pairs are not rejected.
pub fn validate_limit_order_price(
    price: Decimal,
    decimals0: u8,
    decimals1: u8,
) -> Result<(), &'static str> {
    if price.is_zero() {
        return Err("limit price must be positive");
    }
    let human = human_scale_limit_price(price, decimals0, decimals1)?;
    if human.is_zero() {
        return Err("limit price below minimum");
    }
    if human < MIN_LIMIT_PRICE {
        return Err("limit price below minimum");
    }
    if human > MAX_LIMIT_PRICE {
        return Err("limit price above maximum");
    }
    Ok(())
}

#[cw_serde]
pub struct LimitOrderPlacementItem {
    pub price: Decimal,
    /// Gross CW20 for this rung (maker fee deducted at placement).
    pub amount: Uint128,
    pub max_adjust_steps: u32,
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// Optional predecessor order id for O(1) insert verify (GitLab #256 / #261).
    #[serde(default)]
    pub hint_after_order_id: Option<u64>,
}

#[cw_serde]
pub enum LimitLadderDistribution {
    Equal,
}

#[cw_serde]
pub struct LimitOrderLadderSpec {
    pub side: LimitOrderSide,
    pub start_price: Decimal,
    pub end_price: Decimal,
    pub count: u32,
    pub total_amount: Uint128,
    pub distribution: LimitLadderDistribution,
    pub max_adjust_steps: u32,
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// Optional predecessor for the head-most rung in book order (GitLab #266).
    #[serde(default)]
    pub hint_after_order_id: Option<u64>,
}

#[cw_serde]
pub struct LimitOrderConfigResponse {
    pub max_batch_rungs: u32,
}

/// Clamp governance-configured cap to the hard ceiling.
pub fn clamp_max_batch_rungs(max_rungs: u32) -> u32 {
    max_rungs.clamp(1, MAX_LIMIT_BATCH_RUNGS_HARD_CAP)
}

/// Expand a ladder spec into per-rung placement items.
///
/// `decimals0` / `decimals1` are the pair asset CW20 decimals (same as placement validation).
pub fn expand_limit_ladder(
    spec: &LimitOrderLadderSpec,
    max_rungs: u32,
    decimals0: u8,
    decimals1: u8,
) -> Result<Vec<LimitOrderPlacementItem>, StdError> {
    if spec.count < 2 {
        return Err(StdError::generic_err("ladder count must be at least 2"));
    }
    if spec.count > max_rungs {
        return Err(StdError::generic_err(format!(
            "ladder count {0} exceeds pair max_batch_rungs {max_rungs}",
            spec.count
        )));
    }
    if spec.start_price.is_zero() || spec.end_price.is_zero() {
        return Err(StdError::generic_err("ladder prices must be positive"));
    }
    validate_limit_order_price(spec.start_price, decimals0, decimals1)
        .map_err(StdError::generic_err)?;
    validate_limit_order_price(spec.end_price, decimals0, decimals1)
        .map_err(StdError::generic_err)?;
    if spec.total_amount.is_zero() {
        return Err(StdError::generic_err(
            "ladder total_amount must be positive",
        ));
    }

    let count = spec.count;
    let prices = ladder_prices(spec.start_price, spec.end_price, count)?;
    for price in &prices {
        validate_limit_order_price(*price, decimals0, decimals1).map_err(StdError::generic_err)?;
    }
    let amounts = ladder_amounts_equal(spec.total_amount, count)?;
    let boundary_idx =
        ladder_boundary_rung_index(&spec.side, spec.start_price, spec.end_price, count);

    Ok(prices
        .into_iter()
        .zip(amounts)
        .enumerate()
        .map(|(idx, (price, amount))| LimitOrderPlacementItem {
            price,
            amount,
            max_adjust_steps: spec.max_adjust_steps,
            expires_at: spec.expires_at,
            hint_after_order_id: if idx == boundary_idx as usize {
                spec.hint_after_order_id
            } else {
                None
            },
        })
        .collect())
}

/// Index of the head-most rung in book order (GitLab #266).
fn ladder_boundary_rung_index(
    side: &LimitOrderSide,
    start: Decimal,
    end: Decimal,
    count: u32,
) -> u32 {
    match side {
        LimitOrderSide::Bid => {
            if start >= end {
                0
            } else {
                count - 1
            }
        }
        LimitOrderSide::Ask => {
            if start <= end {
                0
            } else {
                count - 1
            }
        }
    }
}

fn ladder_prices(start: Decimal, end: Decimal, count: u32) -> Result<Vec<Decimal>, StdError> {
    if count == 1 {
        return Ok(vec![start]);
    }
    let steps = Decimal::from_ratio(count as u128 - 1, 1u128);
    let delta = end
        .checked_sub(start)
        .map_err(|_| StdError::generic_err("ladder price range overflow"))?;
    let step_size = delta
        .checked_div(steps)
        .map_err(|_| StdError::generic_err("ladder price step divide"))?;

    let mut out = Vec::with_capacity(count as usize);
    for i in 0..count {
        if i == 0 {
            out.push(start);
        } else if i == count - 1 {
            out.push(end);
        } else {
            let offset = step_size
                .checked_mul(Decimal::from_ratio(i as u128, 1u128))
                .map_err(|_| StdError::generic_err("ladder price multiply"))?;
            out.push(
                start
                    .checked_add(offset)
                    .map_err(|_| StdError::generic_err("ladder price add"))?,
            );
        }
    }
    Ok(out)
}

fn ladder_amounts_equal(total: Uint128, count: u32) -> Result<Vec<Uint128>, StdError> {
    let n = count as u128;
    let base = total.checked_div(Uint128::from(n))?;
    let mut amounts = vec![base; count as usize];
    let assigned = base.checked_mul(Uint128::from(n))?;
    let remainder = total.checked_sub(assigned)?;
    if !remainder.is_zero() {
        let last = amounts
            .last_mut()
            .ok_or_else(|| StdError::generic_err("empty ladder"))?;
        *last = last.checked_add(remainder)?;
    }
    Ok(amounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder_spec(count: u32, total: u128) -> LimitOrderLadderSpec {
        LimitOrderLadderSpec {
            side: LimitOrderSide::Bid,
            start_price: Decimal::from_ratio(9u128, 10u128),
            end_price: Decimal::one(),
            count,
            total_amount: Uint128::new(total),
            distribution: LimitLadderDistribution::Equal,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: None,
        }
    }

    #[test]
    fn expand_equal_ladder_sums_amounts() {
        let spec = ladder_spec(5, 1000);
        let items = expand_limit_ladder(&spec, 20, 6, 6).unwrap();
        assert_eq!(items.len(), 5);
        let sum: Uint128 = items.iter().map(|i| i.amount).sum();
        assert_eq!(sum, Uint128::new(1000));
    }

    #[test]
    fn expand_rejects_count_over_max() {
        let spec = ladder_spec(25, 100);
        assert!(expand_limit_ladder(&spec, 20, 6, 6).is_err());
    }

    #[test]
    fn validate_limit_price_rejects_dust_and_extreme() {
        assert!(validate_limit_order_price(Decimal::zero(), 6, 6).is_err());
        assert!(validate_limit_order_price(Decimal::raw(1), 6, 6).is_err());
        assert!(validate_limit_order_price(MIN_LIMIT_PRICE, 6, 6).is_ok());
        assert!(validate_limit_order_price(Decimal::from_ratio(1u128, 10u128), 6, 6).is_ok());
        assert!(validate_limit_order_price(MAX_LIMIT_PRICE, 6, 6).is_ok());
        assert!(validate_limit_order_price(MAX_LIMIT_PRICE + Decimal::raw(1), 6, 6).is_err());
    }

    /// GitLab #529 — UST1 (6) / USTR (18) raw ~7.9e13 is human ~79, inside the band.
    #[test]
    fn validate_limit_price_accepts_six_vs_eighteen_raw() {
        let raw_ust1_ustr = Decimal::from_ratio(78_760_000_000_000u128, 1u128);
        assert!(validate_limit_order_price(raw_ust1_ustr, 6, 18).is_ok());
        let human = human_scale_limit_price(raw_ust1_ustr, 6, 18).unwrap();
        assert!(human > Decimal::from_ratio(78u128, 1u128));
        assert!(human < Decimal::from_ratio(79u128, 1u128));

        let raw_ustr_ust1 = Decimal::from_ratio(13u128, 1_000_000_000_000_000u128);
        assert!(validate_limit_order_price(raw_ustr_ust1, 18, 6).is_ok());
        assert!(validate_limit_order_price(Decimal::raw(1), 6, 18).is_err());
        assert!(validate_limit_order_price(MAX_LIMIT_PRICE, 6, 18).is_ok());
    }

    #[test]
    fn expand_ladder_rejects_out_of_band_end_price() {
        let spec = LimitOrderLadderSpec {
            side: LimitOrderSide::Ask,
            start_price: Decimal::from_ratio(1u128, 10u128),
            end_price: Decimal::raw(1),
            count: 3,
            total_amount: Uint128::new(300),
            distribution: LimitLadderDistribution::Equal,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: None,
        };
        assert!(expand_limit_ladder(&spec, 20, 6, 6).is_err());
    }

    #[test]
    fn expand_ladder_accepts_six_vs_eighteen_raw_band() {
        let spec = LimitOrderLadderSpec {
            side: LimitOrderSide::Ask,
            start_price: Decimal::from_ratio(70_000_000_000_000u128, 1u128),
            end_price: Decimal::from_ratio(80_000_000_000_000u128, 1u128),
            count: 3,
            total_amount: Uint128::new(300),
            distribution: LimitLadderDistribution::Equal,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: None,
        };
        let items = expand_limit_ladder(&spec, 20, 6, 18).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn expand_ladder_anchor_on_boundary_rung_ascending_bid() {
        let spec = LimitOrderLadderSpec {
            side: LimitOrderSide::Bid,
            start_price: Decimal::from_ratio(99u128, 100u128),
            end_price: Decimal::one(),
            count: 3,
            total_amount: Uint128::new(300),
            distribution: LimitLadderDistribution::Equal,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: Some(42),
        };
        let items = expand_limit_ladder(&spec, 20, 6, 6).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].hint_after_order_id, None);
        assert_eq!(items[1].hint_after_order_id, None);
        assert_eq!(items[2].hint_after_order_id, Some(42));
    }
}
