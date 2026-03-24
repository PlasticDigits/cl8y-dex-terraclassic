use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct TraderRow {
    pub id: i32,
    pub address: String,
    pub total_trades: i64,
    pub total_volume: BigDecimal,
    pub volume_24h: BigDecimal,
    pub volume_7d: BigDecimal,
    pub volume_30d: BigDecimal,
    pub tier_id: i16,
    pub tier_name: String,
    pub registered: bool,
    pub first_trade_at: Option<DateTime<Utc>>,
    pub last_trade_at: Option<DateTime<Utc>>,
    pub total_realized_pnl: BigDecimal,
    pub best_trade_pnl: BigDecimal,
    pub worst_trade_pnl: BigDecimal,
    pub total_fees_paid: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn upsert_trader(
    pool: &PgPool,
    address: &str,
    trade_volume: &BigDecimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, first_trade_at, last_trade_at)
         VALUES ($1, 1, $2, NOW(), NOW())
         ON CONFLICT (address)
           DO UPDATE SET total_trades = traders.total_trades + 1,
                        total_volume = traders.total_volume + $2,
                        first_trade_at = COALESCE(traders.first_trade_at, EXCLUDED.first_trade_at),
                        last_trade_at = NOW(),
                        updated_at = NOW()",
    )
    .bind(address)
    .bind(trade_volume)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_trader(pool: &PgPool, address: &str) -> Result<Option<TraderRow>, sqlx::Error> {
    sqlx::query_as::<_, TraderRow>("SELECT * FROM traders WHERE address = $1")
        .bind(address)
        .fetch_optional(pool)
        .await
}

pub async fn get_leaderboard(
    pool: &PgPool,
    sort_by: &str,
    limit: i64,
) -> Result<Vec<TraderRow>, sqlx::Error> {
    let order_col = match sort_by {
        "volume_24h" => "volume_24h",
        "volume_7d" => "volume_7d",
        "volume_30d" => "volume_30d",
        "total_trades" => "total_trades",
        "total_realized_pnl" => "total_realized_pnl",
        "best_trade_pnl" => "best_trade_pnl",
        "worst_trade_pnl" => "worst_trade_pnl",
        "total_fees_paid" => "total_fees_paid",
        _ => "total_volume",
    };

    let sql = format!("SELECT * FROM traders ORDER BY {} DESC LIMIT $1", order_col);

    sqlx::query_as::<_, TraderRow>(&sql)
        .bind(limit)
        .fetch_all(pool)
        .await
}

pub async fn update_trader_tier(
    pool: &PgPool,
    address: &str,
    tier_id: i16,
    tier_name: &str,
    registered: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE traders SET tier_id = $2, tier_name = $3, registered = $4, updated_at = NOW()
         WHERE address = $1",
    )
    .bind(address)
    .bind(tier_id)
    .bind(tier_name)
    .bind(registered)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn refresh_rolling_volumes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let cutoff_24h = now - chrono::Duration::hours(24);
    let cutoff_7d = now - chrono::Duration::days(7);
    let cutoff_30d = now - chrono::Duration::days(30);

    sqlx::query(
        "UPDATE traders t SET
           volume_24h = COALESCE(sub.vol_24h, 0),
           volume_7d  = COALESCE(sub.vol_7d, 0),
           volume_30d = COALESCE(sub.vol_30d, 0),
           updated_at = NOW()
         FROM (
           SELECT
             sender,
             SUM(CASE WHEN block_timestamp >= $1 THEN offer_amount ELSE 0 END) AS vol_24h,
             SUM(CASE WHEN block_timestamp >= $2 THEN offer_amount ELSE 0 END) AS vol_7d,
             SUM(CASE WHEN block_timestamp >= $3 THEN offer_amount ELSE 0 END) AS vol_30d
           FROM swap_events
           WHERE block_timestamp >= $3
           GROUP BY sender
         ) sub
         WHERE t.address = sub.sender",
    )
    .bind(cutoff_24h)
    .bind(cutoff_7d)
    .bind(cutoff_30d)
    .execute(pool)
    .await?;
    Ok(())
}
