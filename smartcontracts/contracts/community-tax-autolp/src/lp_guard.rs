//! LP1398: an explicit manager pair bind approves reserves per LP share. A
//! permissionless skim cannot refresh this economic anchor from mutable spot.
use cosmwasm_std::{Addr, Decimal, Deps, StdError, Uint128, Uint256};
use dex_common::pair::{PoolResponse, QueryMsg};
use dex_common::types::{AssetInfo, PairInfo};

use crate::error::ContractError;
use crate::state::LpPolicy;

fn math_error(error: impl ToString) -> ContractError {
    StdError::generic_err(error.to_string()).into()
}

/// Floors the initial raw ratio, then rounds the retained minimum upward.
/// A nonzero economic leg always requires at least one raw output/share.
pub fn minimum_for(
    amount: Uint128,
    numerator: Uint128,
    denominator: Uint128,
    tolerance: Decimal,
) -> Result<Uint128, ContractError> {
    let expected = amount
        .checked_multiply_ratio(numerator, denominator)
        .map_err(math_error)?;
    let scale = Uint256::from(Decimal::one().atomics());
    let retained = Uint256::from(expected)
        .checked_mul(Uint256::from((Decimal::one() - tolerance).atomics()))
        .map_err(math_error)?;
    let rounded = retained
        .checked_add(scale - Uint256::one())
        .map_err(math_error)?
        .checked_div(scale)
        .map_err(math_error)?;
    Ok(Uint128::try_from(rounded)
        .map_err(math_error)?
        .max(Uint128::one()))
}

pub fn pool_reserves(
    pool: &PoolResponse,
    token: &Addr,
    quote: &Addr,
) -> Result<(Uint128, Uint128), ContractError> {
    let tax = AssetInfo::Token {
        contract_addr: token.to_string(),
    };
    let other = AssetInfo::Token {
        contract_addr: quote.to_string(),
    };
    match (&pool.assets[0].info, &pool.assets[1].info) {
        (a, b) if a == &tax && b == &other => Ok((pool.assets[0].amount, pool.assets[1].amount)),
        (a, b) if a == &other && b == &tax => Ok((pool.assets[1].amount, pool.assets[0].amount)),
        _ => Err(ContractError::InvalidLpPool {}),
    }
}

/// Empty pools may be bound during creation; they cannot skim until initialized
/// and explicitly rebound by the manager. Native quotes are unsupported.
pub fn capture_policy(
    deps: Deps,
    pair: &Addr,
    token: &Addr,
    quote: Option<&Addr>,
) -> Result<Option<LpPolicy>, ContractError> {
    let Some(quote) = quote else {
        return Ok(None);
    };
    let info: PairInfo = deps.querier.query_wasm_smart(pair, &QueryMsg::Pair {})?;
    let pool: PoolResponse = deps.querier.query_wasm_smart(pair, &QueryMsg::Pool {})?;
    let (tax_reserve, quote_reserve) = pool_reserves(&pool, token, quote)?;
    Ok(Some(LpPolicy {
        pair: pair.clone(),
        quote: quote.clone(),
        lp_token: deps.api.addr_validate(info.liquidity_token.as_str())?,
        tax_reserve,
        quote_reserve,
        total_share: pool.total_share,
    }))
}

pub fn validate_policy(
    deps: Deps,
    policy: &LpPolicy,
    token: &Addr,
    pair: &Addr,
    quote: &Addr,
    tolerance: Decimal,
) -> Result<(), ContractError> {
    if policy.pair != pair
        || policy.quote != quote
        || policy.tax_reserve.is_zero()
        || policy.quote_reserve.is_zero()
        || policy.total_share.is_zero()
    {
        return Err(ContractError::InvalidLpPool {});
    }
    let info: PairInfo = deps.querier.query_wasm_smart(pair, &QueryMsg::Pair {})?;
    if info.liquidity_token != policy.lp_token {
        return Err(ContractError::InvalidLpPool {});
    }
    let pool: PoolResponse = deps.querier.query_wasm_smart(pair, &QueryMsg::Pool {})?;
    let (tax, other) = pool_reserves(&pool, token, quote)?;
    if tax.is_zero() || other.is_zero() || pool.total_share.is_zero() {
        return Err(ContractError::InvalidLpPool {});
    }
    // Compare each reserve per share, not just the spot ratio: proportional
    // donations/dilution must not make an old LP minimum economically weaker.
    for (current, approved) in [(tax, policy.tax_reserve), (other, policy.quote_reserve)] {
        let actual = Uint256::from(current) * Uint256::from(policy.total_share);
        let baseline = Uint256::from(approved) * Uint256::from(pool.total_share);
        let deviation = actual.abs_diff(baseline);
        let allowed = baseline
            .checked_multiply_ratio(tolerance.atomics(), Decimal::one().atomics())
            .map_err(math_error)?;
        if deviation > allowed {
            return Err(ContractError::StaleLpPolicy {});
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_minimum_rounds_up_and_rejects_overflow() {
        assert_eq!(
            minimum_for(
                Uint128::new(101),
                Uint128::one(),
                Uint128::one(),
                Decimal::percent(1)
            )
            .unwrap(),
            Uint128::new(100)
        );
        assert_eq!(
            minimum_for(
                Uint128::one(),
                Uint128::one(),
                Uint128::new(100),
                Decimal::percent(1)
            )
            .unwrap(),
            Uint128::one()
        );
        assert!(minimum_for(
            Uint128::MAX,
            Uint128::MAX,
            Uint128::one(),
            Decimal::percent(1)
        )
        .is_err());
        assert!(minimum_for(
            Uint128::one(),
            Uint128::one(),
            Uint128::zero(),
            Decimal::percent(1)
        )
        .is_err());
    }
}
