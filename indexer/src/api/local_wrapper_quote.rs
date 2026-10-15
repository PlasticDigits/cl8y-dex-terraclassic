//! Native wrapper observations and exact arithmetic for local route quotes (#1452).
//! Only `observe` performs I/O, from the independent scheduled ingestion worker.
use crate::lcd::{LcdClient, LcdError};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, AtomicU8, Ordering},
};
use tracing::Instrument;

use bigdecimal::num_bigint::BigInt;
use chrono::Utc;
use serde_json::{json, Value};

use super::AppState;
const SCALE: u128 = 1_000_000_000_000_000_000;

#[derive(Debug, Clone)]
pub struct NativeMapping {
    pub cw20: String,
    pub maximum: Option<u128>,
    pub used: u128,
    pub tax_cap: Option<u128>,
}
#[derive(Debug, Clone)]
pub struct WrapperSnapshot {
    pub mapper: String,
    pub treasury: String,
    pub observed_at_ms: i64,
    pub mappings: HashMap<String, NativeMapping>,
    pub fee_wrap_bps: u16,
    pub fee_unwrap_bps: u16,
    pub tax_atomics: Option<u128>,
    /// None means the scheduled router prerequisite was unavailable.
    pub router_unwrap_supported: Option<bool>,
    pub mapper_floor_version: Option<u8>,
    pub treasury_floor_version: Option<u8>,
}
fn raw(v: &Value) -> Option<u128> {
    v.as_str()?.parse().ok()
}
fn fee(v: &Value) -> Option<u16> {
    let n = v.as_u64()?;
    (n <= 10_000).then_some(n as u16)
}
fn ratio(a: u128, n: u128, d: u128) -> Option<u128> {
    (BigInt::from(a) * BigInt::from(n) / BigInt::from(d))
        .to_string()
        .parse()
        .ok()
}
fn net_fee(a: u128, bps: u16) -> Option<u128> {
    a.checked_sub(ratio(a, u128::from(bps), 10_000)?)
}
fn inverse_fee(target: u128, bps: u16) -> Option<u128> {
    if target == 0 || bps >= 10_000 {
        return None;
    }
    ratio(target - 1, 10_000, 10_000 - u128::from(bps))?.checked_add(1)
}
fn tax_rate(text: &str) -> Option<u128> {
    let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
    if integer != "0" || fraction.len() > 18 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    format!("{fraction:0<18}").parse().ok()
}
impl WrapperSnapshot {
    /// Availability is independent of amount-specific rate/fee protection.
    pub(crate) fn is_fresh(&self, now: i64) -> bool {
        now >= self.observed_at_ms && now - self.observed_at_ms <= 20_000
    }
    /// Explicit message variants reject old deployments rather than allowing
    /// serde's unknown-field handling to silently discard a reviewed floor.
    pub fn direct_floor_protocol(&self, wrap: bool) -> Option<u8> {
        (self.mapper_floor_version == Some(1) && (!wrap || self.treasury_floor_version == Some(1)))
            .then_some(1)
    }
    fn mapping(&self, denom: &str, amount: u128, now: i64) -> Option<&NativeMapping> {
        // Rate usage is conservatively retained until ingestion observes a reset.
        // Do not predict a chain window transition from browser/process wall time.
        if !self.is_fresh(now) || amount == 0 {
            return None;
        }
        let m = self.mappings.get(denom)?;
        if m.maximum
            .is_some_and(|limit| m.used.checked_add(amount).is_none_or(|used| used > limit))
        {
            return None;
        }
        Some(m)
    }
    pub fn wrap_input(&self, denom: &str, gross: u128, now: i64) -> Option<(String, u128)> {
        let m = self.mapping(denom, gross, now)?;
        let minted = net_fee(gross, self.fee_wrap_bps)?;
        (minted > 0).then(|| (m.cw20.clone(), minted))
    }
    pub fn unwrap_output(
        &self,
        denom: &str,
        wrapped: u128,
        now: i64,
    ) -> Option<(String, u128, u128)> {
        if self.router_unwrap_supported != Some(true) {
            return None;
        }
        self.unwrap_output_direct(denom, wrapped, now)
    }
    /// Direct mapper settlement does not depend on the DEX router configuration.
    pub fn unwrap_output_direct(
        &self,
        denom: &str,
        wrapped: u128,
        now: i64,
    ) -> Option<(String, u128, u128)> {
        let m = self.mapping(denom, wrapped, now)?;
        let base = net_fee(wrapped, self.fee_unwrap_bps)?;
        let tax = ratio(base, self.tax_atomics?, SCALE)?.min(m.tax_cap?);
        let net = base.checked_sub(tax)?;
        (net > 0).then(|| (m.cw20.clone(), net, base))
    }
    pub fn inverse_wrap(&self, target: u128) -> Option<u128> {
        inverse_fee(target, self.fee_wrap_bps)
    }
    pub fn inverse_unwrap(&self, denom: &str, target: u128) -> Option<u128> {
        if target == 0 {
            return None;
        }
        let rate = self.tax_atomics?;
        if rate >= SCALE {
            return None;
        }
        let cap = self.mappings.get(denom)?.tax_cap?;
        let uncapped = ratio(target - 1, SCALE, SCALE - rate)?.checked_add(1);
        let capped = target.checked_add(cap);
        let base = match (uncapped, capped) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) | (None, Some(a)) => a,
            _ => return None,
        };
        inverse_fee(base, self.fee_unwrap_bps)
    }
}

fn floor_protocol(value: &Value) -> Option<u8> {
    (value["direct_wrapper_floor_version"].as_u64() == Some(1)).then_some(1)
}

/// Never include upstream bodies, addresses, or endpoints in producer evidence.
fn observation_reason(error: &LcdError) -> &'static str {
    match error {
        LcdError::AdmissionDeferred => "capacity",
        LcdError::AllEndpointsFailed(message) if message.contains("circuits open") => {
            "circuit_open"
        }
        LcdError::AllEndpointsFailed(_) | LcdError::Request(_) => "provider_unavailable",
        LcdError::ChainIdentity(_) => "chain_identity",
        LcdError::HistoricalHeightUnavailable => "historical_height",
        LcdError::LaggingProvider => "lagging",
        LcdError::ContractQueryRejected(_) => "contract_rejected",
        LcdError::QueryGasLimitExceeded(_) => "query_gas_limit",
        LcdError::HttpRejected(_) => "http_rejected",
        LcdError::Deserialize(_) | LcdError::Base64(_) | LcdError::PairResponseMalformed(_) => {
            "malformed"
        }
    }
}

async fn observed_read<T>(
    phase: &'static str,
    read: impl std::future::Future<Output = Result<T, LcdError>>,
) -> Option<T> {
    match read.await {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(
                phase,
                reason = observation_reason(&error),
                "Native wrapper read unavailable"
            );
            None
        }
    }
}

async fn observe_router(lcd: &LcdClient, router: Option<&str>, mapper: &str) -> Option<bool> {
    let Some(router) = router else {
        return Some(false);
    };
    let config: Value = observed_read(
        "router_config",
        lcd.query_contract_bounded(router, &json!({"config":{}}), 8_192),
    )
    .await?;
    Some(config["wrap_mapper"].as_str() == Some(mapper))
}

/// Router configuration is independent of treasury/mapping reads. Start its
/// unchanged two-second clock early so bank-tax reads cannot consume its lane
/// admission window. Cancellation remains owned by the same outer poll.
async fn observe_router_bounded(
    lcd: &LcdClient,
    router: Option<&str>,
    mapper: &str,
) -> Option<bool> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        observe_router(lcd, router, mapper),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!(
                phase = "router_config",
                reason = "deadline",
                "Native wrapper read unavailable"
            );
            None
        }
    }
}

async fn observe_floor_protocol(lcd: &LcdClient, address: &str) -> Option<u8> {
    let response = observed_read(
        "floor_capability",
        lcd.query_contract_bounded(address, &json!({"execution_capabilities":{}}), 1_024),
    )
    .await?;
    floor_protocol(&response)
}

#[derive(Debug, Default, PartialEq)]
struct OutputObservations {
    tax_atomics: Option<u128>,
    router_supported: Option<bool>,
    uluna_cap: Option<u128>,
    uusd_cap: Option<u128>,
}

async fn bounded_observation<F, T>(deadline: tokio::time::Instant, observation: F) -> Option<T>
where
    F: std::future::Future<Output = Option<T>>,
{
    tokio::time::timeout_at(deadline, observation)
        .await
        .ok()
        .flatten()
}

/// Independent scheduled reads share the same absolute subdeadline and the
/// caller's existing LCD pacing/permits. A slow unrelated denom must not erase
/// completed required fields. Dropping this future cancels all local work.
async fn collect_output_observations<T, R, L, U>(
    remaining: std::time::Duration,
    tax: T,
    router: R,
    uluna: L,
    uusd: U,
) -> OutputObservations
where
    T: std::future::Future<Output = Option<u128>>,
    R: std::future::Future<Output = Option<bool>>,
    L: std::future::Future<Output = Option<u128>>,
    U: std::future::Future<Output = Option<u128>>,
{
    if remaining.is_zero() {
        return OutputObservations::default();
    }
    let deadline = tokio::time::Instant::now() + remaining;
    let (tax, router, uluna, uusd) = tokio::join!(
        bounded_observation(deadline, tax),
        bounded_observation(deadline, router),
        bounded_observation(deadline, uluna),
        bounded_observation(deadline, uusd),
    );
    OutputObservations {
        tax_atomics: tax,
        router_supported: router,
        uluna_cap: uluna,
        uusd_cap: uusd,
    }
}

async fn collect_floor_protocols<M, T>(
    remaining: std::time::Duration,
    mapper: M,
    treasury: T,
) -> (Option<u8>, Option<u8>)
where
    M: std::future::Future<Output = Option<u8>>,
    T: std::future::Future<Output = Option<u8>>,
{
    if remaining.is_zero() {
        return (None, None);
    }
    let deadline = tokio::time::Instant::now() + remaining;
    tokio::join!(
        bounded_observation(deadline, mapper),
        bounded_observation(deadline, treasury)
    )
}

async fn observe_tax_cap(lcd: &LcdClient, denom: &str, mapped: bool) -> Option<u128> {
    if !mapped {
        return None;
    }
    let value: Value = observed_read(
        "tax_cap",
        lcd.get_bounded(&format!("/terra/treasury/v1beta1/tax_caps/{denom}"), 8_192),
    )
    .await?;
    raw(&value["tax_cap"])
}

async fn observe_mapping(
    lcd: &LcdClient,
    mapper: &str,
    bindings: &[Value],
    denom: &str,
) -> Option<NativeMapping> {
    if !bindings
        .iter()
        .any(|v| v["denom"].as_str() == Some(denom) && v["wrapper"].as_str() == Some(mapper))
    {
        return None;
    }
    let mapping_query = json!({"denom_mapping":{"denom":denom}});
    let rate_query = json!({"rate_limit":{"denom":denom}});
    let (response, rate) = tokio::join!(
        observed_read(
            "denom_mapping",
            lcd.query_contract_bounded::<Value>(mapper, &mapping_query, 8_192)
        ),
        observed_read(
            "rate_limit",
            lcd.query_contract_bounded::<Value>(mapper, &rate_query, 8_192)
        ),
    );
    let response = response?;
    let rate = rate?;
    if response["denom"].as_str() != Some(denom) || rate.get("config").is_none() {
        return None;
    }
    let cw20 = response["cw20_addr"]
        .as_str()
        .filter(|s| s.starts_with("terra1") && s.len() <= 128)?;
    let maximum = if rate["config"].is_null() {
        None
    } else {
        if rate["config"]["window_seconds"]
            .as_u64()
            .is_none_or(|n| n == 0)
        {
            return None;
        }
        Some(raw(&rate["config"]["max_amount_per_window"])?)
    };
    Some(NativeMapping {
        cw20: cw20.into(),
        maximum,
        used: raw(&rate["amount_used"])?,
        tax_cap: None,
    })
}

/// Bounded scheduled observation. Two known native denoms only; no caller-supplied
/// endpoint/address. Missing output tax data leaves native output unsupported,
/// without discarding an independently valid input-wrap observation.
pub async fn observe(state: &AppState) -> Option<WrapperSnapshot> {
    observe_config(
        &state.lcd,
        state.wrap_mapper_address.as_deref()?,
        state.router_address.as_deref(),
    )
    .await
}

async fn observe_config(
    lcd: &LcdClient,
    mapper: &str,
    router: Option<&str>,
) -> Option<WrapperSnapshot> {
    static NEXT_POLL: AtomicU64 = AtomicU64::new(1);
    let poll_id = NEXT_POLL.fetch_add(1, Ordering::Relaxed);
    let phase = AtomicU8::new(0);
    async {
        let started = std::time::Instant::now();
        let observed_at_ms = Utc::now().timestamp_millis();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            observe_inner(lcd, mapper, router, &phase, observed_at_ms),
        )
        .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(Some(snapshot)) => {
                tracing::info!(elapsed_ms, observed_at_ms,
                    mappings = snapshot.mappings.len(),
                    output_tax = snapshot.tax_atomics.is_some(),
                    router = ?snapshot.router_unwrap_supported,
                    mapper_floor = snapshot.mapper_floor_version.is_some(),
                    treasury_floor = snapshot.treasury_floor_version.is_some(),
                    uluna_cap = snapshot.mappings.get("uluna").is_some_and(|m| m.tax_cap.is_some()),
                    uusd_cap = snapshot.mappings.get("uusd").is_some_and(|m| m.tax_cap.is_some()),
                    "Native wrapper observations completed");
                Some(snapshot)
            }
            outcome => {
                let phase = match phase.load(Ordering::Relaxed) {
                    0 => "chain",
                    1 => "mapper",
                    2 => "treasury",
                    3 => "mappings",
                    4 => "output",
                    _ => "floor_capabilities",
                };
                let reason = if outcome.is_err() {
                    "deadline"
                } else {
                    "missing_or_invalid"
                };
                tracing::warn!(
                    phase,
                    reason,
                    elapsed_ms,
                    observed_at_ms,
                    "Native wrapper observations unavailable"
                );
                None
            }
        }
    }
    .instrument(tracing::info_span!("native_wrapper_poll", poll_id))
    .await
}
async fn observe_inner(
    lcd: &LcdClient,
    mapper: &str,
    router: Option<&str>,
    phase: &AtomicU8,
    observed_at_ms: i64,
) -> Option<WrapperSnapshot> {
    let started = std::time::Instant::now();
    observed_read("chain", lcd.require_verified_chain()).await?;
    phase.store(1, Ordering::Relaxed);
    let config: Value = observed_read(
        "mapper_config",
        lcd.query_contract_bounded(mapper, &json!({"config":{}}), 8_192),
    )
    .await?;
    if config["paused"].as_bool() != Some(false) {
        return None;
    }
    let treasury = config["treasury"].as_str()?.to_owned();
    if !treasury.starts_with("terra1") || treasury.len() > 128 {
        return None;
    }
    let (wrap, unwrap) = match (config.get("fee_wrap_bps"), config.get("fee_unwrap_bps")) {
        (Some(a), Some(b)) if !a.is_null() && !b.is_null() => (fee(a)?, fee(b)?),
        (a, b) if a.is_none_or(Value::is_null) && b.is_none_or(Value::is_null) => {
            let n = fee(&config["fee_bps"])?;
            (n, n)
        }
        _ => return None,
    };
    // These are independent prerequisites. Keeping them serial made ordinary
    // provider latency consume the output/capability publication budget (#1505).
    phase.store(2, Ordering::Relaxed);
    let config_query = json!({"config":{}});
    let bindings_query = json!({"denom_wrappers":{}});
    let (router_supported, treasury_config, bindings) = tokio::join!(
        observe_router_bounded(lcd, router, mapper),
        observed_read(
            "treasury_config",
            lcd.query_contract_bounded::<Value>(&treasury, &config_query, 8_192)
        ),
        observed_read(
            "denom_bindings",
            lcd.query_contract_bounded::<Value>(&treasury, &bindings_query, 8_192)
        ),
    );
    if treasury_config?["wrapping_paused"].as_bool() != Some(false) {
        return None;
    }
    let bindings = bindings?;
    let bindings = bindings["wrappers"].as_array()?;
    if bindings.len() > 64 {
        return None;
    }
    phase.store(3, Ordering::Relaxed);
    let (uluna, uusd) = tokio::join!(
        observe_mapping(lcd, mapper, bindings, "uluna"),
        observe_mapping(lcd, mapper, bindings, "uusd"),
    );
    let mut mappings: HashMap<String, NativeMapping> = [("uluna", uluna), ("uusd", uusd)]
        .into_iter()
        .filter_map(|(denom, mapping)| mapping.map(|mapping| (denom.into(), mapping)))
        .collect();
    // Native input does not depend on bank-tax endpoints or router unwrap support.
    // Optional output work has its own subdeadline and cannot discard valid input.
    let remaining = std::time::Duration::from_millis(7_900)
        .saturating_sub(started.elapsed())
        .min(std::time::Duration::from_secs(2));
    phase.store(4, Ordering::Relaxed);
    let output = collect_output_observations(
        remaining,
        async {
            let tax: Option<Value> = observed_read(
                "tax_rate",
                lcd.get_bounded("/terra/tax/v1beta1/params", 8_192),
            )
            .await;
            tax.as_ref()
                .and_then(|v| {
                    v["params"]["burn_tax_rate"]
                        .as_str()
                        .or_else(|| v["burn_tax_rate"].as_str())
                })
                .and_then(tax_rate)
        },
        async { router_supported },
        observe_tax_cap(lcd, "uluna", mappings.contains_key("uluna")),
        observe_tax_cap(lcd, "uusd", mappings.contains_key("uusd")),
    )
    .await;
    let remaining = std::time::Duration::from_millis(7_900)
        .saturating_sub(started.elapsed())
        .min(std::time::Duration::from_secs(2));
    phase.store(5, Ordering::Relaxed);
    let floors = collect_floor_protocols(
        remaining,
        observe_floor_protocol(lcd, mapper),
        observe_floor_protocol(lcd, &treasury),
    )
    .await;
    for (denom, cap) in [("uluna", output.uluna_cap), ("uusd", output.uusd_cap)] {
        if let Some(m) = mappings.get_mut(denom) {
            m.tax_cap = cap;
        }
    }
    Some(WrapperSnapshot {
        mapper: mapper.into(),
        treasury,
        observed_at_ms,
        mappings,
        fee_wrap_bps: wrap,
        fee_unwrap_bps: unwrap,
        tax_atomics: output.tax_atomics,
        router_unwrap_supported: output.router_supported,
        mapper_floor_version: floors.0,
        treasury_floor_version: floors.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    async fn wrapper_fixture(
        router_delay: Duration,
        bank_delay: Duration,
    ) -> (wiremock::MockServer, LcdClient) {
        use base64::Engine;
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "default_node_info":{"network":"columbus-5"}
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(move |request: &wiremock::Request| {
                let path = request.url.path();
                let body = if let Some(encoded) = path.split("/smart/").nth(1) {
                    let query: Value = serde_json::from_slice(
                        &base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .unwrap(),
                    )
                    .unwrap();
                    let data = if query.get("config").is_some() {
                        if path.contains("terra1mapper") {
                            json!({"paused":false,"treasury":"terra1treasury","fee_bps":100})
                        } else if path.contains("terra1treasury") {
                            json!({"wrapping_paused":false})
                        } else {
                            json!({"wrap_mapper":"terra1mapper"})
                        }
                    } else if query.get("denom_wrappers").is_some() {
                        json!({"wrappers":[{"denom":"uluna","wrapper":"terra1mapper"},
                            {"denom":"uusd","wrapper":"terra1mapper"}]})
                    } else if let Some(mapping) = query.get("denom_mapping") {
                        json!({"denom":mapping["denom"],"cw20_addr":"terra1wrapped"})
                    } else if query.get("rate_limit").is_some() {
                        json!({"config":null,"amount_used":"0"})
                    } else if query.get("execution_capabilities").is_some() {
                        json!({"direct_wrapper_floor_version":1})
                    } else {
                        panic!("unexpected scheduled smart query")
                    };
                    json!({"data":data})
                } else if path.contains("tax_caps") {
                    json!({"tax_cap":"1000"})
                } else if path.contains("/terra/tax/") {
                    json!({"params":{"burn_tax_rate":"0.005"}})
                } else {
                    panic!("unexpected scheduled HTTP path")
                };
                let delay = if path.contains("terra1router") {
                    router_delay
                } else if path.starts_with("/terra/") {
                    bank_delay
                } else {
                    Duration::from_millis(800)
                };
                ResponseTemplate::new(200)
                    .set_body_json(body)
                    .set_delay(delay)
            })
            .with_priority(10)
            .mount(&server)
            .await;
        // Scoped fixture permits a slow-but-successful router response; the
        // producer's two-second subdeadline and runtime policy stay unchanged.
        let policy = crate::lcd::ProviderPolicy {
            attempt_timeout_ms: 2_500,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(vec![server.uri()], 2_500, 1, "columbus-5", policy)
            .with_ingestion_budget();
        (server, lcd)
    }

    #[tokio::test]
    async fn issue1505_early_router_avoids_reproduced_output_lane_deadline() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(1_800), Duration::from_millis(50)).await;
        // The old phase polls tax first, placing the slow router read behind its
        // 250ms pacing turn. It loses its two-second deadline while both bank
        // caps complete, exactly the deployed failure's field pattern.
        let previous = collect_output_observations(
            Duration::from_secs(2),
            async {
                let value: Value = lcd
                    .get_bounded("/terra/tax/v1beta1/params", 8_192)
                    .await
                    .ok()?;
                tax_rate(value["params"]["burn_tax_rate"].as_str()?)
            },
            observe_router(&lcd, Some("terra1router"), "terra1mapper"),
            observe_tax_cap(&lcd, "uluna", true),
            observe_tax_cap(&lcd, "uusd", true),
        )
        .await;
        assert_eq!(previous.router_supported, None);
        assert!(previous.tax_atomics.is_some());
        assert!(previous.uluna_cap.is_some());
        assert!(previous.uusd_cap.is_some());
        let before_count = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
            .count();
        let before = Utc::now().timestamp_millis();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect("early router must complete within unchanged publication deadline");
        assert_eq!(snapshot.router_unwrap_supported, Some(true));
        assert!(snapshot.tax_atomics.is_some());
        for denom in ["uluna", "uusd"] {
            assert!(snapshot
                .unwrap_output(denom, 10_000, Utc::now().timestamp_millis())
                .is_some());
        }
        assert!(snapshot.observed_at_ms >= before && snapshot.observed_at_ms <= before + 100);
        let after_count = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
            .count();
        assert_eq!(after_count - before_count, 13, "unchanged data-read count");
        let counters = lcd.provider_counters().await;
        assert_eq!(
            counters[0].failures, 0,
            "scheduled deadline cancellation is neutral"
        );
        assert_eq!(counters[0].in_flight, 0);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_early_router_deadline_is_typed_and_keeps_native_input_only() {
        let (_server, lcd) =
            wrapper_fixture(Duration::from_millis(3_000), Duration::from_millis(50)).await;
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect("optional router timeout must not erase completed native input");
        assert_eq!(snapshot.router_unwrap_supported, None);
        let now = Utc::now().timestamp_millis();
        assert!(snapshot.wrap_input("uusd", 10_000, now).is_some());
        assert!(snapshot.unwrap_output("uusd", 10_000, now).is_none());
        assert!(logs_contain("router_config"));
        assert!(logs_contain("deadline"));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].caller_cancellations, 1);
        assert_eq!(counters[0].in_flight, 0);
    }

    /// Real HTTP plus the production ingestion lane: seven serial prerequisite
    /// reads at800ms left<2s for output and no floor window. The same calls now
    /// complete both native directions without raising any deadline or limit.
    #[tokio::test]
    async fn issue1505_scheduled_native_poll_keeps_output_and_floor_budget() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(800)).await;
        let before = Utc::now().timestamp_millis();
        let start = std::time::Instant::now();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect("ordinary provider latency must not erase native publication");
        assert!(start.elapsed() < Duration::from_millis(7_900));
        assert!(snapshot.observed_at_ms >= before);
        assert!(snapshot.observed_at_ms <= before + 100);
        assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
        assert_eq!(snapshot.direct_floor_protocol(false), Some(1));
        for denom in ["uluna", "uusd"] {
            assert_eq!(
                snapshot.wrap_input(denom, 10_000, before + 8_000),
                Some(("terra1wrapped".into(), 9_900))
            );
            assert_eq!(
                snapshot.unwrap_output(denom, 10_000, before + 8_000),
                Some(("terra1wrapped".into(), 9_851, 9_900))
            );
            assert!(snapshot
                .wrap_input(denom, 10_000, snapshot.observed_at_ms + 20_001)
                .is_none());
            assert!(snapshot
                .unwrap_output(denom, 10_000, snapshot.observed_at_ms + 20_001)
                .is_none());
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.iter().filter(|request| request.url.path()
            != "/cosmos/base/tendermint/v1beta1/node_info").count(), 13,
            "same thirteen bounded observation reads, with independently required identity attestations");
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].in_flight, 0);
    }

    async fn delayed<T>(millis: u64, value: Option<T>) -> Option<T> {
        tokio::time::sleep(Duration::from_millis(millis)).await;
        value
    }

    async fn immediate<T>(calls: Arc<AtomicUsize>, value: T) -> Option<T> {
        calls.fetch_add(1, Ordering::SeqCst);
        Some(value)
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_zero_remaining_phase_starts_no_observation_or_capability_reads() {
        let calls = Arc::new(AtomicUsize::new(0));
        let output = collect_output_observations(
            Duration::ZERO,
            immediate(calls.clone(), 0u128),
            immediate(calls.clone(), true),
            immediate(calls.clone(), 1000u128),
            immediate(calls.clone(), 1000u128),
        )
        .await;
        let floors = collect_floor_protocols(
            Duration::ZERO,
            immediate(calls.clone(), 1u8),
            immediate(calls.clone(), 1u8),
        )
        .await;
        assert_eq!(output, OutputObservations::default());
        assert_eq!(floors, (None, None));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    async fn paced<T>(
        permits: Arc<tokio::sync::Semaphore>,
        next: Arc<tokio::sync::Mutex<tokio::time::Instant>>,
        calls: Arc<AtomicUsize>,
        value: T,
    ) -> Option<T> {
        let _permit = permits.acquire_owned().await.unwrap();
        let mut tick = next.lock().await;
        tokio::time::sleep_until(*tick).await;
        *tick = tokio::time::Instant::now() + Duration::from_millis(250);
        drop(tick);
        calls.fetch_add(1, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(600)).await;
        Some(value)
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_output_reads_complete_with_original_two_permits_four_rps_budget() {
        let permits = Arc::new(tokio::sync::Semaphore::new(2));
        let next = Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now()));
        let calls = Arc::new(AtomicUsize::new(0));
        let start = tokio::time::Instant::now();
        // Independent timing: admissions at 0/250/600/850ms; each response
        // takes600ms, so all four complete at1450ms. Sequential needs2400ms.
        let result = collect_output_observations(
            Duration::from_secs(2),
            paced(
                permits.clone(),
                next.clone(),
                calls.clone(),
                15_000_000_000_000_000u128,
            ),
            paced(permits.clone(), next.clone(), calls.clone(), true),
            paced(permits.clone(), next.clone(), calls.clone(), 1000u128),
            paced(permits.clone(), next.clone(), calls.clone(), 2000u128),
        )
        .await;
        assert_eq!(
            result,
            OutputObservations {
                tax_atomics: Some(15_000_000_000_000_000),
                router_supported: Some(true),
                uluna_cap: Some(1000),
                uusd_cap: Some(2000)
            }
        );
        assert_eq!(start.elapsed(), Duration::from_millis(1450));
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert_eq!(permits.available_permits(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_slow_unrelated_uluna_cap_preserves_usable_uusd_with_original_expiry() {
        let start = tokio::time::Instant::now();
        let result = collect_output_observations(
            Duration::from_secs(2),
            delayed(20, Some(15_000_000_000_000_000u128)),
            delayed(15, Some(true)),
            delayed(3000, Some(1000u128)),
            delayed(25, Some(1000u128)),
        )
        .await;
        assert_eq!(start.elapsed(), Duration::from_secs(2));
        assert_eq!(result.uluna_cap, None);
        let mut s = fixture();
        s.mappings.insert(
            "uusd".into(),
            NativeMapping {
                cw20: "cust".into(),
                maximum: None,
                used: 0,
                tax_cap: result.uusd_cap,
            },
        );
        s.mappings.get_mut("uluna").unwrap().tax_cap = result.uluna_cap;
        s.tax_atomics = result.tax_atomics;
        s.router_unwrap_supported = result.router_supported;
        // Independent ledger:10000 -200 wrapperfee =9800; taxfloor147 =>9653.
        assert_eq!(
            s.unwrap_output("uusd", 10_000, 3000),
            Some(("cust".into(), 9653, 9800))
        );
        assert!(s.unwrap_output("uluna", 10_000, 3000).is_none());
        assert!(s.wrap_input("uluna", 10_000, 3000).is_some());
        assert_eq!(s.observed_at_ms, 1000);
        assert!(s.unwrap_output("uusd", 10_000, 21_001).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_failed_required_tax_stays_unknown_while_other_fields_complete() {
        let result = collect_output_observations(
            Duration::from_secs(2),
            delayed(10, None::<u128>),
            delayed(20, Some(true)),
            delayed(25, Some(1000u128)),
            delayed(30, Some(2000u128)),
        )
        .await;
        assert_eq!(result.tax_atomics, None);
        assert_eq!(result.router_supported, Some(true));
        assert_eq!(result.uusd_cap, Some(2000));
        let mut s = fixture();
        s.tax_atomics = result.tax_atomics;
        assert!(s.unwrap_output("uluna", 10_000, 1000).is_none());
        assert!(s.wrap_input("uluna", 10_000, 1000).is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_router_negative_is_not_changed_to_unknown_or_success() {
        let result = collect_output_observations(
            Duration::from_secs(2),
            delayed(5, Some(0u128)),
            delayed(10, Some(false)),
            delayed(15, Some(1000u128)),
            delayed(20, Some(1000u128)),
        )
        .await;
        assert_eq!(result.router_supported, Some(false));
        let mut s = fixture();
        s.router_unwrap_supported = result.router_supported;
        assert!(s.unwrap_output("uluna", 10_000, 1000).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_slow_treasury_capability_preserves_mapper_but_never_authorizes_wrap() {
        let start = tokio::time::Instant::now();
        let (mapper, treasury) = collect_floor_protocols(
            Duration::from_secs(2),
            delayed(20, Some(1u8)),
            delayed(3000, Some(1u8)),
        )
        .await;
        assert_eq!(start.elapsed(), Duration::from_secs(2));
        let mut s = fixture();
        s.mapper_floor_version = mapper;
        s.treasury_floor_version = treasury;
        assert_eq!(s.direct_floor_protocol(false), Some(1));
        assert_eq!(s.direct_floor_protocol(true), None);
        assert_eq!(s.observed_at_ms, 1000);
        assert!(s.unwrap_output_direct("uluna", 10_000, 21_001).is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn issue1499_cancellation_releases_existing_budget_and_never_spawns_detached_work() {
        let permits = Arc::new(tokio::sync::Semaphore::new(2));
        let next = Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now()));
        let calls = Arc::new(AtomicUsize::new(0));
        let result = tokio::time::timeout(
            Duration::from_millis(50),
            collect_output_observations(
                Duration::from_secs(2),
                paced(permits.clone(), next.clone(), calls.clone(), 1u128),
                paced(permits.clone(), next.clone(), calls.clone(), true),
                paced(permits.clone(), next.clone(), calls.clone(), 1u128),
                paced(permits.clone(), next.clone(), calls.clone(), 1u128),
            ),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(permits.available_permits(), 2);
        let accepted = calls.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert_eq!(calls.load(Ordering::SeqCst), accepted);
    }
    fn fixture() -> WrapperSnapshot {
        WrapperSnapshot {
            mapper: "mapper".into(),
            treasury: "treasury".into(),
            observed_at_ms: 1000,
            mappings: HashMap::from([(
                "uluna".into(),
                NativeMapping {
                    cw20: "clunc".into(),
                    maximum: None,
                    used: 0,
                    tax_cap: Some(1000),
                },
            )]),
            fee_wrap_bps: 200,
            fee_unwrap_bps: 200,
            tax_atomics: Some(15_000_000_000_000_000),
            router_unwrap_supported: Some(true),
            mapper_floor_version: None,
            treasury_floor_version: None,
        }
    }
    #[test]
    fn wrap_charges_fee_once_without_bank_burn_tax_and_inverse_is_minimal() {
        let s = fixture();
        assert_eq!(
            s.wrap_input("uluna", 10_000, 1000),
            Some(("clunc".into(), 9800))
        );
        for n in [1, 2, 99, 10_000, 1_000_000] {
            let gross = s.inverse_wrap(n).unwrap();
            assert!(s.wrap_input("uluna", gross, 1000).unwrap().1 >= n);
            if gross > 1 {
                assert!(s.wrap_input("uluna", gross - 1, 1000).unwrap().1 < n);
            }
        }
    }
    #[test]
    fn unwrap_mapper_fee_and_actual_capped_tax_keep_router_base_separate() {
        let s = fixture();
        assert_eq!(
            s.unwrap_output("uluna", 10_000, 1000),
            Some(("clunc".into(), 9653, 9800))
        );
        for n in [1, 99, 10_000, 1_000_000] {
            let gross = s.inverse_unwrap("uluna", n).unwrap();
            assert!(s.unwrap_output("uluna", gross, 1000).unwrap().1 >= n);
            if gross > 1 {
                assert!(s
                    .unwrap_output("uluna", gross - 1, 1000)
                    .is_none_or(|v| v.1 < n));
            }
        }
    }
    #[test]
    fn stale_future_unknown_tax_mapping_and_rate_budget_fail_without_fallback() {
        let mut s = fixture();
        assert!(s.wrap_input("uluna", 1, 999).is_none());
        assert!(s.wrap_input("uluna", 1, 21_001).is_none());
        assert!(s.wrap_input("uusd", 1, 1000).is_none());
        s.tax_atomics = None;
        assert!(s.unwrap_output("uluna", 100, 1000).is_none());
        assert!(s.wrap_input("uluna", 100, 1000).is_some());
        let m = s.mappings.get_mut("uluna").unwrap();
        m.maximum = Some(100);
        m.used = 99;
        assert!(s.wrap_input("uluna", 2, 1000).is_none());
        assert!(s.wrap_input("uluna", 1, 1000).is_some());
    }
    #[test]
    fn huge_amounts_and_fee_rate_parsing_fail_safely() {
        let s = fixture();
        assert!(s.wrap_input("uluna", u128::MAX, 1000).is_some());
        assert!(s.inverse_wrap(u128::MAX).is_none());
        assert_eq!(tax_rate("0.015"), Some(15_000_000_000_000_000));
        for v in ["1.5", "-0.01", "0.nan", "0.1234567890123456789"] {
            assert!(tax_rate(v).is_none());
        }
    }
    #[test]
    fn direct_floor_capability_requires_exact_supported_protocol_at_each_boundary() {
        let mut s = fixture();
        assert_eq!(s.direct_floor_protocol(true), None);
        assert_eq!(s.direct_floor_protocol(false), None);
        s.mapper_floor_version = Some(1);
        assert_eq!(s.direct_floor_protocol(false), Some(1));
        assert_eq!(s.direct_floor_protocol(true), None);
        s.treasury_floor_version = Some(1);
        assert_eq!(s.direct_floor_protocol(true), Some(1));
        s.mapper_floor_version = Some(2);
        assert_eq!(s.direct_floor_protocol(false), None);
        for invalid in [
            json!({}),
            json!({"direct_wrapper_floor_version":true}),
            json!({"direct_wrapper_floor_version":"1"}),
            json!({"direct_wrapper_floor_version":0}),
            json!({"direct_wrapper_floor_version":2}),
        ] {
            assert_eq!(floor_protocol(&invalid), None);
        }
        assert_eq!(
            floor_protocol(&json!({"direct_wrapper_floor_version":1})),
            Some(1)
        );
    }
    #[test]
    fn direct_unwrap_keeps_arithmetic_without_router_but_never_waives_tax_or_expiry() {
        let mut s = fixture();
        s.router_unwrap_supported = Some(false);
        assert!(s.unwrap_output("uluna", 10_000, 1000).is_none());
        assert_eq!(
            s.unwrap_output_direct("uluna", 10_000, 1000),
            Some(("clunc".into(), 9653, 9800))
        );
        assert!(s.unwrap_output_direct("uluna", 10_000, 21_001).is_none());
        s.tax_atomics = None;
        assert!(s.unwrap_output_direct("uluna", 10_000, 1000).is_none());
    }
}
