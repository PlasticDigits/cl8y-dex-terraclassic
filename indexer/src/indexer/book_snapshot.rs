//! Background mirror writer for pool reserves + resting limit books (GitLab #279 Phase 1b, #322).
//!
//! ## Freshness contract (Phase 1c consumers)
//!
//! | Field | Meaning |
//! |-------|---------|
//! | **Cadence** | Target interval between full snapshot cycles — config `book_snapshot_interval_ms` (default [`BOOK_SNAPSHOT_DEFAULT_INTERVAL_MS`]). |
//! | **Max staleness** | Wall-clock TTL beyond which Phase 1c should treat mirror rows as stale and **degrade** (fall back to LCD or mark quote degraded): [`book_snapshot_max_staleness_ms`] (cadence) = cadence × [`BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES`] (tolerates one missed cycle). At default cadence use [`BOOK_SNAPSHOT_MAX_STALENESS_MS`]. |
//! | **Block height** | Each cycle stamps the same `block_height` on reserves + resting rows when LCD returns latest height; Phase 1c can reason about block lag vs chain head. |
//!
//! **Degrade-not-error:** missing snapshot (`get_pair_reserves` → `None`), empty book, or stale `snapshot_at` must not hard-fail the solver — Phase 1c falls back per pair/hop.
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
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::db::queries::pair_reserves;
use crate::db::queries::pairs::{self, PairRow};
use crate::db::queries::resting_orders::{self, RestingOrderInput};
use crate::lcd::types::{FeeConfigResponse, PoolResponse};
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
            height_timeout: Duration::from_secs(2),
        }
    }
}

/// Bound a hostile/ever-growing DLL even when individual LCD calls are fast.
pub const BOOK_SNAPSHOT_MAX_ORDERS_PER_SIDE: usize = 1_000;
/// Pool, fee, head and individual order responses are bounded independently.
pub const BOOK_SNAPSHOT_RESPONSE_MAX_BYTES: usize = 65_536;
/// Latest block envelopes can include transactions; cap separately from tiny smart queries.
pub const BOOK_SNAPSHOT_HEIGHT_RESPONSE_MAX_BYTES: usize = 1_048_576;

pub async fn run_book_snapshot_loop(pool: PgPool, lcd: LcdClient, snapshot_interval_ms: u64) {
    let mut ticker = tokio::time::interval(Duration::from_millis(snapshot_interval_ms.max(1)));
    // Slow passes skip missed ticks instead of building a retry burst or adding pass duration
    // to every interval. Never overlap two cycles from this loop.
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        if let Err(e) = snapshot_all_pairs(&pool, &lcd).await {
            tracing::error!("Book snapshot cycle failed: {}", e);
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
    if limits.concurrency == 0 || limits.pair_timeout.is_zero() || limits.height_timeout.is_zero() {
        return Err("snapshot limits must be positive".into());
    }
    let started = Instant::now();
    let block_height = match tokio::time::timeout(
        limits.height_timeout,
        lcd.get_latest_block_height_bounded(BOOK_SNAPSHOT_HEIGHT_RESPONSE_MAX_BYTES),
    )
    .await
    {
        Ok(Ok(h)) if h > 0 => Some(h),
        outcome => {
            tracing::warn!(?outcome, "Book snapshot: latest block height unavailable");
            None
        }
    };
    let pairs = pairs::get_all_pairs(pool).await?;
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
                    snapshot_single_pair(&pool, &lcd, &pair, block_height),
                ).await;
                let latency_ms = started.elapsed().as_millis() as u64;
                let success = match result {
                    Ok(Ok(())) => true,
                    Ok(Err(e)) => {
                        tracing::warn!(pair_id = pair.id, pair_addr = %pair.contract_address,
                            latency_ms, failure_reason = %e, "Book snapshot pair failed; prior snapshot retained");
                        false
                    }
                    Err(_) => {
                        tracing::warn!(pair_id = pair.id, pair_addr = %pair.contract_address,
                            latency_ms, failure_reason = "pair_timeout", "Book snapshot pair timed out; prior snapshot retained");
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
    let addr = &pair.contract_address;

    let pool_resp: PoolResponse = lcd
        .query_contract_bounded(
            addr,
            &json!({ "pool": {} }),
            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
        )
        .await?;
    let reserve_0 = parse_reserve_amount(&pool_resp.assets[0].amount)?;
    let reserve_1 = parse_reserve_amount(&pool_resp.assets[1].amount)?;

    let fee_resp: FeeConfigResponse = lcd
        .query_contract_bounded(
            addr,
            &json!({ "get_fee_config": {} }),
            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
        )
        .await?;
    if fee_resp.fee_config.fee_bps > 10_000 {
        return Err("invalid pair fee_bps".into());
    }
    let fee_bps = fee_resp.fee_config.fee_bps as i16;

    // Concat LCD DLL walks (best price first, then arrival-at-price). Replace
    // assigns `walk_index` from this slice so sim does not re-sort by `order_id` (#1227).
    let mut orders = Vec::new();
    for side in ["bid", "ask"] {
        let side_orders = walk_resting_book_side(lcd, addr, side).await?;
        orders.extend(side_orders);
    }

    let mut tx = pool.begin().await?;
    pair_reserves::upsert_pair_reserves(
        &mut *tx,
        pair.id,
        &reserve_0,
        &reserve_1,
        fee_bps,
        block_height,
    )
    .await?;
    resting_orders::replace_pair_resting_orders_in_tx(&mut tx, pair.id, block_height, &orders)
        .await?;
    tx.commit().await?;

    Ok(())
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
) -> Result<Option<ChainLimitOrderRow>, BoxError> {
    match lcd
        .query_contract_bounded(
            pair_addr,
            &json!({ "limit_order": { "order_id": order_id } }),
            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
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
) -> Result<Vec<RestingOrderInput>, BoxError> {
    let head: Option<u64> = lcd
        .query_contract_bounded(
            pair_addr,
            &json!({ "order_book_head": { "side": side_label } }),
            BOOK_SNAPSHOT_RESPONSE_MAX_BYTES,
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
        let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
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
                ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
            })
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
        let error = walk_resting_book_side(&lcd, "terra1pair", "bid")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("cycle at order_id 1"));
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
