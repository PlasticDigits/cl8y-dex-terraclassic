use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use sqlx::PgPool;

use crate::db::queries::pairs::{self, PairRow};
use crate::lcd::types::{
    AssetInfo, FactoryPairResponse, FeeConfigResponse, HooksResponse, PairInfo, PairsResponse,
};
use crate::lcd::{LcdClient, LcdError};

use super::asset_resolver;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// TTL for in-memory negative cache of factory-rejected (foreign/unlisted) pair addresses.
/// Terraport and other DEXes emit compatible `action=swap` events; re-querying LCD on every
/// such swap is wasteful. A rejected address cannot become factory-listed without a new
/// CreatePair (different contract addr).
const PAIR_REJECT_CACHE_TTL: Duration = Duration::from_secs(3600);
const PAIR_REJECT_CACHE_MAX: usize = 10_000;

#[derive(Debug, thiserror::Error)]
pub enum DiscoverPairError {
    #[error(
        "unlisted/foreign pair {pair_address} (not listed in factory {factory_address})"
    )]
    Unlisted {
        pair_address: String,
        factory_address: String,
    },
    #[error(
        "unlisted/foreign pair {pair_address} (recently rejected; skipping rediscovery)"
    )]
    RejectedCached { pair_address: String },
}

pub fn is_rejected_pair_error(err: &(dyn std::error::Error + 'static)) -> bool {
    err.downcast_ref::<DiscoverPairError>().is_some()
}

fn reject_cache() -> &'static Mutex<HashMap<String, Instant>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn reject_cache_get(pair_addr: &str) -> bool {
    let Ok(mut g) = reject_cache().lock() else {
        return false;
    };
    let now = Instant::now();
    g.retain(|_, at| now.duration_since(*at) <= PAIR_REJECT_CACHE_TTL);
    g.contains_key(pair_addr)
}

fn reject_cache_insert(pair_addr: &str) {
    let Ok(mut g) = reject_cache().lock() else {
        return;
    };
    let now = Instant::now();
    g.retain(|_, at| now.duration_since(*at) <= PAIR_REJECT_CACHE_TTL);
    if g.len() >= PAIR_REJECT_CACHE_MAX && !g.contains_key(pair_addr) {
        // Evict oldest entry when at capacity.
        if let Some(oldest) = g
            .iter()
            .min_by_key(|(_, at)| *at)
            .map(|(k, _)| k.clone())
        {
            g.remove(&oldest);
        }
    }
    g.insert(pair_addr.to_string(), now);
}

#[cfg(test)]
fn reject_cache_clear() {
    if let Ok(mut g) = reject_cache().lock() {
        g.clear();
    }
}

fn lcd_err_is_pair_not_found(err: &LcdError) -> bool {
    match err {
        LcdError::ContractQueryRejected(msg) | LcdError::AllEndpointsFailed(msg) => msg
            .to_ascii_lowercase()
            .contains("pair not found"),
        _ => false,
    }
}

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
/// When `factory_addr` is empty, provenance is skipped with a warning (defensive only —
/// config load rejects empty `FACTORY_ADDRESS` in every RUN_MODE; GitLab #451).
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

    let resp: FactoryPairResponse = match lcd.query_contract(factory_addr, &query).await {
        Ok(r) => r,
        Err(e) if lcd_err_is_pair_not_found(&e) => {
            return Err(DiscoverPairError::Unlisted {
                pair_address: pair_contract_addr.to_string(),
                factory_address: factory_addr.to_string(),
            }
            .into());
        }
        Err(e) => {
            return Err(format!(
                "factory provenance check failed for {} (factory {}): {}",
                pair_contract_addr, factory_addr, e
            )
            .into());
        }
    };

    if resp.pair.contract_addr != pair_contract_addr {
        return Err(DiscoverPairError::Unlisted {
            pair_address: pair_contract_addr.to_string(),
            factory_address: factory_addr.to_string(),
        }
        .into());
    }

    Ok(())
}

/// Look up a pair in the DB, or opportunistically discover it via factory provenance.
/// Returns `Ok(None)` when the address is foreign/unlisted or discovery soft-fails (caller
/// should skip the event without failing the block).
pub async fn get_or_discover_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
) -> Result<Option<PairRow>, BoxError> {
    if let Some(p) = pairs::get_pair_by_address(pool, pair_contract_addr).await? {
        return Ok(Some(p));
    }
    match discover_new_pair(pool, lcd, factory_addr, pair_contract_addr).await {
        Ok(p) => Ok(Some(p)),
        Err(e) if is_rejected_pair_error(&*e) => Ok(None),
        Err(e) => {
            tracing::warn!("Could not discover pair {}: {}", pair_contract_addr, e);
            Ok(None)
        }
    }
}

pub async fn discover_new_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
) -> Result<PairRow, BoxError> {
    if reject_cache_get(pair_contract_addr) {
        tracing::debug!(
            pair = %pair_contract_addr,
            "Skipping rediscovery of recently rejected unlisted/foreign pair"
        );
        return Err(DiscoverPairError::RejectedCached {
            pair_address: pair_contract_addr.to_string(),
        }
        .into());
    }

    tracing::debug!("Discovering new pair at {}", pair_contract_addr);

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

    match verify_factory_provenance(
        lcd,
        factory_addr,
        pair_contract_addr,
        &pair_info.asset_infos,
    )
    .await
    {
        Ok(()) => {}
        Err(e) if is_rejected_pair_error(&*e) => {
            reject_cache_insert(pair_contract_addr);
            tracing::info!(
                pair = %pair_contract_addr,
                factory = %factory_addr,
                "Skipping unlisted/foreign pair (not in factory)"
            );
            return Err(e);
        }
        Err(e) => return Err(e),
    }

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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
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
                                "code": 2,
                                "message": "Generic error: pair not found: query wasm contract failed",
                                "details": []
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
        reject_cache_clear();
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, true).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(
            is_rejected_pair_error(&*err),
            "unexpected error: {err}"
        );
        assert!(
            err.to_string().contains("unlisted/foreign pair"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn verify_factory_provenance_accepts_factory_listed_pair() {
        reject_cache_clear();
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
        reject_cache_clear();
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
        reject_cache_clear();
        let server = MockServer::start().await;
        mount_pair_and_factory_mocks(&server, true, false).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(
            is_rejected_pair_error(&*err),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn discover_new_pair_negative_caches_unlisted_pair() {
        reject_cache_clear();
        let server = MockServer::start().await;
        let factory_hits = Arc::new(AtomicUsize::new(0));
        let hits = factory_hits.clone();

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

                if contract == ATTACKER_PAIR {
                    return ResponseTemplate::new(200)
                        .set_body_json(json!({ "data": pair_info_json(ATTACKER_PAIR) }));
                }
                if contract == FACTORY && query.get("pair").is_some() {
                    hits.fetch_add(1, Ordering::SeqCst);
                    return ResponseTemplate::new(500).set_body_json(json!({
                        "code": 2,
                        "message": "Generic error: pair not found: query wasm contract failed",
                        "details": []
                    }));
                }
                ResponseTemplate::new(404)
            })
            .mount(&server)
            .await;

        // Pool is unused on the reject path (fails before sync_single_pair).
        let pool = sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .expect("lazy pool");
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);

        let err1 = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(is_rejected_pair_error(&*err1), "unexpected: {err1}");
        assert_eq!(factory_hits.load(Ordering::SeqCst), 1);

        let err2 = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err2.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::RejectedCached { .. })
            ),
            "unexpected: {err2}"
        );
        assert_eq!(
            factory_hits.load(Ordering::SeqCst),
            1,
            "second discover must not re-hit factory LCD"
        );
    }

    #[tokio::test]
    async fn lcd_pair_not_found_does_not_fan_out_to_other_endpoints() {
        reject_cache_clear();
        let server_a = MockServer::start().await;
        let server_b = MockServer::start().await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code": 2,
                "message": "Generic error: pair not found: query wasm contract failed",
                "details": []
            })))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(500).set_body_json(json!({
                    "code": 2,
                    "message": "Generic error: pair not found: query wasm contract failed",
                    "details": []
                }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(vec![server_a.uri(), server_b.uri()], 5000, 30000);
        let err = lcd
            .query_contract::<serde_json::Value>(FACTORY, &json!({"pair": {"asset_infos": []}}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, LcdError::ContractQueryRejected(_)),
            "unexpected: {err}"
        );
        assert_eq!(
            hits_b.load(Ordering::SeqCst),
            0,
            "deterministic pair-not-found must not try later LCD endpoints"
        );
    }
}
