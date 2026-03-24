use sqlx::PgPool;

use crate::db::queries::pairs::{self, PairRow};
use crate::lcd::types::*;
use crate::lcd::LcdClient;

use super::asset_resolver;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn sync_all_pairs(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
) -> Result<(), BoxError> {
    if factory_addr.is_empty() {
        tracing::warn!("Factory address is empty, skipping pair discovery");
        return Ok(());
    }

    tracing::info!("Syncing all pairs from factory {}", factory_addr);

    let mut start_after: Option<[serde_json::Value; 2]> = None;
    let mut total = 0u32;

    loop {
        let query = match &start_after {
            Some(sa) => serde_json::json!({
                "pairs": {
                    "start_after": sa,
                    "limit": 30
                }
            }),
            None => serde_json::json!({
                "pairs": {
                    "limit": 30
                }
            }),
        };

        let resp: PairsResponse = match lcd.query_contract(factory_addr, &query).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to query factory pairs: {}", e);
                break;
            }
        };

        if resp.pairs.is_empty() {
            break;
        }

        let batch_len = resp.pairs.len();

        for pair_info in &resp.pairs {
            match sync_single_pair(pool, lcd, pair_info).await {
                Ok(row) => {
                    tracing::info!(
                        "Synced pair {} (id={}, assets={}/{})",
                        row.contract_address,
                        row.id,
                        row.asset_0_id,
                        row.asset_1_id
                    );
                    total += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to sync pair {}: {}", pair_info.contract_addr, e);
                }
            }
        }

        let last = &resp.pairs[batch_len - 1];
        start_after = Some([
            asset_info_to_json(&last.asset_infos[0]),
            asset_info_to_json(&last.asset_infos[1]),
        ]);

        if batch_len < 30 {
            break;
        }
    }

    tracing::info!("Pair discovery complete: {} pairs synced", total);
    Ok(())
}

pub async fn sync_single_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    pair_info: &PairInfo,
) -> Result<PairRow, BoxError> {
    let asset_0_id = asset_resolver::resolve_asset(pool, lcd, &pair_info.asset_infos[0]).await?;
    let asset_1_id = asset_resolver::resolve_asset(pool, lcd, &pair_info.asset_infos[1]).await?;

    let pair_id = pairs::upsert_pair(
        pool,
        &pair_info.contract_addr,
        asset_0_id,
        asset_1_id,
        Some(&pair_info.liquidity_token),
        None,
        &[],
        None,
    )
    .await?;

    let fee_bps: Option<i16> = match lcd
        .query_contract::<FeeConfigResponse>(
            &pair_info.contract_addr,
            &serde_json::json!({"get_fee_config": {}}),
        )
        .await
    {
        Ok(resp) => Some(resp.fee_config.fee_bps as i16),
        Err(e) => {
            tracing::warn!(
                "Failed to query fee config for {}: {}",
                pair_info.contract_addr,
                e
            );
            None
        }
    };

    let hooks: Vec<String> = match lcd
        .query_contract::<HooksResponse>(
            &pair_info.contract_addr,
            &serde_json::json!({"get_hooks": {}}),
        )
        .await
    {
        Ok(resp) => resp.hooks,
        Err(e) => {
            tracing::warn!(
                "Failed to query hooks for {}: {}",
                pair_info.contract_addr,
                e
            );
            Vec::new()
        }
    };

    pairs::update_pair_config(pool, pair_id, fee_bps, &hooks).await?;

    let pair = pairs::get_pair_by_address(pool, &pair_info.contract_addr)
        .await?
        .ok_or_else(|| format!("Pair {} not found after upsert", pair_info.contract_addr))?;

    Ok(pair)
}

pub async fn discover_new_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    pair_contract_addr: &str,
) -> Result<PairRow, BoxError> {
    tracing::info!("Discovering new pair at {}", pair_contract_addr);

    let pair_info: PairInfo = lcd
        .query_contract(pair_contract_addr, &serde_json::json!({"pair": {}}))
        .await?;

    sync_single_pair(pool, lcd, &pair_info).await
}

fn asset_info_to_json(info: &AssetInfo) -> serde_json::Value {
    match info {
        AssetInfo::Token { contract_addr } => {
            serde_json::json!({"token": {"contract_addr": contract_addr}})
        }
        AssetInfo::NativeToken { denom } => {
            serde_json::json!({"native_token": {"denom": denom}})
        }
    }
}
