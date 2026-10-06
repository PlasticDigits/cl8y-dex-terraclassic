use sqlx::PgPool;

use crate::db::queries::pairs::{self, PairRow};
use crate::lcd::pair_discovery_state::{BeginError, DiscoveryOutcome, RejectReason};
use crate::lcd::types::{
    AssetInfo, FactoryPairResponse, FeeConfigResponse, HooksResponse, PairInfo, PairsResponse,
};
use crate::lcd::{LcdClient, LcdError};
use bech32::FromBase32;

use super::asset_resolver;

type BoxError = Box<dyn std::error::Error + Send + Sync>;
const DISCOVERY_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum DiscoverPairError {
    #[error("unlisted/foreign pair {pair_address} (not listed in factory {factory_address})")]
    Unlisted {
        pair_address: String,
        factory_address: String,
    },
    #[error("non-pair/foreign contract {pair_address} (does not support pair query)")]
    NotAPair { pair_address: String },
    #[error("pair {pair_address} recently rejected ({reason:?}); skipping rediscovery")]
    RejectedCached {
        pair_address: String,
        reason: RejectReason,
    },
    #[error("invalid discovery {field} address")]
    InvalidAddress { field: &'static str },
    #[error("pair discovery deferred: {reason}")]
    Deferred { reason: &'static str },
}

pub fn is_rejected_pair_error(err: &(dyn std::error::Error + 'static)) -> bool {
    matches!(
        err.downcast_ref::<DiscoverPairError>(),
        Some(
            DiscoverPairError::Unlisted { .. }
                | DiscoverPairError::NotAPair { .. }
                | DiscoverPairError::RejectedCached { .. }
                | DiscoverPairError::InvalidAddress { field: "pair" }
        )
    )
}

/// Canonical Terra Classic contract addresses: reject before allocating cache keys or
/// interpolating an event-controlled path. SDK contracts may have 20- or 32-byte payloads.
fn validate_discovery_address(address: &str, field: &'static str) -> Result<(), BoxError> {
    let valid = address.len() <= 90
        && address.starts_with("terra1")
        && address.bytes().all(|b| !b.is_ascii_uppercase())
        && bech32::decode(address)
            .ok()
            .is_some_and(|(hrp, data, variant)| {
                hrp == "terra"
                    && variant == bech32::Variant::Bech32
                    && Vec::<u8>::from_base32(&data)
                        .ok()
                        .is_some_and(|v| matches!(v.len(), 20 | 32))
            });
    if valid {
        Ok(())
    } else {
        Err(DiscoverPairError::InvalidAddress { field }.into())
    }
}

fn lcd_err_is_pair_not_found(err: &LcdError) -> bool {
    match err {
        LcdError::ContractQueryRejected(msg) => msg.to_ascii_lowercase().contains("pair not found"),
        _ => false,
    }
}

/// True when `{"pair":{}}` is rejected because the contract is not a Terraport/CL8Y pair
/// (e.g. cw20_bonding, Astroport `pair_base` with different QueryMsg).
fn lcd_err_is_not_a_pair_contract(err: &LcdError) -> bool {
    match err {
        LcdError::ContractQueryRejected(msg) => {
            let lower = msg.to_ascii_lowercase();
            lower.contains("unknown variant") || lower.contains("error parsing into type")
        }
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
    let mut complete = true;

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

        let resp: PairsResponse = match lcd
            .query_contract_bounded(factory_addr, &query, DISCOVERY_RESPONSE_BYTES)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to query factory pairs: {}", e);
                complete = false;
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
                    complete = false;
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

    // Empty DB alone is never evidence of an empty factory. Only a complete successful
    // enumeration (including persistence of every pair) establishes inventory knowledge.
    if complete {
        crate::db::queries::state::set_state(
            pool,
            "factory_inventory_observed_at",
            &chrono::Utc::now().to_rfc3339(),
        )
        .await?;
    }
    tracing::info!(complete, "Pair discovery complete: {} pairs synced", total);
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
/// Invalid or missing factory addresses fail closed before any LCD request.
pub(crate) async fn verify_factory_provenance(
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
    asset_infos: &[AssetInfo; 2],
) -> Result<(), BoxError> {
    validate_discovery_address(factory_addr, "factory")?;
    validate_discovery_address(pair_contract_addr, "pair")?;

    let query = serde_json::json!({
        "pair": {
            "asset_infos": [
                asset_info_to_json(&asset_infos[0]),
                asset_info_to_json(&asset_infos[1]),
            ]
        }
    });

    let resp: FactoryPairResponse = match lcd
        .query_contract_bounded(factory_addr, &query, DISCOVERY_RESPONSE_BYTES)
        .await
    {
        Ok(r) => r,
        Err(e) if lcd_err_is_pair_not_found(&e) => {
            return Err(DiscoverPairError::Unlisted {
                pair_address: pair_contract_addr.to_string(),
                factory_address: factory_addr.to_string(),
            }
            .into());
        }
        Err(e) => return Err(e.into()),
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

/// Look up a pair or discover it with authoritative factory provenance.
/// Admission pressure waits without skipping an event. Invalid/foreign pairs and
/// transient discovery soft-failures preserve the existing event skip policy;
/// failures remain visible in warnings, fixed counters and the typed direct API.
/// Waiting futures belong to callers, not an unbounded state queue.
pub async fn get_or_discover_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
) -> Result<Option<PairRow>, BoxError> {
    loop {
        if let Some(p) = pairs::get_pair_by_address(pool, pair_contract_addr).await? {
            return Ok(Some(p));
        }
        match discover_new_pair(pool, lcd, factory_addr, pair_contract_addr).await {
            Ok(p) => return Ok(Some(p)),
            Err(e) if is_rejected_pair_error(&*e) => return Ok(None),
            Err(e)
                if matches!(
                    e.downcast_ref::<DiscoverPairError>(),
                    Some(DiscoverPairError::Deferred {
                        reason: "rate_limited" | "coalesced" | "in_flight"
                    })
                ) =>
            {
                tokio::time::sleep(std::time::Duration::from_millis(125)).await;
            }
            Err(e)
                if matches!(
                    e.downcast_ref::<DiscoverPairError>(),
                    Some(
                        DiscoverPairError::Deferred { .. }
                            | DiscoverPairError::InvalidAddress { field: "factory" }
                    )
                ) =>
            {
                return Err(e);
            }
            Err(e) => {
                // Preserve availability for compatible foreign events whose contract
                // response is ambiguous. Do not turn their soft-failure into a new
                // attacker-controlled block halt or poison permanent negative evidence.
                tracing::warn!(outcome = "discovery_soft_failure", error = ?e, "Could not discover pair");
                return Ok(None);
            }
        }
    }
}

pub async fn discover_new_pair(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    pair_contract_addr: &str,
) -> Result<PairRow, BoxError> {
    validate_discovery_address(factory_addr, "factory")?;
    validate_discovery_address(pair_contract_addr, "pair")?;
    let state = lcd.pair_discovery_state();
    let permit = match state.begin(factory_addr, pair_contract_addr) {
        Ok(permit) => permit,
        Err(BeginError::Cached(reason)) => {
            lcd.require_verified_chain().await?;
            tracing::debug!(outcome = "cached_rejection", ?reason, "Pair discovery");
            return Err(DiscoverPairError::RejectedCached {
                pair_address: pair_contract_addr.to_string(),
                reason,
            }
            .into());
        }
        Err(BeginError::Deferred(reason)) => {
            tracing::debug!(outcome = "deferred", reason, "Pair discovery");
            return Err(DiscoverPairError::Deferred { reason }.into());
        }
    };

    // Admission bounds identity probes as well as contract queries. Cached negative
    // evidence is separately re-attested before it is returned above.
    lcd.require_verified_chain().await?;
    tracing::debug!("Discovering new pair at {}", pair_contract_addr);

    let pair_info: PairInfo = match lcd
        .query_contract_bounded(
            pair_contract_addr,
            &serde_json::json!({"pair": {}}),
            DISCOVERY_RESPONSE_BYTES,
        )
        .await
    {
        Ok(info) => info,
        Err(e) if lcd_err_is_not_a_pair_contract(&e) => {
            state.insert_rejection(&permit, RejectReason::NotAPair);
            state.record_outcome(DiscoveryOutcome::NotAPair);
            tracing::debug!(outcome = "not_a_pair", "Pair discovery");
            return Err(DiscoverPairError::NotAPair {
                pair_address: pair_contract_addr.to_string(),
            }
            .into());
        }
        Err(e) => {
            state.record_outcome(DiscoveryOutcome::Transient);
            return Err(e.into());
        }
    };

    if pair_info.contract_addr != pair_contract_addr {
        state.record_outcome(DiscoveryOutcome::Transient);
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
            state.insert_rejection(&permit, RejectReason::Unlisted);
            state.record_outcome(DiscoveryOutcome::Unlisted);
            tracing::debug!(outcome = "unlisted", "Pair discovery");
            return Err(e);
        }
        Err(e) => {
            state.record_outcome(DiscoveryOutcome::Transient);
            return Err(e);
        }
    }

    let row = sync_single_pair(pool, lcd, &pair_info)
        .await
        .inspect_err(|_| {
            state.record_outcome(DiscoveryOutcome::Transient);
        })?;
    state.record_outcome(DiscoveryOutcome::Listed);
    tracing::debug!(outcome = "listed", "Pair discovery");
    Ok(row)
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const FACTORY: &str = "terra1qyqszqgpqyqszqgpqyqszqgpqyqszqgp5hm70u";
    const CANONICAL_PAIR: &str = "terra1qgpqyqszqgpqyqszqgpqyqszqgpqyqsz9namy2";
    const ATTACKER_PAIR: &str = "terra1qvpsxqcrqvpsxqcrqvpsxqcrqvpsxqcryru6wt";
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
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        mount_pair_and_factory_mocks(&server, true, true).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(is_rejected_pair_error(&*err), "unexpected error: {err}");
        assert!(
            err.to_string().contains("unlisted/foreign pair"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn verify_factory_provenance_accepts_factory_listed_pair() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        mount_pair_and_factory_mocks(&server, true, true).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let assets = sample_pair_info(CANONICAL_PAIR).asset_infos;

        verify_factory_provenance(&lcd, FACTORY, CANONICAL_PAIR, &assets)
            .await
            .expect("canonical pair should pass");
    }

    #[tokio::test]
    async fn verify_factory_provenance_fails_closed_when_factory_empty() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        mount_pair_and_factory_mocks(&server, true, false).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        verify_factory_provenance(&lcd, "", ATTACKER_PAIR, &assets)
            .await
            .expect_err("empty factory must not skip provenance");
    }

    #[tokio::test]
    async fn verify_factory_provenance_fails_when_factory_has_no_pair() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        mount_pair_and_factory_mocks(&server, true, false).await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let assets = sample_pair_info(ATTACKER_PAIR).asset_infos;

        let err = verify_factory_provenance(&lcd, FACTORY, ATTACKER_PAIR, &assets)
            .await
            .unwrap_err();
        assert!(is_rejected_pair_error(&*err), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn discover_new_pair_negative_caches_unlisted_pair() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
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
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");

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
        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
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

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30000,
            "columbus-5",
        );
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

    #[tokio::test]
    async fn discover_new_pair_negative_caches_non_pair_contract() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                h.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(500).set_body_json(json!({
                    "code": 2,
                    "message": "Error parsing into type pair_base::msg::QueryMsg: unknown variant `pair`, expected one of `pool`, `simulate_provide_liquidity`",
                    "details": []
                }))
            })
            .mount(&server)
            .await;

        let pool = sqlx::PgPool::connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .expect("lazy pool");
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");

        let err1 = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err1.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::NotAPair { .. })
            ),
            "unexpected: {err1}"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);

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
            hits.load(Ordering::SeqCst),
            1,
            "second discover must not re-query non-pair contract"
        );
    }

    #[tokio::test]
    async fn lcd_unknown_variant_pair_does_not_fan_out() {
        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code": 2,
                "message": "Error parsing into type cw20_bonding::msg::QueryMsg: unknown variant `pair`",
                "details": []
            })))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(500)
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30000,
            "columbus-5",
        );
        let err = lcd
            .query_contract::<serde_json::Value>(ATTACKER_PAIR, &json!({"pair": {}}))
            .await
            .unwrap_err();
        assert!(
            matches!(err, LcdError::ContractQueryRejected(_)),
            "unexpected: {err}"
        );
        assert_eq!(hits_b.load(Ordering::SeqCst), 0);
    }
    fn synthetic_address(seed: u8) -> String {
        use bech32::ToBase32;
        bech32::encode("terra", [seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
    }

    fn unused_pool() -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_millis(10))
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap()
    }

    #[test]
    fn discovery_classifies_only_authoritative_contract_rejections() {
        for text in [
            "pair not found",
            "unknown variant pair",
            "error parsing into type",
        ] {
            let transient = LcdError::AllEndpointsFailed(text.into());
            assert!(!lcd_err_is_pair_not_found(&transient));
            assert!(!lcd_err_is_not_a_pair_contract(&transient));
        }
        assert!(lcd_err_is_pair_not_found(&LcdError::ContractQueryRejected(
            "pair not found".into()
        )));
        assert!(!is_rejected_pair_error(&DiscoverPairError::Deferred {
            reason: "rate_limit"
        }));
    }

    #[test]
    fn discovery_address_checks_checksum_hrp_variant_and_payload_length() {
        use bech32::ToBase32;
        for length in [20, 32] {
            let addr = bech32::encode(
                "terra",
                vec![7; length].to_base32(),
                bech32::Variant::Bech32,
            )
            .unwrap();
            validate_discovery_address(&addr, "pair").unwrap();
        }
        for length in [0, 19, 21, 31, 33] {
            let addr = bech32::encode(
                "terra",
                vec![7; length].to_base32(),
                bech32::Variant::Bech32,
            )
            .unwrap();
            assert!(validate_discovery_address(&addr, "pair").is_err());
        }
        for (hrp, variant) in [
            ("cosmos", bech32::Variant::Bech32),
            ("terra", bech32::Variant::Bech32m),
        ] {
            let addr = bech32::encode(hrp, [7; 32].to_base32(), variant).unwrap();
            assert!(validate_discovery_address(&addr, "pair").is_err());
        }
        let mut invalid = synthetic_address(7);
        invalid.pop();
        invalid.push('!');
        assert!(validate_discovery_address(&invalid, "pair").is_err());
        let mixed = synthetic_address(7).replacen("terra", "tErra", 1);
        assert!(validate_discovery_address(&mixed, "pair").is_err());
    }

    #[tokio::test]
    async fn discover_new_pair_invalid_inputs_never_reach_lcd() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let lcd = LcdClient::new(vec![server.uri()], 100, 0, "columbus-5");
        let pool = unused_pool();
        for pair in [
            "",
            "terra1attackerpair",
            "terra1../../smart",
            "cosmos1qqqq",
            &"x".repeat(10000),
            &ATTACKER_PAIR.to_uppercase(),
        ] {
            let err = discover_new_pair(&pool, &lcd, FACTORY, pair)
                .await
                .unwrap_err();
            assert!(matches!(
                err.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::InvalidAddress { field: "pair" })
            ));
        }
        let err = discover_new_pair(&pool, &lcd, "", ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<DiscoverPairError>(),
            Some(DiscoverPairError::InvalidAddress { field: "factory" })
        ));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn discover_new_pair_transient_timeout_never_negative_caches() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &wiremock::Request| {
                if h.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(500)
                        .set_body_string("gateway unavailable; pair not found")
                        .set_delay(std::time::Duration::from_secs(2))
                } else {
                    ResponseTemplate::new(500)
                        .set_body_json(json!({"message": "unknown variant pair"}))
                }
            })
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 500, 0, "columbus-5");
        let pool = unused_pool();
        let first = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(!is_rejected_pair_error(&*first), "{first}");
        let second = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(
            matches!(
                second.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::NotAPair { .. })
            ),
            "{second}"
        );
        let third = discover_new_pair(&pool, &lcd.clone(), FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(matches!(
            third.downcast_ref::<DiscoverPairError>(),
            Some(DiscoverPairError::RejectedCached {
                reason: RejectReason::NotAPair,
                ..
            })
        ));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn discover_new_pair_oversized_response_remains_retryable() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let hits = Arc::new(AtomicUsize::new(0));
        let h = hits.clone();
        Mock::given(method("GET"))
            .respond_with(move |_: &wiremock::Request| {
                if h.fetch_add(1, Ordering::SeqCst) == 0 {
                    ResponseTemplate::new(200)
                        .set_body_string("x".repeat(DISCOVERY_RESPONSE_BYTES + 1))
                } else {
                    ResponseTemplate::new(500)
                        .set_body_json(json!({"message": "unknown variant pair"}))
                }
            })
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 0, "columbus-5");
        let pool = unused_pool();
        let err = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(!is_rejected_pair_error(&*err));
        assert!(err.to_string().contains("exceeds 65536 byte limit"));
        let retry = discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(matches!(
            retry.downcast_ref::<DiscoverPairError>(),
            Some(DiscoverPairError::NotAPair { .. })
        ));
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        let counters = lcd.pair_discovery_counters();
        assert_eq!(counters.transient_failures, 1);
        assert_eq!(counters.rejected_not_pair, 1);
    }

    #[tokio::test]
    async fn discover_new_pair_concurrent_duplicate_admission_is_bounded() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(json!({"message": "unknown variant pair"}))
                    .set_delay(std::time::Duration::from_millis(100)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 0, "columbus-5");
        let pool = unused_pool();
        let mut tasks = tokio::task::JoinSet::new();
        for _ in 0..20 {
            let lcd = lcd.clone();
            let pool = pool.clone();
            tasks.spawn(async move {
                discover_new_pair(&pool, &lcd, FACTORY, ATTACKER_PAIR)
                    .await
                    .unwrap_err()
            });
        }
        let mut deferred = 0;
        while let Some(result) = tasks.join_next().await {
            let err = result.unwrap();
            match err.downcast_ref::<DiscoverPairError>() {
                Some(DiscoverPairError::Deferred { .. }) => {
                    assert!(!is_rejected_pair_error(&*err));
                    deferred += 1;
                }
                Some(
                    DiscoverPairError::NotAPair { .. } | DiscoverPairError::RejectedCached { .. },
                ) => {}
                other => panic!("unexpected outcome: {other:?}"),
            }
        }
        assert!(deferred > 0);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.url.path().contains("/smart/"))
                .count(),
            1
        );
        assert_eq!(
            requests.len(),
            3,
            "one admitted pair query and two identity probes"
        );
    }

    #[tokio::test]
    async fn discover_new_pair_foreign_flood_limits_lcd_work_without_cross_factory_starvation() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_json(json!({"message": "unknown variant pair"})),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 0, "columbus-5");
        let pool = unused_pool();
        let started = std::time::Instant::now();
        let mut deferred = 0;
        for i in 0..200 {
            let err = discover_new_pair(&pool, &lcd, FACTORY, &synthetic_address(i))
                .await
                .unwrap_err();
            if matches!(
                err.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::Deferred { .. })
            ) {
                deferred += 1;
            }
        }
        let requests = server.received_requests().await.unwrap();
        let hits = requests
            .iter()
            .filter(|r| r.url.path().contains("/smart/"))
            .count();
        assert_eq!(
            requests.len(),
            hits * 3,
            "deferred discoveries perform no identity or contract HTTP work"
        );
        assert_eq!(hits + deferred, 200);
        assert!(hits <= 33 + (started.elapsed().as_secs_f64() * 8.0) as usize);
        // A different factory receives its own fresh budget even on the same LCD.
        let err = discover_new_pair(&pool, &lcd, &synthetic_address(250), ATTACKER_PAIR)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<DiscoverPairError>(),
                Some(DiscoverPairError::NotAPair { .. })
            ),
            "{err}"
        );
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.url.path().contains("/smart/"))
                .count(),
            hits + 1
        );
        assert!(
            requests.len() <= (hits + 1) * 3 + 1,
            "cached rejection adds at most one identity probe"
        );
        // Cached rejects remain cheap after admission exhaustion.
        let err = discover_new_pair(&pool, &lcd, FACTORY, &synthetic_address(0))
            .await
            .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<DiscoverPairError>(),
            Some(DiscoverPairError::RejectedCached { .. })
        ));
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.url.path().contains("/smart/"))
                .count(),
            hits + 1
        );
        assert!(
            requests.len() <= (hits + 1) * 3 + 1,
            "cached rejection adds at most one identity probe"
        );
    }
}
