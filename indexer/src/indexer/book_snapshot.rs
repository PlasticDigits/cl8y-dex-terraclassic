//! Background mirror writer for pool reserves + resting limit books (GitLab #279 Phase 1b, #322).
//!
//! ## Freshness contract (Phase 1c consumers)
//!
//! | Field | Meaning |
//! |-------|---------|
//! | **Cadence** | Target interval for each independently scheduled pool/book lane — config `book_snapshot_interval_ms` (default [`BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS`]). |
//! | **Max staleness** | Wall-clock TTL beyond which Phase 1c should treat mirror rows as stale and **degrade** (omit cached quote eligibility): [`book_snapshot_max_staleness_ms`] (cadence) = cadence × [`BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES`] (tolerates one missed cycle). At default cadence use [`BOOK_SNAPSHOT_MAX_STALENESS_MS`]. |
//! | **Block height** | Pool reads are pinned to the cycle's observed head; fee/book reads are pinned to their cached reserve height. Hybrid rows publish atomically under that original height/time. Unknown height cannot establish hybrid eligibility. |
//!
//! **Degrade-not-error:** missing snapshot (`get_pair_reserves` → `None`), empty book, or stale `snapshot_at` must not hard-fail the solver — consumers omit unavailable pairs/hops without requesting a refresh.
//!
//! ## LCD budget (documented upper bound per cycle)
//!
//! `BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD` (latest block height)
//! + `pair_count × BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR` (pool + fee + bid head + ask head per pair)
//! + `total_resting_orders` (one `limit_order` query per resting order on both sides).
//!
//! See [`book_snapshot_lcd_budget`].

use std::collections::HashSet;
use std::str::FromStr;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio::time::{Instant, MissedTickBehavior};

use bigdecimal::BigDecimal;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use sqlx::PgPool;

use chrono::Utc;

use crate::db::queries::pairs::{self, PairRow};
use crate::db::queries::resting_orders::{self, RestingOrderInput};
use crate::db::queries::{assets, pair_reserves};
use crate::lcd::types::{AssetInfo, FeeConfigResponse, PoolResponse};
use crate::lcd::{LcdClient, LcdError};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Default snapshot cadence when `BOOK_SNAPSHOT_INTERVAL_MS` is unset (10s).
pub const BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS: u64 = 10_000;

/// Max wall-clock staleness at default cadence; equals [`book_snapshot_max_staleness_ms`] with default interval.
pub const BOOK_SNAPSHOT_MAX_STALENESS_MS: u64 =
    crate::config::book_snapshot_max_staleness_ms(BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS);

/// Per-pair fixed LCD smart queries: `pool`, `get_fee_config`, `order_book_head` (bid), `order_book_head` (ask).
pub const BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR: usize = 4;

/// One `get_latest_block_height` probe per snapshot cycle.
pub const BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD: usize = 1;

/// Documented per-cycle LCD upper bound.
pub fn book_snapshot_lcd_budget(pair_count: usize, total_resting_orders: usize) -> usize {
    BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD
        + pair_count.saturating_mul(BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR)
        + total_resting_orders
}

/// Hard limits on background LCD work; solver freshness TTL is unchanged.
/// A pair holds at most one LCD request at a time, including endpoint failover.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotLimits {
    pub concurrency: usize,
    pub pair_timeout: Duration,
    pub height_timeout: Duration,
}

impl Default for SnapshotLimits {
    fn default() -> Self {
        Self {
            concurrency: 8,
            pair_timeout: Duration::from_secs(4),
            height_timeout: Duration::from_secs(4),
        }
    }
}

/// Bound a hostile/ever-growing DLL even when individual LCD calls are fast.
pub const BOOK_SNAPSHOT_MAX_ORDERS_PER_SIDE: usize = 1_000;
/// Pool, fee, head and individual order responses are bounded independently.
pub const BOOK_SNAPSHOT_RESPONSE_MAX_BYTES: usize = 65_536;
/// Latest block envelopes can include transactions; cap separately from tiny smart queries.
pub const BOOK_SNAPSHOT_HEIGHT_RESPONSE_MAX_BYTES: usize = 1_048_576;

/// Two independent, nonoverlapping lanes: only the pool lane writes current
/// reserves; the book lane consumes those observations and publishes hybrids.
/// Slow books cannot postpone the next successful pool read.
pub async fn run_book_snapshot_loop(pool: PgPool, lcd: LcdClient, snapshot_interval_ms: u64) {
    run_book_snapshot_loop_with_reference_signal(pool, lcd, snapshot_interval_ms, None).await;
}

pub(crate) async fn run_book_snapshot_loop_with_reference_signal(
    pool: PgPool,
    lcd: LcdClient,
    snapshot_interval_ms: u64,
    references: Option<std::sync::Arc<tokio::sync::Notify>>,
) {
    let lcd = lcd.with_scheduled_observation();
    let references = references.as_ref();
    let run_lane = |lane_pool: PgPool, lane_lcd: LcdClient, mode| async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(snapshot_interval_ms.max(1)));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        // Lifetime leadership coalesces even sequential cycles from replicas.
        // Transaction-scoped ownership releases the lane on cancellation.
        let mut leadership = None;
        loop {
            ticker.tick().await;
            if leadership.is_none() {
                let attempt = async {
                    let mut tx = lane_pool.begin().await?;
                    let key: i32 = if matches!(mode, SnapshotMode::Pool) {
                        1
                    } else {
                        2
                    };
                    let admitted: bool =
                        sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(1453, $1)")
                            .bind(key)
                            .fetch_one(&mut *tx)
                            .await?;
                    Ok::<_, sqlx::Error>(if admitted { Some(tx) } else { None })
                }
                .await;
                match attempt {
                    Ok(Some(tx)) => leadership = Some(tx),
                    Ok(None) => continue,
                    Err(e) => {
                        tracing::error!("Liquidity lane leadership unavailable: {}", e);
                        continue;
                    }
                }
            }
            if sqlx::query("SELECT 1")
                .execute(&mut **leadership.as_mut().unwrap())
                .await
                .is_err()
            {
                leadership = None;
                continue;
            }
            let mut limits = SnapshotLimits::default();
            // Keep both production lanes below a single healthy provider's
            // default eight slots, leaving admission/checkpoint headroom. Every
            // caller still obeys the same configured shared provider limits.
            limits.concurrency = if matches!(mode, SnapshotMode::Pool) {
                4
            } else {
                2
            };
            let cycle = snapshot_cycle(
                &lane_pool,
                &lane_lcd,
                limits,
                mode,
                crate::config::book_snapshot_max_staleness_ms(snapshot_interval_ms),
            );
            tokio::pin!(cycle);
            let mut heartbeat = tokio::time::interval(Duration::from_millis(250));
            heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    result = &mut cycle => {
                        match result {
                            Ok(()) if matches!(mode, SnapshotMode::Pool) => {
                                if let Some(signal) = references.as_ref() { signal.notify_one(); }
                            },
                            Ok(()) => {},
                            Err(e) => tracing::error!("Liquidity observation lane failed: {}", e),
                        }
                        break;
                    }
                    _ = heartbeat.tick() => {
                        let alive = tokio::time::timeout(Duration::from_millis(250),
                            sqlx::query("SELECT 1").execute(&mut **leadership.as_mut().unwrap())).await;
                        if !matches!(alive, Ok(Ok(_))) {
                            // Dropping this cycle aborts its JoinSet before re-election.
                            // Raw commits already completed remain monotonic and valid.
                            leadership = None;
                            break;
                        }
                    }
                }
            }
        }
    };
    tokio::join!(
        run_lane(pool.clone(), lcd.clone(), SnapshotMode::Pool),
        run_lane(pool, lcd, SnapshotMode::Book)
    );
}

#[derive(Clone, Copy)]
enum SnapshotMode {
    Pool,
    Book,
    Combined,
}

impl SnapshotMode {
    fn label(self) -> &'static str {
        match self {
            Self::Pool => "pool",
            Self::Book => "book",
            Self::Combined => "combined",
        }
    }
}

/// Wait only for zero-attempt provider capacity denial. The enclosing height
/// deadline owns this future; no provider/lane permit survives these sleeps.
async fn observe_head(
    lcd: &LcdClient,
    minimum_height: i64,
    capacity_waits: &mut usize,
) -> Result<i64, LcdError> {
    loop {
        match lcd
            .get_latest_block_height_bounded_at_least(
                BOOK_SNAPSHOT_HEIGHT_RESPONSE_MAX_BYTES,
                minimum_height,
            )
            .await
        {
            Err(LcdError::AdmissionDeferred) => {
                *capacity_waits += 1;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            result => return result,
        }
    }
}

/// One full snapshot pass over every indexed pair, with bounded concurrency and deadlines.
pub async fn snapshot_all_pairs(pool: &PgPool, lcd: &LcdClient) -> Result<(), BoxError> {
    snapshot_all_pairs_with_limits(pool, lcd, SnapshotLimits::default()).await
}

/// Explicit limits allow deterministic LCD failure/recovery fixtures.
pub async fn snapshot_all_pairs_with_limits(
    pool: &PgPool,
    lcd: &LcdClient,
    limits: SnapshotLimits,
) -> Result<(), BoxError> {
    let lcd = lcd.with_scheduled_observation();
    snapshot_cycle(
        pool,
        &lcd,
        limits,
        SnapshotMode::Combined,
        BOOK_SNAPSHOT_MAX_STALENESS_MS,
    )
    .await
}

async fn snapshot_cycle(
    pool: &PgPool,
    lcd: &LcdClient,
    limits: SnapshotLimits,
    mode: SnapshotMode,
    max_staleness_ms: u64,
) -> Result<(), BoxError> {
    if limits.concurrency == 0 || limits.pair_timeout.is_zero() || limits.height_timeout.is_zero() {
        return Err("snapshot limits must be positive".into());
    }
    // Transaction-scoped locks coalesce lanes across replicas and release on
    // cancellation. They protect ingestion, never a request-triggered refresh.
    let mut lane_lock = if matches!(mode, SnapshotMode::Combined) {
        Some(pool.begin().await?)
    } else {
        None
    };
    let lane_keys: &[i32] = match mode {
        // Production lanes already own lifetime leadership above.
        SnapshotMode::Pool | SnapshotMode::Book => &[],
        SnapshotMode::Combined => &[1, 2],
    };
    for key in lane_keys {
        let admitted: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(1453, $1)")
            .bind(key)
            .fetch_one(&mut **lane_lock.as_mut().unwrap())
            .await?;
        if !admitted {
            return Ok(());
        }
    }
    let started = Instant::now();
    // Every pinned read in this cycle keeps the head observation's original age;
    // queued tail jobs must not renew an old block with a later start timestamp.
    let cycle_observed_at = Utc::now();
    // Use only currently indexed pairs, not orphaned reserve rows. Read the
    // inventory and its original observations once, before selecting a head.
    let mut pairs = pairs::get_all_pairs(pool).await?;
    let ages = pair_reserves::get_all_pair_reserves(pool).await?;
    let minimum_height = pairs
        .iter()
        .filter_map(|pair| ages.get(&pair.id).and_then(|row| row.block_height))
        .filter(|height| *height > 0)
        .max()
        .unwrap_or(0);
    let mut head_capacity_waits = 0;
    let mut head_outcome = "skipped";
    let block_height = if matches!(mode, SnapshotMode::Book) {
        None
    } else {
        match tokio::time::timeout(
            limits.height_timeout,
            observe_head(lcd, minimum_height, &mut head_capacity_waits),
        )
        .await
        {
            Ok(Ok(h)) if h > 0 => {
                head_outcome = "observed";
                Some(h)
            }
            Ok(Ok(_)) => {
                head_outcome = "invalid_height";
                None
            }
            Ok(Err(error)) => {
                head_outcome = if crate::indexer::block_indexer::is_retryable_availability(&error) {
                    "provider_error"
                } else {
                    "integrity_error"
                };
                tracing::warn!(
                    lane = mode.label(),
                    head_outcome,
                    head_capacity_waits,
                    "Book snapshot: latest block height unavailable"
                );
                None
            }
            Err(_) => {
                head_outcome = "deadline";
                tracing::warn!(
                    lane = mode.label(),
                    head_outcome,
                    head_capacity_waits,
                    "Book snapshot: latest block height unavailable"
                );
                None
            }
        }
    };
    // Oldest/missing observations first: slow work cannot always penalize the same tail.
    pairs.sort_by_key(|p| ages.get(&p.id).map(|r| r.snapshot_at));
    let pair_count = pairs.len();
    let mut pending = pairs.into_iter();
    let mut jobs = JoinSet::new();
    let mut failed_pair_count = 0usize;
    let mut refreshed_pair_count = 0usize;
    loop {
        while jobs.len() < limits.concurrency {
            let Some(pair) = pending.next() else {
                break;
            };
            let pool = pool.clone();
            let lcd = lcd.clone();
            jobs.spawn(async move {
                let started = Instant::now();
                let result = tokio::time::timeout(
                    limits.pair_timeout,
                    async {
                        match mode {
                            SnapshotMode::Pool => observe_pool(&pool, &lcd, &pair, block_height, cycle_observed_at).await.map(|_| ()),
                            SnapshotMode::Book => observe_book(&pool, &lcd, &pair, max_staleness_ms).await,
                            SnapshotMode::Combined => {
                                observe_pool(&pool, &lcd, &pair, block_height, cycle_observed_at).await?;
                                observe_book(&pool, &lcd, &pair, max_staleness_ms).await
                            },
                        }
                    },
                ).await;
                let latency_ms = started.elapsed().as_millis() as u64;
                let success = match result {
                    Ok(Ok(())) => true,
                    Ok(Err(e)) => {
                        tracing::warn!(pair_id = pair.id, pair_addr = %pair.contract_address,
                            latency_ms, failure_reason = %e, "Book snapshot pair failed; committed observations retained");
                        false
                    }
                    Err(_) => {
                        tracing::warn!(pair_id = pair.id, pair_addr = %pair.contract_address,
                            latency_ms, failure_reason = "pair_timeout", "Book snapshot pair timed out; committed observations retained");
                        false
                    }
                };
                if success {
                    tracing::debug!(pair_id = pair.id, latency_ms, "Book snapshot pair refreshed");
                }
                success
            });
        }
        let Some(result) = jobs.join_next().await else {
            break;
        };
        if matches!(result, Ok(true)) {
            refreshed_pair_count += 1;
        } else {
            failed_pair_count += 1;
        }
    }
    // Background-only census. Include never-written pairs and impossible future timestamps;
    // a failed pair's old timestamp is never advanced or substituted with zero reserves.
    let ages: (i64, Option<i64>, i64) = sqlx::query_as(
        "SELECT COUNT(*) FILTER (WHERE r.pair_id IS NULL),
                MAX(CASE WHEN r.snapshot_at <= NOW()
                    THEN EXTRACT(EPOCH FROM (NOW() - r.snapshot_at)) * 1000 END)::BIGINT,
                COUNT(*) FILTER (WHERE r.snapshot_at > NOW())
         FROM pairs p LEFT JOIN pair_reserves r ON r.pair_id = p.id",
    )
    .fetch_one(pool)
    .await?;
    tracing::info!(
        lane = mode.label(),
        head_outcome,
        head_capacity_waits,
        minimum_height,
        pair_count,
        refreshed_pair_count,
        failed_pair_count,
        cycle_elapsed_ms = started.elapsed().as_millis() as u64,
        missing_reserve_count = ages.0,
        max_reserve_age_ms = ages.1,
        future_reserve_timestamp_count = ages.2,
        ?block_height,
        "Book snapshot cycle completed"
    );
    Ok(())
}

/// Snapshot one pair (exported for integration tests).
pub async fn snapshot_single_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    pair: &PairRow,
    block_height: Option<i64>,
) -> Result<(), BoxError> {
    observe_pool(pool, lcd, pair, block_height, Utc::now()).await?;
    observe_book(pool, lcd, pair, BOOK_SNAPSHOT_MAX_STALENESS_MS).await
}

async fn observe_pool(
    pool: &PgPool,
    lcd: &LcdClient,
    pair: &PairRow,
    block_height: Option<i64>,
    observed_at: chrono::DateTime<Utc>,
) -> Result<(), BoxError> {
    let namespace = lcd.chain_cache_namespace();
    let reject =
        || crate::indexer::fee_phase::reject(&namespace, pair.id, block_height, observed_at);
    // Preserve observation start, never write completion time for a delayed reply.
    let query = json!({ "pool": {} });
    let response: PoolResponse =
        match query_snapshot(lcd, &pair.contract_address, &query, block_height).await {
            Ok(response) => response,
            Err(error) => {
                if !crate::indexer::block_indexer::is_retryable_availability(&error) {
                    reject();
                }
                return Err(error.into());
            }
        };
    for (asset_id, asset) in [
        (pair.asset_0_id, &response.assets[0]),
        (pair.asset_1_id, &response.assets[1]),
    ] {
        let expected = match assets::get_asset_by_id(pool, asset_id).await {
            Ok(Some(expected)) => expected,
            Ok(None) => {
                reject();
                return Err("missing pair asset".into());
            }
            Err(error) => {
                reject();
                return Err(error.into());
            }
        };
        let matches = match &asset.info {
            AssetInfo::NativeToken { denom } => {
                !expected.is_cw20 && expected.denom.as_ref() == Some(denom)
            }
            AssetInfo::Token { contract_addr } => {
                expected.is_cw20 && expected.contract_address.as_ref() == Some(contract_addr)
            }
        };
        if !matches {
            reject();
            return Err("pool asset identity/order does not match indexed pair".into());
        }
    }
    let (reserve_0, reserve_1) = match (
        parse_reserve_amount(&response.assets[0].amount),
        parse_reserve_amount(&response.assets[1].amount),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(error), _) | (_, Err(error)) => {
            reject();
            return Err(error);
        }
    };
    // Both strings passed Uint128 validation above; keep the exact raw identity
    // in the bounded fee-phase proof before any reserve publication is visible.
    let amounts = (
        response.assets[0].amount.parse::<u128>()?,
        response.assets[1].amount.parse::<u128>()?,
    );
    let guard = if let Some(height) = block_height.filter(|height| *height > 0) {
        if amounts.0 > 0 && amounts.1 > 0 {
            crate::indexer::fee_phase::begin(
                &namespace,
                pair.id,
                (pair.asset_0_id, pair.asset_1_id),
                height,
                observed_at,
                amounts,
            )
        } else {
            reject();
            None
        }
    } else {
        reject();
        None
    };
    let guard = block_height.map(|height| match guard {
        Some(guard) => crate::indexer::fee_phase::ProducerFeeGuard::Qualified(guard),
        None => crate::indexer::fee_phase::ProducerFeeGuard::Untracked(
            crate::indexer::fee_phase::UntrackedFeeGuard::new(
                &namespace,
                pair.id,
                (pair.asset_0_id, pair.asset_1_id),
                height,
                observed_at,
            ),
        ),
    });
    if !pair_reserves::upsert_pool_observation(
        pool,
        pair.id,
        &reserve_0,
        &reserve_1,
        block_height,
        observed_at,
    )
    .await?
    {
        return Err("older or unknown-height pool observation rejected".into());
    }
    // A failed fee read is independent: the successfully committed pool survives.
    // A validated old complete quote may survive only the exact typed pending or
    // availability phase; known/unknown negatives revoke it synchronously.
    if let Some(height) = block_height {
        let guard = guard.expect("a pinned observation owns a fee safety guard");
        let result = observed_fee(pool, lcd, pair, height, Some(guard.safety_callback())).await;
        match &result {
            Ok(fee) => guard.finish_validated_fee(*fee as u16),
            Err(error)
                if crate::indexer::block_indexer::is_retryable_availability(error.as_ref()) =>
            {
                guard.finish_transient()
            }
            Err(_) => guard.finish_negative(),
        }
        result?;
    }
    Ok(())
}

/// Apply only the still-current compatible fee row. Both cached and newly
/// observed fees use this transaction guard; reserve height/time never change.
async fn apply_compatible_fee(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pair_id: i32,
    height: i64,
    fee: i16,
) -> Result<u64, sqlx::Error> {
    if !(0..=10_000).contains(&fee) {
        return Err(sqlx::Error::Protocol("invalid pair fee_bps".into()));
    }
    // Lock the fee row before the reserve row, matching new-fee publication.
    // A cached read cannot reapply a fee superseded while this transaction waits.
    let current: Option<i16> = sqlx::query_scalar(
        "SELECT fee_bps FROM pair_fee_observations
         WHERE pair_id=$1 AND block_height=$2 AND fee_bps=$3
           AND fee_bps BETWEEN 0 AND 10000
           AND observed_at <= NOW() AND observed_at >= NOW()-INTERVAL '60 seconds'
         FOR SHARE",
    )
    .bind(pair_id)
    .bind(height)
    .bind(fee)
    .fetch_optional(&mut **tx)
    .await?;
    if current != Some(fee) {
        return Ok(0);
    }
    let applied =
        sqlx::query("UPDATE pair_reserves SET fee_bps=$3 WHERE pair_id=$1 AND block_height=$2")
            .bind(pair_id)
            .bind(height)
            .bind(fee)
            .execute(&mut **tx)
            .await?;
    Ok(applied.rows_affected())
}

/// Reuse fee configuration for the same pinned height for at most a minute.
/// New heights refresh configuration; failures preserve its actual observation time.
async fn observed_fee(
    pool: &PgPool,
    lcd: &LcdClient,
    pair: &PairRow,
    height: i64,
    callback: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
) -> Result<i16, BoxError> {
    let cached: Option<i16> = sqlx::query_scalar(
        "SELECT fee_bps FROM pair_fee_observations WHERE pair_id = $1 AND block_height = $2
         AND observed_at <= NOW() AND observed_at >= NOW() - INTERVAL '60 seconds'",
    )
    .bind(pair.id)
    .bind(height)
    .fetch_optional(pool)
    .await?;
    if let Some(fee) = cached {
        if !(0..=10_000).contains(&fee) {
            if let Some(callback) = &callback {
                callback();
            }
            return Err("invalid cached pair fee_bps".into());
        }
        let mut tx = pool.begin().await?;
        if apply_compatible_fee(&mut tx, pair.id, height, fee).await? == 0 {
            return Err("compatible cached fee changed before application".into());
        }
        tx.commit().await?;
        return Ok(fee);
    }
    let observed_at = Utc::now();
    let response: FeeConfigResponse = query_snapshot_observed(
        lcd,
        &pair.contract_address,
        &json!({ "get_fee_config": {} }),
        Some(height),
        callback.clone(),
    )
    .await?;
    if response.fee_config.fee_bps > 10_000 {
        if let Some(callback) = &callback {
            callback();
        }
        return Err("invalid pair fee_bps".into());
    }
    let fee = response.fee_config.fee_bps as i16;
    let mut tx = pool.begin().await?;
    // Admit monotonic fee observations first, then expose that same admitted fee
    // to pool-only quote consumers without changing their reserve timestamp.
    sqlx::query(
        "INSERT INTO pair_fee_observations VALUES ($1, $2, $3, $4)
         ON CONFLICT (pair_id) DO UPDATE SET fee_bps = EXCLUDED.fee_bps, observed_at = EXCLUDED.observed_at, block_height = EXCLUDED.block_height
         WHERE EXCLUDED.observed_at >= pair_fee_observations.observed_at AND EXCLUDED.block_height >= pair_fee_observations.block_height",
    ).bind(pair.id).bind(fee).bind(observed_at).bind(height).execute(&mut *tx).await?;
    if apply_compatible_fee(&mut tx, pair.id, height, fee).await? == 0 {
        return Err("compatible fee observation changed before application".into());
    }
    tx.commit().await?;
    Ok(fee)
}

async fn observe_book(
    pool: &PgPool,
    lcd: &LcdClient,
    pair: &PairRow,
    max_staleness_ms: u64,
) -> Result<(), BoxError> {
    // A book refresh never issues a second pool query or labels a stale pool fresh.
    let observation = pair_reserves::get_pair_reserves(pool, pair.id)
        .await?
        .ok_or("pool observation unavailable")?;
    if observation.snapshot_at > Utc::now()
        || Utc::now()
            .signed_duration_since(observation.snapshot_at)
            .num_milliseconds()
            > max_staleness_ms as i64
    {
        return Err("pool observation stale for hybrid generation".into());
    }
    let height = observation
        .block_height
        .ok_or("unknown-height pool cannot form a compatible hybrid generation")?;
    let fee_bps: i16 = sqlx::query_scalar(
        "SELECT fee_bps FROM pair_fee_observations WHERE pair_id = $1 AND block_height = $2
         AND observed_at <= NOW() AND observed_at >= NOW() - INTERVAL '60 seconds'",
    )
    .bind(pair.id)
    .bind(height)
    .fetch_optional(pool)
    .await?
    .ok_or("compatible cached fee unavailable for hybrid generation")?;
    let mut orders = Vec::new();
    for side in ["bid", "ask"] {
        orders.extend(walk_resting_book_side(lcd, &pair.contract_address, side, height).await?);
    }
    let mut tx = pool.begin().await?;
    if pair_reserves::upsert_hybrid_observation(&mut tx, &observation, fee_bps).await? {
        resting_orders::replace_pair_resting_orders_in_tx(
            &mut tx,
            pair.id,
            observation.block_height,
            &orders,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Background-only capacity wait, bounded by the enclosing pair deadline.
/// Preserve successful earlier phases and keep waiters in caller futures rather
/// than an internal queue. Actual upstream/contract errors never retry here.
async fn query_snapshot<T: DeserializeOwned>(
    lcd: &LcdClient,
    pair_address: &str,
    query: &serde_json::Value,
    height: Option<i64>,
) -> Result<T, LcdError> {
    query_snapshot_observed(lcd, pair_address, query, height, None).await
}

async fn query_snapshot_observed<T: DeserializeOwned>(
    lcd: &LcdClient,
    pair_address: &str,
    query: &serde_json::Value,
    height: Option<i64>,
    callback: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
) -> Result<T, LcdError> {
    // Also bound direct test/operator callers lacking a surrounding scheduled job.
    // The production job's four-second deadline still bounds all phases together.
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(LcdError::AdmissionDeferred);
        }
        let request = async {
            match height {
                Some(height) => {
                    if let Some(callback) = &callback {
                        lcd.query_contract_at_height_bounded_observed(
                            pair_address,
                            query,
                            height,
                            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
                            callback.clone(),
                        )
                        .await
                    } else {
                        lcd.query_contract_at_height_bounded(
                            pair_address,
                            query,
                            height,
                            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
                        )
                        .await
                    }
                }
                None => {
                    lcd.query_contract_bounded(
                        pair_address,
                        query,
                        BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
                    )
                    .await
                }
            }
        };
        let result = tokio::time::timeout(remaining, request)
            .await
            .map_err(|_| LcdError::PairResponseMalformed("incomplete snapshot query".into()))?;
        match result {
            Err(LcdError::AdmissionDeferred) => {
                tokio::time::sleep(
                    Duration::from_millis(50)
                        .min(deadline.saturating_duration_since(Instant::now())),
                )
                .await;
            }
            result => return result,
        }
    }
}

fn parse_reserve_amount(raw: &str) -> Result<BigDecimal, BoxError> {
    // CosmWasm reserves are Uint128 strings. Reject negative/decimal/scientific and
    // overlarge representations rather than accepting a malformed empty-pool signal.
    if raw.is_empty() || raw.len() > 39 || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid Uint128 reserve amount".into());
    }
    raw.parse::<u128>()
        .map_err(|_| "reserve amount exceeds Uint128")?;
    parse_decimal_amount(raw, "reserve")
}

fn parse_book_price(raw: &str) -> Result<BigDecimal, BoxError> {
    if raw.is_empty() || raw.len() > 96 || !raw.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return Err("invalid book price representation".into());
    }
    let price = parse_decimal_amount(raw, "price")?;
    if price <= BigDecimal::from(0) {
        return Err("book price must be positive".into());
    }
    Ok(price)
}

fn parse_decimal_amount(raw: &str, label: &str) -> Result<BigDecimal, BoxError> {
    BigDecimal::from_str(raw).map_err(|e| format!("invalid {label} amount {raw:?}: {e}").into())
}

#[derive(Debug, Deserialize)]
struct ChainLimitOrderRow {
    order_id: u64,
    owner: String,
    side: serde_json::Value,
    price: String,
    remaining: String,
    #[serde(default)]
    expires_at: Option<u64>,
    next: Option<u64>,
}

fn chain_side_label(v: &serde_json::Value) -> String {
    v.as_str()
        .map(std::string::ToString::to_string)
        .or_else(|| {
            v.as_object()
                .and_then(|m| m.keys().next().map(std::string::ToString::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn lcd_query_missing_key(err: &LcdError) -> bool {
    match err {
        LcdError::AllEndpointsFailed(msg) | LcdError::ContractQueryRejected(msg) => {
            msg.contains("not found")
        }
        _ => false,
    }
}

async fn fetch_limit_order(
    lcd: &LcdClient,
    pair_addr: &str,
    order_id: u64,
    height: i64,
) -> Result<Option<ChainLimitOrderRow>, BoxError> {
    match query_snapshot(
        lcd,
        pair_addr,
        &json!({ "limit_order": { "order_id": order_id } }),
        Some(height),
    )
    .await
    {
        Ok(row) => Ok(row),
        Err(e) if lcd_query_missing_key(&e) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Walk the full resting book for one side (head → tail linked list).
async fn walk_resting_book_side(
    lcd: &LcdClient,
    pair_addr: &str,
    side_label: &str,
    height: i64,
) -> Result<Vec<RestingOrderInput>, BoxError> {
    let head: Option<u64> = query_snapshot(
        lcd,
        pair_addr,
        &json!({ "order_book_head": { "side": side_label } }),
        Some(height),
    )
    .await?;

    let mut current = head;
    let mut orders = Vec::new();
    let mut seen = HashSet::new();

    while let Some(oid) = current {
        if orders.len() >= BOOK_SNAPSHOT_MAX_ORDERS_PER_SIDE {
            return Err("resting book exceeds per-side snapshot budget".into());
        }
        if oid > i64::MAX as u64 {
            return Err("order_id exceeds database range".into());
        }
        if !seen.insert(oid) {
            return Err(format!("broken book link: cycle at order_id {oid}").into());
        }
        let row_opt = fetch_limit_order(lcd, pair_addr, oid, height).await?;
        let Some(row) = row_opt else {
            return Err(format!("broken book link: limit_order {oid} missing").into());
        };
        if row.order_id != oid
            || row.owner.len() > 128
            || row.expires_at.is_some_and(|v| v > i64::MAX as u64)
        {
            return Err("invalid limit_order identity or expiry".into());
        }
        let row_side = chain_side_label(&row.side);
        if !row_side.eq_ignore_ascii_case(side_label) {
            return Err(format!(
                "order {oid} side {row_side} does not match requested {side_label}"
            )
            .into());
        }

        orders.push(RestingOrderInput {
            order_id: row.order_id as i64,
            side: side_label.to_string(),
            price: parse_book_price(&row.price)?,
            remaining: parse_reserve_amount(&row.remaining)?,
            owner: Some(row.owner),
            expires_at: row.expires_at.map(|v| v as i64),
        });

        current = row.next;
    }

    Ok(orders)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recovery1464_head_capacity_deadline_and_cancel_leave_zero_attempt_penalties() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/held"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_secs(1)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            2_500,
            1_000,
            "columbus-5",
            crate::lcd::ProviderPolicy {
                concurrency: 1,
                attempt_timeout_ms: 2_500,
                deadline_ms: 8_000,
                ..Default::default()
            },
        );
        let held_lcd = lcd.clone();
        let held = tokio::spawn(async move { held_lcd.get::<serde_json::Value>("/held").await });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .any(|r| r.url.path() == "/held")
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let mut waits = 0;
        assert!(tokio::time::timeout(
            Duration::from_millis(120),
            observe_head(&lcd, 0, &mut waits)
        )
        .await
        .is_err());
        assert!(waits >= 2);
        let waiting = lcd.clone();
        let cancelled = tokio::spawn(async move { observe_head(&waiting, 0, &mut 0).await });
        tokio::time::sleep(Duration::from_millis(60)).await;
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].attempts,
                counters[0].failures,
                counters[0].caller_cancellations,
                counters[0].in_flight
            ),
            (1, 0, 0, 1)
        );
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path().ends_with("/blocks/latest"))
                .count(),
            0
        );
        held.await.unwrap().unwrap();
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
    }
    #[tokio::test]
    async fn recovery1464_head_real_availability_and_identity_failures_never_add_retries() {
        use wiremock::matchers::path;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for status in [200, 503] {
            let server = MockServer::start().await;
            Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
                .respond_with(
                    ResponseTemplate::new(status)
                        .set_body_json(json!({"default_node_info":{"network":"wrong-chain"}})),
                )
                .mount(&server)
                .await;
            let lcd = LcdClient::new_with_policy(
                vec![server.uri()],
                500,
                1_000,
                "columbus-5",
                crate::lcd::ProviderPolicy::default(),
            );
            let mut waits = 0;
            let error = tokio::time::timeout(
                Duration::from_millis(500),
                observe_head(&lcd, 0, &mut waits),
            )
            .await
            .unwrap()
            .unwrap_err();
            assert!(if status == 200 {
                matches!(error, LcdError::ChainIdentity(_))
            } else {
                matches!(error, LcdError::AllEndpointsFailed(_))
            });
            assert_eq!(waits, 0);
            assert_eq!(lcd.provider_counters().await[0].attempts, 1);
            assert_eq!(server.received_requests().await.unwrap().len(), 1);
        }
    }
    #[test]
    fn reserve_amounts_accept_real_zero_and_reject_malformed_empty_signals() {
        for raw in ["0", "1", "340282366920938463463374607431768211455"] {
            assert!(parse_reserve_amount(raw).is_ok(), "{raw}");
        }
        for raw in [
            "",
            "-1",
            "0.0",
            "1e9",
            "NaN",
            " 0",
            "340282366920938463463374607431768211456",
        ] {
            assert!(parse_reserve_amount(raw).is_err(), "{raw}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cycle_ticker_skips_missed_ticks_without_pass_plus_sleep_drift() {
        let start = Instant::now();
        let mut ticker = tokio::time::interval(Duration::from_secs(10));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;
        tokio::time::advance(Duration::from_secs(21)).await;
        ticker.tick().await;
        assert_eq!(start.elapsed(), Duration::from_secs(21));
        ticker.tick().await;
        assert_eq!(start.elapsed(), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn broken_link_fails_without_fabricating_empty_book() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path_regex("/smart/"))
            .respond_with(|req: &wiremock::Request| {
                use base64::Engine;
                let encoded = req.url.path().split("/smart/").nth(1).unwrap();
                let query: serde_json::Value = serde_json::from_slice(
                    &base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .unwrap(),
                )
                .unwrap();
                let data = if query.get("order_book_head").is_some() {
                    json!(1)
                } else {
                    json!({ "order_id": 1, "owner": "terra1test", "side": "bid",
                        "price": "1", "remaining": "100", "next": 1 })
                };
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "123")
                    .set_body_json(json!({ "data": data }))
            })
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
        let error = walk_resting_book_side(&lcd, "terra1pair", "bid", 123)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("cycle at order_id 1"));
    }

    #[tokio::test]
    async fn snapshot_query_capacity_wait_is_bounded_for_direct_callers() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path_regex("/smart/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "900")
                    .set_body_json(json!({"data":null}))
                    .set_delay(Duration::from_secs(5)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            6000,
            1000,
            "columbus-5",
            crate::lcd::ProviderPolicy {
                concurrency: 1,
                attempt_timeout_ms: 5000,
                deadline_ms: 6000,
                ..crate::lcd::ProviderPolicy::default()
            },
        );
        let holder_lcd = lcd.clone();
        let holder = tokio::spawn(async move {
            holder_lcd
                .query_contract_at_height_bounded::<serde_json::Value>(
                    "terra1holder",
                    &json!({"hold":{}}),
                    900,
                    BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .any(|r| r.url.path().contains("/smart/"))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            query_snapshot::<serde_json::Value>(&lcd, "terra1pair", &json!({"pool":{}}), Some(900)),
        )
        .await
        .expect("direct snapshot query has its own four-second logical bound");
        assert!(matches!(result, Err(LcdError::AdmissionDeferred)));
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path().contains("/smart/"))
                .count(),
            1,
            "capacity wait never creates another upstream request"
        );
        assert!(!holder.is_finished());
        holder.abort();
        let _ = holder.await;
    }

    #[test]
    fn lcd_budget_matches_documented_formula() {
        assert_eq!(book_snapshot_lcd_budget(0, 0), 1);
        assert_eq!(book_snapshot_lcd_budget(3, 0), 1 + 12);
        assert_eq!(book_snapshot_lcd_budget(2, 5), 1 + 8 + 5);
        assert!(book_snapshot_lcd_budget(10, 100) > 0);
    }

    #[test]
    fn max_staleness_ttl_is_non_zero() {
        assert!(BOOK_SNAPSHOT_MAX_STALENESS_MS > 0);
        assert_eq!(
            BOOK_SNAPSHOT_MAX_STALENESS_MS,
            crate::config::book_snapshot_max_staleness_ms(BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS)
        );
        assert_eq!(crate::config::book_snapshot_max_staleness_ms(5_000), 10_000);
    }
}
