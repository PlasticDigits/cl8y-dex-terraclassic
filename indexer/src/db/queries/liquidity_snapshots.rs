//! Protocol TVL quality and atomic publication (#1380). GET reads materialized rows.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgConnection, PgPool, Postgres, Transaction};

pub const SNAPSHOT_LOOKUP_TOLERANCE: Duration = Duration::minutes(30);
pub const SNAPSHOT_RETENTION: Duration = Duration::days(35);
pub const SNAPSHOT_MIN_INTERVAL: Duration = Duration::minutes(4);

#[derive(Debug, Clone)]
pub struct LiquidityQuality {
    pub status: String,
    pub as_of: Option<DateTime<Utc>>,
    pub coverage_identity: String,
    pub generation: i64,
    pub reasons: Value,
}
impl Default for LiquidityQuality {
    fn default() -> Self {
        Self {
            status: "unknown".into(),
            as_of: None,
            coverage_identity: String::new(),
            generation: 0,
            reasons: json!({"not_observed":1}),
        }
    }
}
impl LiquidityQuality {
    pub fn comparable(&self) -> bool {
        matches!(self.status.as_str(), "fresh" | "verified_empty")
    }
}
#[derive(Debug, Clone, Default)]
pub struct LiquidityRollup {
    pub total_liquidity_usd: Option<BigDecimal>,
    pub liquidity_change_24h_pct: Option<BigDecimal>,
    pub liquidity_change_30d_pct: Option<BigDecimal>,
    pub priced_pair_count: i32,
    pub unpriced_pair_count: i32,
    pub total_liquidity_usd_24h_ago: Option<BigDecimal>,
    pub total_liquidity_usd_30d_ago: Option<BigDecimal>,
    pub quality: LiquidityQuality,
}
#[derive(Debug, Clone)]
pub struct LiquiditySnapshot {
    pub sampled_at: DateTime<Utc>,
    pub total_liquidity_usd: Option<BigDecimal>,
    pub priced_pair_count: i32,
}
#[derive(FromRow)]
struct RollupRow {
    total_liquidity_usd: Option<BigDecimal>,
    liquidity_change_24h_pct: Option<BigDecimal>,
    liquidity_change_30d_pct: Option<BigDecimal>,
    priced_pair_count: i32,
    unpriced_pair_count: i32,
    total_liquidity_usd_24h_ago: Option<BigDecimal>,
    total_liquidity_usd_30d_ago: Option<BigDecimal>,
    liquidity_quality: String,
    liquidity_as_of: Option<DateTime<Utc>>,
    liquidity_coverage_identity: String,
    liquidity_generation: i64,
    liquidity_reasons: Value,
}
impl From<RollupRow> for LiquidityRollup {
    fn from(r: RollupRow) -> Self {
        Self {
            total_liquidity_usd: r.total_liquidity_usd,
            liquidity_change_24h_pct: r.liquidity_change_24h_pct,
            liquidity_change_30d_pct: r.liquidity_change_30d_pct,
            priced_pair_count: r.priced_pair_count,
            unpriced_pair_count: r.unpriced_pair_count,
            total_liquidity_usd_24h_ago: r.total_liquidity_usd_24h_ago,
            total_liquidity_usd_30d_ago: r.total_liquidity_usd_30d_ago,
            quality: LiquidityQuality {
                status: r.liquidity_quality,
                as_of: r.liquidity_as_of,
                coverage_identity: r.liquidity_coverage_identity,
                generation: r.liquidity_generation,
                reasons: r.liquidity_reasons,
            },
        }
    }
}
const ROLLUP_SELECT:&str="SELECT total_liquidity_usd,liquidity_change_24h_pct,liquidity_change_30d_pct,
    priced_pair_count,unpriced_pair_count,total_liquidity_usd_24h_ago,total_liquidity_usd_30d_ago,
    liquidity_quality,liquidity_as_of,liquidity_coverage_identity,liquidity_generation,liquidity_reasons
    FROM global_stats_24h WHERE id=1";
pub async fn get_liquidity_rollup(pool: &PgPool) -> Result<LiquidityRollup, sqlx::Error> {
    Ok(sqlx::query_as::<_, RollupRow>(ROLLUP_SELECT)
        .fetch_optional(pool)
        .await?
        .map(Into::into)
        .unwrap_or_default())
}
pub async fn get_liquidity_rollup_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<LiquidityRollup, sqlx::Error> {
    Ok(sqlx::query_as::<_, RollupRow>(ROLLUP_SELECT)
        .fetch_optional(&mut **tx)
        .await?
        .map(Into::into)
        .unwrap_or_default())
}
/// Serialize both writers before input reads; the lock lasts through publication.
pub async fn begin_liquidity_refresh(
    pool: &PgPool,
) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout='15s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock(1380,569)")
        .execute(&mut *tx)
        .await?;
    // Acquire all input-table locks before reads. READ COMMITTED sees the newest
    // committed inputs after waiting for a prior writer; SHARE locks keep those
    // inputs stable through publication without a stale pre-lock MVCC snapshot.
    sqlx::query("LOCK TABLE assets,pairs,pair_reserves,oracle_prices,hub_prices,indexer_state IN SHARE MODE")
        .execute(&mut *tx).await?;
    Ok(tx)
}
async fn upsert_rollup(conn: &mut PgConnection, r: &LiquidityRollup) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO global_stats_24h(id,total_volume,total_volume_usd,total_trades,updated_at,
        total_liquidity_usd,liquidity_change_24h_pct,liquidity_change_30d_pct,priced_pair_count,
        unpriced_pair_count,total_liquidity_usd_24h_ago,total_liquidity_usd_30d_ago,
        liquidity_quality,liquidity_as_of,liquidity_coverage_identity,liquidity_generation,liquidity_reasons)
        VALUES(1,0,0,0,NOW(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
        ON CONFLICT(id) DO UPDATE SET total_liquidity_usd=$1,liquidity_change_24h_pct=$2,
        liquidity_change_30d_pct=$3,priced_pair_count=$4,unpriced_pair_count=$5,
        total_liquidity_usd_24h_ago=$6,total_liquidity_usd_30d_ago=$7,
        liquidity_quality=$8,liquidity_as_of=$9,liquidity_coverage_identity=$10,
        liquidity_generation=$11,liquidity_reasons=$12")
    .bind(&r.total_liquidity_usd).bind(&r.liquidity_change_24h_pct).bind(&r.liquidity_change_30d_pct)
    .bind(r.priced_pair_count).bind(r.unpriced_pair_count).bind(&r.total_liquidity_usd_24h_ago)
    .bind(&r.total_liquidity_usd_30d_ago).bind(&r.quality.status).bind(r.quality.as_of)
    .bind(&r.quality.coverage_identity).bind(r.quality.generation).bind(&r.quality.reasons)
    .execute(conn).await?;
    Ok(())
}
/// Compatibility insertion for legacy callers: provenance is explicitly unknown.
/// New valuation writes must use atomic publication below.
pub async fn insert_snapshot(
    pool: &PgPool,
    at: DateTime<Utc>,
    usd: &BigDecimal,
    priced: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO global_liquidity_snapshots(sampled_at,total_liquidity_usd,priced_pair_count)
        VALUES($1,$2,$3) ON CONFLICT(sampled_at) DO UPDATE SET total_liquidity_usd=$2,priced_pair_count=$3,
        quality='unknown',as_of=NULL,coverage_identity='',generation=0,reasons=jsonb_build_object('legacy_unqualified',1) ")
        .bind(at).bind(usd).bind(priced).execute(pool).await?;
    Ok(())
}
pub async fn maybe_insert_snapshot(
    pool: &PgPool,
    now: DateTime<Utc>,
    usd: &BigDecimal,
    priced: i32,
) -> Result<bool, sqlx::Error> {
    let last: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MAX(sampled_at) FROM global_liquidity_snapshots")
            .fetch_one(pool)
            .await?;
    if last.is_some_and(|at| now.signed_duration_since(at) < SNAPSHOT_MIN_INTERVAL) {
        return Ok(false);
    }
    insert_snapshot(pool, now, usd, priced).await?;
    Ok(true)
}
/// Nearest terminal observation, then qualify; never search past a degraded close.
pub async fn nearest_comparable_snapshot_tx(
    tx: &mut Transaction<'_, Postgres>,
    target: DateTime<Utc>,
    identity: &str,
) -> Result<Option<LiquiditySnapshot>, sqlx::Error> {
    let row = sqlx::query_as::<_, (DateTime<Utc>, Option<BigDecimal>, i32, String, String)>(
        "SELECT sampled_at,total_liquidity_usd,priced_pair_count,quality,coverage_identity
        FROM global_liquidity_snapshots WHERE sampled_at BETWEEN $1 AND $2
        ORDER BY ABS(EXTRACT(EPOCH FROM(sampled_at-$3))),sampled_at DESC LIMIT 1",
    )
    .bind(target - SNAPSHOT_LOOKUP_TOLERANCE)
    .bind(target + SNAPSHOT_LOOKUP_TOLERANCE)
    .bind(target)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.and_then(|(at, usd, priced, status, coverage)| {
        if !matches!(status.as_str(), "fresh" | "verified_empty") || coverage != identity {
            return None;
        }
        usd.map(|total_liquidity_usd| LiquiditySnapshot {
            sampled_at: at,
            total_liquidity_usd: Some(total_liquidity_usd),
            priced_pair_count: priced,
        })
    }))
}
/// Legacy lookup for existing tests; new delta computation uses comparable lookup.
pub async fn nearest_snapshot(
    pool: &PgPool,
    target: DateTime<Utc>,
) -> Result<Option<LiquiditySnapshot>, sqlx::Error> {
    let row=sqlx::query_as::<_,(DateTime<Utc>,Option<BigDecimal>,i32)>(
        "SELECT sampled_at,total_liquidity_usd,priced_pair_count FROM global_liquidity_snapshots
        WHERE sampled_at BETWEEN $1 AND $2 ORDER BY ABS(EXTRACT(EPOCH FROM(sampled_at-$3))) LIMIT 1")
        .bind(target-SNAPSHOT_LOOKUP_TOLERANCE).bind(target+SNAPSHOT_LOOKUP_TOLERANCE).bind(target)
        .fetch_optional(pool).await?;
    Ok(row.and_then(|(sampled_at, usd, priced_pair_count)| {
        usd.map(|total_liquidity_usd| LiquiditySnapshot {
            sampled_at,
            total_liquidity_usd: Some(total_liquidity_usd),
            priced_pair_count,
        })
    }))
}
/// One terminal source row per four-minute UTC slot. Each attempt updates the
/// actual sampled_at and all grains, so a final outage cannot disappear behind
/// the throttle. Slot boundaries divide UTC hour/day/month exactly.
pub async fn publish_liquidity_refresh(
    tx: &mut Transaction<'_, Postgres>,
    r: &LiquidityRollup,
    stamps: &[super::pair_liquidity_usd::PairLiquidityStamp],
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let previous = get_liquidity_rollup_tx(tx).await?;
    if r.quality.generation <= previous.quality.generation {
        return Err(sqlx::Error::Protocol(
            "older liquidity publication rejected".into(),
        ));
    }
    upsert_rollup(&mut **tx, r).await?;
    super::pair_liquidity_usd::replace_pair_liquidity_usd_tx(tx, stamps, r.quality.generation)
        .await?;
    let slot = DateTime::<Utc>::from_timestamp(now.timestamp().div_euclid(240) * 240, 0)
        .ok_or_else(|| sqlx::Error::Protocol("invalid liquidity sample time".into()))?;
    sqlx::query("INSERT INTO global_liquidity_snapshots(sampled_at,total_liquidity_usd,priced_pair_count,
        quality,as_of,coverage_identity,generation,reasons,terminal_slot)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(terminal_slot) DO UPDATE SET
        sampled_at=$1,total_liquidity_usd=$2,priced_pair_count=$3,quality=$4,as_of=$5,
        coverage_identity=$6,generation=$7,reasons=$8 WHERE global_liquidity_snapshots.generation<$7")
        .bind(now).bind(&r.total_liquidity_usd).bind(r.priced_pair_count).bind(&r.quality.status)
        .bind(r.quality.as_of).bind(&r.quality.coverage_identity).bind(r.quality.generation)
        .bind(&r.quality.reasons).bind(slot).execute(&mut **tx).await?;
    super::protocol_liquidity::refresh_protocol_liquidity_rollups_tx(tx, now).await?;
    sqlx::query("DELETE FROM global_liquidity_snapshots WHERE sampled_at<$1")
        .bind(now - SNAPSHOT_RETENTION)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub async fn prune_snapshots(pool: &PgPool, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM global_liquidity_snapshots WHERE sampled_at<$1")
            .bind(now - SNAPSHOT_RETENTION)
            .execute(pool)
            .await?
            .rows_affected(),
    )
}
