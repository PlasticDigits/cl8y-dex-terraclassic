use bigdecimal::BigDecimal;
use sqlx::PgPool;

use crate::db::queries::positions;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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
        let new_pos = &old_pos - offer_amount;

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
