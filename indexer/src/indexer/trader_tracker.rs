use std::time::Duration;

use bigdecimal::BigDecimal;
use sqlx::PgPool;

use crate::db::queries::traders::{self, TraderRow};
use crate::lcd::LcdClient;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn update_trader_on_swap(
    pool: &PgPool,
    sender: &str,
    trade_volume: &BigDecimal,
) -> Result<(), BoxError> {
    traders::upsert_trader(pool, sender, trade_volume).await?;
    Ok(())
}

pub async fn run_tier_sync_loop(pool: PgPool, lcd: LcdClient, fee_discount_addr: Option<String>) {
    let addr = match fee_discount_addr {
        Some(a) if !a.is_empty() => a,
        _ => {
            tracing::info!("No fee_discount_address configured, tier sync disabled");
            return;
        }
    };

    loop {
        tokio::time::sleep(Duration::from_secs(600)).await;

        tracing::info!("Running trader tier sync...");
        if let Err(e) = sync_tiers(&pool, &lcd, &addr).await {
            tracing::error!("Tier sync failed: {}", e);
        }
    }
}

async fn sync_tiers(
    pool: &PgPool,
    lcd: &LcdClient,
    fee_discount_addr: &str,
) -> Result<(), BoxError> {
    let all_traders: Vec<TraderRow> = sqlx::query_as("SELECT * FROM traders")
        .fetch_all(pool)
        .await?;

    tracing::info!("Syncing tiers for {} traders", all_traders.len());

    for trader in &all_traders {
        match lcd
            .query_contract::<serde_json::Value>(
                fee_discount_addr,
                &serde_json::json!({"get_registration": {"trader": &trader.address}}),
            )
            .await
        {
            Ok(val) => {
                let tier_id = val.get("tier_id").and_then(|v| v.as_u64()).unwrap_or(0) as i16;
                let tier_name = val
                    .get("tier_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("None")
                    .to_string();
                let registered = val
                    .get("registered")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if let Err(e) = traders::update_trader_tier(
                    pool,
                    &trader.address,
                    tier_id,
                    &tier_name,
                    registered,
                )
                .await
                {
                    tracing::warn!("Failed to update tier for {}: {}", trader.address, e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to query registration for {}: {}", trader.address, e);
            }
        }
    }

    Ok(())
}
