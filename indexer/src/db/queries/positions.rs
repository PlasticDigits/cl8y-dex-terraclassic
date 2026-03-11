use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct PositionRow {
    pub id: i32,
    pub trader_address: String,
    pub pair_id: i32,
    pub net_position_quote: BigDecimal,
    pub avg_entry_price: BigDecimal,
    pub total_cost_base: BigDecimal,
    pub realized_pnl: BigDecimal,
    pub trade_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_position(
    pool: &PgPool,
    trader_address: &str,
    pair_id: i32,
) -> Result<Option<PositionRow>, sqlx::Error> {
    sqlx::query_as::<_, PositionRow>(
        "SELECT * FROM trader_positions WHERE trader_address = $1 AND pair_id = $2",
    )
    .bind(trader_address)
    .bind(pair_id)
    .fetch_optional(pool)
    .await
}

pub async fn upsert_position(
    pool: &PgPool,
    trader_address: &str,
    pair_id: i32,
    net_position_quote: &BigDecimal,
    avg_entry_price: &BigDecimal,
    total_cost_base: &BigDecimal,
    realized_pnl: &BigDecimal,
    trade_count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO trader_positions
         (trader_address, pair_id, net_position_quote, avg_entry_price,
          total_cost_base, realized_pnl, trade_count)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (trader_address, pair_id)
           DO UPDATE SET net_position_quote = $3,
                        avg_entry_price = $4,
                        total_cost_base = $5,
                        realized_pnl = $6,
                        trade_count = $7,
                        updated_at = NOW()",
    )
    .bind(trader_address)
    .bind(pair_id)
    .bind(net_position_quote)
    .bind(avg_entry_price)
    .bind(total_cost_base)
    .bind(realized_pnl)
    .bind(trade_count)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_positions_for_trader(
    pool: &PgPool,
    trader_address: &str,
) -> Result<Vec<PositionRow>, sqlx::Error> {
    sqlx::query_as::<_, PositionRow>(
        "SELECT * FROM trader_positions WHERE trader_address = $1 ORDER BY pair_id",
    )
    .bind(trader_address)
    .fetch_all(pool)
    .await
}

pub async fn update_trader_pnl(
    pool: &PgPool,
    address: &str,
    trade_pnl: &BigDecimal,
    fees: &BigDecimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE traders SET
           total_realized_pnl = total_realized_pnl + $2,
           best_trade_pnl = GREATEST(best_trade_pnl, $2),
           worst_trade_pnl = LEAST(worst_trade_pnl, $2),
           total_fees_paid = total_fees_paid + $3,
           updated_at = NOW()
         WHERE address = $1",
    )
    .bind(address)
    .bind(trade_pnl)
    .bind(fees)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_trader_fees_only(
    pool: &PgPool,
    address: &str,
    fees: &BigDecimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE traders SET
           total_fees_paid = total_fees_paid + $2,
           updated_at = NOW()
         WHERE address = $1",
    )
    .bind(address)
    .bind(fees)
    .execute(pool)
    .await?;
    Ok(())
}
