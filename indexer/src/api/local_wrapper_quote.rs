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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapperRevocation {
    MapperPaused,
    TreasuryWrappingPaused,
}
impl WrapperRevocation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MapperPaused => "mapper_paused",
            Self::TreasuryWrappingPaused => "treasury_wrapping_paused",
        }
    }
}
/// A validated negative revokes the previous namespace snapshot immediately.
/// Transport/schema loss remains distinct and cannot manufacture compatibility.
#[derive(Debug, Clone)]
pub enum WrapperObservation {
    Available(WrapperSnapshot),
    Unavailable,
    Revoked(WrapperRevocation),
}
fn raw(v: &Value) -> Option<u128> {
    v.as_str()?.parse().ok()
}
fn fee(v: &Value) -> Option<u16> {
    let n = v.as_u64()?;
    (n <= 10_000).then_some(n as u16)
}
fn mapper_fees(config: &Value) -> Option<(u16, u16)> {
    match (config.get("fee_wrap_bps"), config.get("fee_unwrap_bps")) {
        (Some(a), Some(b)) if !a.is_null() && !b.is_null() => Some((fee(a)?, fee(b)?)),
        (a, b) if a.is_none_or(Value::is_null) && b.is_none_or(Value::is_null) => {
            let n = fee(&config["fee_bps"])?;
            Some((n, n))
        }
        _ => None,
    }
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
        LcdError::AllEndpointsFailed(message) if message == "total deadline exceeded" => "deadline",
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

/// Fixed physical-read identities; no caller-supplied asset or address enters
/// dispatch or diagnostic labels.
#[derive(Clone, Copy)]
enum ReadSlot {
    Chain,
    MapperConfig,
    RouterConfig,
    TreasuryConfig,
    DenomBindings,
    UlunaMapping,
    UlunaRate,
    UlunaCap,
    UusdMapping,
    UusdRate,
    UusdCap,
    TaxRate,
    MapperFloor,
    TreasuryFloor,
}
impl ReadSlot {
    fn name(self) -> &'static str {
        match self {
            Self::Chain => "chain",
            Self::MapperConfig => "mapper_config",
            Self::RouterConfig => "router_config",
            Self::TreasuryConfig => "treasury_config",
            Self::DenomBindings => "denom_bindings",
            Self::UlunaMapping => "uluna_mapping",
            Self::UlunaRate => "uluna_rate",
            Self::UlunaCap => "uluna_cap",
            Self::UusdMapping => "uusd_mapping",
            Self::UusdRate => "uusd_rate",
            Self::UusdCap => "uusd_cap",
            Self::TaxRate => "tax_rate",
            Self::MapperFloor => "mapper_floor",
            Self::TreasuryFloor => "treasury_floor",
        }
    }
    fn phase(self) -> &'static str {
        match self {
            Self::UlunaMapping | Self::UusdMapping => "denom_mapping",
            Self::UlunaRate | Self::UusdRate => "rate_limit",
            Self::UlunaCap | Self::UusdCap => "tax_cap",
            Self::MapperFloor | Self::TreasuryFloor => "floor_capability",
            _ => self.name(),
        }
    }
    fn denom(self) -> Option<usize> {
        match self {
            Self::UlunaMapping | Self::UlunaRate | Self::UlunaCap => Some(0),
            Self::UusdMapping | Self::UusdRate | Self::UusdCap => Some(1),
            _ => None,
        }
    }
}
struct ObservationDispatch {
    global: tokio::sync::Semaphore,
    denoms: [tokio::sync::Semaphore; 2],
}
impl ObservationDispatch {
    fn new(global: usize) -> Self {
        Self {
            global: tokio::sync::Semaphore::new(global),
            denoms: std::array::from_fn(|_| tokio::sync::Semaphore::new(1)),
        }
    }
}
#[derive(Clone, Copy)]
enum ReadStage {
    DenomDispatch,
    GlobalDispatch,
    LcdRead,
}
impl ReadStage {
    fn name(self) -> &'static str {
        match self {
            Self::DenomDispatch => "denom_dispatch",
            Self::GlobalDispatch => "global_dispatch",
            Self::LcdRead => "lcd_read",
        }
    }
}
/// One failure event per polled physical read, including outer cancellation.
/// Successful reads are silent. LCD time includes its admission/identity work;
/// lcd_polled never asserts that a network request reached an upstream.
struct ReadWitness {
    slot: ReadSlot,
    poll_started: tokio::time::Instant,
    deadline: tokio::time::Instant,
    stage_started: tokio::time::Instant,
    stage: ReadStage,
    denom_wait: std::time::Duration,
    global_wait: std::time::Duration,
    span: tracing::Span,
    done: bool,
}
impl ReadWitness {
    fn new(reads: &ObservationReads<'_>, slot: ReadSlot) -> Self {
        Self {
            slot,
            poll_started: reads.poll_started,
            deadline: reads.deadline,
            stage_started: tokio::time::Instant::now(),
            stage: if slot.denom().is_some() {
                ReadStage::DenomDispatch
            } else {
                ReadStage::GlobalDispatch
            },
            denom_wait: std::time::Duration::ZERO,
            global_wait: std::time::Duration::ZERO,
            span: tracing::Span::current(),
            done: false,
        }
    }
    fn advance(&mut self, stage: ReadStage) {
        let now = tokio::time::Instant::now();
        let elapsed = now.saturating_duration_since(self.stage_started);
        match self.stage {
            ReadStage::DenomDispatch => self.denom_wait = elapsed,
            ReadStage::GlobalDispatch => self.global_wait = elapsed,
            ReadStage::LcdRead => {}
        }
        self.stage = stage;
        self.stage_started = now;
    }
    fn fail(&mut self, reason: &'static str) {
        if self.done {
            return;
        }
        self.done = true;
        let now = tokio::time::Instant::now();
        let elapsed = now.saturating_duration_since(self.stage_started);
        let mut denom_wait = self.denom_wait;
        let mut global_wait = self.global_wait;
        let mut active = std::time::Duration::ZERO;
        let lcd_polled = matches!(self.stage, ReadStage::LcdRead);
        match self.stage {
            ReadStage::DenomDispatch => denom_wait = elapsed,
            ReadStage::GlobalDispatch => global_wait = elapsed,
            ReadStage::LcdRead => active = elapsed,
        }
        let ms = |duration: std::time::Duration| duration.as_millis().min(60_000) as u64;
        let _entered = self.span.enter();
        tracing::warn!(parent: &self.span, phase=self.slot.phase(),slot=self.slot.name(),reason,
            stage=self.stage.name(),poll_elapsed_ms=ms(now.saturating_duration_since(self.poll_started)),
            denom_wait_ms=ms(denom_wait),global_wait_ms=ms(global_wait),active_ms=ms(active),lcd_polled,
            "Native wrapper read unavailable");
    }
}
impl Drop for ReadWitness {
    fn drop(&mut self) {
        let reason = if tokio::time::Instant::now() >= self.deadline {
            "deadline"
        } else {
            "cancelled"
        };
        self.fail(reason);
    }
}
/// Poll-local dispatch starts each unchanged LCD budget only when admitted.
/// A denom acquires its one-read slot BEFORE global admission, so it cannot
/// monopolize both global slots or hold one while waiting for itself.
#[derive(Clone, Copy)]
struct ObservationReads<'a> {
    lcd: &'a LcdClient,
    dispatch: &'a ObservationDispatch,
    poll_started: tokio::time::Instant,
    deadline: tokio::time::Instant,
}
impl ObservationReads<'_> {
    fn read<'a, T: 'a>(
        &'a self,
        slot: ReadSlot,
        read: impl std::future::Future<Output = Result<T, LcdError>> + 'a,
    ) -> impl std::future::Future<Output = Option<T>> + 'a {
        // Box before the returned async state can capture the transport future.
        let read = Box::pin(read);
        async move {
            let mut witness = ReadWitness::new(self, slot);
            let result = bounded_observation(None, self.deadline, async {
                let _denom = if let Some(index) = slot.denom() {
                    let permit = self.dispatch.denoms[index].acquire().await.ok()?;
                    witness.advance(ReadStage::GlobalDispatch);
                    Some(permit)
                } else {
                    None
                };
                if self.deadline <= tokio::time::Instant::now() {
                    return None;
                }
                let _global = self.dispatch.global.acquire().await.ok()?;
                if self.deadline <= tokio::time::Instant::now() {
                    return None;
                }
                witness.advance(ReadStage::LcdRead);
                match read.await {
                    Ok(value) => Some(value),
                    Err(error) => {
                        witness.fail(observation_reason(&error));
                        None
                    }
                }
            })
            .await;
            if result.is_some() {
                witness.done = true;
            }
            result
        }
    }
}

async fn observe_router_with_reads(
    reads: &ObservationReads<'_>,
    router: Option<&str>,
    mapper: &str,
) -> Option<bool> {
    let Some(router) = router else {
        return Some(false);
    };
    let config: Value = reads
        .read(
            ReadSlot::RouterConfig,
            reads
                .lcd
                .query_contract_bounded(router, &json!({"config":{}}), 8_192),
        )
        .await?;
    Some(config["wrap_mapper"].as_str() == Some(mapper))
}

/// Router admission, fresh identity and smart-query work retain the LCD client's
/// existing attempt/total limits. A shorter wrapper-only timeout could cancel a
/// healthy 2.5s provider attempt before it completed. The original publication
/// margin still cancels this owned future without delaying native input beyond
/// the whole poll's deadline.
async fn observe_router_bounded_with_reads(
    reads: &ObservationReads<'_>,
    router: Option<&str>,
    mapper: &str,
    deadline: tokio::time::Instant,
) -> Option<bool> {
    // Preserve configured absence as a synchronous authoritative negative.
    if router.is_none() {
        return Some(false);
    }
    // Timeout polls its inner future first, even for an elapsed deadline. Refuse
    // network work before constructing/polling LCD admission or identity reads.
    if deadline <= tokio::time::Instant::now() {
        tracing::warn!(
            phase = "router_config",
            reason = "deadline",
            "Native wrapper read unavailable"
        );
        return None;
    }
    match tokio::time::timeout_at(deadline, observe_router_with_reads(reads, router, mapper)).await
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

async fn observe_floor_protocol(
    reads: &ObservationReads<'_>,
    address: &str,
    slot: ReadSlot,
) -> Option<u8> {
    let response = reads
        .read(
            slot,
            reads
                .lcd
                .query_contract_bounded(address, &json!({"execution_capabilities":{}}), 1_024),
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

async fn bounded_observation<F, T>(
    phase: Option<&'static str>,
    deadline: tokio::time::Instant,
    observation: F,
) -> Option<T>
where
    F: std::future::Future<Output = Option<T>>,
{
    let deadline_log = || {
        if let Some(phase) = phase {
            tracing::warn!(
                phase,
                reason = "deadline",
                "Native wrapper read unavailable"
            );
        }
    };
    // timeout_at polls its inner future first: expired work must refuse before
    // LCD admission, identity validation or a provider attempt can start.
    if deadline <= tokio::time::Instant::now() {
        deadline_log();
        return None;
    }
    match tokio::time::timeout_at(deadline, observation).await {
        Ok(result) => result,
        Err(_) => {
            deadline_log();
            None
        }
    }
}

/// Independent scheduled reads share the same absolute subdeadline and the
/// caller's existing LCD pacing/permits. A slow unrelated denom must not erase
/// completed required fields. Dropping this future cancels all local work.
#[cfg(test)]
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
    collect_output_observations_until(deadline, tax, router, uluna, uusd).await
}

async fn collect_output_observations_until<T, R, L, U>(
    deadline: tokio::time::Instant,
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
    let (tax, router, uluna, uusd) = tokio::join!(
        bounded_observation(Some("tax_rate"), deadline, tax),
        // Router work has its own original-margin guard outside this phase.
        bounded_observation(None, deadline, router),
        bounded_observation(Some("uluna_tax_cap"), deadline, uluna),
        bounded_observation(Some("uusd_tax_cap"), deadline, uusd),
    );
    OutputObservations {
        tax_atomics: tax,
        router_supported: router,
        uluna_cap: uluna,
        uusd_cap: uusd,
    }
}

#[cfg(test)]
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
    collect_floor_protocols_until(deadline, mapper, treasury).await
}

async fn collect_floor_protocols_until<M, T>(
    deadline: tokio::time::Instant,
    mapper: M,
    treasury: T,
) -> (Option<u8>, Option<u8>)
where
    M: std::future::Future<Output = Option<u8>>,
    T: std::future::Future<Output = Option<u8>>,
{
    tokio::join!(
        bounded_observation(Some("mapper_floor"), deadline, mapper),
        bounded_observation(Some("treasury_floor"), deadline, treasury)
    )
}

async fn observe_tax_cap_with_reads(
    reads: &ObservationReads<'_>,
    denom: &str,
    bound: bool,
) -> Option<u128> {
    if !bound {
        return None;
    }
    let slot = match denom {
        "uluna" => ReadSlot::UlunaCap,
        "uusd" => ReadSlot::UusdCap,
        _ => return None,
    };
    let value: Value = reads
        .read(
            slot,
            reads
                .lcd
                .get_bounded(&format!("/terra/treasury/v1beta1/tax_caps/{denom}"), 8_192),
        )
        .await?;
    let cap = raw(&value["tax_cap"]);
    if cap.is_none() {
        tracing::warn!(
            phase = "tax_cap",
            reason = "malformed",
            "Native wrapper read unavailable"
        );
    }
    cap
}

fn observed_tax_rate(value: &Value) -> Option<u128> {
    let parsed = value["params"]["burn_tax_rate"]
        .as_str()
        .or_else(|| value["burn_tax_rate"].as_str())
        .and_then(tax_rate);
    if parsed.is_none() {
        tracing::warn!(
            phase = "tax_rate",
            reason = "malformed",
            "Native wrapper read unavailable"
        );
    }
    parsed
}

fn has_denom_binding(bindings: &[Value], mapper: &str, denom: &str) -> bool {
    bindings
        .iter()
        .any(|v| v["denom"].as_str() == Some(denom) && v["wrapper"].as_str() == Some(mapper))
}

async fn observe_mapping(
    reads: &ObservationReads<'_>,
    mapper: &str,
    bindings: &[Value],
    denom: &str,
) -> Option<NativeMapping> {
    if !has_denom_binding(bindings, mapper, denom) {
        return None;
    }
    let (mapping_slot, rate_slot) = match denom {
        "uluna" => (ReadSlot::UlunaMapping, ReadSlot::UlunaRate),
        "uusd" => (ReadSlot::UusdMapping, ReadSlot::UusdRate),
        _ => return None,
    };
    let mapping_query = json!({"denom_mapping":{"denom":denom}});
    let rate_query = json!({"rate_limit":{"denom":denom}});
    let (response, rate) = tokio::join!(
        reads.read(
            mapping_slot,
            reads
                .lcd
                .query_contract_bounded::<Value>(mapper, &mapping_query, 8_192)
        ),
        reads.read(
            rate_slot,
            reads
                .lcd
                .query_contract_bounded::<Value>(mapper, &rate_query, 8_192)
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
pub async fn observe(state: &AppState) -> WrapperObservation {
    let Some(mapper) = state.wrap_mapper_address.as_deref() else {
        return WrapperObservation::Unavailable;
    };
    observe_config(&state.lcd, mapper, state.router_address.as_deref()).await
}

async fn observe_config(lcd: &LcdClient, mapper: &str, router: Option<&str>) -> WrapperObservation {
    static NEXT_POLL: AtomicU64 = AtomicU64::new(1);
    let poll_id = NEXT_POLL.fetch_add(1, Ordering::Relaxed);
    let phase = AtomicU8::new(0);
    async {
        let poll_started = tokio::time::Instant::now();
        let started = std::time::Instant::now();
        let observed_at_ms = Utc::now().timestamp_millis();
        let dispatch = ObservationDispatch::new(2);
        let poll_deadline = poll_started + std::time::Duration::from_millis(7_900);
        let deadline = poll_deadline.min(lcd.observation_deadline().unwrap_or(poll_deadline));
        // Scope before constructing any read future: dispatching an already
        // captured LCD future cannot retroactively tighten its caller clock.
        let lcd = lcd.with_observation_deadline(deadline);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started,
            deadline,
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(8),
            observe_inner(&reads, mapper, router, &phase, observed_at_ms),
        )
        .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(WrapperObservation::Available(snapshot)) => {
                tracing::info!(elapsed_ms, observed_at_ms,
                    mappings = snapshot.mappings.len(),
                    output_tax = snapshot.tax_atomics.is_some(),
                    router = ?snapshot.router_unwrap_supported,
                    mapper_floor = snapshot.mapper_floor_version.is_some(),
                    treasury_floor = snapshot.treasury_floor_version.is_some(),
                    uluna_cap = snapshot.mappings.get("uluna").is_some_and(|m| m.tax_cap.is_some()),
                    uusd_cap = snapshot.mappings.get("uusd").is_some_and(|m| m.tax_cap.is_some()),
                    "Native wrapper observations completed");
                WrapperObservation::Available(snapshot)
            }
            Ok(WrapperObservation::Revoked(reason)) => {
                tracing::warn!(
                    reason = reason.as_str(),
                    elapsed_ms,
                    observed_at_ms,
                    "Native wrapper observations revoked"
                );
                WrapperObservation::Revoked(reason)
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
                WrapperObservation::Unavailable
            }
        }
    }
    .instrument(tracing::info_span!("native_wrapper_poll", poll_id))
    .await
}
async fn observe_inner(
    reads: &ObservationReads<'_>,
    mapper: &str,
    router: Option<&str>,
    phase: &AtomicU8,
    observed_at_ms: i64,
) -> WrapperObservation {
    if reads
        .read(ReadSlot::Chain, reads.lcd.require_verified_chain())
        .await
        .is_none()
    {
        return WrapperObservation::Unavailable;
    }
    phase.store(1, Ordering::Relaxed);
    let Some(config) = reads
        .read(
            ReadSlot::MapperConfig,
            reads
                .lcd
                .query_contract_bounded::<Value>(mapper, &json!({"config":{}}), 8_192),
        )
        .await
    else {
        return WrapperObservation::Unavailable;
    };
    match config["paused"].as_bool() {
        Some(true) => return WrapperObservation::Revoked(WrapperRevocation::MapperPaused),
        Some(false) => {}
        None => return WrapperObservation::Unavailable,
    }
    let Some(treasury) = config["treasury"]
        .as_str()
        .filter(|value| value.starts_with("terra1") && value.len() <= 128)
        .map(str::to_owned)
    else {
        return WrapperObservation::Unavailable;
    };
    let Some(fees) = mapper_fees(&config) else {
        return WrapperObservation::Unavailable;
    };
    let deadline = reads.deadline;
    // The router remains independent of every subsequent settlement read.
    let router_read = observe_router_bounded_with_reads(reads, router, mapper, deadline);
    let settlement_read = observe_settlement(
        reads,
        mapper,
        treasury,
        fees,
        observed_at_ms,
        deadline,
        phase,
    );
    tokio::pin!(router_read, settlement_read);
    let (router_supported, outcome) = tokio::select! {
        outcome = &mut settlement_read => {
            // A validated treasury pause cannot wait behind an unrelated router
            // read. Dropping that owned future releases its existing admission.
            if !matches!(outcome, WrapperObservation::Available(_)) {
                return outcome;
            }
            (router_read.await, outcome)
        }
        router_supported = &mut router_read => (router_supported, settlement_read.await),
    };
    match outcome {
        WrapperObservation::Available(mut snapshot) => {
            snapshot.router_unwrap_supported = router_supported;
            WrapperObservation::Available(snapshot)
        }
        other => other,
    }
}

async fn observe_settlement(
    reads: &ObservationReads<'_>,
    mapper: &str,
    treasury: String,
    fees: (u16, u16),
    observed_at_ms: i64,
    deadline: tokio::time::Instant,
    phase: &AtomicU8,
) -> WrapperObservation {
    phase.store(2, Ordering::Relaxed);
    let config_query = json!({"config":{}});
    let bindings_query = json!({"denom_wrappers":{}});
    let treasury_ready = async {
        let config = reads
            .read(
                ReadSlot::TreasuryConfig,
                reads
                    .lcd
                    .query_contract_bounded::<Value>(&treasury, &config_query, 8_192),
            )
            .await
            .ok_or(WrapperObservation::Unavailable)?;
        match config["wrapping_paused"].as_bool() {
            Some(true) => Err(WrapperObservation::Revoked(
                WrapperRevocation::TreasuryWrappingPaused,
            )),
            Some(false) => Ok(()),
            None => Err(WrapperObservation::Unavailable),
        }
    };
    let bindings_ready = async {
        let bindings = reads
            .read(
                ReadSlot::DenomBindings,
                reads
                    .lcd
                    .query_contract_bounded::<Value>(&treasury, &bindings_query, 8_192),
            )
            .await;
        bindings
            .as_ref()
            .and_then(|value| value["wrappers"].as_array())
            .filter(|bindings| bindings.len() <= 64)
            .cloned()
            .ok_or(WrapperObservation::Unavailable)
    };
    // A validated pause cancels unrelated bindings immediately; only successful
    // current config and bindings may authorize any subsequent settlement read.
    let bindings = match tokio::try_join!(treasury_ready, bindings_ready) {
        Ok(((), bindings)) => bindings,
        Err(outcome) => return outcome,
    };
    // Validated treasury bindings permit scheduled cap reads before mapper/rate
    // reads finish. A cap is attached only to a validated same-poll mapping.
    let uluna_bound = has_denom_binding(&bindings, mapper, "uluna");
    let uusd_bound = has_denom_binding(&bindings, mapper, "uusd");
    phase.store(3, Ordering::Relaxed);
    let mappings_read = async {
        let (uluna, uusd) = tokio::join!(
            bounded_observation(
                Some("uluna_mapping"),
                deadline,
                observe_mapping(reads, mapper, &bindings, "uluna")
            ),
            bounded_observation(
                Some("uusd_mapping"),
                deadline,
                observe_mapping(reads, mapper, &bindings, "uusd")
            ),
        );
        let mappings: HashMap<String, NativeMapping> = [("uluna", uluna), ("uusd", uusd)]
            .into_iter()
            .filter_map(|(denom, mapping)| mapping.map(|mapping| (denom.into(), mapping)))
            .collect();
        mappings
    };
    let output = async {
        phase.store(4, Ordering::Relaxed);
        collect_output_observations_until(
            deadline,
            async {
                let tax: Option<Value> = reads
                    .read(
                        ReadSlot::TaxRate,
                        reads.lcd.get_bounded("/terra/tax/v1beta1/params", 8_192),
                    )
                    .await;
                tax.as_ref().and_then(observed_tax_rate)
            },
            async { None },
            observe_tax_cap_with_reads(reads, "uluna", uluna_bound),
            observe_tax_cap_with_reads(reads, "uusd", uusd_bound),
        )
        .await
    };
    let (mut mappings, output) = tokio::join!(mappings_read, output);
    // Start the optional <=2s phase after required mapping/bank reads finish;
    // its own FIFO wait must not consume that capability window. Publication
    // still uses the original absolute margin, with no late read admission.
    phase.store(5, Ordering::Relaxed);
    let floor_deadline =
        deadline.min(tokio::time::Instant::now() + std::time::Duration::from_secs(2));
    let floor_lcd = reads.lcd.with_observation_deadline(floor_deadline);
    let floor_reads = ObservationReads {
        lcd: &floor_lcd,
        deadline: floor_deadline,
        ..*reads
    };
    let floors = collect_floor_protocols_until(
        floor_deadline,
        observe_floor_protocol(&floor_reads, mapper, ReadSlot::MapperFloor),
        observe_floor_protocol(&floor_reads, &treasury, ReadSlot::TreasuryFloor),
    )
    .await;
    for (denom, cap) in [("uluna", output.uluna_cap), ("uusd", output.uusd_cap)] {
        if let Some(mapping) = mappings.get_mut(denom) {
            mapping.tax_cap = cap;
        }
    }
    WrapperObservation::Available(WrapperSnapshot {
        mapper: mapper.into(),
        treasury,
        observed_at_ms,
        mappings,
        fee_wrap_bps: fees.0,
        fee_unwrap_bps: fees.1,
        tax_atomics: output.tax_atomics,
        router_unwrap_supported: output.router_supported,
        mapper_floor_version: floors.0,
        treasury_floor_version: floors.1,
    })
}

#[cfg(test)]
async fn observe_router(lcd: &LcdClient, router: Option<&str>, mapper: &str) -> Option<bool> {
    observe_router_bounded(
        lcd,
        router,
        mapper,
        tokio::time::Instant::now() + std::time::Duration::from_millis(7_900),
    )
    .await
}
#[cfg(test)]
async fn observe_router_bounded(
    lcd: &LcdClient,
    router: Option<&str>,
    mapper: &str,
    deadline: tokio::time::Instant,
) -> Option<bool> {
    let dispatch = ObservationDispatch::new(2);
    let lcd = lcd.with_observation_deadline(deadline);
    let deadline = lcd.observation_deadline().unwrap_or(deadline);
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: tokio::time::Instant::now(),
        deadline,
    };
    observe_router_bounded_with_reads(&reads, router, mapper, deadline).await
}
#[cfg(test)]
async fn observe_tax_cap(lcd: &LcdClient, denom: &str, bound: bool) -> Option<u128> {
    let dispatch = ObservationDispatch::new(2);
    let reads = ObservationReads {
        lcd,
        dispatch: &dispatch,
        poll_started: tokio::time::Instant::now(),
        deadline: tokio::time::Instant::now() + std::time::Duration::from_millis(7_900),
    };
    observe_tax_cap_with_reads(&reads, denom, bound).await
}

#[cfg(test)]
mod tests {
    use super::*;
    impl WrapperObservation {
        fn expect_available(self, message: &str) -> WrapperSnapshot {
            match self {
                Self::Available(snapshot) => snapshot,
                other => panic!("{message}: {other:?}"),
            }
        }
    }
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
        wrapper_fixture_with_smart_delay(router_delay, bank_delay, Duration::from_millis(800)).await
    }

    async fn wrapper_fixture_with_smart_delay(
        router_delay: Duration,
        bank_delay: Duration,
        smart_delay: Duration,
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
                    smart_delay
                };
                ResponseTemplate::new(200)
                    .set_body_json(body)
                    .set_delay(delay)
            })
            .with_priority(10)
            .mount(&server)
            .await;
        // Scoped fixture permits a slow healthy provider response; runtime
        // attempt/total bounds and the original publication margin stay fixed.
        let policy = crate::lcd::ProviderPolicy {
            attempt_timeout_ms: 2_500,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(vec![server.uri()], 2_500, 1, "columbus-5", policy)
            .with_ingestion_budget();
        (server, lcd)
    }

    #[test]
    fn issue1505_parent_deadline_reason_matches_only_internal_total_expiry() {
        assert_eq!(
            observation_reason(&LcdError::AllEndpointsFailed(
                "total deadline exceeded".into()
            )),
            "deadline"
        );
        for message in [
            "prefix total deadline exceeded",
            "total deadline exceeded suffix",
            "total deadline exceeded\nupstream detail",
        ] {
            assert_eq!(
                observation_reason(&LcdError::AllEndpointsFailed(message.into())),
                "provider_unavailable"
            );
        }
        assert_eq!(
            observation_reason(&LcdError::AllEndpointsFailed(
                "all provider circuits open".into()
            )),
            "circuit_open"
        );
        assert_eq!(
            observation_reason(&LcdError::ChainIdentity("invalid identity".into())),
            "chain_identity"
        );
        assert_eq!(
            observation_reason(&LcdError::HistoricalHeightUnavailable),
            "historical_height"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_parent_deadline_dispatch_read_cannot_resume_data_after_parent() {
        use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(Duration::from_millis(50)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            2500,
            1,
            "columbus-5",
            crate::lcd::ProviderPolicy {
                attempt_timeout_ms: 2500,
                ..Default::default()
            },
        )
        .with_scheduled_observation();
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_millis(150);
        let lcd = lcd.with_observation_deadline(deadline);
        let dispatch = ObservationDispatch::new(2);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: started,
            deadline,
        };
        let query = json!({"config":{}});
        let mut work = Box::pin(reads.read(
            ReadSlot::MapperConfig,
            lcd.query_contract_bounded::<Value>("terra1mapper", &query, 8192),
        ));
        let received = async {
            while server.received_requests().await.unwrap().is_empty() {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        };
        tokio::select! {
            value = &mut work => panic!("identity must start before suspension: {value:?}"),
            () = received => {},
        }
        // Keep the owned read unpolled while the identity body becomes ready.
        // The LCD's own 2.5/4 s clocks still have time; only the parent expired.
        tokio::time::sleep_until(deadline + Duration::from_millis(50)).await;
        let result = work.await;
        assert!(result.is_none());
        assert_eq!(
            lcd.data_request_poll_count(),
            0,
            "the dispatch deadline must reach the LCD future captured before polling"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].caller_cancellations, 1);
        assert_eq!(counters[0].in_flight, 0);
        logs_assert(|lines: &[&str]| {
            let events: Vec<_> = lines
                .iter()
                .filter(|line| line.contains("Native wrapper read unavailable"))
                .collect();
            assert_eq!(
                events.len(),
                1,
                "one physical read failure, no aggregate or success stream"
            );
            let event = events[0];
            for field in [
                "slot=\"mapper_config\"",
                "reason=\"deadline\"",
                "stage=\"lcd_read\"",
                "lcd_polled=true",
            ] {
                assert!(event.contains(field), "missing fixed witness field {field}");
            }
            let elapsed = event
                .split("poll_elapsed_ms=")
                .nth(1)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            assert!(
                (200..=60_000).contains(&elapsed),
                "witness retains the original poll clock through suspended parent expiry"
            );
            assert!(!event.contains("http") && !event.contains("terra1mapper"));
            Ok(())
        });
    }

    #[tokio::test]
    async fn issue1505_parent_deadline_full_floor_phase_cannot_resume_data_after_two_seconds() {
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        let (server, lcd) =
            wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
        let identity_calls = Arc::new(AtomicUsize::new(0));
        let observed = identity_calls.clone();
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(move |_: &wiremock::Request| {
                let ordinal = observed.fetch_add(1, Ordering::SeqCst) + 1;
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(if ordinal >= 13 {
                        Duration::from_millis(50)
                    } else {
                        Duration::ZERO
                    })
            })
            .with_priority(1)
            .mount(&server)
            .await;
        let mut work = Box::pin(observe_config(&lcd, "terra1mapper", Some("terra1router")));
        let floor_identity = async {
            while identity_calls.load(Ordering::SeqCst) < 13 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        };
        tokio::select! {
            value = &mut work => panic!("all critical reads precede the optional phase: {value:?}"),
            () = floor_identity => {},
        }
        assert_eq!(
            lcd.data_request_poll_count(),
            11,
            "unchanged critical read count"
        );
        let floor_identity_started = tokio::time::Instant::now();
        // The first floor identity starts at the beginning of its phase. The
        // unchanged 250 ms pacing may leave the other floor queued; neither may
        // first-poll data after this two-second phase expires.
        tokio::time::sleep_until(floor_identity_started + Duration::from_millis(2050)).await;
        let snapshot = work
            .await
            .expect_available("optional expiry preserves critical fields");
        assert_eq!(
            lcd.data_request_poll_count(),
            11,
            "optional floor futures must inherit the phase deadline before construction"
        );
        assert!(snapshot.tax_atomics.is_some());
        assert_eq!(snapshot.router_unwrap_supported, Some(true));
        assert_eq!(snapshot.mappings.len(), 2);
        assert_eq!(snapshot.direct_floor_protocol(false), None);
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].in_flight, 0);
    }

    async fn override_smart_read(
        server: &wiremock::MockServer,
        address: &str,
        query: Value,
        response: Value,
        delay: Duration,
    ) {
        use base64::Engine;
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        let path_value = format!(
            "/cosmwasm/wasm/v1/contract/{address}/smart/{}",
            base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&query).unwrap())
        );
        Mock::given(path(path_value))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data": response}))
                    .set_delay(delay),
            )
            .with_priority(1)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn issue1505_observed_read_boxes_transport_before_capturing_async_state() {
        let (_server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        let dispatch = ObservationDispatch::new(2);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: tokio::time::Instant::now(),
            deadline: tokio::time::Instant::now() + Duration::from_secs(1),
        };
        let read = async {
            let payload = [7u8; 128 * 1024];
            std::hint::black_box(&payload);
            std::future::pending::<()>().await;
            Ok::<_, LcdError>(std::hint::black_box(&payload).len())
        };
        assert!(std::mem::size_of_val(&read) >= 128 * 1024);
        let observation = reads.read(ReadSlot::TaxRate, read);
        assert!(
            std::mem::size_of_val(&observation) < 1024,
            "concurrent phases must capture a pointer, not each inline transport future"
        );
        // Constructing/dropping the owned future does not poll or spawn work.
        drop(observation);
    }

    #[tokio::test(start_paused = true)]
    async fn issue1505_dispatch_expired_phase_never_polls_transport() {
        let (_server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        let dispatch = ObservationDispatch::new(0);
        let polls = AtomicUsize::new(0);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: tokio::time::Instant::now(),
            deadline: tokio::time::Instant::now() + Duration::from_secs(2),
        };
        let read = reads.read(ReadSlot::MapperFloor, async {
            polls.fetch_add(1, Ordering::SeqCst);
            Ok::<_, LcdError>(1)
        });
        let release = async {
            tokio::time::sleep(Duration::from_secs(2)).await;
            dispatch.global.add_permits(1);
        };
        let (result, ()) = tokio::join!(read, release);
        assert_eq!(result, None);
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        assert_eq!(dispatch.global.available_permits(), 1);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_successful_malformed_tax_fields_have_fixed_reasons() {
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        Mock::given(path("/terra/treasury/v1beta1/tax_caps/uusd"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"tax_cap": false})))
            .with_priority(1)
            .mount(&server)
            .await;
        assert_eq!(observe_tax_cap(&lcd, "uusd", true).await, None);
        assert_eq!(
            observed_tax_rate(&json!({"params":{"burn_tax_rate":false}})),
            None
        );
        assert!(logs_contain("tax_cap"));
        assert!(logs_contain("tax_rate"));
        assert!(logs_contain("malformed"));
        assert_eq!(lcd.provider_counters().await[0].failures, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn issue1505_denom_dispatch_is_symmetric_and_releases_cancelled_reads() {
        let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        for (slow, fast) in [
            (
                [ReadSlot::UlunaMapping, ReadSlot::UlunaRate],
                [ReadSlot::UusdMapping, ReadSlot::UusdRate, ReadSlot::UusdCap],
            ),
            (
                [ReadSlot::UusdMapping, ReadSlot::UusdRate],
                [
                    ReadSlot::UlunaMapping,
                    ReadSlot::UlunaRate,
                    ReadSlot::UlunaCap,
                ],
            ),
        ] {
            let dispatch = ObservationDispatch::new(2);
            let started = tokio::time::Instant::now();
            let reads = ObservationReads {
                lcd: &lcd,
                dispatch: &dispatch,
                poll_started: started,
                deadline: started + Duration::from_secs(1),
            };
            let slow_polls = AtomicUsize::new(0);
            let slow_read = || async {
                slow_polls.fetch_add(1, Ordering::SeqCst);
                std::future::pending::<Result<u8, LcdError>>().await
            };
            let fast_read = || async {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok::<_, LcdError>(1u8)
            };
            let fast_reads = async {
                let result = tokio::join!(
                    reads.read(fast[0], fast_read()),
                    reads.read(fast[1], fast_read()),
                    reads.read(fast[2], fast_read()),
                );
                assert_eq!(started.elapsed(), Duration::from_millis(60));
                result
            };
            let (first, second, fast_result) = tokio::join!(
                reads.read(slow[0], slow_read()),
                reads.read(slow[1], slow_read()),
                fast_reads,
            );
            assert_eq!((first, second), (None, None));
            assert_eq!(fast_result, (Some(1), Some(1), Some(1)));
            assert_eq!(slow_polls.load(Ordering::SeqCst), 1);
            assert_eq!(started.elapsed(), Duration::from_secs(1));
            assert_eq!(dispatch.global.available_permits(), 2);
            for denom in &dispatch.denoms {
                assert_eq!(denom.available_permits(), 1);
            }
        }
        assert!(server.received_requests().await.unwrap().is_empty());
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
    }

    #[tokio::test(start_paused = true)]
    #[tracing_test::traced_test]
    async fn issue1505_failed_read_witness_distinguishes_dispatch_from_polled_lcd() {
        use std::{future::Future, task::Poll};
        use tracing::Instrument;
        let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        for stage in 0..3 {
            let dispatch = ObservationDispatch::new(if stage == 1 { 0 } else { 2 });
            let held_denom = if stage == 0 {
                Some(dispatch.denoms[0].acquire().await.unwrap())
            } else {
                None
            };
            let started = tokio::time::Instant::now();
            let reads = ObservationReads {
                lcd: &lcd,
                dispatch: &dispatch,
                poll_started: started,
                deadline: started + Duration::from_secs(1),
            };
            let polls = AtomicUsize::new(0);
            let mut read = Box::pin(
                reads
                    .read(ReadSlot::UlunaMapping, async {
                        polls.fetch_add(1, Ordering::SeqCst);
                        std::future::pending::<Result<u8, LcdError>>().await
                    })
                    .instrument(tracing::info_span!("native_wrapper_poll", poll_id = 6060)),
            );
            std::future::poll_fn(|cx| {
                assert!(read.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            // Drop outside the instrumented span: the witness must preserve its
            // poll identity, including cancellation before the absolute margin.
            tokio::time::advance(Duration::from_millis(if stage == 2 { 1000 } else { 10 })).await;
            drop(read);
            assert_eq!(polls.load(Ordering::SeqCst), usize::from(stage == 2));
            drop(held_denom);
            assert_eq!(dispatch.denoms[0].available_permits(), 1);
            assert_eq!(
                dispatch.global.available_permits(),
                if stage == 1 { 0 } else { 2 }
            );
        }
        let dispatch = ObservationDispatch::new(2);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: tokio::time::Instant::now(),
            deadline: tokio::time::Instant::now() + Duration::from_secs(1),
        };
        assert_eq!(
            reads
                .read(ReadSlot::TaxRate, async {
                    Err::<u8, _>(LcdError::AllEndpointsFailed(
                        "private_endpoint?actor=private_actor upstream_body".into(),
                    ))
                })
                .await,
            None
        );
        assert_eq!(
            reads
                .read(ReadSlot::TaxRate, async { Ok::<_, LcdError>(7) })
                .await,
            Some(7)
        );
        logs_assert(|lines: &[&str]| {
            let events: Vec<_> = lines
                .iter()
                .filter(|line| line.contains("Native wrapper read unavailable"))
                .collect();
            assert_eq!(
                events.len(),
                4,
                "one event per failed read; successes stay silent"
            );
            for (index, expected_stage) in ["denom_dispatch", "global_dispatch", "lcd_read"]
                .iter()
                .enumerate()
            {
                let event = events[index];
                assert!(event.contains(expected_stage));
                assert!(event.contains("poll_id=6060"));
                assert!(event.contains("uluna_mapping"));
                assert!(event.contains(if index == 2 {
                    "lcd_polled=true"
                } else {
                    "lcd_polled=false"
                }));
                assert!(event.contains(if index == 2 { "deadline" } else { "cancelled" }));
                assert!(event.contains(if index == 2 {
                    "active_ms=1000"
                } else if index == 1 {
                    "global_wait_ms=10"
                } else {
                    "denom_wait_ms=10"
                }));
            }
            assert!(events[3].contains("provider_unavailable"));
            assert!(!events.iter().any(|line| line.contains("private_endpoint")
                || line.contains("private_actor")
                || line.contains("upstream_body")));
            Ok(())
        });
        assert!(server.received_requests().await.unwrap().is_empty());
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
    }

    #[tokio::test]
    async fn issue1505_slow_native_denom_cannot_starve_other_native_output() {
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        // Every provider response remains inside its permitted 2.5s attempt.
        // A slow, valid prerequisite leaves a finite original publication budget.
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"config":{}}),
            json!({"paused":false,"treasury":"terra1treasury","fee_bps":100}),
            Duration::from_millis(2200),
        )
        .await;
        override_smart_read(
            &server,
            "terra1treasury",
            json!({"config":{}}),
            json!({"wrapping_paused":false}),
            Duration::from_millis(2400),
        )
        .await;
        // Both uluna reads enter the shared FIFO before the independent uusd
        // observation. Neither may occupy the other denom's admission slot.
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uluna"}}),
            json!({"denom":"uluna","cw20_addr":"terra1wrapped"}),
            Duration::from_millis(2200),
        )
        .await;
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"rate_limit":{"denom":"uluna"}}),
            json!({"config":null,"amount_used":"0"}),
            Duration::from_millis(2200),
        )
        .await;
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uusd"}}),
            json!({"denom":"uusd","cw20_addr":"terra1wrapped"}),
            Duration::from_millis(250),
        )
        .await;
        Mock::given(path("/terra/treasury/v1beta1/tax_caps/uluna"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"tax_cap":"1000"}))
                    .set_delay(Duration::from_millis(800)),
            )
            .with_priority(1)
            .mount(&server)
            .await;
        let before = Utc::now().timestamp_millis();
        let started = std::time::Instant::now();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect_available("valid common prerequisites retain a same-poll native view");
        let reads = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
            .count();
        let counters = lcd.provider_counters().await;
        println!("denom_isolation_control elapsed_ms={} mappings={} uluna_mapping={} uusd_mapping={} tax={} router={:?} uluna_cap={} uusd_cap={} data_reads={} provider_failures={} caller_cancellations={}",
            started.elapsed().as_millis(),snapshot.mappings.len(),snapshot.mappings.contains_key("uluna"),
            snapshot.mappings.contains_key("uusd"),snapshot.tax_atomics.is_some(),snapshot.router_unwrap_supported,
            snapshot.mappings.get("uluna").is_some_and(|mapping|mapping.tax_cap.is_some()),
            snapshot.mappings.get("uusd").is_some_and(|mapping|mapping.tax_cap.is_some()),
            reads,counters[0].failures,counters[0].caller_cancellations);
        assert!(snapshot.observed_at_ms >= before && snapshot.observed_at_ms <= before + 100);
        assert!(started.elapsed() < Duration::from_secs(8));
        assert!(
            reads <= 13,
            "scheduling cannot add logical observation reads"
        );
        assert_eq!(counters[0].in_flight, 0);
        assert_eq!(
            counters[0].failures, 0,
            "permitted slow replies are not provider failures"
        );
        assert!(
            snapshot
                .unwrap_output("uusd", 10000, snapshot.observed_at_ms + 8000)
                .is_some(),
            "one denom's mapping/rate work cannot strand the independent USTC output prerequisites"
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_output_attempts_are_not_clipped_by_old_two_second_phase() {
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        for (path_value, response, slow_tax) in [
            (
                "/terra/tax/v1beta1/params",
                json!({"params":{"burn_tax_rate":"0.005"}}),
                true,
            ),
            (
                "/terra/treasury/v1beta1/tax_caps/uusd",
                json!({"tax_cap":"1000"}),
                false,
            ),
        ] {
            let (server, lcd) =
                wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
            Mock::given(path(path_value))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(response)
                        .set_delay(Duration::from_millis(2200)),
                )
                .with_priority(1)
                .mount(&server)
                .await;
            let read_tax = || async {
                let value: Value = lcd
                    .get_bounded("/terra/tax/v1beta1/params", 8192)
                    .await
                    .ok()?;
                tax_rate(value["params"]["burn_tax_rate"].as_str()?)
            };
            let old = collect_output_observations(
                Duration::from_secs(2),
                read_tax(),
                async { None },
                observe_tax_cap(&lcd, "uluna", true),
                observe_tax_cap(&lcd, "uusd", true),
            )
            .await;
            assert!(
                if slow_tax {
                    old.tax_atomics.is_none()
                } else {
                    old.uusd_cap.is_none()
                },
                "old2s clip must reproduce the unavailable healthy2.2s read"
            );
            let before_count = server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
                .count();
            let before = Utc::now().timestamp_millis();
            let start = std::time::Instant::now();
            let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
                .await
                .expect_available("same13reads and original7.9s margin must admit healthy output");
            let diagnostics = format!("slow_tax={slow_tax} elapsed_ms={} tax={:?} router={:?} uluna_cap={:?} uusd_cap={:?} mapper_floor={:?} treasury_floor={:?} counters={:?}",
                start.elapsed().as_millis(), snapshot.tax_atomics, snapshot.router_unwrap_supported,
                snapshot.mappings.get("uluna").and_then(|m| m.tax_cap), snapshot.mappings.get("uusd").and_then(|m| m.tax_cap),
                snapshot.mapper_floor_version, snapshot.treasury_floor_version, lcd.provider_counters().await);
            assert!(start.elapsed() < Duration::from_millis(7900));
            assert!(snapshot.observed_at_ms >= before && snapshot.observed_at_ms <= before + 100);
            assert!(snapshot.tax_atomics.is_some(), "{diagnostics}");
            assert_eq!(
                snapshot.direct_floor_protocol(true),
                Some(1),
                "{diagnostics}"
            );
            assert_eq!(
                snapshot.direct_floor_protocol(false),
                Some(1),
                "{diagnostics}"
            );
            for denom in ["uluna", "uusd"] {
                assert!(snapshot.wrap_input(denom, 10000, before + 8000).is_some());
                assert!(
                    snapshot
                        .unwrap_output(denom, 10000, before + 8000)
                        .is_some(),
                    "denom={denom} {diagnostics}"
                );
            }
            let after_count = server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
                .count();
            assert_eq!(after_count - before_count, 13);
            assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
            assert_eq!(lcd.provider_counters().await[0].failures, 0);
        }
    }

    #[tokio::test]
    async fn issue1505_slow_uluna_cap_preserves_uusd_and_same_poll_input() {
        use wiremock::{matchers::path, Mock, ResponseTemplate};
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        Mock::given(path("/terra/treasury/v1beta1/tax_caps/uluna"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"tax_cap":"1000"}))
                    .set_delay(Duration::from_secs(20)),
            )
            .with_priority(1)
            .mount(&server)
            .await;
        let before = Utc::now().timestamp_millis();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect_available("unrelated cap timeout cannot discard completed siblings");
        assert_eq!(snapshot.mappings["uluna"].tax_cap, None);
        assert_eq!(snapshot.mappings["uusd"].tax_cap, Some(1000));
        assert!(snapshot
            .unwrap_output("uusd", 10000, before + 8000)
            .is_some());
        assert!(snapshot
            .unwrap_output("uluna", 10000, before + 8000)
            .is_none());
        for denom in ["uluna", "uusd"] {
            assert!(snapshot.wrap_input(denom, 10000, before + 8000).is_some());
        }
        assert!(snapshot
            .unwrap_output("uusd", 10000, snapshot.observed_at_ms + 20001)
            .is_none());
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
    }

    #[tokio::test]
    async fn issue1505_binding_cap_never_attaches_to_an_invalid_mapping() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uusd"}}),
            json!({"denom":"uusd","cw20_addr":false}),
            Duration::from_millis(50),
        )
        .await;
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect_available("other validated mapping remains independent");
        assert!(!snapshot.mappings.contains_key("uusd"));
        assert!(snapshot
            .wrap_input("uusd", 10000, Utc::now().timestamp_millis())
            .is_none());
        assert!(snapshot
            .wrap_input("uluna", 10000, Utc::now().timestamp_millis())
            .is_some());
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path() == "/terra/treasury/v1beta1/tax_caps/uusd")
                .count(),
            1,
            "binding admits the bounded read, never the invalid mapping"
        );
    }

    #[tokio::test]
    async fn issue1505_mapper_pause_is_typed_without_fabricated_treasury_fields() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"config":{}}),
            json!({"paused":true}),
            Duration::from_millis(10),
        )
        .await;
        assert!(matches!(
            observe_config(&lcd, "terra1mapper", Some("terra1router")).await,
            WrapperObservation::Revoked(WrapperRevocation::MapperPaused)
        ));
        assert_eq!(
            server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .filter(|r| r.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn issue1505_malformed_pause_fields_are_unavailable_not_revocations() {
        for paused in [json!("true"), Value::Null] {
            let (server, lcd) =
                wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
            override_smart_read(
                &server,
                "terra1mapper",
                json!({"config":{}}),
                json!({"paused":paused,"treasury":"terra1treasury","fee_bps":100}),
                Duration::from_millis(10),
            )
            .await;
            assert!(matches!(
                observe_config(&lcd, "terra1mapper", Some("terra1router")).await,
                WrapperObservation::Unavailable
            ));
        }
    }

    #[tokio::test]
    async fn issue1505_treasury_pause_cancels_slow_bindings_and_router_immediately() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(2400), Duration::from_millis(50)).await;
        override_smart_read(
            &server,
            "terra1mapper",
            json!({"config":{}}),
            json!({"paused":false,"treasury":"terra1treasury","fee_bps":100}),
            Duration::from_millis(10),
        )
        .await;
        override_smart_read(
            &server,
            "terra1treasury",
            json!({"config":{}}),
            json!({"wrapping_paused":true}),
            Duration::from_millis(10),
        )
        .await;
        override_smart_read(
            &server,
            "terra1treasury",
            json!({"denom_wrappers":{}}),
            json!({"wrappers":[]}),
            Duration::from_millis(2300),
        )
        .await;
        let start = std::time::Instant::now();
        assert!(matches!(
            observe_config(&lcd, "terra1mapper", Some("terra1router")).await,
            WrapperObservation::Revoked(WrapperRevocation::TreasuryWrappingPaused)
        ));
        assert!(
            start.elapsed() < Duration::from_millis(1500),
            "known pause must not join-wait2.3/2.4s siblings"
        );
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
        assert_eq!(lcd.provider_counters().await[0].failures, 0);
    }

    #[tokio::test(start_paused = true)]
    #[tracing_test::traced_test]
    async fn issue1505_output_publication_deadline_has_fixed_phase_evidence() {
        let result = collect_output_observations_until(
            tokio::time::Instant::now() + Duration::from_millis(100),
            delayed(2200, Some(0u128)),
            async { None },
            delayed(10, Some(1000)),
            delayed(10, Some(1000)),
        )
        .await;
        assert_eq!(result.tax_atomics, None);
        assert_eq!(result.uusd_cap, Some(1000));
        assert!(logs_contain("tax_rate"));
        assert!(logs_contain("deadline"));
    }

    #[tokio::test]
    async fn issue1505_expired_router_margin_starts_no_provider_attempt() {
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        // Warm the actual shared production lane, then let its existing pacing
        // turn elapse. A fresh timer's first yield can mask an eager inner poll.
        assert_eq!(observe_tax_cap(&lcd, "uluna", true).await, Some(1000));
        tokio::time::sleep(Duration::from_millis(300)).await;
        let before = lcd.provider_counters().await;
        let wire_before = server.received_requests().await.unwrap().len();
        let deadline = tokio::time::Instant::now() - Duration::from_millis(1);
        assert_eq!(
            observe_router_bounded(&lcd, None, "terra1mapper", deadline).await,
            Some(false),
            "configured absence remains an authoritative negative without I/O"
        );
        assert_eq!(
            observe_router_bounded(&lcd, Some("terra1router"), "terra1mapper", deadline).await,
            None
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(
            counters[0].attempts, before[0].attempts,
            "an expired margin must never start admission or identity work"
        );
        assert_eq!(
            counters[0].caller_cancellations,
            before[0].caller_cancellations
        );
        assert_eq!(counters[0].failures, before[0].failures);
        assert_eq!(counters[0].in_flight, 0);
        assert_eq!(server.received_requests().await.unwrap().len(), wire_before);
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
            .expect_available("early router must complete within unchanged publication deadline");
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
    async fn issue1505_router_attempt_fits_existing_policy_without_two_second_clipping() {
        let (_server, lcd) =
            wrapper_fixture(Duration::from_millis(2_200), Duration::from_millis(50)).await;
        let old = observe_router_bounded(
            &lcd,
            Some("terra1router"),
            "terra1mapper",
            tokio::time::Instant::now() + Duration::from_secs(2),
        )
        .await;
        assert_eq!(
            old, None,
            "old independent2s cap clips a valid2.2s provider response"
        );
        let before = Utc::now().timestamp_millis();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect_available("router and remaining work share the original bounded poll");
        assert_eq!(snapshot.router_unwrap_supported, Some(true));
        assert!(snapshot.observed_at_ms >= before && snapshot.observed_at_ms <= before + 100);
        assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
        for denom in ["uluna", "uusd"] {
            assert!(snapshot
                .wrap_input(denom, 10_000, Utc::now().timestamp_millis())
                .is_some());
            assert!(snapshot
                .unwrap_output(denom, 10_000, Utc::now().timestamp_millis())
                .is_some());
        }
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].caller_cancellations, 1);
        assert_eq!(counters[0].in_flight, 0);
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_router_publication_margin_cancels_owned_read_neutrally() {
        let (_server, lcd) =
            wrapper_fixture(Duration::from_millis(2_200), Duration::from_millis(50)).await;
        // Model the original margin left after earlier prerequisite work. The
        // caller deadline stays authoritative even though the provider is healthy.
        let router = observe_router_bounded(
            &lcd,
            Some("terra1router"),
            "terra1mapper",
            tokio::time::Instant::now() + Duration::from_millis(500),
        )
        .await;
        assert_eq!(router, None);
        assert!(logs_contain("router_config"));
        assert!(logs_contain("deadline"));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].caller_cancellations, 1);
        assert_eq!(counters[0].in_flight, 0);
    }

    #[tokio::test]
    async fn issue1505_unavailable_router_still_publishes_completed_native_input() {
        use wiremock::{
            matchers::{method, path_regex},
            Mock, ResponseTemplate,
        };
        let (server, lcd) =
            wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/cosmwasm/wasm/v1/contract/terra1router/smart/",
            ))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code":3,"message":"query wasm contract failed: unknown variant `config`"
            })))
            .with_priority(1)
            .mount(&server)
            .await;
        let before = Utc::now().timestamp_millis();
        let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
            .await
            .expect_available("router read rejection cannot erase independent native input");
        assert_eq!(snapshot.router_unwrap_supported, None);
        assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
        assert!(snapshot.observed_at_ms >= before && snapshot.observed_at_ms <= before + 100);
        for denom in ["uluna", "uusd"] {
            assert!(snapshot
                .wrap_input(denom, 10_000, Utc::now().timestamp_millis())
                .is_some());
            assert!(snapshot
                .unwrap_output(denom, 10_000, Utc::now().timestamp_millis())
                .is_none());
        }
        let count = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
            .count();
        assert_eq!(count, 13);
        let counters = lcd.provider_counters().await;
        assert_eq!(
            counters[0].failures, 0,
            "contract-local rejection is not provider failure"
        );
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
            .expect_available("ordinary provider latency must not erase native publication");
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
