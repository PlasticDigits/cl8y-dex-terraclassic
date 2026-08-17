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
    pub total_volume_7d_usd: BigDecimal,
    pub total_volume_30d_usd: BigDecimal,
    pub total_trades_7d: i64,
    pub total_trades_30d: i64,
    pub active_pairs_24h: i64,
    pub unique_traders_24h: i64,
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
               LEAST(COALESCE(SUM(offer_amount), 0), POWER(10::numeric, 38) - 1) AS volume,
               LEAST(COALESCE(SUM(volume_usd), 0), POWER(10::numeric, 20) - POWER(10::numeric, -18)) AS volume_usd,
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
           SELECT se.pair_id,
                  LEAST(
                    COALESCE(SUM(CASE WHEN se.offer_asset_id = p.asset_0_id THEN se.return_amount ELSE se.offer_amount END), 0),
                    POWER(10::numeric, 38) - 1
                  ),
                  NOW()
           FROM swap_events se
           INNER JOIN pairs p ON p.id = se.pair_id
           WHERE se.block_timestamp >= $1
           GROUP BY se.pair_id
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

/// Rebuild rolling 24h/7d/30d global overview stats (materialized single-row table).
/// Aggregates run here (~5 min), never on the /overview request path (GitLab #550 / AC7).
pub async fn refresh_global_stats(pool: &PgPool) -> Result<(), sqlx::Error> {
    let now = Utc::now();
    let cutoff_24h = now - chrono::Duration::hours(24);
    let cutoff_7d = now - chrono::Duration::days(7);
    let cutoff_30d = now - chrono::Duration::days(30);

    sqlx::query(
        r#"INSERT INTO global_stats_24h (
               id, total_volume, total_volume_usd, total_trades, updated_at,
               total_volume_7d_usd, total_volume_30d_usd,
               total_trades_7d, total_trades_30d,
               active_pairs_24h, unique_traders_24h
           )
           SELECT 1,
                  LEAST(COALESCE(SUM(offer_amount) FILTER (WHERE block_timestamp >= $1), 0), POWER(10::numeric, 38) - 1),
                  LEAST(COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $1), 0), POWER(10::numeric, 20) - POWER(10::numeric, -18)),
                  COUNT(*) FILTER (WHERE block_timestamp >= $1),
                  NOW(),
                  LEAST(COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $2), 0), POWER(10::numeric, 20) - POWER(10::numeric, -18)),
                  LEAST(COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $3), 0), POWER(10::numeric, 20) - POWER(10::numeric, -18)),
                  COUNT(*) FILTER (WHERE block_timestamp >= $2),
                  COUNT(*) FILTER (WHERE block_timestamp >= $3),
                  COUNT(DISTINCT pair_id) FILTER (WHERE block_timestamp >= $1),
                  COUNT(DISTINCT sender) FILTER (WHERE block_timestamp >= $1)
           FROM swap_events
           WHERE block_timestamp >= $3
           ON CONFLICT (id)
             DO UPDATE SET total_volume = EXCLUDED.total_volume,
                          total_volume_usd = EXCLUDED.total_volume_usd,
                          total_trades = EXCLUDED.total_trades,
                          updated_at = EXCLUDED.updated_at,
                          total_volume_7d_usd = EXCLUDED.total_volume_7d_usd,
                          total_volume_30d_usd = EXCLUDED.total_volume_30d_usd,
                          total_trades_7d = EXCLUDED.total_trades_7d,
                          total_trades_30d = EXCLUDED.total_trades_30d,
                          active_pairs_24h = EXCLUDED.active_pairs_24h,
                          unique_traders_24h = EXCLUDED.unique_traders_24h"#,
    )
    .bind(cutoff_24h)
    .bind(cutoff_7d)
    .bind(cutoff_30d)
    .execute(pool)
    .await?;

    Ok(())
}

/// Recompute `swap_events.volume_usd` from stored amounts + P522-Q catalog + latest oracles.
/// Idempotent — a second run does not double USD (GitLab #548 **I13** / **A13**).
/// Keep SQL in sync with `indexer/migrations/20260817120000_backfill_swap_volume_usd_catalog.sql`.
pub async fn backfill_swap_volume_usd(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        r#"
WITH latest_oracle AS (
    SELECT DISTINCT ON (ticker) ticker, price_usd
    FROM oracle_prices
    WHERE source = 'average'
    ORDER BY ticker, fetched_at DESC
),
ustc AS (
    SELECT price_usd FROM latest_oracle WHERE ticker = 'ustc'
),
lunc AS (
    SELECT price_usd FROM latest_oracle WHERE ticker = 'lunc'
),
catalog AS (
    SELECT
        a.id,
        a.decimals,
        CASE
            WHEN a.denom = 'uusd' THEN (SELECT price_usd FROM ustc)
            WHEN a.denom = 'uluna' THEN (SELECT price_usd FROM lunc)
            WHEN NOT a.is_cw20 AND a.denom IS NOT NULL AND a.denom NOT IN ('uusd', 'uluna') THEN NULL
            WHEN UPPER(a.symbol) = 'UST1' AND a.contract_address IS NOT NULL THEN 1::numeric
            WHEN UPPER(a.symbol) IN ('USTC', 'CUSTC')
                 AND (a.denom = 'uusd' OR a.contract_address IS NOT NULL)
                THEN (SELECT price_usd FROM ustc)
            WHEN UPPER(a.symbol) IN ('LUNC', 'CLUNC')
                 AND (a.denom = 'uluna' OR a.contract_address IS NOT NULL)
                THEN (SELECT price_usd FROM lunc)
            WHEN UPPER(a.symbol) = 'USTR' AND a.contract_address IS NOT NULL
                THEN 2.5::numeric * (SELECT price_usd FROM ustc)
            ELSE NULL
        END AS usd_per_human
    FROM assets a
),
priced AS (
    SELECT
        se.id,
        CASE
            WHEN q.usd_per_human IS NOT NULL AND q.usd_per_human > 0
                 AND q.decimals BETWEEN 0 AND 38 THEN
                CASE
                    WHEN se.offer_asset_id = p.asset_1_id THEN
                        se.offer_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
                    ELSE
                        se.return_amount / POWER(10::numeric, q.decimals) * q.usd_per_human
                END
            WHEN o.usd_per_human IS NOT NULL AND o.usd_per_human > 0
                 AND o.decimals BETWEEN 0 AND 38 THEN
                se.offer_amount / POWER(10::numeric, o.decimals) * o.usd_per_human
            WHEN k.usd_per_human IS NOT NULL AND k.usd_per_human > 0
                 AND k.decimals BETWEEN 0 AND 38 THEN
                se.return_amount / POWER(10::numeric, k.decimals) * k.usd_per_human
            ELSE NULL
        END AS raw_usd
    FROM swap_events se
    JOIN pairs p ON p.id = se.pair_id
    JOIN catalog q ON q.id = p.asset_1_id
    JOIN catalog o ON o.id = se.offer_asset_id
    JOIN catalog k ON k.id = se.ask_asset_id
)
UPDATE swap_events se
SET volume_usd = CASE
    WHEN pr.raw_usd IS NULL OR pr.raw_usd <= 0 OR pr.raw_usd >= POWER(10::numeric, 20) THEN NULL
    ELSE pr.raw_usd
END
FROM priced pr
WHERE se.id = pr.id
"#,
    )
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
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
    let now = Utc::now();
    let cutoff_24h = now - chrono::Duration::hours(24);
    let cutoff_7d = now - chrono::Duration::days(7);
    let cutoff_30d = now - chrono::Duration::days(30);

    #[derive(FromRow)]
    struct AggRow {
        total_volume: Option<BigDecimal>,
        total_volume_usd: Option<BigDecimal>,
        total_trades: Option<i64>,
        total_volume_7d_usd: Option<BigDecimal>,
        total_volume_30d_usd: Option<BigDecimal>,
        total_trades_7d: Option<i64>,
        total_trades_30d: Option<i64>,
        active_pairs_24h: Option<i64>,
        unique_traders_24h: Option<i64>,
    }

    let agg = sqlx::query_as::<_, AggRow>(
        "SELECT
            SUM(offer_amount) FILTER (WHERE block_timestamp >= $1) AS total_volume,
            COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $1), 0) AS total_volume_usd,
            COUNT(*) FILTER (WHERE block_timestamp >= $1) AS total_trades,
            COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $2), 0) AS total_volume_7d_usd,
            COALESCE(SUM(volume_usd) FILTER (WHERE block_timestamp >= $3), 0) AS total_volume_30d_usd,
            COUNT(*) FILTER (WHERE block_timestamp >= $2) AS total_trades_7d,
            COUNT(*) FILTER (WHERE block_timestamp >= $3) AS total_trades_30d,
            COUNT(DISTINCT pair_id) FILTER (WHERE block_timestamp >= $1) AS active_pairs_24h,
            COUNT(DISTINCT sender) FILTER (WHERE block_timestamp >= $1) AS unique_traders_24h
         FROM swap_events WHERE block_timestamp >= $3",
    )
    .bind(cutoff_24h)
    .bind(cutoff_7d)
    .bind(cutoff_30d)
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
        total_volume_7d_usd: agg.total_volume_7d_usd.unwrap_or_default(),
        total_volume_30d_usd: agg.total_volume_30d_usd.unwrap_or_default(),
        total_trades_7d: agg.total_trades_7d.unwrap_or(0),
        total_trades_30d: agg.total_trades_30d.unwrap_or(0),
        active_pairs_24h: agg.active_pairs_24h.unwrap_or(0),
        unique_traders_24h: agg.unique_traders_24h.unwrap_or(0),
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
        total_volume_7d_usd: BigDecimal,
        total_volume_30d_usd: BigDecimal,
        total_trades_7d: i64,
        total_trades_30d: i64,
        active_pairs_24h: i64,
        unique_traders_24h: i64,
    }

    let rollup = sqlx::query_as::<_, RollupRow>(
        "SELECT total_volume, total_volume_usd, total_trades,
                total_volume_7d_usd, total_volume_30d_usd,
                total_trades_7d, total_trades_30d,
                active_pairs_24h, unique_traders_24h
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;

    // Migration seeds id=1 with zeros; refresh runs after pair sync. Fall back to a live
    // aggregate when the rollup is still uninitialized but swap_events has 24h data.
    if rollup.total_trades == 0 {
        let cutoff_24h = Utc::now() - chrono::Duration::hours(24);
        let has_recent_swaps: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM swap_events WHERE block_timestamp >= $1)",
        )
        .bind(cutoff_24h)
        .fetch_one(pool)
        .await?;

        if has_recent_swaps {
            return get_global_stats_live(pool).await;
        }
    }

    let pair_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pairs")
        .fetch_one(pool)
        .await?;

    Ok(GlobalStats {
        total_volume_24h: rollup.total_volume,
        total_volume_24h_usd: rollup.total_volume_usd,
        total_trades_24h: rollup.total_trades,
        pair_count,
        total_volume_7d_usd: rollup.total_volume_7d_usd,
        total_volume_30d_usd: rollup.total_volume_30d_usd,
        total_trades_7d: rollup.total_trades_7d,
        total_trades_30d: rollup.total_trades_30d,
        active_pairs_24h: rollup.active_pairs_24h,
        unique_traders_24h: rollup.unique_traders_24h,
    })
}
