use sqlx::PgPool;

use crate::db::queries::pairs::{self, PairRow};
use crate::lcd::types::{
    AssetInfo, FactoryPairResponse, FeeConfigResponse, HooksResponse, PairInfo, PairsResponse,
};
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

/// Ensures `pair_contract_addr` is the factory-listed pair for `asset_infos`.
/// When `factory_addr` is empty (dev only), provenance is skipped with a warning.
pub(crate) async fn verify_factory_provenance(
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
    asset_infos: &[AssetInfo; 2],
) -> Result<(), BoxError> {
    if factory_addr.is_empty() {
        tracing::warn!(
            "FACTORY_ADDRESS is empty; skipping factory provenance check for {}",
            pair_contract_addr
        );
        return Ok(());
    }

    let query = serde_json::json!({
        "pair": {
            "asset_infos": [
                asset_info_to_json(&asset_infos[0]),
                asset_info_to_json(&asset_infos[1]),
            ]
        }
    });

    let resp: FactoryPairResponse = lcd
        .query_contract(factory_addr, &query)
        .await
        .map_err(|e| {
            format!(
                "factory provenance check failed for {} (factory {}): {}",
                pair_contract_addr, factory_addr, e
            )
        })?;

    if resp.pair.contract_addr != pair_contract_addr {
        return Err(format!(
            "pair {} is not listed in factory {} (factory maps assets to {})",
            pair_contract_addr, factory_addr, resp.pair.contract_addr
        )
        .into());
    }

    Ok(())
}

pub async fn discover_new_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
) -> Result<PairRow, BoxError> {
    tracing::info!("Discovering new pair at {}", pair_contract_addr);

    let pair_info: PairInfo = lcd
        .query_contract(pair_contract_addr, &serde_json::json!({"pair": {}}))
        .await?;

    if pair_info.contract_addr != pair_contract_addr {
        return Err(format!(
            "pair query at {} returned contract_addr {}",
            pair_contract_addr, pair_info.contract_addr
        )
        .into());
    }

    verify_factory_provenance(lcd, factory_addr, pair_contract_addr, &pair_info.asset_infos)
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use serde_json::json;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const FACTORY: &str = "terra1factory";
    const CANONICAL_PAIR: &str = "terra1canonicalpair";
    const ATTACKER_PAIR: &str = "terra1attackerpair";
    const TOKEN_A: &str = "terra1tokena";
    const TOKEN_B: &str = "terra1tokenb";

    fn sample_pair_info(contract_addr: &str) -> PairInfo {
        PairInfo {
            asset_infos: [
                AssetInfo::Token {
                    contract_addr: TOKEN_A.to_string(),
                },
                AssetInfo::Token {
                    contract_addr: TOKEN_B.to_string(),
                },
            ],
            contract_addr: contract_addr.to_string(),
            liquidity_token: format!("{contract_addr}_lp"),
        }
    }

    fn pair_info_json(contract_addr: &str) -> serde_json::Value {
        json!({
            "asset_infos": [
                { "token": { "contract_addr": TOKEN_A } },
                { "token": { "contract_addr": TOKEN_B } }
            ],
            "contract_addr": contract_addr,
            "liquidity_token": format!("{contract_addr}_lp")
        })
    }

    async fn mount_pair_and_factory_mocks(
        server: &MockServer,
        attacker_responds_to_pair_query: bool,
        factory_lists_canonical: bool,
    ) {
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |req: &wiremock::Request| {
                let path = req.url.path();
                let segments: Vec<_> = path.split('/').collect();
                let contract = segments.get(5).copied().unwrap_or("");
                let query_b64 = segments.get(7).copied().unwrap_or("");
                let query_bytes = base64::engine::general_purpose::STANDARD
                    .decode(query_b64)
                    .unwrap_or_default();
                let query: serde_json::Value =
                    serde_json::from_slice(&query_bytes).unwrap_or(json!({}));

                let body = if contract == ATTACKER_PAIR {
                    if !attacker_responds_to_pair_query {
                        return ResponseTemplate::new(500);
                    }
                    json!({ "data": pair_info_json(ATTACKER_PAIR) })
                } else if contract == CANONICAL_PAIR {
                    json!({ "data": pair_info_json(CANONICAL_PAIR) })
                } else if contract == FACTORY {
                    if query.get("pair").is_some() {
                        if factory_lists_canonical {
                            json!({ "data": { "pair": pair_info_json(CANONICAL_PAIR) } })
                        } else {
                            return ResponseTemplate::new(500).set_body_json(json!({
                                "message": "pair not found"
                            }));
                        }
                    } else {
                        return ResponseTemplate::new(404);
                    }
                } else {
                    return ResponseTemplate::new(404);
                };

                ResponseTemplate::new(200).set_body_json(body)
            })
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn verify_factory_provenance_rejects_unlisted_pair() {
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, true).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not listed in factory"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn verify_factory_provenance_accepts_factory_listed_pair() {
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, true).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(CANONICAL_PAIR).asset_infos;

        verify_factory_provenance(&lcd, FACTORY, CANONICAL_PAIR, &assets)
            .await
            .expect("canonical pair should pass");
    }

    #[tokio::test]
    async fn verify_factory_provenance_skipped_when_factory_empty() {
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, false).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        verify_factory_provenance(&lcd, "", ATTACKER_PAIR, &assets)
            .await
            .expect("empty factory skips provenance in dev");
    }

    #[tokio::test]
    async fn verify_factory_provenance_fails_when_factory_has_no_pair() {
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, false).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("factory provenance check failed"),
            "unexpected error: {err}"
        );
    }
}
