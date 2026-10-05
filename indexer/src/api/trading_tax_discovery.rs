//! Trading capability discovery is independent of the single-code Create/Manage catalog.
//! Immutable code registrations attest implementation; live instance responses attest version,
//! configured router and current rates. Unknown implementations never become ordinary by a
//! rejected `get_config`. Positive legacy taxes remain unsupported until migration.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;
use tokio::sync::Semaphore;

use super::AppState;
use super::community_tax_rank::{CataloguedTaxToken, RouterHopTaxPolicy};

pub const DISCOVERY_TTL: Duration = Duration::from_secs(5);
pub const MAX_DISCOVERY_ASSETS: usize = 64;
const MAX_CACHE_ENTRIES: usize = 512;
const QUERY_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug)]
pub enum TradingTax {
    Ordinary { code_id: Option<i64> },
    Community(CataloguedTaxToken),
    Unsupported,
    Unavailable,
}

type Cache = HashMap<String, (Instant, TradingTax)>;
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
fn compatible(cap: &Capability, router: Option<&str>) -> bool {
    cap.funding_version == 1
        && cap.router.as_deref() == router
        && router.is_some()
        && cap.buy_bps <= 10_000
        && cap.sell_bps <= 10_000
}

pub async fn discover(state: &AppState, addr: &str, policy: &RouterHopTaxPolicy) -> TradingTax {
    if !addr.starts_with("terra1") {
        return TradingTax::Ordinary { code_id: None };
    }
    if state.lcd.require_verified_chain().await.is_err() {
        return TradingTax::Unavailable;
    }
    let key = format!(
        "{}:{:?}:{}:{}",
        state.lcd.chain_cache_namespace(),
        policy,
        state.router_address.as_deref().unwrap_or(""),
        addr
    );
    if let Ok(cache) = cache().lock() {
        if let Some((at, value)) = cache.get(&key) {
            if at.elapsed() < DISCOVERY_TTL {
                return value.clone();
            }
        }
    }
    // Global concurrency and timeout bound amplification even across concurrent route requests.
    let result = tokio::time::timeout(QUERY_TIMEOUT, async {
        let _permit = discovery_semaphore().acquire().await.ok()?;
        let code = state.lcd.get_contract_code_id(addr).await.ok()?;
        let code_id = i64::try_from(code).ok()?;
        if policy.tax_free_code_ids.contains(&code_id)
            && policy.route_tax_code_ids.contains(&code_id)
        {
            return Some(TradingTax::Unsupported); // Conflicting registrations never waive tax.
        }
        if policy.tax_free_code_ids.contains(&code_id) {
            return Some(TradingTax::Ordinary {
                code_id: Some(code_id),
            });
        }
        let hash = if !policy.route_tax_data_hashes.is_empty() {
            Some(
                state
                    .lcd
                    .get_code_data_hash(code)
                    .await
                    .ok()?
                    .trim()
                    .trim_start_matches("0x")
                    .to_ascii_lowercase(),
            )
        } else {
            None
        };
        let trusted = registered(code_id, hash.as_deref(), policy);
        if trusted {
            let cap: Capability = state
                .lcd
                .query_contract(addr, &json!({"route_tax_capability":{}}))
                .await
                .ok()?;
            if !compatible(&cap, state.router_address.as_deref()) {
                return Some(TradingTax::Unsupported);
            }
            return Some(TradingTax::Community(CataloguedTaxToken {
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
        match state
            .lcd
            .query_contract::<TaxConfig>(addr, &json!({"get_config":{}}))
            .await
        {
            Ok(config) if config.buy_bps <= 10_000 && config.sell_bps <= 10_000 => {
                Some(TradingTax::Community(CataloguedTaxToken {
                    contract_address: addr.into(),
                    code_id: Some(code_id),
                    data_hash: hash,
                    buy_bps: config.buy_bps,
                    sell_bps: config.sell_bps,
                    exemption_directory: true,
                    route_funding_version: None,
                }))
            }
            Ok(_) => Some(TradingTax::Unsupported),
            Err(crate::lcd::LcdError::ContractQueryRejected(_)) => {
                // Actual legacy fee APIs must not be waived merely because get_config rejects.
                // Until a separate audited implementation is registered, both known legacy fee
                // APIs and unrecognized implementations remain explicitly unsupported.
                for query in [json!({"tax_map":{}}), json!({"tax_info":{}})] {
                    if state
                        .lcd
                        .query_contract::<serde_json::Value>(addr, &query)
                        .await
                        .is_ok()
                    {
                        return Some(TradingTax::Unsupported);
                    }
                }
                Some(TradingTax::Unsupported)
            }
            Err(_) => None,
        }
    })
    .await;
    let value = result.ok().flatten().unwrap_or(TradingTax::Unavailable);
    if matches!(value, TradingTax::Unavailable) {
        return value; // Identity or upstream failures must never populate discovery caches.
    }
    if let Ok(mut cache) = cache().lock() {
        cache.retain(|_, (at, _)| at.elapsed() < DISCOVERY_TTL);
        if cache.len() >= MAX_CACHE_ENTRIES {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), value.clone()));
    }
    value
}

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
        assert_ne!(first.chain_cache_namespace(), second.chain_cache_namespace());
    }
}
