use std::time::Duration;

use sqlx::{Connection, PgPool};

use crate::config::Config;
use crate::db::queries::state;
use crate::lcd::LcdClient;

use super::{
    asset_code_id_freeze, block_indexer, book_snapshot, community_tokens,
    fee_discount_registry_health, oracle, pair_discovery, reorg_alert, trader_tracker,
    venus_vfdusd, volume_aggregator,
};
use crate::indexer::fee_discount_registry_health::FeeDiscountRegistryHealth;

use super::lifecycle::IndexerHealth;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

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
    run_indexer_with_health(
        pool,
        lcd,
        config,
        cancel,
        oracle_prices,
        venus_vfdusd,
        fee_discount_registry_health,
        IndexerHealth::default(),
    )
    .await
}

pub async fn run_indexer_with_health(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
    cancel: CancellationToken,
    oracle_prices: oracle::OraclePriceHandles,
    venus_vfdusd: venus_vfdusd::SharedVenusVfdusd,
    fee_discount_registry_health: FeeDiscountRegistryHealth,
    health: IndexerHealth,
) -> Result<(), BoxError> {
    let worker_cancel = cancel.child_token();
    let mut workers = JoinSet::new();
    let result = tokio::select! {
        biased;
        _ = worker_cancel.cancelled() => Ok(()),
        result = run_indexer_inner(pool, lcd, config, worker_cancel.clone(), oracle_prices,
            venus_vfdusd, fee_discount_registry_health, &health, &mut workers) => result,
    };
    if let Err(error) = &result {
        let reason = error
            .downcast_ref::<block_indexer::BlockIndexError>()
            .map_or("indexing_failure", |error| {
                if matches!(error, block_indexer::BlockIndexError::ReorgDetected { .. }) {
                    "reorg"
                } else {
                    "block_integrity_failure"
                }
            });
        health.halt(reason);
    } else {
        health.stopped();
    }
    worker_cancel.cancel();
    workers.shutdown().await;
    result
}

fn spawn_worker<F>(
    workers: &mut JoinSet<()>,
    cancel: &CancellationToken,
    health: &IndexerHealth,
    worker: F,
) where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let cancel = cancel.clone();
    let halted = health.halt_token();
    workers.spawn(async move {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {},
            _ = halted.cancelled() => {},
            _ = worker => {},
        }
    });
}

async fn run_indexer_inner(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
    cancel: CancellationToken,
    oracle_prices: oracle::OraclePriceHandles,
    venus_vfdusd: venus_vfdusd::SharedVenusVfdusd,
    fee_discount_registry_health: FeeDiscountRegistryHealth,
    health: &IndexerHealth,
    workers: &mut JoinSet<()>,
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
    // The dedicated lock session is monitored even while the poller is awaiting a provider.
    // Session loss latches unready and cancels owned work; bounded heartbeat detection
    // can briefly overlap another writer, so this is not distributed exactly-once delivery.
    let lock_health = health.clone();
    let lock_cancel = cancel.clone();
    spawn_worker(workers, &cancel, health, async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !matches!(
                tokio::time::timeout(
                    Duration::from_millis(500),
                    sqlx::query("SELECT 1").execute(&mut lock_conn)
                )
                .await,
                Ok(Ok(_))
            ) {
                lock_health.halt("leadership_lost");
                lock_cancel.cancel();
                break;
            }
        }
    });

    tracing::info!("Starting pair discovery from factory...");
    if let Err(e) = pair_discovery::sync_all_pairs(&pool, &lcd, &config.factory_address).await {
        tracing::error!("Initial pair sync failed: {}", e);
    }

    // Heal swap_amm hops dropped by UNIQUE (tx_hash, source, ordinal) before first
    // rollup (GitLab #1269). Replay cannot: trade_exists skips swap insert.
    if let Err(e) = crate::db::queries::protocol_fees::backfill_missing_swap_amm_fees(&pool).await {
        tracing::error!("swap_amm hop fee backfill failed: {e}");
    }

    if let Err(e) =
        crate::db::queries::traders::heal_trader_lifetime_from_swaps_if_needed(&pool).await
    {
        tracing::error!("trader lifetime heal failed: {e}");
    }

    // Token + trader windows too — do not wait for the 5 min loop (GitLab #577 **D5**).
    volume_aggregator::refresh_all_volume_windows(&pool, true).await;

    // Heal leftover 18-dec undercounts after NUMERIC(38,18) overflow (#676).
    if let Err(e) = super::position_tracker::repair_positions_if_trade_count_mismatch(&pool).await {
        tracing::error!("trader_positions rebuild failed: {e}");
    }

    let vol_pool = pool.clone();
    spawn_worker(workers, &cancel, health, async move {
        volume_aggregator::run_volume_refresh_loop(vol_pool).await;
    });

    let tier_pool = pool.clone();
    let tier_lcd = lcd.clone();
    let fee_addr = config.fee_discount_address.clone();
    let tier_reconcile_secs = config.tier_sync_reconcile_interval_secs;
    spawn_worker(workers, &cancel, health, async move {
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
        spawn_worker(workers, &cancel, health, async move {
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
        spawn_worker(workers, &cancel, health, async move {
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
        spawn_worker(workers, &cancel, health, async move {
            community_tokens::run_probe_loop(tax_pool, tax_lcd, tax_cfg, tax_cancel).await;
        });
    }

    let oracle_pool = pool.clone();
    let oracle_interval = config.oracle_poll_interval_ms;
    let oracle_handles = oracle_prices.clone();
    spawn_worker(workers, &cancel, health, async move {
        oracle::run_oracle_loop(oracle_pool, oracle_interval, oracle_handles).await;
    });

    let venus_pool = pool.clone();
    let venus_cfg = venus_vfdusd::VenusPollerConfig::from_indexer_config(&config);
    let venus_handle = venus_vfdusd.clone();
    spawn_worker(workers, &cancel, health, async move {
        venus_vfdusd::run_venus_vfdusd_loop(venus_pool, venus_cfg, venus_handle).await;
    });
    let ustc_price = oracle_prices.ustc.clone();

    if let Some(mapper) = config.wrap_mapper_address.clone().filter(|a| !a.is_empty()) {
        let backing_pool = pool.clone();
        let backing_lcd = lcd.clone();
        let backing_cancel = cancel.clone();
        spawn_worker(workers, &cancel, health, async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                tokio::select! {
                    _ = backing_cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if super::listing_principal::refresh_custody(&backing_pool, &backing_lcd, &mapper).await.is_err() {
                            tracing::warn!(event_kind="listing_principal", "Mapper backing refresh unavailable");
                        }
                    }
                }
            }
        });
    }

    let snapshot_pool = pool.clone();
    let snapshot_lcd = lcd.clone();
    let snapshot_interval = config.book_snapshot_interval_ms;
    spawn_worker(workers, &cancel, health, async move {
        book_snapshot::run_book_snapshot_loop(snapshot_pool, snapshot_lcd, snapshot_interval).await;
    });

    let hub_cfg = crate::indexer::hub_usd::HubUsdConfig::from_indexer_config(&config);
    let hub_pool = pool.clone();
    let hub_ustc = oracle_prices.ustc.clone();
    let hub_lunc = oracle_prices.lunc.clone();
    let hub_interval = std::time::Duration::from_millis(snapshot_interval.max(1_000));
    spawn_worker(workers, &cancel, health, async move {
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

        let latest = match block_indexer::retry_availability(
            &cancel,
            health,
            Some(last_indexed.saturating_add(1)),
            Duration::from_millis(config.poll_interval_ms),
            || async {
                lcd.get_latest_block_height().await.map_err(|source| {
                    block_indexer::BlockIndexError::Lcd {
                        height: last_indexed,
                        source,
                    }
                })
            },
        )
        .await
        {
            Ok(h) => h,
            Err(block_indexer::BlockIndexError::Cancelled) => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        health.progress(last_indexed, latest);

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

            match block_indexer::retry_availability(
                &cancel,
                health,
                Some(height),
                Duration::from_millis(config.poll_interval_ms),
                || block_indexer::verify_checkpoint_unchanged(&lcd, &pool, last_indexed),
            )
            .await
            {
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
                Err(block_indexer::BlockIndexError::Cancelled) => return Ok(()),
                Err(e) => {
                    if let block_indexer::BlockIndexError::ReorgDetected {
                        height,
                        stored,
                        canonical,
                    } = &e
                    {
                        health.halt("reorg");
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

            match block_indexer::index_block_with_retries_cancel(
                &pool,
                &lcd,
                &config,
                height,
                &ustc_price,
                &cancel,
                health,
            )
            .await
            {
                Ok(_meta) => {
                    last_indexed = height;
                    health.progress(last_indexed, latest);
                    if height % 100 == 0 {
                        tracing::info!(height, latest, discovery = ?lcd.pair_discovery_counters(), "Indexed block");
                    }
                }
                Err(block_indexer::BlockIndexError::Cancelled) => return Ok(()),
                Err(block_indexer::BlockIndexError::ReorgDetected {
                    height: reorg_height,
                    stored,
                    canonical,
                }) => {
                    health.halt("reorg");
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
                Err(e) => return Err(e.into()),
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
