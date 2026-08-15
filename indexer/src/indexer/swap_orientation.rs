//! Canonical swap price and volume orientation at index time.
//!
//! **Invariant (GitLab #466):** `asset_0` = base, `asset_1` = quote. Stored `swap_events.price`
//! and candle OHLC orientation are always **quote per base** regardless of which leg was offered.
//!
//! **Invariant (GitLab #522):** `swap_events.price` is **human** quote-per-base
//! (`raw_ratio × 10^(decimals_base − decimals_quote)`). Volume buckets stay in raw chain units.
//! USD of 1 human base is a separate field (`price_usd`) — see [`pair_price_usd`](super::pair_price_usd).

use bigdecimal::BigDecimal;

use super::pair_price_usd::human_price_scale;

/// Normalized swap leg: human quote-per-base price and oriented volume buckets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrientedSwapLeg {
    pub price: BigDecimal,
    pub volume_base: BigDecimal,
    pub volume_quote: BigDecimal,
}

/// Map a raw swap to canonical **human** quote-per-base price and base/quote volumes.
///
/// - Offer base (`asset_0`): raw `price = return / offer`, volumes = offer / return.
/// - Offer quote (`asset_1`): raw `price = offer / return`, volumes = return / offer.
/// - Then scale by `10^(decimals_base − decimals_quote)`.
pub fn orient_swap_leg(
    pair_asset_0_id: i32,
    offer_asset_id: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
    decimals_base: i16,
    decimals_quote: i16,
) -> OrientedSwapLeg {
    let zero = BigDecimal::from(0);
    let offer_is_base = offer_asset_id == pair_asset_0_id;
    let scale = human_price_scale(decimals_base, decimals_quote);

    if offer_is_base {
        let raw = if offer_amount > &zero {
            return_amount / offer_amount
        } else {
            zero.clone()
        };
        OrientedSwapLeg {
            price: raw * scale,
            volume_base: offer_amount.clone(),
            volume_quote: return_amount.clone(),
        }
    } else {
        let raw = if return_amount > &zero {
            offer_amount / return_amount
        } else {
            zero.clone()
        };
        OrientedSwapLeg {
            price: raw * scale,
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
        let leg = orient_swap_leg(1, 1, &bd("100"), &bd("5050"), 6, 6);
        assert_eq!(leg.price, bd("50.5"));
        assert_eq!(leg.volume_base, bd("100"));
        assert_eq!(leg.volume_quote, bd("5050"));
    }

    #[test]
    fn offer_quote_inverts_to_same_quote_per_base() {
        let forward = orient_swap_leg(1, 1, &bd("100"), &bd("5050"), 6, 6);
        let reverse = orient_swap_leg(1, 2, &bd("5050"), &bd("100"), 6, 6);
        assert_eq!(forward.price, reverse.price);
        assert_eq!(reverse.volume_base, bd("100"));
        assert_eq!(reverse.volume_quote, bd("5050"));
    }

    #[test]
    fn zero_offer_yields_zero_price() {
        let leg = orient_swap_leg(1, 1, &bd("0"), &bd("100"), 6, 6);
        assert_eq!(leg.price, BigDecimal::from(0));
    }

    #[test]
    fn mixed_decimals_6_vs_18_scales_ustr_print() {
        // UST1 (6) → USTR (18): raw 9297047794755092035 / 116624
        let forward = orient_swap_leg(
            1,
            1,
            &bd("116624"),
            &bd("9297047794755092035"),
            6,
            18,
        );
        let reverse = orient_swap_leg(
            1,
            2,
            &bd("9297047794755092035"),
            &bd("116624"),
            6,
            18,
        );
        assert_eq!(forward.price, reverse.price);
        let human = forward.price.to_string().parse::<f64>().unwrap();
        assert!(
            (human - 79.72).abs() < 0.02,
            "expected ~79.72 USTR/UST1, got {human}"
        );
        assert!(human < 1_000.0, "must not remain raw 1e13 scale");
    }

    #[test]
    fn same_decimals_6_vs_6_leaves_ratio() {
        let leg = orient_swap_leg(1, 1, &bd("116651"), &bd("24102109"), 6, 6);
        let human = leg.price.to_string().parse::<f64>().unwrap();
        assert!((human - 206.62).abs() < 0.02, "got {human}");
    }
}
