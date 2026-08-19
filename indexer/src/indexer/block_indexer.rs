//! Single-block ingestion: LCD fetch, parser dispatch, checkpoint commit, reorg guard.
//!
//! Invariants: [`docs/indexer-invariants.md`](../../../docs/indexer-invariants.md) — GitLab **#236**.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::state;
use crate::lcd::{BlockTxsResult, LcdClient, LcdError};

use super::{oracle, parser};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, thiserror::Error)]
pub enum BlockIndexError {
    #[error(
        "chain reorg detected at height {height}: stored hash {stored} != canonical {canonical}"
    )]
    ReorgDetected {
        height: i64,
        stored: String,
        canonical: String,
    },
    #[error("block processing failed at height {height}: {source}")]
    ProcessingFailed {
        height: i64,
        #[source]
        source: BoxError,
    },
    #[error("LCD error at height {height}: {source}")]
    Lcd {
        height: i64,
        #[source]
        source: LcdError,
    },
    #[error("state error at height {height}: {source}")]
    State {
        height: i64,
        #[source]
        source: sqlx::Error,
    },
    #[error(
        "block {height} failed after {attempts} attempts; indexer halted (see indexer_failed_blocks)"
    )]
    MaxRetriesExceeded { height: i64, attempts: u32 },
}

/// Result of the C3 hash guard. `Resync` is **not** a chain reorg — the DB cursor moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointVerify {
    Unchanged,
    /// In-memory poller height does not match `last_indexed_height` (overlapping writer or rewind).
    Resync {
        db_height: i64,
    },
}

/// Verify the stored checkpoint hash still matches canonical chain (reorg guard).
///
/// Compares LCD(hash at **DB** height) to the hash stored with that height. If the in-memory
/// `height` is stale (another poller already committed H+1), returns [`CheckpointVerify::Resync`]
/// instead of treating hash(H+1) vs LCD(H) as a reorg.
pub async fn verify_checkpoint_unchanged(
    lcd: &LcdClient,
    pool: &PgPool,
    height: i64,
) -> Result<CheckpointVerify, BlockIndexError> {
    if height <= 0 {
        return Ok(CheckpointVerify::Unchanged);
    }

    let (db_height, stored) = state::get_indexer_checkpoint(pool)
        .await
        .map_err(|e| BlockIndexError::State { height, source: e })?;

    if db_height != height {
        tracing::warn!(
            poller_height = height,
            db_height,
            "Indexer checkpoint height differs from poller cursor; resyncing (not a reorg)"
        );
        return Ok(CheckpointVerify::Resync { db_height });
    }

    let Some(stored_hash) = stored.filter(|h| !h.is_empty()) else {
        tracing::warn!(
            height,
            "No last_indexed_block_hash stored; skipping reorg check (legacy cursor or post-recovery)"
        );
        return Ok(CheckpointVerify::Unchanged);
    };

    let canonical = lcd
        .get_block_hash(height)
        .await
        .map_err(|e| BlockIndexError::Lcd { height, source: e })?;

    if stored_hash != canonical {
        return Err(BlockIndexError::ReorgDetected {
            height,
            stored: stored_hash,
            canonical,
        });
    }

    Ok(CheckpointVerify::Unchanged)
}

#[derive(Debug)]
pub struct IndexedBlockMeta {
    pub tx_count: usize,
    pub page_count: u32,
}

/// Fetch txs, parse, and commit checkpoint for one height. Does **not** advance on error.
pub async fn index_block(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    height: i64,
    ustc_price: &oracle::SharedPrice,
) -> Result<IndexedBlockMeta, BlockIndexError> {
    let block_hash = lcd
        .get_block_hash(height)
        .await
        .map_err(|e| BlockIndexError::Lcd { height, source: e })?;

    let BlockTxsResult { txs, page_count } = lcd
        .get_block_txs(
            height,
            config.block_tx_page_limit,
            config.block_tx_max_pages,
        )
        .await
        .map_err(|e| BlockIndexError::Lcd { height, source: e })?;

    let tx_count = txs.len();

    if !txs.is_empty() {
        let block_time = resolve_block_time(lcd, height, txs[0].timestamp.as_deref()).await?;

        parser::process_block_txs(pool, lcd, config, &txs, height, block_time, ustc_price)
            .await
            .map_err(|e| BlockIndexError::ProcessingFailed { height, source: e })?;
    }

    state::set_indexer_checkpoint(pool, height, &block_hash)
        .await
        .map_err(|e| BlockIndexError::State { height, source: e })?;

    state::clear_failed_block(pool, height)
        .await
        .map_err(|e| BlockIndexError::State { height, source: e })?;

    tracing::info!(
        height,
        tx_count,
        page_count,
        block_hash = %block_hash,
        "Indexed block"
    );

    Ok(IndexedBlockMeta {
        tx_count,
        page_count,
    })
}

pub async fn index_block_with_retries(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    height: i64,
    ustc_price: &oracle::SharedPrice,
) -> Result<IndexedBlockMeta, BlockIndexError> {
    let max_retries = config.block_process_max_retries;
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        match index_block(pool, lcd, config, height, ustc_price).await {
            Ok(meta) => return Ok(meta),
            Err(e @ BlockIndexError::ReorgDetected { .. }) => return Err(e),
            Err(e @ BlockIndexError::MaxRetriesExceeded { .. }) => return Err(e),
            Err(e @ BlockIndexError::ProcessingFailed { .. })
            | Err(e @ BlockIndexError::Lcd { .. })
            | Err(e @ BlockIndexError::State { .. }) => {
                let msg = e.to_string();
                if let Err(db_err) = state::record_failed_block(pool, height, &msg).await {
                    tracing::error!(
                        height,
                        error = %db_err,
                        "Failed to record indexer_failed_blocks row"
                    );
                }

                if attempt >= max_retries {
                    tracing::error!(
                        height,
                        attempt,
                        max_retries,
                        error = %msg,
                        "Block indexing halted after max retries"
                    );
                    return Err(BlockIndexError::MaxRetriesExceeded {
                        height,
                        attempts: attempt,
                    });
                }

                let backoff_ms = config
                    .block_process_retry_backoff_ms
                    .saturating_mul(attempt as u64);
                tracing::warn!(
                    height,
                    attempt,
                    max_retries,
                    backoff_ms,
                    error = %msg,
                    "Block indexing failed; retrying"
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

/// Resolve chain time for a block: tx timestamp first, then LCD block header (GitLab #243 / M8).
async fn resolve_block_time(
    lcd: &LcdClient,
    height: i64,
    tx_timestamp: Option<&str>,
) -> Result<DateTime<Utc>, BlockIndexError> {
    if let Some(ts) = tx_timestamp {
        if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
            return Ok(dt.with_timezone(&Utc));
        }
        tracing::warn!(
            height,
            tx_timestamp = ts,
            "Invalid tx timestamp; falling back to block header time"
        );
    } else {
        tracing::warn!(
            height,
            "Missing tx timestamp; falling back to block header time"
        );
    }

    let block = lcd
        .get_block_at_height(height)
        .await
        .map_err(|e| BlockIndexError::Lcd { height, source: e })?;

    match DateTime::parse_from_rfc3339(&block.block.header.time) {
        Ok(dt) => Ok(dt.with_timezone(&Utc)),
        Err(e) => {
            tracing::error!(
                height,
                header_time = %block.block.header.time,
                error = %e,
                "Block header time missing or invalid; refusing wall-clock fallback"
            );
            Err(BlockIndexError::ProcessingFailed {
                height,
                source: format!(
                    "block {} has no usable chain timestamp (tx and header both invalid)",
                    height
                )
                .into(),
            })
        }
    }
}

fn parse_rfc3339_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rfc3339_valid() {
        let dt = parse_rfc3339_utc("2024-06-01T12:00:00Z").expect("valid");
        assert_eq!(dt.to_rfc3339(), "2024-06-01T12:00:00+00:00");
    }

    #[test]
    fn parse_rfc3339_invalid_returns_none() {
        assert!(parse_rfc3339_utc("not-a-date").is_none());
    }
}
