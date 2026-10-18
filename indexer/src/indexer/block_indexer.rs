//! Single-block ingestion: LCD fetch, parser dispatch, checkpoint commit, reorg guard.
//!
//! Invariants: [`docs/indexer-invariants.md`](../../../docs/indexer-invariants.md) — GitLab **#236**.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::state;
use crate::lcd::{BlockTxsResult, LcdClient, LcdError};

use super::{lifecycle::IndexerHealth, oracle, parser};
use tokio_util::sync::CancellationToken;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Process-independent refusal of unsafe replay, using the existing failure ledger.
/// Legacy generic provider failures never acquire this marker implicitly.
pub const UNSAFE_BLOCK_FAILURE_PREFIX: &str = "unsafe_integrity_failure: ";

#[derive(Debug, thiserror::Error)]
pub enum BlockIndexError {
    #[error("block indexing cancelled")]
    Cancelled,
    #[error("unsafe failure at block {height} requires reviewed accounting repair and explicit failure-marker clearance before replay")]
    UnsafeFailurePending { height: i64 },
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

/// Only typed availability/capacity failures may bypass the fatal retry budget.
/// Inspect parser cause chains without matching untrusted upstream error text.
pub fn is_retryable_availability(error: &(dyn std::error::Error + 'static)) -> bool {
    // A valid block/checkpoint/tip request can receive HTTP 400 from an
    // unavailable provider. These reads precede all economic writes; retain the
    // checkpoint and retry through the existing paced, cancellation-owned loop.
    // Keep this check outside cause traversal: the same HTTP error during
    // ProcessingFailed may follow persisted projections and must remain fatal.
    // No response has passed chain/height/hash validation merely by retrying.
    if matches!(
        error.downcast_ref::<BlockIndexError>(),
        Some(BlockIndexError::Lcd {
            source: LcdError::HttpRejected(400),
            ..
        })
    ) {
        return true;
    }
    let mut cause = Some(error);
    while let Some(error) = cause {
        if let Some(lcd) = error.downcast_ref::<LcdError>() {
            return matches!(
                lcd,
                LcdError::AdmissionDeferred
                    | LcdError::AllEndpointsFailed(_)
                    | LcdError::Request(_)
                    | LcdError::LaggingProvider
            );
        }
        if let Some(super::pair_discovery::DiscoverPairError::Deferred {
            reason: "rate_limited" | "in_flight",
        }) = error.downcast_ref::<super::pair_discovery::DiscoverPairError>()
        {
            return true;
        }
        cause = error.source();
    }
    false
}

/// Caller-owned retry scheduling; no detached work or queued requests. Cancellation drops
/// the current provider operation/permit and preserves the checkpoint.
pub async fn retry_availability<T, F, Fut>(
    cancel: &CancellationToken,
    health: &IndexerHealth,
    pending_height: Option<i64>,
    retry_delay: Duration,
    mut operation: F,
) -> Result<T, BlockIndexError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, BlockIndexError>>,
{
    loop {
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(BlockIndexError::Cancelled),
            result = operation() => result,
        };
        match result {
            Err(ref error) if is_retryable_availability(error) => {
                health.unavailable(pending_height);
                tracing::warn!(
                    pending_height,
                    "Indexer provider unavailable; retaining checkpoint"
                );
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return Err(BlockIndexError::Cancelled),
                    _ = tokio::time::sleep(retry_delay.clamp(Duration::from_millis(50), Duration::from_secs(30))) => {}
                }
            }
            result => return result,
        }
    }
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
        if let Some(mapper) = config.wrap_mapper_address.as_deref() {
            super::listing_principal::ingest_block(
                pool,
                lcd,
                mapper,
                &txs,
                height,
                block_time,
                &block_hash,
            )
            .await
            .map_err(|source| BlockIndexError::ProcessingFailed { height, source })?;
        }
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
    index_block_with_retry_policy(pool, lcd, config, height, ustc_price, None).await
}

pub async fn index_block_with_retries_cancel(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    height: i64,
    ustc_price: &oracle::SharedPrice,
    cancel: &CancellationToken,
    health: &IndexerHealth,
) -> Result<IndexedBlockMeta, BlockIndexError> {
    index_block_with_retry_policy(
        pool,
        lcd,
        config,
        height,
        ustc_price,
        Some((cancel, health)),
    )
    .await
}

async fn index_block_with_retry_policy(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    height: i64,
    ustc_price: &oracle::SharedPrice,
    lifecycle: Option<(&CancellationToken, &IndexerHealth)>,
) -> Result<IndexedBlockMeta, BlockIndexError> {
    // A fresh process must not forget that an earlier attempt left incomplete
    // projections. Read before upstream work; only explicit reviewed repair and
    // marker clearance permit replay. Old generic outage rows remain retryable.
    let failed_message = sqlx::query_scalar::<_, String>(
        "SELECT error_message FROM indexer_failed_blocks WHERE height = $1",
    )
    .bind(height)
    .fetch_optional(pool);
    let previous = if let Some((cancel, _)) = lifecycle {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(BlockIndexError::Cancelled),
            result = failed_message => result,
        }
    } else {
        failed_message.await
    };
    let previous = match previous {
        Ok(message) => message,
        Err(source) => {
            if let Some((_, health)) = lifecycle {
                health.halt("block_integrity_failure");
            }
            return Err(BlockIndexError::State { height, source });
        }
    };
    if previous
        .as_deref()
        .is_some_and(|message| message.starts_with(UNSAFE_BLOCK_FAILURE_PREFIX))
    {
        if let Some((_, health)) = lifecycle {
            health.halt("block_integrity_failure");
        }
        return Err(BlockIndexError::UnsafeFailurePending { height });
    }
    let max_retries = config.block_process_max_retries;
    let mut attempt = 0u32;

    loop {
        attempt += 1;
        let result = if let Some((cancel, health)) = lifecycle {
            retry_availability(
                cancel,
                health,
                Some(height),
                Duration::from_millis(config.poll_interval_ms),
                || index_block(pool, lcd, config, height, ustc_price),
            )
            .await
        } else {
            index_block(pool, lcd, config, height, ustc_price).await
        };
        match result {
            Err(e @ BlockIndexError::Cancelled)
            | Err(e @ BlockIndexError::UnsafeFailurePending { .. }) => return Err(e),
            Ok(meta) => return Ok(meta),
            Err(e @ BlockIndexError::ReorgDetected { .. }) => return Err(e),
            Err(e @ BlockIndexError::MaxRetriesExceeded { .. }) => return Err(e),
            Err(e @ BlockIndexError::ProcessingFailed { .. })
            | Err(e @ BlockIndexError::State { .. })
                if !is_retryable_availability(&e) =>
            {
                // Economic projections span several SQL writes. A failed write may
                // follow a persisted swap, whose replay guard intentionally skips
                // additive projections. Never let automatic replay turn that partial
                // state into a successful checkpoint; recovery must be explicit.
                if let Some((_, health)) = lifecycle {
                    health.halt("block_integrity_failure");
                }
                if let Err(db_err) = state::record_failed_block(
                    pool,
                    height,
                    &format!("{UNSAFE_BLOCK_FAILURE_PREFIX}{e}"),
                )
                .await
                {
                    tracing::error!(height, error = %db_err, "Failed to record indexer_failed_blocks row");
                }
                tracing::error!(
                    height,
                    "Unsafe block processing/state failure; halting without automatic replay"
                );
                return Err(e);
            }
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
                if let Some((cancel, _)) = lifecycle {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => return Err(BlockIndexError::Cancelled),
                        _ = tokio::time::sleep(Duration::from_millis(backoff_ms.min(30_000))) => {}
                    }
                } else {
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
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
    fn recovery1464_availability_classification_preserves_integrity() {
        for error in [
            LcdError::AdmissionDeferred,
            LcdError::AllEndpointsFailed("outage".into()),
            LcdError::LaggingProvider,
        ] {
            let nested = BlockIndexError::ProcessingFailed {
                height: 41,
                source: error.into(),
            };
            assert!(is_retryable_availability(&nested));
        }
        for error in [
            LcdError::ChainIdentity("wrong chain".into()),
            LcdError::HistoricalHeightUnavailable,
            LcdError::Deserialize("invalid".into()),
            LcdError::PairResponseMalformed("malformed".into()),
            LcdError::ContractQueryRejected("rejected".into()),
            LcdError::QueryGasLimitExceeded("exhausted".into()),
        ] {
            assert!(!is_retryable_availability(&BlockIndexError::Lcd {
                height: 41,
                source: error
            }));
        }
        assert!(!is_retryable_availability(
            &super::super::pair_discovery::DiscoverPairError::Deferred {
                reason: "factory_namespace_capacity",
            }
        ));
        assert!(!is_retryable_availability(
            &BlockIndexError::ProcessingFailed {
                height: 41,
                source: "accounting overflow".into()
            }
        ));
        assert!(!is_retryable_availability(&BlockIndexError::State {
            height: 41,
            source: sqlx::Error::RowNotFound
        }));
        assert!(!is_retryable_availability(
            &BlockIndexError::ReorgDetected {
                height: 41,
                stored: "A".into(),
                canonical: "B".into()
            }
        ));
    }

    #[test]
    fn recovery1464_http400_retry_is_only_for_prewrite_block_reads() {
        assert!(is_retryable_availability(&BlockIndexError::Lcd {
            height: 41,
            source: LcdError::HttpRejected(400),
        }));
        // A parser may already have committed economic projections. Its HTTP
        // failure must still quarantine the block, never automatically replay.
        assert!(!is_retryable_availability(
            &BlockIndexError::ProcessingFailed {
                height: 41,
                source: LcdError::HttpRejected(400).into(),
            }
        ));
        assert!(!is_retryable_availability(&LcdError::HttpRejected(400)));
        for status in [401, 403, 404, 422] {
            assert!(!is_retryable_availability(&BlockIndexError::Lcd {
                height: 41,
                source: LcdError::HttpRejected(status),
            }));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn recovery1464_deferrals_do_not_exhaust_same_height() {
        let cancel = CancellationToken::new();
        let health = IndexerHealth::default();
        health.progress(40, 41);
        let attempts = std::sync::atomic::AtomicUsize::new(0);
        let result = retry_availability(
            &cancel,
            &health,
            Some(41),
            Duration::from_millis(50),
            || {
                let attempt = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move {
                    match attempt {
                        0..=6 => Err(BlockIndexError::Lcd {
                            height: 41,
                            source: LcdError::AdmissionDeferred,
                        }),
                        7..=12 => Err(BlockIndexError::Lcd {
                            height: 41,
                            source: LcdError::AllEndpointsFailed("all circuits open".into()),
                        }),
                        _ => Ok(41),
                    }
                }
            },
        )
        .await
        .unwrap();
        assert_eq!(result, 41);
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 14);
        assert_eq!(
            health.snapshot().checkpoint_height,
            40,
            "only successful checkpoint commit may advance readiness"
        );
        assert_eq!(health.snapshot().pending_height, Some(41));
    }

    #[tokio::test(start_paused = true)]
    async fn recovery1464_cancel_releases_inflight_operation_and_delay() {
        struct Dropped(std::sync::Arc<std::sync::atomic::AtomicBool>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let cancel = CancellationToken::new();
        let canceller = cancel.clone();
        let health = IndexerHealth::default();
        let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dropped_operation = dropped.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            canceller.cancel();
        });
        let error = retry_availability::<(), _, _>(
            &cancel,
            &health,
            Some(41),
            Duration::from_secs(30),
            || {
                let guard = Dropped(dropped_operation.clone());
                async move {
                    let _guard = guard;
                    std::future::pending().await
                }
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, BlockIndexError::Cancelled));
        assert!(dropped.load(std::sync::atomic::Ordering::SeqCst));

        let cancel = CancellationToken::new();
        let canceller = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            canceller.cancel();
        });
        let began = tokio::time::Instant::now();
        let error = retry_availability::<(), _, _>(
            &cancel,
            &health,
            Some(41),
            Duration::from_secs(30),
            || async {
                Err(BlockIndexError::Lcd {
                    height: 41,
                    source: LcdError::AdmissionDeferred,
                })
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, BlockIndexError::Cancelled));
        assert!(began.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test(start_paused = true)]
    async fn recovery1464_reorg_and_accounting_errors_do_not_enter_availability_loop() {
        let cancel = CancellationToken::new();
        let health = IndexerHealth::default();
        let attempts = std::sync::atomic::AtomicUsize::new(0);
        let error = retry_availability::<(), _, _>(
            &cancel,
            &health,
            Some(41),
            Duration::from_secs(1),
            || {
                attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async {
                    Err(BlockIndexError::ProcessingFailed {
                        height: 41,
                        source: "accounting overflow".into(),
                    })
                }
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(error, BlockIndexError::ProcessingFailed { .. }));
        assert_eq!(attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

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
