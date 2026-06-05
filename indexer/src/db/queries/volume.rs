use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, FromRow)]
pub struct TokenVolumeRow {
    pub id: i64,
    pub asset_id: i32,
    pub window: String,
    pub volume: BigDecimal,
    pub volume_usd: BigDecimal,
    pub trade_count: i64,
    pub unique_traders: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalStats {
    pub total_volume_24h: BigDecimal,
    pub total_volume_24h_usd: BigDecimal,
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
            r#"INSERT INTO token_volume_stats (asset_id, "window", volume, volume_usd, trade_count, unique_traders, updated_at)
             SELECT
               offer_asset_id AS asset_id,
               $1 AS "window",
               SUM(offer_amount) AS volume,
               COALESCE(SUM(volume_usd), 0) AS volume_usd,
               COUNT(*) AS trade_count,
               COUNT(DISTINCT sender) AS unique_traders,
               NOW() AS updated_at
             FROM swap_events
             WHERE block_timestamp >= $2
             GROUP BY offer_asset_id
             ON CONFLICT (asset_id, "window")
               DO UPDATE SET volume = EXCLUDED.volume,
                            volume_usd = EXCLUDED.volume_usd,
                            trade_count = EXCLUDED.trade_count,
                            unique_traders = EXCLUDED.unique_traders,
                            updated_at = NOW()"#,
        )
        .bind(window)
        .bind(cutoff)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Rebuild rolling 24h quote-side volume per pair (materialized table for pair list sort).
pub async fn refresh_pair_volumes(pool: &PgPool) -> Result<(), sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    sqlx::query(
        r#"INSERT INTO pair_volume_24h (pair_id, volume_quote, updated_at)
           SELECT pair_id, SUM(return_amount), NOW()
           FROM swap_events
           WHERE block_timestamp >= $1
           GROUP BY pair_id
           ON CONFLICT (pair_id)
             DO UPDATE SET volume_quote = EXCLUDED.volume_quote,
                          updated_at = EXCLUDED.updated_at"#,
    )
    .bind(cutoff)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"UPDATE pair_volume_24h pv
           SET volume_quote = 0, updated_at = NOW()
           WHERE NOT EXISTS (
             SELECT 1 FROM swap_events se
             WHERE se.pair_id = pv.pair_id AND se.block_timestamp >= $1
           )"#,
    )
    .bind(cutoff)
    .execute(pool)
    .await?;

    Ok(())
}

/// Rebuild rolling 24h global overview stats (materialized single-row table).
pub async fn refresh_global_stats(pool: &PgPool) -> Result<(), sqlx::Error> {
    let cutoff = Utc::now() - chrono::Duration::hours(24);

    sqlx::query(
        r#"INSERT INTO global_stats_24h (id, total_volume, total_volume_usd, total_trades, updated_at)
           SELECT 1,
                  COALESCE(SUM(offer_amount), 0),
                  COALESCE(SUM(volume_usd), 0),
                  COUNT(*),
                  NOW()
           FROM swap_events
           WHERE block_timestamp >= $1
           ON CONFLICT (id)
             DO UPDATE SET total_volume = EXCLUDED.total_volume,
                          total_volume_usd = EXCLUDED.total_volume_usd,
                          total_trades = EXCLUDED.total_trades,
                          updated_at = EXCLUDED.updated_at"#,
    )
    .bind(cutoff)
    .execute(pool)
    .await?;

    Ok(())
}

fn overview_global_stats_live_query() -> bool {
    std::env::var("OVERVIEW_GLOBAL_STATS_LIVE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub async fn get_token_volume(
    pool: &PgPool,
    asset_id: i32,
) -> Result<Vec<TokenVolumeRow>, sqlx::Error> {
    sqlx::query_as::<_, TokenVolumeRow>(
        r#"SELECT * FROM token_volume_stats WHERE asset_id = $1 ORDER BY "window""#,
    )
    .bind(asset_id)
    .fetch_all(pool)
    .await
}

/// Live aggregate over `swap_events` (debug / parity checks). Set `OVERVIEW_GLOBAL_STATS_LIVE=1`.
pub async fn get_global_stats_live(pool: &PgPool) -> Result<GlobalStats, sqlx::Error> {
    let cutoff_24h = Utc::now() - chrono::Duration::hours(24);

    #[derive(FromRow)]
    struct AggRow {
        total_volume: Option<BigDecimal>,
        total_volume_usd: Option<BigDecimal>,
        total_trades: Option<i64>,
    }

    let agg = sqlx::query_as::<_, AggRow>(
        "SELECT SUM(offer_amount) AS total_volume,
                COALESCE(SUM(volume_usd), 0) AS total_volume_usd,
                COUNT(*) AS total_trades
         FROM swap_events WHERE block_timestamp >= $1",
    )
    .bind(cutoff_24h)
    .fetch_one(pool)
    .await?;

    let pair_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs")
        .fetch_one(pool)
        .await?;

    Ok(GlobalStats {
        total_volume_24h: agg.total_volume.unwrap_or_default(),
        total_volume_24h_usd: agg.total_volume_usd.unwrap_or_default(),
        total_trades_24h: agg.total_trades.unwrap_or(0),
        pair_count,
    })
}

pub async fn get_global_stats(pool: &PgPool) -> Result<GlobalStats, sqlx::Error> {
    if overview_global_stats_live_query() {
        return get_global_stats_live(pool).await;
    }

    #[derive(FromRow)]
    struct RollupRow {
        total_volume: BigDecimal,
        total_volume_usd: BigDecimal,
        total_trades: i64,
    }

    let rollup = sqlx::query_as::<_, RollupRow>(
        "SELECT total_volume, total_volume_usd, total_trades FROM global_stats_24h WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;

    let pair_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs")
        .fetch_one(pool)
        .await?;

    Ok(GlobalStats {
        total_volume_24h: rollup.total_volume,
        total_volume_24h_usd: rollup.total_volume_usd,
        total_trades_24h: rollup.total_trades,
        pair_count,
    })
}
