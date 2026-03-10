use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct TokenVolumeRow {
    pub id: i64,
    pub asset_id: i32,
    pub window: String,
    pub volume: BigDecimal,
    pub trade_count: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalStats {
    pub total_volume_24h: BigDecimal,
    pub total_trades_24h: i64,
    pub pair_count: i64,
}

pub async fn refresh_token_volumes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let cutoff_24h = now - chrono::Duration::hours(24);
    let cutoff_7d = now - chrono::Duration::days(7);
    let cutoff_30d = now - chrono::Duration::days(30);

    for (window, cutoff) in [("24h", cutoff_24h), ("7d", cutoff_7d), ("30d", cutoff_30d)] {
        sqlx::query(
            "INSERT INTO token_volume_stats (asset_id, window, volume, trade_count, updated_at)
             SELECT
               offer_asset_id AS asset_id,
               $1 AS window,
               SUM(offer_amount) AS volume,
               COUNT(*) AS trade_count,
               NOW() AS updated_at
             FROM swap_events
             WHERE block_timestamp >= $2
             GROUP BY offer_asset_id
             ON CONFLICT (asset_id, window)
               DO UPDATE SET volume = EXCLUDED.volume,
                            trade_count = EXCLUDED.trade_count,
                            updated_at = NOW()",
        )
        .bind(window)
        .bind(cutoff)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_token_volume(
    pool: &PgPool,
    asset_id: i32,
) -> Result<Vec<TokenVolumeRow>, sqlx::Error> {
    sqlx::query_as::<_, TokenVolumeRow>(
        "SELECT * FROM token_volume_stats WHERE asset_id = $1 ORDER BY window",
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
}

pub async fn get_global_stats(pool: &PgPool) -> Result<GlobalStats, sqlx::Error> {
    let cutoff_24h = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct AggRow {
        total_volume: Option<BigDecimal>,
        total_trades: Option<i64>,
    }

    let agg = sqlx::query_as::<_, AggRow>(
        "SELECT SUM(offer_amount) AS total_volume, COUNT(*) AS total_trades
         FROM swap_events WHERE block_timestamp >= $1",
    )
    .bind(cutoff_24h)
    .fetch_one(pool)
    .await?;

    let pair_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs")
            .fetch_one(pool)
            .await?;

    Ok(GlobalStats {
        total_volume_24h: agg.total_volume.unwrap_or_default(),
        total_trades_24h: agg.total_trades.unwrap_or(0),
        pair_count,
    })
}
