use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::state;
use crate::lcd::LcdClient;

use super::{pair_discovery, parser, trader_tracker, volume_aggregator};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run_indexer(pool: PgPool, lcd: LcdClient, config: Config) -> Result<(), BoxError> {
    tracing::info!("Starting pair discovery from factory...");
    if let Err(e) = pair_discovery::sync_all_pairs(&pool, &lcd, &config.factory_address).await {
        tracing::error!("Initial pair sync failed: {}", e);
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
        let latest = match lcd.get_latest_block_height().await {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("Failed to get latest block height: {}", e);
                tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)).await;
                continue;
            }
        };

        if last_indexed >= latest {
            tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)).await;
            continue;
        }

        for height in (last_indexed + 1)..=latest {
            match lcd.get_block_txs(height).await {
                Ok(tx_resp) => {
                    let txs = tx_resp.tx_responses.unwrap_or_default();

                    if !txs.is_empty() {
                        let block_time = parse_block_time(txs[0].timestamp.as_deref());

                        if let Err(e) = parser::process_block_txs(
                            &pool,
                            &lcd,
                            &config,
                            &txs,
                            height,
                            block_time,
                        )
                        .await
                        {
                            tracing::error!("Error processing block {}: {}", height, e);
                        }
                    }

                    if let Err(e) = state::set_last_indexed_height(&pool, height).await {
                        tracing::error!("Failed to update last_indexed_height: {}", e);
                    }
                    last_indexed = height;

                    if height % 100 == 0 {
                        tracing::info!("Indexed block {} / {}", height, latest);
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to fetch block {}: {}", height, e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    break;
                }
            }
        }

        if last_indexed >= latest {
            tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)).await;
        }
    }
}

fn parse_block_time(ts: Option<&str>) -> DateTime<Utc> {
    ts.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}
