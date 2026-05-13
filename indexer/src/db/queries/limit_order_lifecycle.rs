//! Limit order **placement**, **cancellation**, and **parked-expired / refunded** lifecycle from wasm events.
//!
//! **Invariant (GitLab #135):** `list_placements_for_pair` omits any `(pair_id, order_id)` that appears in
//! `limit_order_cancellations`, so HTTP `GET .../limit-placements` reflects orders not cancelled on-chain.
//!
//! **Invariant (GitLab #142):** `lifecycle_status` distinguishes **`active`** resting orders from
//! **`parked_expired`** (removed during a match walk when `expires_at` has passed — maker claims via
//! `ClaimExpiredLimitOrder`) and **`refunded`** after a successful claim. Default HTTP listing includes
//! **`active`** and **`parked_expired`** so makers see claimable rows; **`refunded`** is excluded unless
//! filtered explicitly.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementLifecycleFilter {
    /// `active` + `parked_expired` (excludes `refunded`).
    DefaultOpen,
    ActiveOnly,
    ParkedExpiredOnly,
    RefundedOnly,
    /// All non-cancelled placements including `refunded`.
    All,
}

#[derive(Debug, Clone, FromRow)]
pub struct PlacementRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub order_id: i64,
    pub owner: Option<String>,
    pub side: Option<String>,
    pub price: Option<BigDecimal>,
    pub expires_at: Option<i64>,
    pub lifecycle_status: String,
    pub remaining_escrow: Option<BigDecimal>,
    pub parked_block_height: Option<i64>,
    pub parked_block_timestamp: Option<DateTime<Utc>>,
    pub parked_tx_hash: Option<String>,
    pub refunded_block_height: Option<i64>,
    pub refunded_block_timestamp: Option<DateTime<Utc>>,
    pub refunded_tx_hash: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CancellationRow {
    pub id: i64,
    pub pair_id: i32,
    pub block_height: i64,
    pub block_timestamp: DateTime<Utc>,
    pub tx_hash: String,
    pub order_id: i64,
    pub owner: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_placement(
    pool: &PgPool,
    pair_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    order_id: i64,
    owner: Option<&str>,
    side: Option<&str>,
    price: Option<&BigDecimal>,
    expires_at: Option<i64>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner, side, price, expires_at, lifecycle_status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
         ON CONFLICT (tx_hash, pair_id, order_id) DO NOTHING
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(order_id)
    .bind(owner)
    .bind(side)
    .bind(price)
    .bind(expires_at)
    .fetch_optional(pool)
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_cancellation(
    pool: &PgPool,
    pair_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    order_id: i64,
    owner: Option<&str>,
) -> Result<Option<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO limit_order_cancellations
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (tx_hash, pair_id, order_id) DO NOTHING
         RETURNING id",
    )
    .bind(pair_id)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(order_id)
    .bind(owner)
    .fetch_optional(pool)
    .await
}

/// Transition **`active` → `parked_expired`** when the pair emits `limit_order_expired_parked` in a taker tx.
#[allow(clippy::too_many_arguments)]
pub async fn apply_parked_expired(
    pool: &PgPool,
    pair_id: i32,
    order_id: i64,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
    remaining: &BigDecimal,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE limit_order_placements
         SET lifecycle_status = 'parked_expired',
             remaining_escrow = $1,
             parked_block_height = $2,
             parked_block_timestamp = $3,
             parked_tx_hash = $4
         WHERE pair_id = $5 AND order_id = $6 AND lifecycle_status = 'active'",
    )
    .bind(remaining)
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(pair_id)
    .bind(order_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

/// Transition **`parked_expired` → `refunded`** when the maker executes `claim_expired_limit_order`.
#[allow(clippy::too_many_arguments)]
pub async fn apply_claim_refund(
    pool: &PgPool,
    pair_id: i32,
    order_id: i64,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: &str,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE limit_order_placements
         SET lifecycle_status = 'refunded',
             refunded_block_height = $1,
             refunded_block_timestamp = $2,
             refunded_tx_hash = $3
         WHERE pair_id = $4 AND order_id = $5 AND lifecycle_status = 'parked_expired'",
    )
    .bind(block_height)
    .bind(block_timestamp)
    .bind(tx_hash)
    .bind(pair_id)
    .bind(order_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn list_placements_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
    lifecycle: PlacementLifecycleFilter,
) -> Result<Vec<PlacementRow>, sqlx::Error> {
    let lifecycle_clause: &'static str = match lifecycle {
        PlacementLifecycleFilter::DefaultOpen => {
            "AND p.lifecycle_status IN ('active', 'parked_expired')"
        }
        PlacementLifecycleFilter::ActiveOnly => "AND p.lifecycle_status = 'active'",
        PlacementLifecycleFilter::ParkedExpiredOnly => "AND p.lifecycle_status = 'parked_expired'",
        PlacementLifecycleFilter::RefundedOnly => "AND p.lifecycle_status = 'refunded'",
        PlacementLifecycleFilter::All => "",
    };

    match before_id {
        Some(bid) => {
            let sql = format!(
                "SELECT p.* FROM limit_order_placements p
                 WHERE p.pair_id = $1 AND p.id < $3
                   AND NOT EXISTS (
                     SELECT 1 FROM limit_order_cancellations c
                     WHERE c.pair_id = p.pair_id AND c.order_id = p.order_id
                   )
                   {lifecycle_clause}
                 ORDER BY p.id DESC LIMIT $2",
            );
            sqlx::query_as::<_, PlacementRow>(&sql)
                .bind(pair_id)
                .bind(limit)
                .bind(bid)
                .fetch_all(pool)
                .await
        }
        None => {
            let sql = format!(
                "SELECT p.* FROM limit_order_placements p
                 WHERE p.pair_id = $1
                   AND NOT EXISTS (
                     SELECT 1 FROM limit_order_cancellations c
                     WHERE c.pair_id = p.pair_id AND c.order_id = p.order_id
                   )
                   {lifecycle_clause}
                 ORDER BY p.id DESC LIMIT $2",
            );
            sqlx::query_as::<_, PlacementRow>(&sql)
                .bind(pair_id)
                .bind(limit)
                .fetch_all(pool)
                .await
        }
    }
}

pub async fn list_cancellations_for_pair(
    pool: &PgPool,
    pair_id: i32,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<CancellationRow>, sqlx::Error> {
    match before_id {
        Some(bid) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations WHERE pair_id = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations WHERE pair_id = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(pair_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}

/// All indexed cancellations for a wallet (`owner` attribute on the wasm event when present).
pub async fn list_cancellations_for_owner(
    pool: &PgPool,
    owner: &str,
    pair_id: Option<i32>,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<CancellationRow>, sqlx::Error> {
    match (before_id, pair_id) {
        (Some(bid), Some(pid)) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations
                 WHERE owner = $1 AND pair_id = $4 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(owner)
            .bind(limit)
            .bind(bid)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (None, Some(pid)) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations
                 WHERE owner = $1 AND pair_id = $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(owner)
            .bind(limit)
            .bind(pid)
            .fetch_all(pool)
            .await
        }
        (Some(bid), None) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations
                 WHERE owner = $1 AND id < $3
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(owner)
            .bind(limit)
            .bind(bid)
            .fetch_all(pool)
            .await
        }
        (None, None) => {
            sqlx::query_as::<_, CancellationRow>(
                "SELECT * FROM limit_order_cancellations
                 WHERE owner = $1
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(owner)
            .bind(limit)
            .fetch_all(pool)
            .await
        }
    }
}
