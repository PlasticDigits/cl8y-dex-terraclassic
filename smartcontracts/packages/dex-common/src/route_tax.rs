//! Versioned route-local funding protocol (#1381). `amount` is always a gross
//! route budget; sell tax retains its economic-input basis, buy tax its output basis.
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdError, StdResult, Uint128};

pub const FUNDING_VERSION: u16 = 1;
#[cw_serde]
pub enum RouteTaxKind {
    Sell,
    Buy,
}
#[cw_serde]
pub enum TokenQueryMsg {
    RouteTaxCapability {},
    RouteTaxPreview {
        kind: RouteTaxKind,
        amount: Uint128,
        trader: Option<String>,
        pair: String,
    },
}
#[cw_serde]
pub struct RouteTaxCapabilityResponse {
    pub funding_version: u16,
    pub router: Option<Addr>,
    pub buy_bps: u16,
    pub sell_bps: u16,
}
#[cw_serde]
pub struct RouteTaxPreviewResponse {
    pub funding_version: u16,
    pub gross: Uint128,
    pub economic: Uint128,
    pub tax: Uint128,
    pub net: Uint128,
}
#[cw_serde]
pub enum RouterQueryMsg {
    RouteTaxContext {},
}
#[cw_serde]
pub struct RouteTransfer {
    pub token: Addr,
    pub recipient: Addr,
    pub amount: Uint128,
}
#[cw_serde]
pub struct RouteTaxContextResponse {
    pub funding_version: u16,
    pub trader: Addr,
    pub pair: Addr,
    pub offer_token: Addr,
    pub ask_token: Addr,
    pub gross: Uint128,
    pub economic: Uint128,
    pub transfers: Vec<RouteTransfer>,
    /// Tokens already acquired and checked in this same authenticated route.
    pub acquired_tokens: Vec<Addr>,
}
#[cw_serde]
pub enum TokenExecuteMsg {
    RouteTransfer { recipient: String, amount: Uint128 },
}

/// Largest integer S with S + floor(S*bps/10000) <= G. A bounded binary search
/// uses subtraction to avoid overflowing S+tax even for Uint128::MAX budgets.
pub fn sell_from_gross(gross: Uint128, bps: u16) -> StdResult<RouteTaxPreviewResponse> {
    if bps > 10_000 {
        return Err(StdError::generic_err("route tax bps exceeds 10000"));
    }
    let (mut lo, mut hi) = (0u128, gross.u128());
    while lo < hi {
        let mid = lo + (hi - lo) / 2 + (hi - lo) % 2;
        let s = Uint128::new(mid);
        let tax = s.multiply_ratio(bps as u128, 10_000u128);
        if tax <= gross.checked_sub(s)? {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let economic = Uint128::new(lo);
    let tax = economic.multiply_ratio(bps as u128, 10_000u128);
    Ok(RouteTaxPreviewResponse {
        funding_version: FUNDING_VERSION,
        gross,
        economic,
        tax,
        net: economic,
    })
}
pub fn buy_from_gross(gross: Uint128, bps: u16) -> StdResult<RouteTaxPreviewResponse> {
    if bps > 10_000 {
        return Err(StdError::generic_err("route tax bps exceeds 10000"));
    }
    let tax = gross.multiply_ratio(bps as u128, 10_000u128);
    Ok(RouteTaxPreviewResponse {
        funding_version: FUNDING_VERSION,
        gross,
        economic: gross,
        tax,
        net: gross.checked_sub(tax)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximal_offer_and_extreme_conservation() {
        let p = sell_from_gross(Uint128::new(1_010_000), 100).unwrap();
        assert_eq!(p.economic, Uint128::new(1_000_000));
        assert_eq!(p.tax, Uint128::new(10_000));
        for bps in [0, 1, 100, 2500, 10_000] {
            for g in [0, 1, 99, 100, 101, 10_001, u128::MAX] {
                let p = sell_from_gross(Uint128::new(g), bps).unwrap();
                assert!(p.economic.checked_add(p.tax).unwrap() <= p.gross);
                if p.economic < p.gross {
                    let next = p.economic.checked_add(Uint128::one()).unwrap();
                    assert!(next
                        .checked_add(next.multiply_ratio(bps as u128, 10_000u128))
                        .map_or(true, |x| x > p.gross));
                }
            }
        }
    }
}
