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

/// Minimum limit price (token1 per token0) at placement, ladder expansion, and price update.
///
/// Rejects dust prices where `1/price` overflows `Uint128::checked_mul_floor` against realistic
/// swap notionals (GitLab #467). `Decimal::raw(1)` = 1e-18 is the former attack vector; at 1e-9
/// the mul_floor overflow threshold is ~3.4e29 raw units on the operand.
pub const MIN_LIMIT_PRICE: Decimal = Decimal::raw(1_000_000_000);

/// Maximum limit price — symmetric reciprocal bound for `fill × price` overflow (#467).
///
/// At 1e9 token1 per token0, bid `max_fill × price` stays within `Uint128` for `remaining` up to
/// `Uint128::MAX`.
pub const MAX_LIMIT_PRICE: Decimal = Decimal::raw(1_000_000_000_000_000_000_000_000_000);

/// Placement / price-update gate for limit order prices (GitLab #467).
pub fn validate_limit_order_price(price: Decimal) -> Result<(), &'static str> {
    if price.is_zero() {
        return Err("limit price must be positive");
    }
    if price < MIN_LIMIT_PRICE {
        return Err("limit price below minimum");
    }
    if price > MAX_LIMIT_PRICE {
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
pub fn expand_limit_ladder(
    spec: &LimitOrderLadderSpec,
    max_rungs: u32,
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
    validate_limit_order_price(spec.start_price).map_err(StdError::generic_err)?;
    validate_limit_order_price(spec.end_price).map_err(StdError::generic_err)?;
    if spec.total_amount.is_zero() {
        return Err(StdError::generic_err(
            "ladder total_amount must be positive",
        ));
    }

    let count = spec.count;
    let prices = ladder_prices(spec.start_price, spec.end_price, count)?;
    for price in &prices {
        validate_limit_order_price(*price).map_err(StdError::generic_err)?;
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
        let items = expand_limit_ladder(&spec, 20).unwrap();
        assert_eq!(items.len(), 5);
        let sum: Uint128 = items.iter().map(|i| i.amount).sum();
        assert_eq!(sum, Uint128::new(1000));
    }

    #[test]
    fn expand_rejects_count_over_max() {
        let spec = ladder_spec(25, 100);
        assert!(expand_limit_ladder(&spec, 20).is_err());
    }

    #[test]
    fn validate_limit_price_rejects_dust_and_extreme() {
        assert!(validate_limit_order_price(Decimal::zero()).is_err());
        assert!(validate_limit_order_price(Decimal::raw(1)).is_err());
        assert!(validate_limit_order_price(MIN_LIMIT_PRICE).is_ok());
        assert!(validate_limit_order_price(Decimal::from_ratio(1u128, 10u128)).is_ok());
        assert!(validate_limit_order_price(MAX_LIMIT_PRICE).is_ok());
        assert!(validate_limit_order_price(MAX_LIMIT_PRICE + Decimal::raw(1)).is_err());
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
        assert!(expand_limit_ladder(&spec, 20).is_err());
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
        let items = expand_limit_ladder(&spec, 20).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].hint_after_order_id, None);
        assert_eq!(items[1].hint_after_order_id, None);
        assert_eq!(items[2].hint_after_order_id, Some(42));
    }
}
