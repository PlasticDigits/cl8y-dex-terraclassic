//! Native wrapper observations and exact arithmetic for local route quotes (#1452).
//! Only `observe` performs I/O, from the independent scheduled ingestion worker.
use std::collections::HashMap;

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

async fn observe_floor_protocol(state: &AppState, address: &str) -> Option<u8> {
    let response = state
        .lcd
        .query_contract_bounded(address, &json!({"execution_capabilities":{}}), 1_024)
        .await
        .ok()?;
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

async fn observe_tax_cap(state: &AppState, denom: &str, mapped: bool) -> Option<u128> {
    if !mapped {
        return None;
    }
    let value: Value = state
        .lcd
        .get_bounded(&format!("/terra/treasury/v1beta1/tax_caps/{denom}"), 8_192)
        .await
        .ok()?;
    raw(&value["tax_cap"])
}

/// Bounded scheduled observation. Two known native denoms only; no caller-supplied
/// endpoint/address. Missing output tax data leaves native output unsupported,
/// without discarding an independently valid input-wrap observation.
pub async fn observe(state: &AppState) -> Option<WrapperSnapshot> {
    tokio::time::timeout(std::time::Duration::from_secs(8), observe_inner(state))
        .await
        .ok()
        .flatten()
}
async fn observe_inner(state: &AppState) -> Option<WrapperSnapshot> {
    let started = std::time::Instant::now();
    let mapper = state.wrap_mapper_address.as_deref()?;
    let observed_at_ms = Utc::now().timestamp_millis();
    state.lcd.require_verified_chain().await.ok()?;
    let config: Value = state
        .lcd
        .query_contract_bounded(mapper, &json!({"config":{}}), 8_192)
        .await
        .ok()?;
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
    let treasury_config: Value = state
        .lcd
        .query_contract_bounded(&treasury, &json!({"config":{}}), 8_192)
        .await
        .ok()?;
    if treasury_config["wrapping_paused"].as_bool() != Some(false) {
        return None;
    }
    let bindings: Value = state
        .lcd
        .query_contract_bounded(&treasury, &json!({"denom_wrappers":{}}), 8_192)
        .await
        .ok()?;
    let bindings = bindings["wrappers"].as_array()?;
    if bindings.len() > 64 {
        return None;
    }
    let mut mappings: HashMap<String, NativeMapping> = HashMap::new();
    for denom in ["uluna", "uusd"] {
        if !bindings
            .iter()
            .any(|v| v["denom"].as_str() == Some(denom) && v["wrapper"].as_str() == Some(mapper))
        {
            continue;
        }
        let response: Value = match state
            .lcd
            .query_contract_bounded(mapper, &json!({"denom_mapping":{"denom":denom}}), 8_192)
            .await
        {
            Ok(v) => v,
            Err(_) => continue,
        };
        if response["denom"].as_str() != Some(denom) {
            continue;
        }
        let Some(cw20) = response["cw20_addr"]
            .as_str()
            .filter(|s| s.starts_with("terra1") && s.len() <= 128)
        else {
            continue;
        };
        let rate: Value = match state
            .lcd
            .query_contract_bounded(mapper, &json!({"rate_limit":{"denom":denom}}), 8_192)
            .await
        {
            Ok(v) => v,
            Err(_) => continue,
        };
        if rate.get("config").is_none() {
            continue;
        }
        let maximum = if rate["config"].is_null() {
            None
        } else {
            let Some(n) = raw(&rate["config"]["max_amount_per_window"]) else {
                continue;
            };
            if rate["config"]["window_seconds"]
                .as_u64()
                .is_none_or(|n| n == 0)
            {
                continue;
            }
            Some(n)
        };
        let Some(used) = raw(&rate["amount_used"]) else {
            continue;
        };
        mappings.insert(
            denom.into(),
            NativeMapping {
                cw20: cw20.into(),
                maximum,
                used,
                tax_cap: None,
            },
        );
    }
    // Native input does not depend on bank-tax endpoints or router unwrap support.
    // Optional output work has its own subdeadline and cannot discard valid input.
    let remaining = std::time::Duration::from_millis(7_900)
        .saturating_sub(started.elapsed())
        .min(std::time::Duration::from_secs(2));
    let output = collect_output_observations(
        remaining,
        async {
            let tax: Option<Value> = state
                .lcd
                .get_bounded("/terra/tax/v1beta1/params", 8_192)
                .await
                .ok();
            tax.as_ref()
                .and_then(|v| {
                    v["params"]["burn_tax_rate"]
                        .as_str()
                        .or_else(|| v["burn_tax_rate"].as_str())
                })
                .and_then(tax_rate)
        },
        async {
            if let Some(router) = state.router_address.as_deref() {
                let cfg: Option<Value> = state
                    .lcd
                    .query_contract_bounded(router, &json!({"config":{}}), 8_192)
                    .await
                    .ok();
                cfg.map(|v| v["wrap_mapper"].as_str() == Some(mapper))
            } else {
                Some(false)
            }
        },
        observe_tax_cap(state, "uluna", mappings.contains_key("uluna")),
        observe_tax_cap(state, "uusd", mappings.contains_key("uusd")),
    )
    .await;
    for (denom, cap) in [("uluna", output.uluna_cap), ("uusd", output.uusd_cap)] {
        if let Some(m) = mappings.get_mut(denom) {
            m.tax_cap = cap;
        }
    }
    // Optional capability failures preserve useful arithmetic but never signing.
    // This work has a separate deadline and shares the scheduled provider budget.
    // Leave a small publication margin inside the outer eight-second deadline;
    // slow optional reads must not discard completed arithmetic observations.
    let remaining = std::time::Duration::from_millis(7_900)
        .saturating_sub(started.elapsed())
        .min(std::time::Duration::from_secs(2));
    let floors = collect_floor_protocols(
        remaining,
        observe_floor_protocol(state, mapper),
        observe_floor_protocol(state, &treasury),
    )
    .await;
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
