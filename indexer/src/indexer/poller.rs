use std::time::Duration;

use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::{state, volume};
use crate::lcd::LcdClient;

use super::{
    block_indexer, book_snapshot, fee_discount_registry_health, oracle, pair_discovery,
    trader_tracker, volume_aggregator,
};
use crate::indexer::fee_discount_registry_health::FeeDiscountRegistryHealth;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run_indexer(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
    cancel: tokio_util::sync::CancellationToken,
    ustc_price: oracle::SharedPrice,
    fee_discount_registry_health: FeeDiscountRegistryHealth,
) -> Result<(), BoxError> {
    tracing::info!("Starting pair discovery from factory...");
    if let Err(e) = pair_discovery::sync_all_pairs(&pool, &lcd, &config.factory_address).await {
        tracing::error!("Initial pair sync failed: {}", e);
    }

    if let Err(e) = volume::refresh_pair_volumes(&pool).await {
        tracing::warn!("Initial pair 24h volume refresh failed: {}", e);
    }

    if let Err(e) = volume::refresh_global_stats(&pool).await {
        tracing::warn!("Initial global 24h stats refresh failed: {}", e);
    }

    let vol_pool = pool.clone();
    tokio::spawn(async move {
        volume_aggregator::run_volume_refresh_loop(vol_pool).await;
    });

    let tier_pool = pool.clone();
    let tier_lcd = lcd.clone();
    let fee_addr = config.fee_discount_address.clone();
    tokio::spawn(async move {
        trader_tracker::run_tier_sync_loop(tier_pool, tier_lcd, fee_addr).await;
    });

    if let Some(fee_addr) = config
        .fee_discount_address
        .clone()
        .filter(|a| !a.is_empty())
    {
        let probe_lcd = lcd.clone();
        let probe_health = fee_discount_registry_health.clone();
        let probe_cancel = cancel.clone();
        tokio::spawn(async move {
            fee_discount_registry_health::run_fee_discount_registry_probe_loop(
                probe_lcd,
                fee_addr,
                probe_health,
                probe_cancel,
            )
            .await;
        });
    }

    let oracle_pool = pool.clone();
    let oracle_interval = config.oracle_poll_interval_ms;
    let oracle_price = ustc_price.clone();
    tokio::spawn(async move {
        oracle::run_oracle_loop(oracle_pool, oracle_interval, oracle_price).await;
    });

    let snapshot_pool = pool.clone();
    let snapshot_lcd = lcd.clone();
    let snapshot_interval = config.book_snapshot_interval_ms;
    tokio::spawn(async move {
        book_snapshot::run_book_snapshot_loop(snapshot_pool, snapshot_lcd, snapshot_interval).await;
    });

    let mut last_indexed = state::get_last_indexed_height(&pool).await?;
    if last_indexed == 0 {
        if let Some(start) = config.start_block {
            last_indexed = start.saturating_sub(1);
            tracing::info!(
                "Using start_block config, beginning at height {}",
                last_indexed + 1
            );
        }
    }
    tracing::info!("Indexer starting from height {}", last_indexed + 1);

    loop {
        if cancel.is_cancelled() {
            tracing::info!("Indexer shutting down gracefully");
            break;
        }

        let latest = match lcd.get_latest_block_height().await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to get latest block height: {}", e);
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)) => {},
                    _ = cancel.cancelled() => {
                        tracing::info!("Indexer shutting down gracefully");
                        break;
                    }
                }
                continue;
            }
        };

        if last_indexed >= latest {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)) => {},
                _ = cancel.cancelled() => {
                    tracing::info!("Indexer shutting down gracefully");
                    break;
                }
            }
            continue;
        }

        for height in (last_indexed + 1)..=latest {
            if cancel.is_cancelled() {
                tracing::info!("Indexer shutting down gracefully (mid-catchup)");
                return Ok(());
            }

            if let Err(e) =
                block_indexer::verify_checkpoint_unchanged(&lcd, &pool, last_indexed).await
            {
                tracing::error!(
                    last_indexed,
                    error = %e,
                    "Reorg detected — halting indexer (see docs/runbooks/indexer-reorg-replay-dedup.md)"
                );
                return Err(e.into());
            }

            match block_indexer::index_block_with_retries(
                &pool,
                &lcd,
                &config,
                height,
                &ustc_price,
            )
            .await
            {
                Ok(_meta) => {
                    last_indexed = height;
                    if height % 100 == 0 {
                        tracing::info!("Indexed block {} / {}", height, latest);
                    }
                }
                Err(block_indexer::BlockIndexError::ReorgDetected {
                    height: reorg_height,
                    stored,
                    canonical,
                }) => {
                    tracing::error!(
                        reorg_height,
                        stored_hash = %stored,
                        canonical_hash = %canonical,
                        "Reorg detected — halting indexer"
                    );
                    return Err(block_indexer::BlockIndexError::ReorgDetected {
                        height: reorg_height,
                        stored,
                        canonical,
                    }
                    .into());
                }
                Err(block_indexer::BlockIndexError::MaxRetriesExceeded { height, attempts }) => {
                    tracing::error!(
                        height,
                        attempts,
                        "Block indexing halted; cursor unchanged at {}",
                        last_indexed
                    );
                    return Err(block_indexer::BlockIndexError::MaxRetriesExceeded {
                        height,
                        attempts,
                    }
                    .into());
                }
                Err(e) => {
                    tracing::error!(height, error = %e, "Unexpected block index error");
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(2)) => {},
                        _ = cancel.cancelled() => {
                            tracing::info!("Indexer shutting down gracefully");
                            return Ok(());
                        }
                    }
                    break;
                }
            }
        }

        if last_indexed >= latest {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)) => {},
                _ = cancel.cancelled() => {
                    tracing::info!("Indexer shutting down gracefully");
                    break;
                }
            }
        }
    }

    Ok(())
}
