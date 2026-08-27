//! Per-pair quote exposure and PnL. Net quote position is clamped to ≥ 0 after sells (`net_quote_after_sell`). See `docs/indexer-invariants.md`.
//!
//! Stored amounts are **raw chain NUMERIC** (GitLab **#551**):
//! - `net_position_quote` — raw **quote** (`asset_1`)
//! - `total_cost_base` / `realized_pnl` — raw **base** (`asset_0`)
//! - `avg_entry_price` — raw base / raw quote (not human, not USD)
//!
//! JSON keeps those raw strings. The dApp scales with `asset_*_decimals` from
//! `GET /api/v1/traders/{addr}/positions`. Do **not** sum `traders.total_realized_pnl`
//! across pairs as a single unit. Unrealized mark-to-market is dApp-side
//! (GitLab **#675**) from hub prices vs `total_cost_base` — not stored here.

use bigdecimal::BigDecimal;
use sqlx::PgPool;

use crate::db::queries::positions;
use crate::indexer::pair_price_usd::ten_pow_i32;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Human **base per 1 human quote** from the stored raw ratio (GitLab #551).
///
/// `human = raw_avg × 10^(decimals_quote − decimals_base)`.
/// Same-decimal 6/6 pairs are unchanged; mixed 6/18 pairs must be scaled
/// before display. Inverse of tape human quote-per-base ([#522] P522-1).
/// The dApp applies the same scale in `traderPositionDisplay.ts`.
#[allow(dead_code)] // Documented formula; dApp mirrors it in traderPositionDisplay.ts.
pub fn human_avg_entry_base_per_quote(
    raw_avg: &BigDecimal,
    decimals_base: i16,
    decimals_quote: i16,
) -> BigDecimal {
    raw_avg * ten_pow_i32(i32::from(decimals_quote) - i32::from(decimals_base))
}

/// After selling `offer_amount` of quote, clamp net quote position to ≥ 0.
pub(crate) fn net_quote_after_sell(old_pos: &BigDecimal, offer_amount: &BigDecimal) -> BigDecimal {
    let zero = BigDecimal::from(0);
    let raw = old_pos - offer_amount;
    if raw < zero {
        zero
    } else {
        raw
    }
}

/// Update trader position and P&L after a swap.
///
/// For a pair with asset_0 (base) and asset_1 (quote):
/// - Offering asset_0 = "buying quote" (opening/adding to position)
/// - Offering asset_1 = "selling quote" (closing/reducing position, realizing P&L)
pub async fn update_position_on_swap(
    pool: &PgPool,
    pair_id: i32,
    pair_asset_0_id: i32,
    sender: &str,
    offer_asset_id: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
    spread_amount: Option<&BigDecimal>,
    commission_amount: Option<&BigDecimal>,
) -> Result<(), BoxError> {
    let zero = BigDecimal::from(0);
    let fees = spread_amount.unwrap_or(&zero) + commission_amount.unwrap_or(&zero);

    let existing = positions::get_position(pool, sender, pair_id).await?;
    let (old_pos, old_avg, old_cost, old_rpnl, old_count) = match &existing {
        Some(p) => (
            p.net_position_quote.clone(),
            p.avg_entry_price.clone(),
            p.total_cost_base.clone(),
            p.realized_pnl.clone(),
            p.trade_count,
        ),
        None => (zero.clone(), zero.clone(), zero.clone(), zero.clone(), 0),
    };

    let buying_quote = offer_asset_id == pair_asset_0_id;

    if buying_quote {
        // Offering base (asset_0), receiving quote (asset_1) -> open/add position
        let new_pos = &old_pos + return_amount;
        let new_cost = &old_cost + offer_amount;
        let new_avg = if new_pos > zero {
            &new_cost / &new_pos
        } else {
            zero.clone()
        };

        positions::upsert_position(
            pool,
            sender,
            pair_id,
            &new_pos,
            &new_avg,
            &new_cost,
            &old_rpnl,
            old_count + 1,
        )
        .await?;

        // No realized P&L on buys, just accumulate fees
        positions::update_trader_fees_only(pool, sender, &fees).await?;
    } else {
        // Offering quote (asset_1), receiving base (asset_0) -> close/reduce position
        let exit_price = if offer_amount > &zero {
            return_amount / offer_amount
        } else {
            zero.clone()
        };

        let trade_pnl = (&exit_price - &old_avg) * offer_amount;
        let new_rpnl = &old_rpnl + &trade_pnl;
        let new_pos = net_quote_after_sell(&old_pos, offer_amount);

        let (new_cost, new_avg) = if new_pos > zero {
            let cost = &new_pos * &old_avg;
            let avg = old_avg.clone();
            (cost, avg)
        } else {
            (zero.clone(), zero.clone())
        };

        positions::upsert_position(
            pool,
            sender,
            pair_id,
            &new_pos,
            &new_avg,
            &new_cost,
            &new_rpnl,
            old_count + 1,
        )
        .await?;

        positions::update_trader_pnl(pool, sender, &trade_pnl, &fees).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn net_quote_clamped_when_oversold() {
        let old = BigDecimal::from_str("50").unwrap();
        let sell = BigDecimal::from_str("100").unwrap();
        assert_eq!(net_quote_after_sell(&old, &sell), BigDecimal::from(0));
    }

    #[test]
    fn net_quote_partial_close() {
        let old = BigDecimal::from_str("100").unwrap();
        let sell = BigDecimal::from_str("30").unwrap();
        assert_eq!(net_quote_after_sell(&old, &sell), BigDecimal::from(70));
    }

    #[test]
    fn net_quote_exact_close() {
        let old = BigDecimal::from_str("42").unwrap();
        let sell = BigDecimal::from_str("42").unwrap();
        assert_eq!(net_quote_after_sell(&old, &sell), BigDecimal::from(0));
    }

    #[test]
    fn human_avg_same_decimals_unchanged() {
        let raw = BigDecimal::from_str("0.00496").unwrap();
        assert_eq!(human_avg_entry_base_per_quote(&raw, 6, 6), raw);
    }

    #[test]
    fn human_avg_scales_mixed_6_18() {
        // 1 UST1 (1e6) for 80 USTR (80e18) → raw 1e6/80e18 = 1.25e-14
        let raw = BigDecimal::from_str("0.0000000000000125").unwrap();
        let human = human_avg_entry_base_per_quote(&raw, 6, 18);
        let expected = BigDecimal::from_str("0.0125").unwrap();
        assert_eq!(human, expected);
    }
}
