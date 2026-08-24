//! Skim slippage floor (GitLab #610 / audit M-2).
//!
//! Permissionless `SkimToLp` always attaches `max_spread` (and optional
//! `min_return`). The caller cannot override the floor downward.

use cosmwasm_std::Decimal;

use crate::error::ContractError;

/// Default skim `max_spread` = 100 bps (1%). Same as pair default when omitted.
pub const DEFAULT_SKIM_MAX_SPREAD_BPS: u64 = 100;
/// Hard cap: manager cannot loosen the floor past 200 bps (2%).
pub const MAX_SKIM_MAX_SPREAD_BPS: u64 = 200;

pub fn default_skim_max_spread() -> Decimal {
    bps_to_decimal(DEFAULT_SKIM_MAX_SPREAD_BPS)
}

pub fn max_skim_max_spread() -> Decimal {
    bps_to_decimal(MAX_SKIM_MAX_SPREAD_BPS)
}

pub fn bps_to_decimal(bps: u64) -> Decimal {
    Decimal::from_ratio(bps, 10_000u64)
}

/// Reject a manager-set spread above the documented cap (**M610-3**).
pub fn clamp_skim_max_spread(spread: Decimal) -> Result<Decimal, ContractError> {
    if spread > max_skim_max_spread() {
        return Err(ContractError::SkimSpreadTooWide {});
    }
    Ok(spread)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_100_bps() {
        assert_eq!(default_skim_max_spread(), Decimal::percent(1));
    }

    #[test]
    fn cap_is_200_bps() {
        assert_eq!(max_skim_max_spread(), Decimal::percent(2));
        assert!(clamp_skim_max_spread(Decimal::percent(2)).is_ok());
        assert_eq!(
            clamp_skim_max_spread(Decimal::percent(3)).unwrap_err(),
            ContractError::SkimSpreadTooWide {}
        );
    }
}
