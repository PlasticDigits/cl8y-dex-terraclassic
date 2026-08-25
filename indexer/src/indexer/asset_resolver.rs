use sqlx::PgPool;

use crate::db::queries::assets;
use crate::lcd::types::{AssetInfo, Cw20TokenInfoResponse};
use crate::lcd::LcdClient;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Retail catalog labels for known bank denoms (GitLab #630).
/// Unknown denoms stay denom/denom (fail closed). Wrap CW20s are not natives.
pub fn native_retail_meta(denom: &str) -> (&'static str, &'static str) {
    match denom.to_ascii_lowercase().as_str() {
        "uluna" => ("Terra Luna Classic", "LUNC"),
        "uusd" => ("TerraClassicUSD", "USTC"),
        _ => ("", ""),
    }
}

pub fn is_known_bank_denom(denom: &str) -> bool {
    matches!(denom.to_ascii_lowercase().as_str(), "uluna" | "uusd")
}

fn native_insert_labels<'a>(denom: &'a str) -> (&'a str, &'a str) {
    let (name, symbol) = native_retail_meta(denom);
    if name.is_empty() {
        (denom, denom)
    } else {
        (name, symbol)
    }
}

pub async fn resolve_asset(
    pool: &PgPool,
    lcd: &LcdClient,
    asset_info: &AssetInfo,
) -> Result<i32, BoxError> {
    match asset_info {
        AssetInfo::Token { contract_addr } => {
            let partial_asset = assets::get_asset_by_contract(pool, contract_addr).await?;
            if let Some(ref asset) = partial_asset {
                if !asset.name.trim().is_empty() && !asset.symbol.trim().is_empty() {
                    return Ok(asset.id);
                }
                tracing::debug!(
                    "Refreshing CW20 metadata for {} (name/symbol missing in DB)",
                    contract_addr
                );
            }

            let token_info: Cw20TokenInfoResponse = match lcd
                .query_contract(contract_addr, &serde_json::json!({"token_info": {}}))
                .await
            {
                Ok(info) => info,
                Err(e) => {
                    if let Some(asset) = partial_asset {
                        tracing::warn!(
                            "LCD token_info failed for {} (using existing asset id {}, will retry metadata refresh): {}",
                            contract_addr,
                            asset.id,
                            e
                        );
                        return Ok(asset.id);
                    }
                    return Err(e.into());
                }
            };

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
            let (name, symbol) = native_insert_labels(denom);
            if let Some(asset) = assets::get_asset_by_denom(pool, denom).await? {
                if is_known_bank_denom(denom) && asset.symbol.eq_ignore_ascii_case(denom) {
                    assets::upsert_asset(pool, None, Some(denom), false, name, symbol, 6, None)
                        .await?;
                    tracing::info!(
                        "Repaired native asset labels: {} -> {} / {}",
                        denom,
                        name,
                        symbol
                    );
                }
                return Ok(asset.id);
            }

            let id =
                assets::upsert_asset(pool, None, Some(denom), false, name, symbol, 6, None).await?;

            tracing::info!(
                "Resolved new native asset: {} ({}) -> id {}",
                symbol,
                denom,
                id
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_bank_denoms_map_to_retail_tickers() {
        assert_eq!(native_retail_meta("uluna"), ("Terra Luna Classic", "LUNC"));
        assert_eq!(native_retail_meta("ULUNA"), ("Terra Luna Classic", "LUNC"));
        assert_eq!(native_retail_meta("uusd"), ("TerraClassicUSD", "USTC"));
        assert_eq!(native_retail_meta("UUSD"), ("TerraClassicUSD", "USTC"));
        assert!(is_known_bank_denom("uluna"));
        assert!(is_known_bank_denom("uusd"));
    }

    #[test]
    fn unknown_denoms_stay_raw() {
        assert_eq!(native_retail_meta("usdr"), ("", ""));
        assert_eq!(native_retail_meta("ibc/ABC"), ("", ""));
        assert_eq!(native_insert_labels("usdr"), ("usdr", "usdr"));
        assert_eq!(native_insert_labels("ibc/ABC"), ("ibc/ABC", "ibc/ABC"));
        assert!(!is_known_bank_denom("usdr"));
        assert!(!is_known_bank_denom("ufoo"));
    }

    #[test]
    fn wrap_cw20_addresses_are_not_bank_denoms() {
        assert!(!is_known_bank_denom(
            "terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg"
        ));
    }
}
