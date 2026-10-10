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
    pub router_unwrap_supported: bool,
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
        if !self.router_unwrap_supported {
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
    let output = tokio::time::timeout(remaining, async {
        let tax: Option<Value> = state
            .lcd
            .get_bounded("/terra/tax/v1beta1/params", 8_192)
            .await
            .ok();
        let tax_atomics = tax
            .as_ref()
            .and_then(|v| {
                v["params"]["burn_tax_rate"]
                    .as_str()
                    .or_else(|| v["burn_tax_rate"].as_str())
            })
            .and_then(tax_rate);
        let router_unwrap_supported = if let Some(router) = state.router_address.as_deref() {
            let cfg: Option<Value> = state
                .lcd
                .query_contract_bounded(router, &json!({"config":{}}), 8_192)
                .await
                .ok();
            cfg.is_some_and(|v| v["wrap_mapper"].as_str() == Some(mapper))
        } else {
            false
        };
        let mut caps = HashMap::new();
        for denom in mappings.keys() {
            let cap: Option<Value> = state
                .lcd
                .get_bounded(&format!("/terra/treasury/v1beta1/tax_caps/{denom}"), 8_192)
                .await
                .ok();
            caps.insert(denom.clone(), cap.as_ref().and_then(|v| raw(&v["tax_cap"])));
        }
        (tax_atomics, router_unwrap_supported, caps)
    })
    .await
    .ok();
    let (tax_atomics, router_unwrap_supported) = if let Some((rate, supported, caps)) = output {
        for (denom, cap) in caps {
            if let Some(m) = mappings.get_mut(&denom) {
                m.tax_cap = cap;
            }
        }
        (rate, supported)
    } else {
        (None, false)
    };
    // Optional capability failures preserve useful arithmetic but never signing.
    // This work has a separate deadline and shares the scheduled provider budget.
    // Leave a small publication margin inside the outer eight-second deadline;
    // slow optional reads must not discard completed arithmetic observations.
    let remaining = std::time::Duration::from_millis(7_900)
        .saturating_sub(started.elapsed())
        .min(std::time::Duration::from_secs(2));
    let floors = tokio::time::timeout(remaining, async {
        (
            observe_floor_protocol(state, mapper).await,
            observe_floor_protocol(state, &treasury).await,
        )
    })
    .await
    .unwrap_or((None, None));
    Some(WrapperSnapshot {
        mapper: mapper.into(),
        treasury,
        observed_at_ms,
        mappings,
        fee_wrap_bps: wrap,
        fee_unwrap_bps: unwrap,
        tax_atomics,
        router_unwrap_supported,
        mapper_floor_version: floors.0,
        treasury_floor_version: floors.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
            router_unwrap_supported: true,
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
        s.router_unwrap_supported = false;
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
