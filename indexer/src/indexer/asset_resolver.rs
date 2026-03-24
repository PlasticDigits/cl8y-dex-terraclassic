use sqlx::PgPool;

use crate::db::queries::assets;
use crate::lcd::types::{AssetInfo, Cw20TokenInfoResponse};
use crate::lcd::LcdClient;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn resolve_asset(
    pool: &PgPool,
    lcd: &LcdClient,
    asset_info: &AssetInfo,
) -> Result<i32, BoxError> {
    match asset_info {
        AssetInfo::Token { contract_addr } => {
            if let Some(asset) = assets::get_asset_by_contract(pool, contract_addr).await? {
                return Ok(asset.id);
            }

            let token_info: Cw20TokenInfoResponse = lcd
                .query_contract(contract_addr, &serde_json::json!({"token_info": {}}))
                .await?;

            let id = assets::upsert_asset(
                pool,
                Some(contract_addr),
                None,
                true,
                &token_info.name,
                &token_info.symbol,
                token_info.decimals as i16,
                None,
            )
            .await?;

            tracing::info!(
                "Resolved new CW20 asset: {} ({}) -> id {}",
                token_info.symbol,
                contract_addr,
                id
            );
            Ok(id)
        }
        AssetInfo::NativeToken { denom } => {
            if let Some(asset) = assets::get_asset_by_denom(pool, denom).await? {
                return Ok(asset.id);
            }

            let id =
                assets::upsert_asset(pool, None, Some(denom), false, denom, denom, 6, None).await?;

            tracing::info!("Resolved new native asset: {} -> id {}", denom, id);
            Ok(id)
        }
    }
}

pub async fn resolve_asset_str(
    pool: &PgPool,
    lcd: &LcdClient,
    asset_str: &str,
) -> Result<i32, BoxError> {
    let info = if asset_str.starts_with("terra1") && asset_str.len() >= 44 {
        AssetInfo::Token {
            contract_addr: asset_str.to_string(),
        }
    } else {
        AssetInfo::NativeToken {
            denom: asset_str.to_string(),
        }
    };
    resolve_asset(pool, lcd, &info).await
}
