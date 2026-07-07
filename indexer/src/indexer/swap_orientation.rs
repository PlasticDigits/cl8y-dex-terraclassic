//! Canonical swap price and volume orientation at index time.
//!
//! **Invariant (GitLab #466):** `asset_0` = base, `asset_1` = quote. Stored `swap_events.price`
//! and candle OHLC are always **quote per base** regardless of which leg was offered. Volume
//! buckets sum base/quote legs by asset id, not raw offer/return columns.

use bigdecimal::BigDecimal;

/// Normalized swap leg: quote-per-base price and oriented volume buckets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientedSwapLeg {
    pub price: BigDecimal,
    pub volume_base: BigDecimal,
    pub volume_quote: BigDecimal,
}

/// Map a raw swap to canonical quote-per-base price and base/quote volumes.
///
/// - Offer base (`asset_0`): `price = return / offer`, volumes = offer / return.
/// - Offer quote (`asset_1`): `price = offer / return`, volumes = return / offer.
pub fn orient_swap_leg(
    pair_asset_0_id: i32,
    offer_asset_id: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> OrientedSwapLeg {
    let zero = BigDecimal::from(0);
    let offer_is_base = offer_asset_id == pair_asset_0_id;

    if offer_is_base {
        let price = if offer_amount > &zero {
            return_amount / offer_amount
        } else {
            zero.clone()
        };
        OrientedSwapLeg {
            price,
            volume_base: offer_amount.clone(),
            volume_quote: return_amount.clone(),
        }
    } else {
        let price = if return_amount > &zero {
            offer_amount / return_amount
        } else {
            zero.clone()
        };
        OrientedSwapLeg {
            price,
            volume_base: return_amount.clone(),
            volume_quote: offer_amount.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn offer_base_price_is_return_over_offer() {
        let leg = orient_swap_leg(1, 1, &bd("100"), &bd("5050"));
        assert_eq!(leg.price, bd("50.5"));
        assert_eq!(leg.volume_base, bd("100"));
        assert_eq!(leg.volume_quote, bd("5050"));
    }

    #[test]
    fn offer_quote_inverts_to_same_quote_per_base() {
        let forward = orient_swap_leg(1, 1, &bd("100"), &bd("5050"));
        let reverse = orient_swap_leg(1, 2, &bd("5050"), &bd("100"));
        assert_eq!(forward.price, reverse.price);
        assert_eq!(reverse.volume_base, bd("100"));
        assert_eq!(reverse.volume_quote, bd("5050"));
    }

    #[test]
    fn zero_offer_yields_zero_price() {
        let leg = orient_swap_leg(1, 1, &bd("0"), &bd("100"));
        assert_eq!(leg.price, BigDecimal::from(0));
    }
}
