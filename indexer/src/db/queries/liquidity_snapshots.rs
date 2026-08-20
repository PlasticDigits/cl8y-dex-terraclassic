//! Materialized protocol pool TVL on `global_stats_24h` + snapshot history (GitLab #569).
//!
//! GET `/overview` reads these columns only — never walks `global_liquidity_snapshots`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};

/// ±30 min window around now-24h / now-30d when picking a Δ% baseline.
pub const SNAPSHOT_LOOKUP_TOLERANCE: Duration = Duration::minutes(30);

/// Keep enough history that a 30d lookup still succeeds (issue: retain ≥ 35 days).
pub const SNAPSHOT_RETENTION: Duration = Duration::days(35);

/// Bound snapshot-table growth; hub refresh is ~10s, TVL snapshots stay ~5 min.
pub const SNAPSHOT_MIN_INTERVAL: Duration = Duration::minutes(4);

#[derive(Debug, Clone, Default)]
pub struct LiquidityRollup {
    pub total_liquidity_usd: BigDecimal,
    pub liquidity_change_24h_pct: Option<BigDecimal>,
    pub liquidity_change_30d_pct: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub unpriced_pair_count: i32,
    pub total_liquidity_usd_24h_ago: Option<BigDecimal>,
    pub total_liquidity_usd_30d_ago: Option<BigDecimal>,
}

#[derive(Debug, Clone)]
pub struct LiquiditySnapshot {
    pub sampled_at: DateTime<Utc>,
    pub total_liquidity_usd: BigDecimal,
    pub priced_pair_count: i32,
}

/// Persist current TVL + Δ% on the single `global_stats_24h` row without touching volume columns.
pub async fn upsert_liquidity_rollup(
    pool: &PgPool,
    rollup: &LiquidityRollup,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO global_stats_24h (
               id, total_volume, total_volume_usd, total_trades, updated_at,
               total_liquidity_usd, liquidity_change_24h_pct, liquidity_change_30d_pct,
               priced_pair_count, unpriced_pair_count,
               total_liquidity_usd_24h_ago, total_liquidity_usd_30d_ago
           )
           VALUES (
               1, 0, 0, 0, NOW(),
               $1, $2, $3, $4, $5, $6, $7
           )
           ON CONFLICT (id) DO UPDATE SET
               total_liquidity_usd = EXCLUDED.total_liquidity_usd,
               liquidity_change_24h_pct = EXCLUDED.liquidity_change_24h_pct,
               liquidity_change_30d_pct = EXCLUDED.liquidity_change_30d_pct,
               priced_pair_count = EXCLUDED.priced_pair_count,
               unpriced_pair_count = EXCLUDED.unpriced_pair_count,
               total_liquidity_usd_24h_ago = EXCLUDED.total_liquidity_usd_24h_ago,
               total_liquidity_usd_30d_ago = EXCLUDED.total_liquidity_usd_30d_ago"#,
    )
    .bind(&rollup.total_liquidity_usd)
    .bind(&rollup.liquidity_change_24h_pct)
    .bind(&rollup.liquidity_change_30d_pct)
    .bind(rollup.priced_pair_count)
    .bind(rollup.unpriced_pair_count)
    .bind(&rollup.total_liquidity_usd_24h_ago)
    .bind(&rollup.total_liquidity_usd_30d_ago)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_liquidity_rollup(pool: &PgPool) -> Result<LiquidityRollup, sqlx::Error> {
    #[derive(FromRow)]
    struct Row {
        total_liquidity_usd: BigDecimal,
        liquidity_change_24h_pct: Option<BigDecimal>,
        liquidity_change_30d_pct: Option<BigDecimal>,
        priced_pair_count: i32,
        unpriced_pair_count: i32,
        total_liquidity_usd_24h_ago: Option<BigDecimal>,
        total_liquidity_usd_30d_ago: Option<BigDecimal>,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT total_liquidity_usd, liquidity_change_24h_pct, liquidity_change_30d_pct,
                priced_pair_count, unpriced_pair_count,
                total_liquidity_usd_24h_ago, total_liquidity_usd_30d_ago
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row
        .map(|r| LiquidityRollup {
            total_liquidity_usd: r.total_liquidity_usd,
            liquidity_change_24h_pct: r.liquidity_change_24h_pct,
            liquidity_change_30d_pct: r.liquidity_change_30d_pct,
            priced_pair_count: r.priced_pair_count,
            unpriced_pair_count: r.unpriced_pair_count,
            total_liquidity_usd_24h_ago: r.total_liquidity_usd_24h_ago,
            total_liquidity_usd_30d_ago: r.total_liquidity_usd_30d_ago,
        })
        .unwrap_or_default())
}

pub async fn insert_snapshot(
    pool: &PgPool,
    sampled_at: DateTime<Utc>,
    total_liquidity_usd: &BigDecimal,
    priced_pair_count: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO global_liquidity_snapshots (sampled_at, total_liquidity_usd, priced_pair_count)
           VALUES ($1, $2, $3)
           ON CONFLICT (sampled_at) DO UPDATE SET
               total_liquidity_usd = EXCLUDED.total_liquidity_usd,
               priced_pair_count = EXCLUDED.priced_pair_count"#,
    )
    .bind(sampled_at)
    .bind(total_liquidity_usd)
    .bind(priced_pair_count)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn maybe_insert_snapshot(
    pool: &PgPool,
    now: DateTime<Utc>,
    total_liquidity_usd: &BigDecimal,
    priced_pair_count: i32,
) -> Result<bool, sqlx::Error> {
    let last: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MAX(sampled_at) FROM global_liquidity_snapshots")
            .fetch_one(pool)
            .await?;
    if let Some(last) = last {
        if now.signed_duration_since(last) < SNAPSHOT_MIN_INTERVAL {
            return Ok(false);
        }
    }
    insert_snapshot(pool, now, total_liquidity_usd, priced_pair_count).await?;
    Ok(true)
}

/// Nearest snapshot to `target` within ±[`SNAPSHOT_LOOKUP_TOLERANCE`]. None if the window is empty.
pub async fn nearest_snapshot(
    pool: &PgPool,
    target: DateTime<Utc>,
) -> Result<Option<LiquiditySnapshot>, sqlx::Error> {
    let lo = target - SNAPSHOT_LOOKUP_TOLERANCE;
    let hi = target + SNAPSHOT_LOOKUP_TOLERANCE;
    sqlx::query_as::<_, (DateTime<Utc>, BigDecimal, i32)>(
        "SELECT sampled_at, total_liquidity_usd, priced_pair_count
         FROM global_liquidity_snapshots
         WHERE sampled_at >= $1 AND sampled_at <= $2
         ORDER BY ABS(EXTRACT(EPOCH FROM (sampled_at - $3))) ASC
         LIMIT 1",
    )
    .bind(lo)
    .bind(hi)
    .bind(target)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(
            |(sampled_at, total_liquidity_usd, priced_pair_count)| LiquiditySnapshot {
                sampled_at,
                total_liquidity_usd,
                priced_pair_count,
            },
        )
    })
}

pub async fn prune_snapshots(pool: &PgPool, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
    let cutoff = now - SNAPSHOT_RETENTION;
    let res = sqlx::query("DELETE FROM global_liquidity_snapshots WHERE sampled_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}
