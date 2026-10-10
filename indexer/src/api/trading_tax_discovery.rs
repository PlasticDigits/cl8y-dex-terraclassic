//! Trading capability discovery is independent of the single-code Create/Manage catalog.
//! Immutable code registrations attest implementation; live instance responses attest version,
//! configured router and current rates. Live code identity is checked before every cache hit.
//! Unknown implementations never become ordinary by a
//! rejected `get_config`. Positive legacy taxes remain unsupported until migration.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;
use tokio::sync::Semaphore;

use super::community_tax_rank::{CataloguedTaxToken, RouterHopTaxPolicy};
use super::AppState;

pub const DISCOVERY_TTL: Duration = Duration::from_secs(5);
#[cfg(test)]
pub const MAX_DISCOVERY_ASSETS: usize = 64;
const MAX_CACHE_ENTRIES: usize = 512;
const QUERY_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Debug)]
pub enum TradingTax {
    Ordinary { code_id: Option<i64> },
    Community(CataloguedTaxToken),
    Unsupported,
    Unavailable,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CacheKey {
    namespace: usize,
    policy: String,
    router: Option<String>,
    address: String,
    code_id: i64,
    data_hash: Option<String>,
}

type Cache = HashMap<CacheKey, (Instant, TradingTax)>;

fn invalidate_instance(namespace: usize, address: &str) {
    if let Ok(mut cache) = cache().lock() {
        cache.retain(|key, _| key.namespace != namespace || key.address != address);
    }
}
fn cache() -> &'static Mutex<Cache> {
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}
pub(crate) fn discovery_semaphore() -> &'static Semaphore {
    static LIMIT: OnceLock<Semaphore> = OnceLock::new();
    LIMIT.get_or_init(|| Semaphore::new(4))
}

#[derive(Deserialize)]
struct Capability {
    funding_version: u16,
    router: Option<String>,
    buy_bps: u16,
    sell_bps: u16,
}
#[derive(Deserialize)]
struct TaxConfig {
    buy_bps: u16,
    sell_bps: u16,
}

fn registered(code_id: i64, hash: Option<&str>, policy: &RouterHopTaxPolicy) -> bool {
    policy.route_tax_code_ids.contains(&code_id)
        || hash.is_some_and(|h| policy.route_tax_data_hashes.contains(h))
}
fn needs_data_hash(code_id: i64, policy: &RouterHopTaxPolicy) -> bool {
    !policy.tax_free_code_ids.contains(&code_id)
        && !policy.route_tax_code_ids.contains(&code_id)
        && !policy.route_tax_data_hashes.is_empty()
}
fn compatible(cap: &Capability, router: Option<&str>) -> bool {
    cap.funding_version == 1
        && cap.router.as_deref() == router
        && router.is_some()
        && cap.buy_bps <= 10_000
        && cap.sell_bps <= 10_000
}

pub async fn discover(state: &AppState, addr: &str, policy: &RouterHopTaxPolicy) -> TradingTax {
    discover_with_lcd(&state.lcd, state.router_address.as_deref(), addr, policy).await
}

async fn discover_with_lcd(
    lcd: &crate::lcd::LcdClient,
    router: Option<&str>,
    addr: &str,
    policy: &RouterHopTaxPolicy,
) -> TradingTax {
    discover_detailed(lcd, router, addr, policy, None)
        .await
        .unwrap_or(TradingTax::Unavailable)
}

/// Scheduled admission needs typed failures to retain observations only for
/// provider availability, never wrong-chain, malformed or changed identities.
pub(crate) async fn discover_scheduled(
    state: &AppState,
    addr: &str,
    policy: &RouterHopTaxPolicy,
    previous_code: Option<u64>,
) -> Result<TradingTax, crate::lcd::LcdError> {
    discover_detailed(
        &state.lcd,
        state.router_address.as_deref(),
        addr,
        policy,
        previous_code,
    )
    .await
}

async fn discover_detailed(
    lcd: &crate::lcd::LcdClient,
    router: Option<&str>,
    addr: &str,
    policy: &RouterHopTaxPolicy,
    previous_code: Option<u64>,
) -> Result<TradingTax, crate::lcd::LcdError> {
    discover_with_budget(lcd, router, addr, policy, previous_code, QUERY_TIMEOUT).await
}
async fn discover_with_budget(
    lcd: &crate::lcd::LcdClient,
    router: Option<&str>,
    addr: &str,
    policy: &RouterHopTaxPolicy,
    previous_code: Option<u64>,
    budget: Duration,
) -> Result<TradingTax, crate::lcd::LcdError> {
    use crate::lcd::LcdError;
    if !addr.starts_with("terra1") {
        return Ok(TradingTax::Ordinary { code_id: None });
    }
    let result = tokio::time::timeout(budget, async {
        lcd.require_verified_chain().await?;
        let _permit = discovery_semaphore()
            .acquire()
            .await
            .map_err(|_| LcdError::AdmissionDeferred)?;
        let code = lcd.get_contract_code_id(addr).await?;
        // Return immediately: an unavailable subsequent capability/pin read must
        // never hide a successfully observed migration behind old qualification.
        if previous_code.is_some_and(|previous| previous != code) {
            return Err(LcdError::PairResponseMalformed(
                "observed token code identity changed".into(),
            ));
        }
        let code_id = i64::try_from(code)
            .map_err(|_| LcdError::PairResponseMalformed("invalid token code identity".into()))?;
        // A positively registered immutable code identity already attests this
        // implementation. Hash fallback is necessary only for otherwise
        // unregistered codes; failed redundant hash reads cannot erase coverage.
        let hash = if needs_data_hash(code_id, policy) {
            Some(
                lcd.get_code_data_hash(code)
                    .await?
                    .trim()
                    .trim_start_matches("0x")
                    .to_ascii_lowercase(),
            )
        } else {
            None
        };
        let key = CacheKey {
            namespace: lcd.cache_namespace(),
            policy: format!("{policy:?}"),
            router: router.map(str::to_owned),
            address: addr.into(),
            code_id,
            data_hash: hash.clone(),
        };
        if let Ok(mut cache) = cache().lock() {
            cache.retain(|candidate, (at, _)| {
                at.elapsed() < DISCOVERY_TTL
                    && (candidate.namespace != key.namespace
                        || candidate.address != key.address
                        || (candidate.code_id == code_id && candidate.data_hash == hash))
            });
            if let Some((_, value)) = cache.get(&key) {
                return Ok(value.clone());
            }
        }
        let value = discover_identity(lcd, router, addr, code_id, hash, policy).await?;
        if matches!(value, TradingTax::Unsupported) {
            // Already observed negative evidence must survive any later outage.
            return Ok(value);
        }
        if lcd.get_contract_code_id(addr).await? != code {
            return Err(LcdError::PairResponseMalformed(
                "token code identity changed during observation".into(),
            ));
        }
        if let Ok(mut cache) = cache().lock() {
            cache.retain(|_, (at, _)| at.elapsed() < DISCOVERY_TTL);
            if cache.len() >= MAX_CACHE_ENTRIES {
                if let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, (at, _))| *at)
                    .map(|(key, _)| key.clone())
                {
                    cache.remove(&oldest);
                }
            }
            cache.insert(key, (Instant::now(), value.clone()));
        }
        Ok(value)
    })
    .await
    .unwrap_or_else(|_| {
        // Cancellation may hide integrity evidence already observed inside a
        // still-running provider failover. Only completed typed availability
        // errors can preserve qualification; an incomplete census fails closed.
        Err(LcdError::PairResponseMalformed(
            "incomplete token observation".into(),
        ))
    });
    if result.is_err() {
        invalidate_instance(lcd.cache_namespace(), addr);
    }
    result
}

async fn discover_identity(
    lcd: &crate::lcd::LcdClient,
    router: Option<&str>,
    addr: &str,
    code_id: i64,
    hash: Option<String>,
    policy: &RouterHopTaxPolicy,
) -> Result<TradingTax, crate::lcd::LcdError> {
    if policy.tax_free_code_ids.contains(&code_id) && policy.route_tax_code_ids.contains(&code_id) {
        return Ok(TradingTax::Unsupported); // Conflicting registrations never waive tax.
    }
    if policy.tax_free_code_ids.contains(&code_id) {
        return Ok(TradingTax::Ordinary {
            code_id: Some(code_id),
        });
    }
    let trusted = registered(code_id, hash.as_deref(), policy);
    if trusted {
        let cap: Capability = lcd
            .query_contract(addr, &json!({"route_tax_capability":{}}))
            .await?;
        if !compatible(&cap, router) {
            return Ok(TradingTax::Unsupported);
        }
        return Ok(TradingTax::Community(CataloguedTaxToken {
            contract_address: addr.into(),
            code_id: Some(code_id),
            data_hash: hash,
            buy_bps: cap.buy_bps,
            sell_bps: cap.sell_bps,
            exemption_directory: true,
            route_funding_version: Some(1),
        }));
    }
    // Observe live legacy community instances even when absent from catalog (ALPHA 11666).
    // Observation does not attest execution capability or authorize ordinary classification.
    match lcd
        .query_contract::<TaxConfig>(addr, &json!({"get_config":{}}))
        .await
    {
        Ok(config) if config.buy_bps <= 10_000 && config.sell_bps <= 10_000 => {
            Ok(TradingTax::Community(CataloguedTaxToken {
                contract_address: addr.into(),
                code_id: Some(code_id),
                data_hash: hash,
                buy_bps: config.buy_bps,
                sell_bps: config.sell_bps,
                exemption_directory: true,
                route_funding_version: None,
            }))
        }
        Ok(_) => Ok(TradingTax::Unsupported),
        Err(crate::lcd::LcdError::ContractQueryRejected(_)) => {
            // Actual legacy fee APIs must not be waived merely because get_config rejects.
            // Until a separate audited implementation is registered, both known legacy fee
            // APIs and unrecognized implementations remain explicitly unsupported.
            for query in [json!({"tax_map":{}}), json!({"tax_info":{}})] {
                if lcd
                    .query_contract::<serde_json::Value>(addr, &query)
                    .await
                    .is_ok()
                {
                    return Ok(TradingTax::Unsupported);
                }
            }
            Ok(TradingTax::Unsupported)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
pub async fn router_funding_supported(state: &AppState) -> bool {
    let Some(router) = state.router_address.as_deref() else {
        return false;
    };
    // The router instance is deployment configuration, rather than user-provided metadata.
    tokio::time::timeout(
        QUERY_TIMEOUT,
        state
            .lcd
            .query_contract::<serde_json::Value>(router, &json!({"config":{}})),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .and_then(|v| v.get("funding_version").and_then(|v| v.as_u64()))
        == Some(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_capability_requires_registered_immutable_implementation() {
        let mut policy = RouterHopTaxPolicy::default();
        assert!(
            !registered(11666, None, &policy),
            "arbitrary get_config/capability is not attestation"
        );
        policy.route_tax_code_ids.insert(11667);
        assert!(registered(11667, None, &policy));
        assert!(!registered(11666, None, &policy));
        policy.route_tax_data_hashes.insert("abcdef".into());
        assert!(registered(11668, Some("abcdef"), &policy));
        assert!(!registered(11668, Some("badcode"), &policy));
    }

    #[test]
    fn router_version_and_bounded_rates_must_match_actual_instance() {
        let mut cap = Capability {
            funding_version: 1,
            router: Some("terra1router".into()),
            buy_bps: 450,
            sell_bps: 100,
        };
        assert!(compatible(&cap, Some("terra1router")));
        assert!(!compatible(&cap, Some("terra1foreign")));
        assert!(!compatible(&cap, None));
        cap.funding_version = 2;
        assert!(!compatible(&cap, Some("terra1router")));
        cap.funding_version = 1;
        cap.sell_bps = 10_001;
        assert!(!compatible(&cap, Some("terra1router")));
    }

    #[test]
    fn discovery_cache_cannot_cross_lcd_client_or_chain_lifetimes() {
        let first = crate::lcd::LcdClient::new(
            vec!["http://chain-a.invalid".into()],
            100,
            100,
            "columbus-5",
        );
        let clone = first.clone();
        let second = crate::lcd::LcdClient::new(
            vec!["http://chain-b.invalid".into()],
            100,
            100,
            "columbus-5",
        );
        assert_eq!(first.chain_cache_namespace(), clone.chain_cache_namespace());
        assert_ne!(
            first.chain_cache_namespace(),
            second.chain_cache_namespace()
        );
    }

    const TOKEN: &str = "terra1synthetictoken";
    const ROUTER: &str = "terra1syntheticrouter";

    #[derive(Default)]
    struct SyntheticInstance {
        code: std::sync::atomic::AtomicU64,
        capability_queries: std::sync::atomic::AtomicUsize,
        fail_identity: std::sync::atomic::AtomicBool,
        legacy_config: std::sync::atomic::AtomicBool,
        migrate_during_capability: std::sync::atomic::AtomicBool,
        hash: Mutex<String>,
        router: Mutex<String>,
    }

    async fn synthetic_lcd() -> (
        wiremock::MockServer,
        std::sync::Arc<SyntheticInstance>,
        crate::lcd::LcdClient,
    ) {
        use std::sync::atomic::Ordering::SeqCst;
        use wiremock::matchers::{method, path, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let instance = std::sync::Arc::new(SyntheticInstance::default());
        instance.code.store(202, SeqCst);
        *instance.hash.lock().unwrap() = "abcdef".into();
        *instance.router.lock().unwrap() = ROUTER.into();
        let live = instance.clone();
        Mock::given(method("GET"))
            .and(path(format!("/cosmwasm/wasm/v1/contract/{TOKEN}")))
            .respond_with(move |_: &wiremock::Request| {
                if live.fail_identity.load(SeqCst) {
                    return ResponseTemplate::new(200).set_body_json(json!({}));
                }
                ResponseTemplate::new(200).set_body_json(json!({
                    "contract_info": {"code_id": live.code.load(SeqCst).to_string()}
                }))
            })
            .mount(&server)
            .await;
        let live = instance.clone();
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/code/[0-9]+"))
            .respond_with(move |_: &wiremock::Request| {
                ResponseTemplate::new(200).set_body_json(json!({
                    "code_info": {"data_hash": *live.hash.lock().unwrap()}
                }))
            })
            .mount(&server)
            .await;
        let live = instance.clone();
        Mock::given(method("GET"))
            .and(path_regex(r".*/smart/.*"))
            .respond_with(move |request: &wiremock::Request| {
                use base64::Engine;
                let encoded = request.url.path_segments().unwrap().next_back().unwrap();
                let query: serde_json::Value = serde_json::from_slice(
                    &base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .unwrap(),
                )
                .unwrap();
                if query.get("route_tax_capability").is_some() {
                    live.capability_queries.fetch_add(1, SeqCst);
                    if live.migrate_during_capability.load(SeqCst) {
                        live.code.store(303, SeqCst);
                    }
                    return ResponseTemplate::new(200).set_body_json(json!({
                        "data": {"funding_version": 1, "router": *live.router.lock().unwrap(),
                            "buy_bps": 450, "sell_bps": 100}
                    }));
                }
                // The unregistered synthetic implementation exposes malformed tax config;
                // even a compatible capability would not attest this implementation.
                ResponseTemplate::new(200).set_body_json(json!({
                    "data": {"buy_bps": if live.legacy_config.load(SeqCst) { 450 } else { 10001 },
                        "sell_bps": 100}
                }))
            })
            .mount(&server)
            .await;
        let lcd = crate::lcd::LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
        (server, instance, lcd)
    }

    fn synthetic_policy() -> RouterHopTaxPolicy {
        let mut policy = RouterHopTaxPolicy::default();
        policy.tax_free_code_ids.insert(101);
        policy.route_tax_code_ids.insert(202);
        policy
    }

    #[tokio::test]
    async fn recovery1464_registered_ordinary_avoids_redundant_hash_under_circuit_pressure() {
        use std::sync::atomic::Ordering::SeqCst;
        let (healthy, instance, _) = synthetic_lcd().await;
        instance.code.store(101, SeqCst);
        // Production code envelopes contain inline WASM and exceed the scheduled
        // 64KiB response limit. Known positive identity must not download them.
        wiremock::Mock::given(wiremock::matchers::path("/cosmwasm/wasm/v1/code/101"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({
                "code_info":{"code_id":"101","data_hash":"abcdef"},
                "data":"A".repeat(70*1024)
            })))
            .with_priority(1)
            .mount(&healthy)
            .await;
        let first = wiremock::MockServer::start().await;
        let second = wiremock::MockServer::start().await;
        let lcd = crate::lcd::LcdClient::new_with_policy(
            vec![first.uri(), second.uri(), healthy.uri()],
            1000,
            60_000,
            "columbus-5",
            crate::lcd::ProviderPolicy {
                deadline_ms: 8000,
                attempt_timeout_ms: 2500,
                max_attempts: 3,
                ..Default::default()
            },
        );
        // Open failed endpoints through normal shared provider outcomes.
        for server in [&first, &second] {
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .respond_with(wiremock::ResponseTemplate::new(503))
                .mount(server)
                .await;
        }
        wiremock::Mock::given(wiremock::matchers::path("/force_outage"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({"ok":true})))
            .mount(&healthy)
            .await;
        lcd.get::<serde_json::Value>("/force_outage").await.unwrap();
        let mut policy = synthetic_policy();
        policy.route_tax_data_hashes.insert("abcdef".into());
        assert!(!needs_data_hash(101, &policy));
        assert!(!needs_data_hash(202, &policy));
        assert!(needs_data_hash(303, &policy));
        let background = lcd.with_ingestion_budget();
        for _ in 0..2 {
            assert!(matches!(
                discover_detailed(&background, Some(ROUTER), TOKEN, &policy, Some(101)).await,
                Ok(TradingTax::Ordinary { code_id: Some(101) })
            ));
        }
        let requests = healthy.received_requests().await.unwrap();
        assert!(
            !requests
                .iter()
                .any(|request| request.url.path().starts_with("/cosmwasm/wasm/v1/code/")),
            "immutable positive code registration does not depend on redundant hash RPC"
        );
        assert_eq!(instance.capability_queries.load(SeqCst), 0);
    }
    #[tokio::test]
    async fn recovery1464_scheduled_discovery_retains_only_typed_provider_outages() {
        use crate::lcd::LcdError;
        use std::sync::atomic::Ordering::SeqCst;
        let (server, instance, lcd) = synthetic_lcd().await;
        let policy = synthetic_policy();
        instance.code.store(101, SeqCst);
        assert!(matches!(
            discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, None).await,
            Ok(TradingTax::Ordinary { code_id: Some(101) })
        ));
        instance.fail_identity.store(true, SeqCst);
        let malformed = discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, Some(101))
            .await
            .unwrap_err();
        assert!(!crate::indexer::block_indexer::is_retryable_availability(
            &malformed
        ));
        instance.fail_identity.store(false, SeqCst);
        instance.code.store(202, SeqCst);
        assert!(matches!(
            discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, Some(101)).await,
            Err(LcdError::PairResponseMalformed(_))
        ));
        assert_eq!(
            instance.capability_queries.load(SeqCst),
            0,
            "observed code change invalidates before capability I/O"
        );
        instance.migrate_during_capability.store(true, SeqCst);
        assert!(matches!(
            discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, Some(202)).await,
            Err(LcdError::PairResponseMalformed(_))
        ));
        server.reset().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let failure = discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, Some(202))
            .await
            .unwrap_err();
        assert!(crate::indexer::block_indexer::is_retryable_availability(
            &failure
        ));
    }

    #[tokio::test]
    async fn recovery1464_outer_discovery_deadline_cannot_hide_inner_integrity_failure() {
        use std::sync::atomic::Ordering::SeqCst;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let (first, instance, _) = synthetic_lcd().await;
        instance.code.store(101, SeqCst);
        let second = MockServer::start().await;
        crate::lcd::mock_chain_identity(&second, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path(format!("/cosmwasm/wasm/v1/contract/{TOKEN}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "contract_info":{"code_id":"101"}
            })))
            .mount(&second)
            .await;
        let lcd = crate::lcd::LcdClient::new_with_policy(
            vec![first.uri(), second.uri()],
            5_000,
            1,
            "columbus-5",
            crate::lcd::ProviderPolicy {
                attempt_timeout_ms: 5_000,
                deadline_ms: 5_000,
                ..Default::default()
            },
        );
        let policy = synthetic_policy();
        assert!(matches!(
            discover_detailed(&lcd, Some(ROUTER), TOKEN, &policy, Some(101)).await,
            Ok(TradingTax::Ordinary { code_id: Some(101) })
        ));
        assert!(cache()
            .lock()
            .unwrap()
            .keys()
            .any(|key| key.namespace == lcd.cache_namespace() && key.address == TOKEN));
        first.reset().await;
        crate::lcd::mock_chain_identity(&first, "wrong-chain").await;
        second.reset().await;
        Mock::given(method("GET"))
            .and(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(Duration::from_secs(4)),
            )
            .mount(&second)
            .await;
        let error = discover_with_budget(
            &lcd,
            Some(ROUTER),
            TOKEN,
            &policy,
            Some(101),
            Duration::from_secs(3),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            crate::lcd::LcdError::PairResponseMalformed(_)
        ));
        assert!(!crate::indexer::block_indexer::is_retryable_availability(
            &error
        ));
        assert!(!cache()
            .lock()
            .unwrap()
            .keys()
            .any(|key| key.namespace == lcd.cache_namespace() && key.address == TOKEN));
    }

    #[tokio::test]
    async fn code_migration_invalidates_cached_ordinary_funded_and_unsupported_verdicts() {
        use std::sync::atomic::Ordering::SeqCst;
        let (_server, instance, lcd) = synthetic_lcd().await;
        let policy = synthetic_policy();
        instance.code.store(101, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Ordinary { code_id: Some(101) }
        ));
        instance.code.store(202, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(CataloguedTaxToken {
                code_id: Some(202),
                route_funding_version: Some(1),
                ..
            })
        ));
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(_)
        ));
        assert_eq!(
            instance.capability_queries.load(SeqCst),
            1,
            "unchanged live code identity retains the bounded capability cache"
        );
        instance.code.store(303, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unsupported
        ));
        assert_eq!(
            instance.capability_queries.load(SeqCst),
            1,
            "unregistered implementation cannot self-attest using capability"
        );
        instance.code.store(202, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(_)
        ));
        assert_eq!(
            instance.capability_queries.load(SeqCst),
            2,
            "migration removes previous instance verdicts before a return migration"
        );
    }

    #[tokio::test]
    async fn legacy_observation_becomes_funded_immediately_after_in_place_migration() {
        use std::sync::atomic::Ordering::SeqCst;
        let (_server, instance, lcd) = synthetic_lcd().await;
        let policy = synthetic_policy();
        instance.code.store(404, SeqCst);
        instance.legacy_config.store(true, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(CataloguedTaxToken {
                code_id: Some(404),
                route_funding_version: None,
                ..
            })
        ));
        assert_eq!(
            instance.capability_queries.load(SeqCst),
            0,
            "legacy tax observation cannot self-attest a funded implementation"
        );
        instance.code.store(202, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(CataloguedTaxToken {
                code_id: Some(202),
                route_funding_version: Some(1),
                ..
            })
        ));
        assert_eq!(instance.capability_queries.load(SeqCst), 1);
    }

    #[tokio::test]
    async fn unavailable_live_identity_cannot_reuse_cached_capability() {
        use std::sync::atomic::Ordering::SeqCst;
        let (_server, instance, lcd) = synthetic_lcd().await;
        let policy = synthetic_policy();
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(_)
        ));
        instance.fail_identity.store(true, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unavailable
        ));
        instance.fail_identity.store(false, SeqCst);
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(_)
        ));
        assert_eq!(instance.capability_queries.load(SeqCst), 2);
    }

    #[tokio::test]
    async fn hash_and_expected_router_are_part_of_live_discovery_identity() {
        use std::sync::atomic::Ordering::SeqCst;
        let (_server, instance, lcd) = synthetic_lcd().await;
        let mut policy = RouterHopTaxPolicy::default();
        policy.route_tax_data_hashes.insert("abcdef".into());
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Community(_)
        ));
        *instance.hash.lock().unwrap() = "different-code-same-cw2-version".into();
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unsupported
        ));
        *instance.hash.lock().unwrap() = "abcdef".into();
        assert!(matches!(
            discover_with_lcd(&lcd, Some("terra1replacementrouter"), TOKEN, &policy).await,
            TradingTax::Unsupported
        ));
        *instance.router.lock().unwrap() = "terra1replacementrouter".into();
        // A different configured router has a distinct cache identity. The same-code
        // live capability itself retains its documented TTL after router rotation.
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unsupported
        ));
        assert_eq!(instance.capability_queries.load(SeqCst), 3);
    }

    #[tokio::test]
    async fn migration_during_capability_query_is_not_attested_or_cached() {
        use std::sync::atomic::Ordering::SeqCst;
        let (_server, instance, lcd) = synthetic_lcd().await;
        instance.migrate_during_capability.store(true, SeqCst);
        let policy = synthetic_policy();
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unavailable
        ));
        assert!(matches!(
            discover_with_lcd(&lcd, Some(ROUTER), TOKEN, &policy).await,
            TradingTax::Unsupported
        ));
        assert_eq!(instance.capability_queries.load(SeqCst), 1);
    }
}
