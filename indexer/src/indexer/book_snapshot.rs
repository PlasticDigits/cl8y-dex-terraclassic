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

use std::str::FromStr;
use std::time::Duration;

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

pub use crate::config::BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES;

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

pub async fn run_book_snapshot_loop(pool: PgPool, lcd: LcdClient, snapshot_interval_ms: u64) {
    let interval = Duration::from_millis(snapshot_interval_ms);

    loop {
        if let Err(e) = snapshot_all_pairs(&pool, &lcd).await {
            tracing::error!("Book snapshot cycle failed: {}", e);
        }
        tokio::time::sleep(interval).await;
    }
}

/// One full snapshot pass over every indexed pair.
pub async fn snapshot_all_pairs(pool: &PgPool, lcd: &LcdClient) -> Result<(), BoxError> {
    let block_height = match lcd.get_latest_block_height().await {
        Ok(h) => Some(h),
        Err(e) => {
            tracing::warn!("Book snapshot: could not read latest block height: {}", e);
            None
        }
    };

    let pairs = pairs::get_all_pairs(pool).await?;
    tracing::debug!(
        pair_count = pairs.len(),
        ?block_height,
        "Book snapshot cycle starting"
    );

    for pair in &pairs {
        if let Err(e) = snapshot_single_pair(pool, lcd, pair, block_height).await {
            tracing::warn!(
                pair_id = pair.id,
                pair_addr = %pair.contract_address,
                "Book snapshot skipped pair: {}",
                e
            );
        }
    }

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

    let pool_resp: PoolResponse = lcd.query_contract(addr, &json!({ "pool": {} })).await?;
    let reserve_0 = parse_uint_amount(&pool_resp.assets[0].amount, "reserve_0")?;
    let reserve_1 = parse_uint_amount(&pool_resp.assets[1].amount, "reserve_1")?;

    let fee_resp: FeeConfigResponse = lcd
        .query_contract(addr, &json!({ "get_fee_config": {} }))
        .await?;
    let fee_bps = fee_resp.fee_config.fee_bps as i16;

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

fn parse_uint_amount(raw: &str, label: &str) -> Result<BigDecimal, BoxError> {
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
        LcdError::AllEndpointsFailed(msg) => msg.contains("not found"),
        _ => false,
    }
}

async fn fetch_limit_order(
    lcd: &LcdClient,
    pair_addr: &str,
    order_id: u64,
) -> Result<Option<ChainLimitOrderRow>, BoxError> {
    match lcd
        .query_contract(
            pair_addr,
            &json!({ "limit_order": { "order_id": order_id } }),
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
        .query_contract(
            pair_addr,
            &json!({ "order_book_head": { "side": side_label } }),
        )
        .await?;

    let mut current = head;
    let mut orders = Vec::new();

    while let Some(oid) = current {
        let row_opt = fetch_limit_order(lcd, pair_addr, oid).await?;
        let Some(row) = row_opt else {
            return Err(format!("broken book link: limit_order {oid} missing").into());
        };
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
            price: parse_uint_amount(&row.price, "price")?,
            remaining: parse_uint_amount(&row.remaining, "remaining")?,
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
