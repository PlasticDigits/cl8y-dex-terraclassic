use std::time::Duration;

use sqlx::{Connection, PgPool};

use crate::config::Config;
use crate::db::queries::state;
use crate::lcd::LcdClient;

use super::{
    asset_code_id_freeze, block_indexer, book_snapshot, community_tokens, fee_discount_registry_health,
    oracle, pair_discovery, reorg_alert, trader_tracker, venus_vfdusd, volume_aggregator,
};
use crate::indexer::fee_discount_registry_health::FeeDiscountRegistryHealth;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run_indexer(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
    cancel: tokio_util::sync::CancellationToken,
    oracle_prices: oracle::OraclePriceHandles,
    venus_vfdusd: venus_vfdusd::SharedVenusVfdusd,
    fee_discount_registry_health: FeeDiscountRegistryHealth,
) -> Result<(), BoxError> {
    // Dedicated session (not a pool checkout): advisory locks survive `PoolConnection` return.
    let mut lock_conn = sqlx::PgConnection::connect(&config.database_url).await?;
    let mut last_wait_log = std::time::Instant::now()
        .checked_sub(Duration::from_secs(10))
        .unwrap_or_else(std::time::Instant::now);
    loop {
        if cancel.is_cancelled() {
            tracing::info!("Indexer shutting down before acquiring poller lock");
            return Ok(());
        }
        if state::try_acquire_poller_lock(&mut lock_conn).await? {
            tracing::info!("Acquired indexer poller advisory lock");
            break;
        }
        if last_wait_log.elapsed() >= Duration::from_secs(10) {
            tracing::warn!("Another indexer holds the poller lock; waiting (overlapping deploy?)");
            last_wait_log = std::time::Instant::now();
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(500)) => {}
            _ = cancel.cancelled() => {
                tracing::info!("Indexer shutting down before acquiring poller lock");
                return Ok(());
            }
        }
    }
    let _poller_lock = lock_conn;

    tracing::info!("Starting pair discovery from factory...");
    if let Err(e) = pair_discovery::sync_all_pairs(&pool, &lcd, &config.factory_address).await {
        tracing::error!("Initial pair sync failed: {}", e);
    }

    // Token + trader windows too — do not wait for the 5 min loop (GitLab #577 **D5**).
    volume_aggregator::refresh_all_volume_windows(&pool, true).await;

    let vol_pool = pool.clone();
    tokio::spawn(async move {
        volume_aggregator::run_volume_refresh_loop(vol_pool).await;
    });

    let tier_pool = pool.clone();
    let tier_lcd = lcd.clone();
    let fee_addr = config.fee_discount_address.clone();
    let tier_reconcile_secs = config.tier_sync_reconcile_interval_secs;
    tokio::spawn(async move {
        trader_tracker::run_tier_reconcile_loop(tier_pool, tier_lcd, fee_addr, tier_reconcile_secs)
            .await;
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

    {
        let freeze_pool = pool.clone();
        let freeze_lcd = lcd.clone();
        let freeze_factory = config.factory_address.clone();
        let freeze_cancel = cancel.clone();
        tokio::spawn(async move {
            asset_code_id_freeze::run_code_id_freeze_probe_loop(
                freeze_pool,
                freeze_lcd,
                freeze_factory,
                freeze_cancel,
            )
            .await;
        });
    }

    {
        let tax_pool = pool.clone();
        let tax_lcd = lcd.clone();
        let tax_cfg = config.clone();
        let tax_cancel = cancel.clone();
        tokio::spawn(async move {
            community_tokens::run_probe_loop(tax_pool, tax_lcd, tax_cfg, tax_cancel).await;
        });
    }

    let oracle_pool = pool.clone();
    let oracle_interval = config.oracle_poll_interval_ms;
    let oracle_handles = oracle_prices.clone();
    tokio::spawn(async move {
        oracle::run_oracle_loop(oracle_pool, oracle_interval, oracle_handles).await;
    });

    let venus_pool = pool.clone();
    let venus_cfg = venus_vfdusd::VenusPollerConfig::from_indexer_config(&config);
    let venus_handle = venus_vfdusd.clone();
    tokio::spawn(async move {
        venus_vfdusd::run_venus_vfdusd_loop(venus_pool, venus_cfg, venus_handle).await;
    });
    let ustc_price = oracle_prices.ustc.clone();

    let snapshot_pool = pool.clone();
    let snapshot_lcd = lcd.clone();
    let snapshot_interval = config.book_snapshot_interval_ms;
    tokio::spawn(async move {
        book_snapshot::run_book_snapshot_loop(snapshot_pool, snapshot_lcd, snapshot_interval).await;
    });

    let hub_cfg = crate::indexer::hub_usd::HubUsdConfig::from_indexer_config(&config);
    let hub_pool = pool.clone();
    let hub_ustc = oracle_prices.ustc.clone();
    let hub_lunc = oracle_prices.lunc.clone();
    let hub_interval = std::time::Duration::from_millis(snapshot_interval.max(1_000));
    tokio::spawn(async move {
        crate::db::queries::hub_prices::run_hub_usd_refresh_loop(
            hub_pool,
            hub_cfg,
            hub_ustc,
            hub_lunc,
            hub_interval,
        )
        .await;
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

            match block_indexer::verify_checkpoint_unchanged(&lcd, &pool, last_indexed).await {
                Ok(block_indexer::CheckpointVerify::Unchanged) => {}
                Ok(block_indexer::CheckpointVerify::Resync { db_height }) => {
                    tracing::warn!(
                        last_indexed,
                        db_height,
                        "Adopting database checkpoint and restarting catch-up"
                    );
                    last_indexed = db_height;
                    break;
                }
                Err(e) => {
                    if let block_indexer::BlockIndexError::ReorgDetected {
                        height,
                        stored,
                        canonical,
                    } = &e
                    {
                        reorg_alert::emit_reorg_halt(&reorg_alert::ReorgHaltDetails::new(
                            *height,
                            stored.clone(),
                            canonical.clone(),
                        ))
                        .await;
                    }
                    return Err(e.into());
                }
            }

            match block_indexer::index_block_with_retries(&pool, &lcd, &config, height, &ustc_price)
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
                    reorg_alert::emit_reorg_halt(&reorg_alert::ReorgHaltDetails::new(
                        reorg_height,
                        stored.clone(),
                        canonical.clone(),
                    ))
                    .await;
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
