mod chain_identity;
mod provider_pool;
pub use provider_pool::{ProviderCounters, ProviderPolicy};
pub mod pair_discovery_state;
pub mod types;
#[cfg(test)]
pub(crate) use chain_identity::mock_chain_identity;
pub use chain_identity::{valid_chain_id, ChainIdentityHealth, EndpointIdentity};

use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

use base64::Engine;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use pair_discovery_state::PairDiscoveryState;
pub use types::*;

/// Limit the decoded HTTP body before allocating/deserializing an untrusted snapshot.
/// Enforce the limit while streaming as Content-Length can be absent or dishonest.
async fn bounded_response_text(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<String, LcdError> {
    if response
        .content_length()
        .is_some_and(|n| n > max_bytes as u64)
    {
        return Err(LcdError::Deserialize(format!(
            "LCD response exceeds {max_bytes} byte limit"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > max_bytes.saturating_sub(body.len()) {
            return Err(LcdError::Deserialize(format!(
                "LCD response exceeds {max_bytes} byte limit"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| LcdError::Deserialize("LCD response is not UTF-8".into()))
}

/// Redact LCD path for WARN logs (GitLab #379 L-04).
fn lcd_log_path(path: &str) -> &'static str {
    if path.starts_with("/cosmwasm/") {
        "/cosmwasm/..."
    } else if path.starts_with("/cosmos/tx/") {
        "/cosmos/tx/..."
    } else if path.starts_with("/cosmos/base/tendermint/") {
        "/cosmos/base/tendermint/..."
    } else {
        "<lcd-path>"
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LcdError {
    #[error("All LCD endpoints failed: {0}")]
    AllEndpointsFailed(String),
    /// Shared provider admission must wait without successfully omitting a discovery event.
    #[error("LCD provider admission deferred")]
    AdmissionDeferred,
    #[error("Chain identity rejected: {0}")]
    ChainIdentity(String),
    /// Deterministic CosmWasm smart-query reject (e.g. factory `pair not found`,
    /// or foreign contract `unknown variant \`pair\``). Not an LCD outage — do not
    /// fan out to other endpoints or WARN.
    #[error("LCD contract query rejected: {0}")]
    ContractQueryRejected(String),
    /// A provider's finite smart-query gas ceiling was exhausted. Do not repeat
    /// the same query on another endpoint or mark the provider unhealthy.
    #[error("LCD smart-query gas limit exceeded: {0}")]
    QueryGasLimitExceeded(String),
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
    /// Query-local schema/size rejection; never opens a provider circuit or discovery negative cache.
    #[error("LCD pair response malformed: {0}")]
    PairResponseMalformed(String),
    #[error("LCD provider materially lagging")]
    LaggingProvider,
    #[error("LCD historical height unavailable")]
    HistoricalHeightUnavailable,
    /// A nonretryable HTTP protocol rejection, without response bodies or URLs.
    #[error("LCD HTTP protocol rejected ({0})")]
    HttpRejected(u16),
    #[error("Base64 decode error: {0}")]
    Base64(String),
}

/// Local scheduling evidence, separate from public error semantics. A deferred
/// scan made no provider attempt and owns no provider/lane permit while waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanDeferral {
    Capacity,
    Circuit,
}
struct ScanOutcome<T> {
    result: Result<T, LcdError>,
    attempts: usize,
    deferral: Option<ScanDeferral>,
}
impl<T> ScanOutcome<T> {
    fn completed(result: Result<T, LcdError>, attempts: usize) -> Self {
        Self {
            result,
            attempts,
            deferral: None,
        }
    }
    fn deferred(error: LcdError, deferral: ScanDeferral) -> Self {
        Self {
            result: Err(error),
            attempts: 0,
            deferral: Some(deferral),
        }
    }
}

/// Preserve already observed typed integrity evidence before async metadata
/// cleanup. Transport errors deliberately remain availability, not integrity.
fn record_integrity_evidence(
    slot: &std::sync::Mutex<Option<LcdError>>,
    error: &LcdError,
    observer: Option<&(dyn Fn() + Send + Sync)>,
) {
    let copied = match error {
        LcdError::ChainIdentity(message) => LcdError::ChainIdentity(message.clone()),
        LcdError::HistoricalHeightUnavailable => LcdError::HistoricalHeightUnavailable,
        LcdError::Deserialize(message) => LcdError::Deserialize(message.clone()),
        LcdError::Base64(message) => LcdError::Base64(message.clone()),
        LcdError::HttpRejected(status) => LcdError::HttpRejected(*status),
        LcdError::PairResponseMalformed(message) => {
            LcdError::PairResponseMalformed(message.clone())
        }
        _ => return,
    };
    // The owner must revoke retained qualifications before async provider cleanup.
    // Availability and query-local below-frontier successes never reach this hook.
    if let Some(observer) = observer {
        observer();
    }
    let mut retained = slot.lock().unwrap();
    if matches!(copied, LcdError::PairResponseMalformed(_))
        && matches!(
            *retained,
            Some(LcdError::ChainIdentity(_) | LcdError::HistoricalHeightUnavailable)
        )
    {
        return;
    }
    *retained = Some(copied);
}

/// CosmWasm LCD returns HTTP 500 for smart-query contract errors. Same answer on
/// every LCD — not an upstream outage (GitLab #493).
///
/// Classic factory/foreign-pair phrases always classify. Broader wasmd envelope
/// markers (`query wasm contract failed`, `codespace wasm`) apply only on
/// `/cosmwasm/wasm/v1/contract/...` so plain infra 500s and non-wasm REST paths
/// still fan out / WARN.
fn is_deterministic_contract_query_reject(
    status: reqwest::StatusCode,
    path: &str,
    body: &str,
) -> bool {
    if status.as_u16() == 404 && path.starts_with("/cosmwasm/wasm/v1/contract/") {
        // A verified Cosmos not-found envelope is pair-local. A proxy's arbitrary
        // 404 body cannot establish permanent contract absence.
        return serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .is_some_and(|value| {
                let message = value["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                value["code"].as_u64() == Some(5)
                    && message.contains("contract")
                    && (message.contains("not found") || message.contains("no such contract"))
            });
    }
    if status.as_u16() != 500 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    let classic = lower.contains("pair not found")
        || lower.contains("unknown variant")
        || lower.contains("error parsing into type");
    if classic {
        return true;
    }
    if !path.starts_with("/cosmwasm/wasm/v1/contract/") {
        return false;
    }
    lower.contains("query wasm contract failed") || lower.contains("codespace wasm")
}

fn should_cooldown_endpoint(status: reqwest::StatusCode) -> bool {
    let code = status.as_u16();
    // Rate limits and gateway failures — back off this endpoint and try others.
    code == 403 || code == 429 || code == 408 || code >= 500
}

fn is_query_gas_limit_exceeded(status: reqwest::StatusCode, path: &str, body: &str) -> bool {
    if status.as_u16() != 500
        || !path.starts_with("/cosmwasm/wasm/v1/contract/")
        || !path.contains("/smart/")
    {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("out of gas") && lower.contains("gaswanted") && lower.contains("gasused")
}

#[derive(Clone)]
struct Endpoint {
    url: String,
    provider: Arc<provider_pool::ProviderState>,
    observed_chain_id: Option<String>,
    verified: bool,
    identity_error: Option<String>,
    identity_checked_at: Option<String>,
}

#[derive(Clone)]
pub struct LcdClient {
    inner: Arc<LcdClientInner>,
    ingestion_budget: Option<Arc<IngestionBudget>>,
    scheduled_observation: bool,
    safety_observer: Option<Arc<dyn Fn() + Send + Sync>>,
    observation_deadline: Option<tokio::time::Instant>,
}
struct IngestionBudget {
    permits: Arc<tokio::sync::Semaphore>,
    next_request: tokio::sync::Mutex<tokio::time::Instant>,
}

struct LcdClientInner {
    endpoints: RwLock<Vec<Endpoint>>,
    client: Client,
    cooldown_duration: Duration,
    provider_policy: ProviderPolicy,
    next_endpoint: AtomicUsize,
    latest_height: AtomicI64,
    discovery_namespace: usize,
    expected_chain_id: String,
    pair_discovery_state: Arc<PairDiscoveryState>,
    #[cfg(test)]
    endpoint_metadata_waits: AtomicUsize,
    #[cfg(test)]
    data_request_polls: AtomicUsize,
}

const LATEST_BLOCK_PATH: &str = "/cosmos/base/tendermint/v1beta1/blocks/latest";

#[derive(Clone, Copy, Default)]
struct RequestConstraints<'a> {
    max_bytes: Option<usize>,
    historical_height: Option<i64>,
    minimum_latest_height: Option<i64>,
    safety_observer: Option<&'a (dyn Fn() + Send + Sync)>,
}

// A valid response can be healthy yet ineligible for this caller's pool frontier.
// Keep that distinction private: it neither quarantines providers nor resets
// this operation's attempt count through an outer availability retry.
enum EndpointOutcome<T> {
    Accepted(T),
    BelowMinimum,
    Expired(DeadlineOrigin),
}

#[derive(Clone, Copy)]
enum DeadlineOrigin {
    Caller,
    Attempt,
}

/// Both clocks remain absolute across identity metadata awaits. Expiry is an
/// internal outcome so caller cancellation keeps its existing health policy;
/// it must not become a completed error that disarms quarantine accounting.
struct EndpointAttempt<'a> {
    index: usize,
    endpoint: &'a Endpoint,
    caller_deadline: tokio::time::Instant,
    own_deadline: tokio::time::Instant,
}
impl EndpointAttempt<'_> {
    fn expired(&self) -> Option<DeadlineOrigin> {
        if tokio::time::Instant::now() < self.caller_deadline.min(self.own_deadline) {
            None
        } else if self.own_deadline <= self.caller_deadline {
            Some(DeadlineOrigin::Attempt)
        } else {
            Some(DeadlineOrigin::Caller)
        }
    }
}

impl LcdClient {
    pub fn new(
        urls: Vec<String>,
        timeout_ms: u64,
        cooldown_ms: u64,
        expected_chain_id: impl Into<String>,
    ) -> Self {
        Self::new_with_policy(
            urls,
            timeout_ms,
            cooldown_ms,
            expected_chain_id,
            ProviderPolicy::default(),
        )
    }

    pub fn new_with_policy(
        urls: Vec<String>,
        timeout_ms: u64,
        cooldown_ms: u64,
        expected_chain_id: impl Into<String>,
        provider_policy: ProviderPolicy,
    ) -> Self {
        let provider_policy = provider_policy.bounded();
        let expected_chain_id = expected_chain_id.into();
        assert!(
            valid_chain_id(&expected_chain_id),
            "invalid expected chain identifier"
        );
        static NEXT_NAMESPACE: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(1);
        // Duplicate URLs share a single slot and cannot multiply their provider budget.
        let mut unique = std::collections::HashSet::new();
        let endpoints = urls
            .into_iter()
            .map(|url| url.trim_end_matches('/').to_string())
            .filter(|url| unique.insert(url.clone()))
            .take(32)
            .map(|url| Endpoint {
                provider: provider_pool::ProviderState::new(
                    provider_policy.clone(),
                    Duration::from_millis(cooldown_ms)
                        .min(Duration::from_millis(provider_policy.max_backoff_ms)),
                ),
                url: url.trim_end_matches('/').to_string(),
                observed_chain_id: None,
                verified: false,
                identity_error: None,
                identity_checked_at: None,
            })
            .collect();

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(
                timeout_ms.max(1).min(provider_policy.attempt_timeout_ms),
            ))
            .build()
            .expect("failed to build reqwest client");

        Self {
            ingestion_budget: None,
            scheduled_observation: false,
            safety_observer: None,
            observation_deadline: None,
            inner: Arc::new(LcdClientInner {
                endpoints: RwLock::new(endpoints),
                expected_chain_id,
                client,
                cooldown_duration: Duration::from_millis(cooldown_ms)
                    .min(Duration::from_millis(provider_policy.max_backoff_ms)),
                provider_policy,
                next_endpoint: AtomicUsize::new(0),
                latest_height: AtomicI64::new(0),
                discovery_namespace: NEXT_NAMESPACE
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                pair_discovery_state: Arc::new(PairDiscoveryState::default()),
                #[cfg(test)]
                endpoint_metadata_waits: AtomicUsize::new(0),
                #[cfg(test)]
                data_request_polls: AtomicUsize::new(0),
            }),
        }
    }

    /// Fee observations can revoke retained qualifications immediately on known
    /// unsafe evidence, even while provider metadata cleanup is blocked.
    pub(crate) fn with_fee_safety_callback(&self, callback: Arc<dyn Fn() + Send + Sync>) -> Self {
        let mut next = self.clone();
        next.safety_observer = Some(callback);
        next
    }

    /// This view shares all provider state and limits. Only scheduled ingestion
    /// may avoid quarantining a closed provider when its caller budget expires;
    /// HTTP/transport/own attempt failures and half-open probes still quarantine.
    pub(crate) fn with_scheduled_observation(&self) -> Self {
        let mut next = self.clone();
        next.scheduled_observation = true;
        next
    }

    /// Owned producer scopes may only tighten an existing caller deadline.
    /// All provider, pacing, namespace and cancellation policy stays shared.
    pub(crate) fn with_observation_deadline(&self, deadline: tokio::time::Instant) -> Self {
        let mut next = self.clone();
        next.observation_deadline = Some(
            self.observation_deadline
                .map_or(deadline, |current| current.min(deadline)),
        );
        next
    }

    pub(crate) fn observation_deadline(&self) -> Option<tokio::time::Instant> {
        self.observation_deadline
    }

    #[cfg(test)]
    pub(crate) fn data_request_poll_count(&self) -> usize {
        self.inner.data_request_polls.load(Ordering::SeqCst)
    }
    /// A bounded background-only view sharing chain identity and namespace. All
    /// quote workers share its two permits and aggregate four requests/second;
    /// each provider failover attempt consumes the same pacing budget.
    pub(crate) fn with_ingestion_budget(&self) -> Self {
        let mut next = self.with_scheduled_observation();
        next.ingestion_budget = Some(Arc::new(IngestionBudget {
            permits: Arc::new(tokio::sync::Semaphore::new(2)),
            next_request: tokio::sync::Mutex::new(tokio::time::Instant::now()),
        }));
        next
    }
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, LcdError> {
        self.get_with_body_limit(path, None).await
    }

    /// Bound background configuration responses before buffering or decoding them.
    pub async fn get_bounded<T: DeserializeOwned>(
        &self,
        path: &str,
        max_bytes: usize,
    ) -> Result<T, LcdError> {
        self.get_with_body_limit(path, Some(max_bytes)).await
    }

    async fn get_with_body_limit<T: DeserializeOwned>(
        &self,
        path: &str,
        max_bytes: Option<usize>,
    ) -> Result<T, LcdError> {
        self.get_at_height_with_body_limit(path, max_bytes, None)
            .await
    }

    /// Background historical bank snapshots must never silently read the latest state.
    pub async fn get_at_height<T: DeserializeOwned>(
        &self,
        path: &str,
        height: i64,
    ) -> Result<T, LcdError> {
        if height <= 0 {
            return Err(LcdError::Deserialize(
                "invalid historical query height".into(),
            ));
        }
        self.get_at_height_with_body_limit(path, Some(16_384), Some(height))
            .await
    }

    async fn get_at_height_with_body_limit<T: DeserializeOwned>(
        &self,
        path: &str,
        max_bytes: Option<usize>,
        height: Option<i64>,
    ) -> Result<T, LcdError> {
        self.get_with_constraints(
            path,
            RequestConstraints {
                max_bytes,
                historical_height: height,
                minimum_latest_height: None,
                safety_observer: self.safety_observer.as_deref(),
            },
        )
        .await
    }

    async fn get_with_constraints<T: DeserializeOwned>(
        &self,
        path: &str,
        mut constraints: RequestConstraints<'_>,
    ) -> Result<T, LcdError> {
        if self.ingestion_budget.is_some() {
            constraints.max_bytes = Some(constraints.max_bytes.unwrap_or(65_536).min(65_536));
        }
        let policy_deadline = tokio::time::Instant::now()
            + Duration::from_millis(self.inner.provider_policy.deadline_ms);
        let deadline = self
            .observation_deadline
            .map_or(policy_deadline, |parent| parent.min(policy_deadline));
        let attempted = AtomicUsize::new(0);
        let integrity_error = std::sync::Mutex::new(None);
        let deferred_error = std::sync::Mutex::new(None);
        let deadline_result = || {
            if let Some(error) = integrity_error.lock().unwrap().take() {
                Err(error)
            } else if attempted.load(Ordering::Relaxed) == 0 {
                Err(deferred_error
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap_or(LcdError::AdmissionDeferred))
            } else {
                Err(LcdError::AllEndpointsFailed(
                    "total deadline exceeded".into(),
                ))
            }
        };
        tokio::time::timeout_at(deadline, async {
            let mut circuit_logged = false;
            loop {
                // Timeout polls its inner future first. Never begin a new scan
                // after the original budget, even when this task was not polled.
                if tokio::time::Instant::now() >= deadline {
                    return deadline_result();
                }
                let scan = self
                    .get_bounded_attempts(path, constraints, &attempted, &integrity_error, deadline)
                    .await;
                if self.ingestion_budget.is_some()
                    && attempted.load(Ordering::Relaxed) == 0
                    && scan.attempts == 0
                    && scan.deferral.is_some()
                {
                    if scan.deferral == Some(ScanDeferral::Circuit) && !circuit_logged {
                        tracing::warn!(
                            phase = "data",
                            reason = "circuit_open",
                            attempts = 0,
                            "LCD scheduled admission deferred"
                        );
                        circuit_logged = true;
                    }
                    // Retain the public terminal classification at hard expiry.
                    // Every scan-owned permit was dropped before this owned wait.
                    *deferred_error.lock().unwrap() = scan.result.err();
                    tokio::time::sleep(
                        Duration::from_millis(50)
                            .min(deadline.saturating_duration_since(tokio::time::Instant::now())),
                    )
                    .await;
                    continue;
                }
                if scan.attempts == 0 && tokio::time::Instant::now() >= deadline {
                    return deadline_result();
                }
                return scan.result;
            }
        })
        .await
        .unwrap_or_else(|_| deadline_result())
    }

    async fn get_bounded_attempts<T: DeserializeOwned>(
        &self,
        path: &str,
        constraints: RequestConstraints<'_>,
        attempted: &AtomicUsize,
        integrity_error: &std::sync::Mutex<Option<LcdError>>,
        deadline: tokio::time::Instant,
    ) -> ScanOutcome<T> {
        let count = self.inner.endpoints.read().await.len();
        if count == 0 {
            return ScanOutcome::completed(
                Err(LcdError::AllEndpointsFailed(
                    "no endpoints configured".into(),
                )),
                0,
            );
        }
        let start = self.inner.next_endpoint.fetch_add(1, Ordering::Relaxed) % count;
        let mut attempts = 0;
        let mut errors = Vec::new();
        let mut capacity_deferred = false;
        let mut deadline_expired = false;
        // This query owns one cursor and one attempted set. Scheduled callers
        // may finish capacity-denied alternatives, never restart an attempted
        // endpoint or grant a new attempt/deadline budget after real I/O.
        let mut pending: std::collections::VecDeque<_> =
            (0..count).map(|offset| (start + offset) % count).collect();
        let mut attempted_endpoints = vec![false; count];
        let mut capacity_pending = Vec::new();
        let mut continuation_logged = false;
        loop {
            let idx = if let Some(index) = pending.pop_front() {
                index
            } else {
                if !self.scheduled_observation
                    || attempts == 0
                    || attempted.load(Ordering::Relaxed) >= self.inner.provider_policy.max_attempts
                    || capacity_pending.is_empty()
                {
                    break;
                }
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    deadline_expired = true;
                    break;
                }
                if !continuation_logged {
                    tracing::warn!(
                        phase = "data",
                        reason = "capacity",
                        attempt_count = attempted.load(Ordering::Relaxed),
                        unattempted_capacity_count = capacity_pending.len(),
                        original_remaining_ms = remaining
                            .as_millis()
                            .min(u128::from(self.inner.provider_policy.deadline_ms))
                            as u64,
                        "LCD scheduled alternatives deferred"
                    );
                    continuation_logged = true;
                }
                // All iteration-owned provider and ingestion permits have
                // dropped. Waiting spends only this caller's original time.
                tokio::time::sleep(Duration::from_millis(50).min(remaining)).await;
                if tokio::time::Instant::now() >= deadline {
                    deadline_expired = true;
                    break;
                }
                pending.extend(capacity_pending.drain(..));
                continue;
            };
            if attempted_endpoints[idx] {
                continue;
            }
            let ep = self.inner.endpoints.read().await[idx].clone();
            if tokio::time::Instant::now() >= deadline {
                deadline_expired = true;
                break;
            }
            // Background pacing owns only its lane permit. Acquire provider capacity
            // after the wait: queued work must neither starve checkpoint reads nor
            // open a healthy provider's circuit when its caller is cancelled.
            let (_ingestion_permit, admission) =
                if let Some(budget) = self.ingestion_budget.as_ref() {
                    let lane = match budget.permits.clone().acquire_owned().await {
                        Ok(lane) => lane,
                        Err(_) => {
                            return ScanOutcome::completed(
                                Err(LcdError::AllEndpointsFailed(
                                    "Ingestion budget closed".into(),
                                )),
                                attempts,
                            )
                        }
                    };
                    let mut next = budget.next_request.lock().await;
                    tokio::time::sleep_until(*next).await;
                    if tokio::time::Instant::now() >= deadline {
                        deadline_expired = true;
                        break;
                    }
                    // No provider permit exists during the pacing await. Serialize
                    // this synchronous admission/charge so only actual attempts
                    // advance the shared rate budget; denied circuits/capacity do
                    // not spend another 250ms before trying a healthy alternative.
                    let admission = ep.provider.admit();
                    if admission.is_ok() {
                        *next = tokio::time::Instant::now() + Duration::from_millis(250);
                    }
                    (Some(lane), admission)
                } else {
                    (None, ep.provider.admit())
                };
            let mut permit = match admission {
                Ok(permit) => permit,
                Err(reason) => {
                    capacity_deferred |= reason.is_capacity();
                    if reason.is_capacity() {
                        capacity_pending.push(idx);
                    }
                    continue;
                }
            };
            if self.scheduled_observation {
                permit.scheduled_observation();
            }
            attempted_endpoints[idx] = true;
            attempts += 1;
            attempted.fetch_add(1, Ordering::Relaxed);
            let own_deadline = tokio::time::Instant::now()
                + Duration::from_millis(self.inner.provider_policy.attempt_timeout_ms);
            let outcome = tokio::time::timeout_at(
                own_deadline,
                self.query_endpoint::<T>(
                    EndpointAttempt {
                        index: idx,
                        endpoint: &ep,
                        caller_deadline: deadline,
                        own_deadline,
                    },
                    path,
                    constraints,
                    &mut permit,
                    integrity_error,
                ),
            )
            .await;
            if outcome.is_ok() && !matches!(&outcome, Ok(Ok(EndpointOutcome::Expired(_)))) {
                permit.completed();
            }
            match outcome {
                Ok(Ok(EndpointOutcome::Accepted(value))) => {
                    permit.succeeded();
                    self.inner
                        .next_endpoint
                        .store((idx + 1) % count, Ordering::Relaxed);
                    return ScanOutcome::completed(Ok(value), attempts);
                }
                Ok(Ok(EndpointOutcome::BelowMinimum)) => {
                    // Successful chain-attested I/O still proves provider health.
                    // A lower valid head cannot advance this pool frontier; try
                    // an alternative inside the same attempts/deadline budget.
                    permit.succeeded();
                    errors.push(format!("endpoint[{idx}] latest height below query minimum"));
                }
                Ok(Ok(EndpointOutcome::Expired(DeadlineOrigin::Caller))) => {
                    // Leave the permit armed: scheduled closed-provider work
                    // cancels neutrally; generic/half-open work quarantines.
                    deadline_expired = true;
                    break;
                }
                Ok(Err(
                    error @ (LcdError::ContractQueryRejected(_)
                    | LcdError::QueryGasLimitExceeded(_)),
                )) => {
                    // Deterministic contract outcomes prove provider availability, including recovery.
                    permit.succeeded();
                    self.inner.next_endpoint.store(idx, Ordering::Relaxed);
                    return ScanOutcome::completed(Err(error), attempts);
                }
                Ok(Err(error @ LcdError::PairResponseMalformed(_))) => {
                    permit.succeeded();
                    errors.push(format!("endpoint[{idx}] query-local malformed response"));
                    // Retain query-local malformed evidence across a later total
                    // deadline as well; repeated bad bodies are not availability.
                    let mut retained = integrity_error.lock().unwrap();
                    if !matches!(
                        *retained,
                        Some(LcdError::ChainIdentity(_) | LcdError::HistoricalHeightUnavailable)
                    ) {
                        *retained = Some(error);
                    }
                }
                Ok(Err(
                    error @ (LcdError::ChainIdentity(_)
                    | LcdError::HistoricalHeightUnavailable
                    | LcdError::Deserialize(_)
                    | LcdError::Base64(_)
                    | LcdError::HttpRejected(_)),
                )) => {
                    // Still try a qualified alternative, but preserve unsafe evidence if
                    // none succeeds. Lifecycle recovery must not mistake it for an outage.
                    errors.push(format!("endpoint[{idx}] integrity rejected"));
                    *integrity_error.lock().unwrap() = Some(error);
                }
                Ok(Err(_)) => errors.push(format!("endpoint[{idx}] provider unavailable")),
                Err(_) | Ok(Ok(EndpointOutcome::Expired(DeadlineOrigin::Attempt))) => {
                    // The provider's own attempt deadline proves failure. Keep
                    // drop armed until async recording completes exactly once.
                    permit.quarantine_if_cancelled();
                    self.mark_failed(idx).await;
                    permit.completed();
                    errors.push(format!("endpoint[{idx}] attempt deadline exceeded"));
                }
            }
            if attempts >= self.inner.provider_policy.max_attempts {
                break;
            }
        }
        if let Some(error) = integrity_error.lock().unwrap().take() {
            return ScanOutcome::completed(Err(error), attempts);
        }
        if deadline_expired {
            return ScanOutcome::completed(
                Err(if attempts == 0 {
                    LcdError::AdmissionDeferred
                } else {
                    LcdError::AllEndpointsFailed("total deadline exceeded".into())
                }),
                attempts,
            );
        }
        if attempts == 0 && capacity_deferred {
            return ScanOutcome::deferred(LcdError::AdmissionDeferred, ScanDeferral::Capacity);
        }
        if attempts == 0 && errors.is_empty() {
            return ScanOutcome::deferred(
                LcdError::AllEndpointsFailed("all provider circuits open".into()),
                ScanDeferral::Circuit,
            );
        }
        ScanOutcome::completed(
            Err(LcdError::AllEndpointsFailed(errors.join("; "))),
            attempts,
        )
    }

    async fn query_endpoint<T: DeserializeOwned>(
        &self,
        attempt: EndpointAttempt<'_>,
        path: &str,
        constraints: RequestConstraints<'_>,
        permit: &mut provider_pool::ProviderPermit,
        integrity_error: &std::sync::Mutex<Option<LcdError>>,
    ) -> Result<EndpointOutcome<T>, LcdError> {
        let idx = attempt.index;
        let url = &attempt.endpoint.url;
        let RequestConstraints {
            max_bytes,
            historical_height: height,
            minimum_latest_height,
            safety_observer,
        } = constraints;
        if let Some(origin) = attempt.expired() {
            return Ok(EndpointOutcome::Expired(origin));
        }
        self.attest_endpoint(idx, url, permit, integrity_error, safety_observer)
            .await?;
        // timeout_at polls its inner future first. Successful attestation can
        // resume from a blocked metadata write after either original deadline;
        // no data transport may be first-polled at that point.
        if let Some(origin) = attempt.expired() {
            return Ok(EndpointOutcome::Expired(origin));
        }
        let mut request = self.inner.client.get(format!("{url}{path}"));
        if let Some(height) = height {
            request = request
                .query(&[("height", height)])
                .header("x-cosmos-block-height", height.to_string());
        }
        // Test-only first-poll witness: wire counts can remain zero when an
        // outer timeout cancels a newly polled transport immediately.
        #[cfg(test)]
        self.inner.data_request_polls.fetch_add(1, Ordering::SeqCst);
        let response = match request.send().await {
            Ok(response) => response,
            Err(_) => {
                permit.quarantine_if_cancelled();
                self.mark_failed(idx).await;
                return Err(LcdError::AllEndpointsFailed(format!(
                    "endpoint[{idx}] transport unavailable"
                )));
            }
        };
        let status = response.status();
        let retry_after = provider_pool::retry_after(
            response.headers(),
            Duration::from_millis(self.inner.provider_policy.max_backoff_ms),
        );
        if !status.is_success() {
            // A cancelled negative body cannot become neutral provider health.
            // Completed contract/gas bodies retain their original local policy.
            permit.quarantine_if_cancelled();
            let wasm_not_found =
                status.as_u16() == 404 && path.starts_with("/cosmwasm/wasm/v1/contract/");
            if !should_cooldown_endpoint(status) && !wasm_not_found {
                // The protocol rejection is known from headers, even if its
                // body/metadata cleanup never completes before this deadline.
                record_integrity_evidence(
                    integrity_error,
                    &LcdError::HttpRejected(status.as_u16()),
                    safety_observer,
                );
            } else if wasm_not_found {
                // Headers already reject this scoped fee observation. Keep the
                // PD1394 body-based contract-absence classification unchanged.
                if let Some(observer) = safety_observer {
                    observer();
                }
            }
        }
        // Unambiguous endpoint failures need no body. Preserve all HTTP 500
        // body classification, including legacy classic/non-smart outcomes.
        if should_cooldown_endpoint(status) && status.as_u16() != 500 {
            self.mark_failed_with_retry(idx, retry_after).await;
            tracing::debug!(endpoint_index = idx, status = %status, path = lcd_log_path(path), "LCD upstream unavailable");
            return Err(LcdError::AllEndpointsFailed(format!(
                "endpoint[{idx}] HTTP {status}"
            )));
        }
        let served_height = response
            .headers()
            .get("x-cosmos-block-height")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok());
        if status.is_success() && height.is_some() && served_height != height {
            permit.quarantine_if_cancelled();
            record_integrity_evidence(
                integrity_error,
                &LcdError::HistoricalHeightUnavailable,
                safety_observer,
            );
            self.mark_failed(idx).await;
            return Err(LcdError::HistoricalHeightUnavailable);
        }
        // Every body is bounded, including errors and otherwise unbounded callers.
        let text = match bounded_response_text(response, max_bytes.unwrap_or(4 * 1024 * 1024)).await
        {
            Ok(body) => body,
            Err(LcdError::Request(_)) => {
                permit.quarantine_if_cancelled();
                self.mark_failed(idx).await;
                return Err(LcdError::AllEndpointsFailed(format!(
                    "endpoint[{idx}] body transport unavailable"
                )));
            }
            Err(error) => {
                // An oversized hostile contract-local body stays local. Explicit
                // gateway/rate-limit status cannot evade quarantine by returning garbage.
                let contract_local_500 = status.as_u16() == 500
                    && path.starts_with("/cosmwasm/wasm/v1/contract/")
                    && path.contains("/smart/");
                if should_cooldown_endpoint(status) && !contract_local_500 {
                    self.mark_failed_with_retry(idx, retry_after).await;
                    return Err(LcdError::AllEndpointsFailed(format!(
                        "endpoint[{idx}] HTTP {status}"
                    )));
                }
                let error = LcdError::PairResponseMalformed(error.to_string());
                record_integrity_evidence(integrity_error, &error, safety_observer);
                return Err(error);
            }
        };
        if !status.is_success() {
            let snippet = text.chars().take(200).collect::<String>();
            if is_query_gas_limit_exceeded(status, path, &text) {
                return Err(LcdError::QueryGasLimitExceeded(snippet));
            }
            if is_deterministic_contract_query_reject(status, path, &text) {
                return Err(LcdError::ContractQueryRejected(if status.as_u16() == 404 {
                    "contract not found".into()
                } else {
                    snippet
                }));
            }
            if should_cooldown_endpoint(status) {
                self.mark_failed_with_retry(idx, retry_after).await;
            }
            tracing::debug!(endpoint_index = idx, status = %status, path = lcd_log_path(path), "LCD upstream unavailable");
            if !should_cooldown_endpoint(status) {
                return Err(LcdError::HttpRejected(status.as_u16()));
            }
            return Err(LcdError::AllEndpointsFailed(format!(
                "endpoint[{idx}] HTTP {status}"
            )));
        }
        if path.starts_with("/cosmos/base/tendermint/v1beta1/blocks/") {
            if let Err(error) =
                chain_identity::validate_block_chain(&text, self.expected_chain_id())
            {
                permit.quarantine_if_cancelled();
                record_integrity_evidence(integrity_error, &error, safety_observer);
                // Update failure and metadata under one guard. A second write
                // await after recording could let cancellation count it again.
                let mut endpoints = self.inner.endpoints.write().await;
                let endpoint = &mut endpoints[idx];
                endpoint.provider.failed(self.inner.cooldown_duration, None);
                endpoint.verified = false;
                endpoint.identity_error = Some(error.to_string());
                return Err(error);
            }
        }
        let historical_body = match chain_identity::validate_requested_height(&text, path) {
            Ok(historical) => historical,
            Err(error) => {
                permit.quarantine_if_cancelled();
                record_integrity_evidence(integrity_error, &error, safety_observer);
                self.mark_failed(idx).await;
                return Err(error);
            }
        };
        let value: T = serde_json::from_str(&text).map_err(|_| {
            let error = LcdError::PairResponseMalformed("response schema invalid".into());
            record_integrity_evidence(integrity_error, &error, safety_observer);
            error
        })?;
        // Historical reads deliberately bypass live lag comparison. Their exact height
        // header above is required, while the original historical time is preserved.
        let mut below_minimum = false;
        let live_height = if height.is_none() && !historical_body {
            let observed = if path.ends_with("/blocks/latest") {
                serde_json::from_str::<serde_json::Value>(&text)
                    .ok()
                    .and_then(|v| {
                        v.pointer("/block/header/height")
                            .and_then(|h| h.as_str())
                            .and_then(|h| h.parse::<i64>().ok())
                    })
            } else {
                served_height
            };
            if let Some(observed) = observed {
                let latest = self.inner.latest_height.load(Ordering::Relaxed);
                if observed <= 0
                    || observed.saturating_add(self.inner.provider_policy.max_height_lag) < latest
                {
                    permit.quarantine_if_cancelled();
                    self.mark_failed(idx).await;
                    return Err(LcdError::LaggingProvider);
                }
            }
            if path == LATEST_BLOCK_PATH {
                if let Some(minimum) = minimum_latest_height {
                    let observed = observed.ok_or_else(|| {
                        let error = LcdError::Deserialize(
                            "latest block height is not a valid integer".into(),
                        );
                        record_integrity_evidence(integrity_error, &error, safety_observer);
                        error
                    })?;
                    below_minimum = observed < minimum;
                }
            }
            observed
        } else {
            None
        };
        // Preserve every typed body/height negative above, then refuse late
        // positives before health success or shared live-height publication.
        // A caller's minimum cannot invent the shared frontier. Actual timely
        // observations still inform its existing lag policy, even when below
        // this query's separate minimum.
        if let Some(origin) = attempt.expired() {
            return Ok(EndpointOutcome::Expired(origin));
        }
        if let Some(observed) = live_height {
            self.inner
                .latest_height
                .fetch_max(observed, Ordering::Relaxed);
        }
        if below_minimum {
            return Ok(EndpointOutcome::BelowMinimum);
        }
        Ok(EndpointOutcome::Accepted(value))
    }

    async fn mark_failed(&self, idx: usize) {
        self.mark_failed_with_retry(idx, None).await;
    }

    async fn mark_failed_with_retry(&self, idx: usize, retry_after: Option<Duration>) {
        let mut endpoints = self.inner.endpoints.write().await;
        if let Some(ep) = endpoints.get_mut(idx) {
            ep.provider
                .failed(self.inner.cooldown_duration, retry_after);
            ep.verified = false;
        }
    }

    /// Fixed bounded endpoint-index counters, without addresses, URLs or response bodies.
    pub async fn provider_counters(&self) -> Vec<ProviderCounters> {
        self.inner
            .endpoints
            .read()
            .await
            .iter()
            .map(|ep| ep.provider.counters())
            .collect()
    }

    /// Process-local identity: isolates bounded discovery caches by LCD client/chain.
    pub(crate) fn cache_namespace(&self) -> usize {
        self.inner.discovery_namespace
    }

    /// Clones share discovery evidence; a newly configured client starts fresh.
    pub(crate) fn pair_discovery_state(&self) -> Arc<PairDiscoveryState> {
        Arc::clone(&self.inner.pair_discovery_state)
    }

    /// Fixed discovery outcomes; no event address or factory becomes a metric label.
    pub fn pair_discovery_counters(&self) -> pair_discovery_state::DiscoveryCounters {
        self.inner.pair_discovery_state.counters()
    }

    pub async fn get_code_data_hash(&self, code_id: u64) -> Result<String, LcdError> {
        let value: serde_json::Value = self
            .get(&format!("/cosmwasm/wasm/v1/code/{}", code_id))
            .await?;
        value
            .get("code_info")
            .and_then(|c| c.get("data_hash"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| LcdError::Deserialize("missing code data hash".into()))
    }

    pub async fn query_contract<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
    ) -> Result<T, LcdError> {
        self.query_contract_with_body_limit(contract_addr, query, None)
            .await
    }

    /// Bounded response variant for pool/book mirrors. The cap covers success and error bodies.
    pub async fn query_contract_bounded<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        max_bytes: usize,
    ) -> Result<T, LcdError> {
        self.query_contract_with_body_limit(contract_addr, query, Some(max_bytes))
            .await
    }

    /// Pin all components of a background observation to an exact provider-confirmed height.
    /// Unsupported history and proxies ignoring the height fail closed and remain retryable.
    pub async fn query_contract_at_height_bounded<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        height: i64,
        max_bytes: usize,
    ) -> Result<T, LcdError> {
        if height <= 0 {
            return Err(LcdError::HistoricalHeightUnavailable);
        }
        self.query_contract_request(contract_addr, query, Some(max_bytes), Some(height))
            .await
    }

    /// Scoped producer hook with the canonical pinned query and body decoding.
    /// The observer owns synchronous invalidation only; it grants no provider authority.
    pub(crate) async fn query_contract_at_height_bounded_observed<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        height: i64,
        max_bytes: usize,
        observer: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<T, LcdError> {
        self.with_fee_safety_callback(observer)
            .query_contract_at_height_bounded(contract_addr, query, height, max_bytes)
            .await
    }

    async fn query_contract_with_body_limit<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        max_bytes: Option<usize>,
    ) -> Result<T, LcdError> {
        self.query_contract_request(contract_addr, query, max_bytes, None)
            .await
    }

    async fn query_contract_request<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        max_bytes: Option<usize>,
        height: Option<i64>,
    ) -> Result<T, LcdError> {
        let query_bytes = serde_json::to_vec(query)
            .map_err(|e| LcdError::Deserialize(format!("Failed to serialize query: {}", e)))?;
        let query_b64 = base64::engine::general_purpose::STANDARD.encode(&query_bytes);

        let path = format!(
            "/cosmwasm/wasm/v1/contract/{}/smart/{}",
            contract_addr, query_b64
        );

        let resp: SmartQueryResponse = self
            .get_at_height_with_body_limit(&path, max_bytes, height)
            .await?;

        serde_json::from_value(resp.data)
            .map_err(|_| LcdError::PairResponseMalformed("contract response schema invalid".into()))
    }

    /// Live wasm `ContractInfo` (code_id + admin) for community-tax attestation (#594).
    pub async fn get_contract_info(
        &self,
        contract_addr: &str,
    ) -> Result<crate::lcd::types::WasmContractInfo, LcdError> {
        let path = format!("/cosmwasm/wasm/v1/contract/{}", contract_addr);
        let env: WasmContractInfoEnvelope = self.get(&path).await?;
        Ok(env.contract_info)
    }

    /// Live wasm `code_id` for a contract instance (GitLab #585 F6 freeze probe).
    pub async fn get_contract_code_id(&self, contract_addr: &str) -> Result<u64, LcdError> {
        self.get_contract_info(contract_addr)
            .await?
            .code_id_u64()
            .ok_or_else(|| {
                LcdError::Deserialize("contract_info.code_id missing or not a u64".to_string())
            })
    }

    pub async fn get_latest_block_height(&self) -> Result<i64, LcdError> {
        self.get_latest_block_height_with_body_limit(None).await
    }

    pub async fn get_latest_block_height_bounded(&self, max_bytes: usize) -> Result<i64, LcdError> {
        self.get_latest_block_height_with_body_limit(Some(max_bytes))
            .await
    }

    /// A scheduled pool read cannot use a head behind its already stored frontier.
    /// This is a query-local constraint, not a different global lag/circuit policy.
    pub(crate) async fn get_latest_block_height_bounded_at_least(
        &self,
        max_bytes: usize,
        minimum_height: i64,
    ) -> Result<i64, LcdError> {
        if minimum_height < 0 {
            return Err(LcdError::PairResponseMalformed(
                "invalid minimum latest height".into(),
            ));
        }
        let response: BlockResponse = self
            .get_with_constraints(
                LATEST_BLOCK_PATH,
                RequestConstraints {
                    max_bytes: Some(max_bytes),
                    minimum_latest_height: Some(minimum_height),
                    safety_observer: self.safety_observer.as_deref(),
                    historical_height: None,
                },
            )
            .await?;
        response
            .block
            .header
            .height
            .parse::<i64>()
            .map_err(|_| LcdError::Deserialize("latest block height is not a valid integer".into()))
    }

    async fn get_latest_block_height_with_body_limit(
        &self,
        max_bytes: Option<usize>,
    ) -> Result<i64, LcdError> {
        let resp: BlockResponse = self
            .get_with_body_limit("/cosmos/base/tendermint/v1beta1/blocks/latest", max_bytes)
            .await?;
        resp.block
            .header
            .height
            .parse::<i64>()
            .map_err(|e| LcdError::Deserialize(format!("Invalid block height: {}", e)))
    }

    pub async fn search_txs(
        &self,
        events: &[(&str, &str)],
        page: u32,
        limit: u32,
    ) -> Result<TxSearchResponse, LcdError> {
        // Cosmos SDK 0.50+ (terrad v4) GetTxsEvent: a single `query` param with conditions
        // joined by " AND ", a 1-based `page`, and a plain `limit`. The legacy `events=` and
        // `pagination.offset|limit` params were removed — on v4 `events=` returns HTTP 500
        // ("query cannot be empty") and the pagination.* params are silently ignored (every
        // page refetches the first). Encode spaces; the raw `=` inside a condition is fine.
        let query = events
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" AND ")
            .replace(' ', "%20");

        let path = format!(
            "/cosmos/tx/v1beta1/txs?query={}&page={}&limit={}&order_by=1",
            query,
            page.max(1),
            limit,
        );

        self.get(&path).await
    }

    pub async fn get_block_at_height(&self, height: i64) -> Result<BlockResponse, LcdError> {
        let path = format!("/cosmos/base/tendermint/v1beta1/blocks/{}", height);
        self.get(&path).await
    }

    pub async fn get_block_hash(&self, height: i64) -> Result<String, LcdError> {
        let resp = self.get_block_at_height(height).await?;
        resp.block_id
            .map(|id| id.hash)
            .ok_or_else(|| LcdError::Deserialize(format!("block {} missing block_id.hash", height)))
    }

    /// Fetch all txs for a block, paginating until `pagination.total` is satisfied or pages are empty.
    pub async fn get_block_txs(
        &self,
        height: i64,
        page_limit: u32,
        max_pages: u32,
    ) -> Result<BlockTxsResult, LcdError> {
        let mut all_txs = Vec::new();
        let mut page = 1u32;
        let mut expected_total: Option<u64> = None;

        loop {
            if page > max_pages {
                return Err(LcdError::Deserialize(format!(
                    "block {} tx pagination exceeded max_pages={} (collected {} txs)",
                    height,
                    max_pages,
                    all_txs.len()
                )));
            }

            let resp = self
                .search_txs(&[("tx.height", &height.to_string())], page, page_limit)
                .await?;

            if expected_total.is_none() {
                // SDK 0.50+ returns the count at top-level `total` with `pagination: null`;
                // older LCDs only set `pagination.total`. Prefer the former, fall back to the latter.
                expected_total = resp
                    .total
                    .as_ref()
                    .or_else(|| resp.pagination.as_ref().and_then(|p| p.total.as_ref()))
                    .and_then(|t| t.parse::<u64>().ok());
            }

            let page_txs = resp.tx_responses.unwrap_or_default();
            let page_len = page_txs.len();
            all_txs.extend(page_txs);

            let done = page_len == 0
                || page_len < page_limit as usize
                || expected_total.is_some_and(|total| all_txs.len() as u64 >= total);

            if done {
                if let Some(total) = expected_total {
                    if all_txs.len() as u64 != total {
                        return Err(LcdError::Deserialize(format!(
                            "block {} incomplete tx fetch: LCD total={} collected={} pages={}",
                            height,
                            total,
                            all_txs.len(),
                            page
                        )));
                    }
                }
                return Ok(BlockTxsResult {
                    txs: all_txs,
                    page_count: page,
                });
            }

            page += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockTxsResult {
    pub txs: Vec<TxResponse>,
    pub page_count: u32,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn bounded_body_caps_chunked_response_without_content_length() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            socket.read(&mut request).await.unwrap();
            socket.write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n14\r\nxxxxxxxxxxxxxxxxxxxx\r\n14\r\nxxxxxxxxxxxxxxxxxxxx\r\n0\r\n\r\n",
            ).await.unwrap();
        });
        let response = reqwest::Client::new()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.content_length(), None);
        let error = super::bounded_response_text(response, 32)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("exceeds 32 byte limit"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn bounded_contract_response_rejects_huge_success_and_error_bodies() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for status in [200, 500] {
            let server = MockServer::start().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(status).set_body_string("x".repeat(65_537)))
                .mount(&server)
                .await;
            let lcd = super::LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
            let result = lcd
                .query_contract_bounded::<serde_json::Value>(
                    "terra1pair",
                    &serde_json::json!({ "pool": {} }),
                    65_536,
                )
                .await;
            let error = result.unwrap_err().to_string();
            assert!(error.contains("exceeds 65536 byte limit"));
            assert!(
                !error.contains(&"x".repeat(20)),
                "oversized bodies never enter errors/logs"
            );
            server.reset().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"data": {"healthy": true}})),
                )
                .mount(&server)
                .await;
            let healthy = lcd
                .query_contract_bounded::<serde_json::Value>(
                    "terra1healthy",
                    &serde_json::json!({"pool": {}}),
                    65_536,
                )
                .await
                .expect("one oversized pair cannot cool down healthy pair queries");
            assert_eq!(healthy["healthy"], true);
        }
    }

    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn tx_at(height: i64, idx: u32) -> serde_json::Value {
        json!({
            "height": height.to_string(),
            "txhash": format!("hash{}_{}", height, idx),
            "timestamp": "2024-01-01T00:00:00Z",
            "logs": [],
            "events": []
        })
    }

    #[tokio::test]
    async fn get_block_txs_single_page() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let height = 42i64;
        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("query", format!("tx.height={}", height)))
            .and(query_param("page", "1"))
            .and(query_param("limit", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": [tx_at(height, 1), tx_at(height, 2)],
                "total": "2"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let result = lcd.get_block_txs(height, 100, 10).await.unwrap();
        assert_eq!(result.txs.len(), 2);
        assert_eq!(result.page_count, 1);
    }

    #[tokio::test]
    async fn get_block_txs_multi_page() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let height = 99i64;
        let page1: Vec<_> = (0..100).map(|i| tx_at(height, i)).collect();
        let page2: Vec<_> = (100..150).map(|i| tx_at(height, i)).collect();

        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": page1,
                "total": "150"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": page2,
                "total": "150"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let result = lcd.get_block_txs(height, 100, 10).await.unwrap();
        assert_eq!(result.txs.len(), 150);
        assert_eq!(result.page_count, 2);
    }

    #[tokio::test]
    async fn get_block_txs_rejects_incomplete_total() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": [tx_at(1, 0)],
                "total": "150"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let err = lcd.get_block_txs(1, 100, 1).await.unwrap_err();
        assert!(err.to_string().contains("incomplete tx fetch"));
    }

    #[tokio::test]
    async fn rate_limit_429_cools_down_and_fails_over() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        // Short cooldown so a follow-up request still skips endpoint A.
        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            60_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("should fail over to endpoint B");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);

        // Second call should not re-hit cooled-down A; B serves again.
        let _val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .unwrap();
        assert_eq!(hits_b.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn classify_deterministic_cosmwasm_rejects() {
        let ok = reqwest::StatusCode::OK;
        let s500 = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let s502 = reqwest::StatusCode::BAD_GATEWAY;
        let cw = "/cosmwasm/wasm/v1/contract/terra1x/smart/abc";
        let tx = "/cosmos/tx/v1beta1/txs?query=tx.height=1";

        assert!(!is_deterministic_contract_query_reject(
            ok,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        assert!(!is_deterministic_contract_query_reject(
            s502,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"Generic error: pair not found: query wasm contract failed"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"Error parsing into type: unknown variant `pair`"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"code":2,"message":"codespace wasm code 9: query wasm contract failed: key not found"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        // Infra-looking body on cosmwasm path without wasm markers → not classified.
        assert!(!is_deterministic_contract_query_reject(
            s500,
            cw,
            "internal LCD failure"
        ));
        // Non-cosmwasm path: classic phrases still classify; bare wasmd phrase does not.
        assert!(is_deterministic_contract_query_reject(
            s500,
            tx,
            r#"{"message":"pair not found"}"#
        ));
        assert!(!is_deterministic_contract_query_reject(
            s500,
            tx,
            r#"{"message":"query cannot be empty"}"#
        ));
    }

    #[tokio::test]
    async fn wasm_query_reject_does_not_fan_out() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

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
                "message": "codespace wasm code 9: query wasm contract failed: key not found",
                "details": []
            })))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let err = lcd
            .query_contract::<serde_json::Value>(
                "terra1pair",
                &json!({"limit_order": {"order_id": 1}}),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, LcdError::ContractQueryRejected(_)),
            "unexpected: {err}"
        );
        assert_eq!(hits_b.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn wasm_query_gas_limit_does_not_fan_out_or_cool_down() {
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let gas_rejection = Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code": 2,
                "message": "out of gas in location: wasm contract; gasWanted: 3000000, gasUsed: 3043317",
            })))
            .expect(1)
            .mount_as_scoped(&server_a)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } })),
            )
            .expect(0)
            .mount(&server_b)
            .await;
        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let error = lcd
            .query_contract::<serde_json::Value>(
                "terra1router",
                &json!({"simulate_swap_operations": {}}),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, LcdError::QueryGasLimitExceeded(_)));
        drop(gas_rejection);
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } })),
            )
            .expect(1)
            .mount(&server_a)
            .await;
        let quote: serde_json::Value = lcd
            .query_contract("terra1router", &json!({"config": {}}))
            .await
            .unwrap();
        assert_eq!(quote["ok"], true);
    }

    #[test]
    fn wasm_query_gas_limit_classification_is_scoped() {
        let path = "/cosmwasm/wasm/v1/contract/terra1router/smart/abc";
        let body = "out of gas in location: wasm contract; gasWanted: 3000000, gasUsed: 3043317";
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        assert!(is_query_gas_limit_exceeded(status, path, body));
        assert!(!is_query_gas_limit_exceeded(
            reqwest::StatusCode::BAD_GATEWAY,
            path,
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            "/cosmos/tx/v1beta1/txs",
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            "/cosmwasm/wasm/v1/contract/terra1router/raw/abc",
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            path,
            "out of gas infrastructure failure"
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            path,
            "gasWanted: 3000000, gasUsed: 2000000"
        ));
    }

    #[tokio::test]
    async fn infra_500_still_fans_out() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal LCD failure"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("infra 500 should fail over");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gateway_502_cools_down_and_fails_over() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            60_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("502 should fail over");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_contract_code_id_parses_string_and_number() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path("/cosmwasm/wasm/v1/contract/terra1token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "contract_info": { "code_id": "10184", "creator": "terra1x", "admin": "", "label": "t" }
            })))
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        assert_eq!(
            lcd.get_contract_code_id("terra1token").await.unwrap(),
            10184
        );
    }
}

#[cfg(test)]
mod provider_resilience_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn payload(server: &MockServer, status: u16) {
        Mock::given(path("/data"))
            .respond_with(ResponseTemplate::new(status).set_body_json(json!({"ok": true})))
            .mount(server)
            .await;
    }
    async fn hits(server: &MockServer, at: &str) -> usize {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == at)
            .count()
    }

    #[tokio::test]
    async fn provider_failures_open_circuit_and_preserve_healthy_failover() {
        for status in [403, 429, 408, 500, 502, 503, 504] {
            let primary = MockServer::start().await;
            let secondary = MockServer::start().await;
            for server in [&primary, &secondary] {
                mock_chain_identity(server, "columbus-5").await;
            }
            payload(&primary, status).await;
            payload(&secondary, 200).await;
            let lcd = LcdClient::new(
                vec![primary.uri(), secondary.uri()],
                1000,
                10_000,
                "columbus-5",
            );
            for _ in 0..4 {
                assert_eq!(
                    lcd.get::<serde_json::Value>("/data").await.unwrap()["ok"],
                    true
                );
            }
            assert_eq!(
                hits(&primary, "/data").await,
                1,
                "HTTP {status} repeated on quarantined provider"
            );
            assert_eq!(hits(&secondary, "/data").await, 4);
            let counters = lcd.provider_counters().await;
            assert_eq!(counters[0].failures, 1);
            assert!(counters[0].circuit_open);
            assert_eq!(counters[1].successes, 4);
        }
    }

    #[tokio::test]
    async fn bounded_retry_after_then_one_half_open_verified_recovery() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(429).insert_header("Retry-After", "999999999999999999999"),
            )
            .mount(&server)
            .await;
        let policy = ProviderPolicy {
            max_backoff_ms: 40,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(vec![server.uri()], 1000, 0, "columbus-5", policy);
        assert!(lcd.get::<serde_json::Value>("/data").await.is_err());
        assert!(lcd.clone().get::<serde_json::Value>("/data").await.is_err());
        assert_eq!(hits(&server, "/data").await, 1);
        server.reset().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_millis(50)),
            )
            .mount(&server)
            .await;
        tokio::time::sleep(Duration::from_millis(45)).await;
        let clone = lcd.clone();
        let (a, b) = tokio::join!(
            lcd.get::<serde_json::Value>("/data"),
            clone.get::<serde_json::Value>("/data")
        );
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].recovery_probes, 1);
        assert!(!counters[0].circuit_open);
        assert_eq!(counters[0].in_flight, 0);
        assert_eq!(hits(&server, "/data").await, 1);
    }

    #[tokio::test]
    async fn slow_provider_timeout_fails_over_with_total_attempt_bound() {
        let primary = MockServer::start().await;
        let secondary = MockServer::start().await;
        for server in [&primary, &secondary] {
            mock_chain_identity(server, "columbus-5").await;
        }
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_secs(1)),
            )
            .mount(&primary)
            .await;
        payload(&secondary, 200).await;
        let policy = ProviderPolicy {
            attempt_timeout_ms: 100,
            deadline_ms: 350,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(
            vec![primary.uri(), secondary.uri()],
            1000,
            10_000,
            "columbus-5",
            policy,
        );
        let start = Instant::now();
        assert!(lcd.get::<serde_json::Value>("/data").await.is_ok());
        assert!(start.elapsed() < Duration::from_millis(350));
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
    }

    #[tokio::test]
    async fn healthy_rotation_and_clone_budgets_are_client_scoped() {
        let a = MockServer::start().await;
        let b = MockServer::start().await;
        for server in [&a, &b] {
            mock_chain_identity(server, "columbus-5").await;
            payload(server, 200).await;
        }
        let policy = ProviderPolicy {
            burst: 2,
            requests_per_second: 1,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(
            vec![a.uri(), a.uri(), b.uri()],
            1000,
            0,
            "columbus-5",
            policy.clone(),
        );
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        lcd.clone().get::<serde_json::Value>("/data").await.unwrap();
        assert!(matches!(
            lcd.get::<serde_json::Value>("/data").await,
            Err(LcdError::AdmissionDeferred)
        ));
        assert!(matches!(
            lcd.require_verified_chain().await,
            Err(LcdError::AdmissionDeferred)
        ));
        assert_eq!(hits(&a, "/data").await, 1);
        assert_eq!(hits(&b, "/data").await, 1);
        assert_eq!(lcd.provider_counters().await.len(), 2);
        let independent = LcdClient::new_with_policy(vec![a.uri()], 1000, 0, "columbus-5", policy);
        independent.get::<serde_json::Value>("/data").await.unwrap();
    }

    #[tokio::test]
    async fn local_malformed_contract_does_not_quarantine_or_negative_cache() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path_regex("^/cosmwasm/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("broken json"))
            .mount(&server)
            .await;
        payload(&server, 200).await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 60_000, "columbus-5");
        assert!(matches!(
            lcd.query_contract::<serde_json::Value>("terra1broken", &json!({"pool":{}}))
                .await,
            Err(LcdError::PairResponseMalformed(_))
        ));
        assert_eq!(lcd.provider_counters().await[0].failures, 0);
        assert!(!lcd.provider_counters().await[0].circuit_open);
        lcd.get::<serde_json::Value>("/data").await.unwrap();
    }

    async fn assert_fee_safety_callback_precedes_metadata_cleanup(identity_failure: bool) {
        use std::sync::atomic::AtomicBool;
        let server = MockServer::start().await;
        let query = json!({"get_fee_config":{}});
        let query_path = format!(
            "/cosmwasm/wasm/v1/contract/terra1fixture/smart/{}",
            base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&query).unwrap())
        );
        if identity_failure {
            Mock::given(path(chain_identity::NODE_INFO_PATH))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_delay(Duration::from_millis(100))
                        .set_body_json(json!({"default_node_info":{"network":"wrong-chain"}})),
                )
                .mount(&server)
                .await;
        } else {
            mock_chain_identity(&server, "columbus-5").await;
            Mock::given(path(query_path.as_str()))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_delay(Duration::from_millis(100))
                        .insert_header("x-cosmos-block-height", "99")
                        .set_body_json(json!({"data":{"fee_bps":30}})),
                )
                .mount(&server)
                .await;
        }
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1_000,
            1_000,
            "columbus-5",
            ProviderPolicy {
                max_attempts: 1,
                ..Default::default()
            },
        );
        let revoked = Arc::new(AtomicBool::new(false));
        let callback: Arc<dyn Fn() + Send + Sync> = {
            let revoked = revoked.clone();
            Arc::new(move || {
                revoked.store(true, Ordering::SeqCst);
            })
        };
        let pending = {
            let lcd = lcd.clone();
            tokio::spawn(async move {
                lcd.query_contract_at_height_bounded_observed::<serde_json::Value>(
                    "terra1fixture",
                    &query,
                    100,
                    16_384,
                    callback,
                )
                .await
            })
        };
        let awaited_path = if identity_failure {
            chain_identity::NODE_INFO_PATH
        } else {
            &query_path
        };
        tokio::time::timeout(Duration::from_secs(1), async {
            while hits(&server, awaited_path).await == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        // The request has left all endpoint reads but cannot finish negative cleanup.
        let held = lcd.inner.endpoints.read().await;
        tokio::time::timeout(Duration::from_millis(500), async {
            while !revoked.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("known negative revokes cached qualification before cleanup lock");
        assert!(
            !pending.is_finished(),
            "cleanup remains blocked after synchronous revocation"
        );
        drop(held);
        let error = pending.await.unwrap().unwrap_err();
        assert!(if identity_failure {
            matches!(error, LcdError::ChainIdentity(_))
        } else {
            matches!(error, LcdError::HistoricalHeightUnavailable)
        });
    }

    #[tokio::test]
    async fn recovery1464_fee_safety_callback_revokes_before_identity_cleanup_await() {
        assert_fee_safety_callback_precedes_metadata_cleanup(true).await;
    }
    #[tokio::test]
    async fn recovery1464_fee_safety_callback_revokes_before_pinned_height_cleanup_await() {
        assert_fee_safety_callback_precedes_metadata_cleanup(false).await;
    }
    #[tokio::test]
    async fn recovery1464_fee_safety_callback_excludes_availability_and_valid_old_head() {
        use std::sync::atomic::AtomicBool;
        let server = MockServer::start().await;
        mock_minimum_head(&server, "100", "columbus-5").await;
        payload(&server, 503).await;
        let lcd = minimum_head_client(
            &[&server],
            ProviderPolicy {
                max_attempts: 1,
                ..Default::default()
            },
        );
        let revoked = Arc::new(AtomicBool::new(false));
        let view = lcd.with_fee_safety_callback({
            let revoked = revoked.clone();
            Arc::new(move || {
                revoked.store(true, Ordering::SeqCst);
            })
        });
        assert!(matches!(
            view.get_latest_block_height_bounded_at_least(16_384, 101)
                .await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert!(
            !revoked.load(Ordering::SeqCst),
            "valid lower frontier is availability"
        );
        assert!(matches!(
            view.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert!(!revoked.load(Ordering::SeqCst), "HTTP503 is availability");
        assert!(
            lcd.safety_observer.is_none(),
            "scoped view leaves generic client unchanged"
        );
    }
    #[tokio::test]
    async fn recovery1464_fee_safety_callback_marks_malformed_envelope_without_quarantine() {
        use std::sync::atomic::AtomicBool;
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/data"))
            .respond_with(ResponseTemplate::new(200).set_body_string("malformed"))
            .mount(&server)
            .await;
        let lcd = minimum_head_client(
            &[&server],
            ProviderPolicy {
                max_attempts: 1,
                ..Default::default()
            },
        );
        let revoked = Arc::new(AtomicBool::new(false));
        let view = lcd.with_fee_safety_callback({
            let revoked = revoked.clone();
            Arc::new(move || {
                revoked.store(true, Ordering::SeqCst);
            })
        });
        assert!(matches!(
            view.get::<serde_json::Value>("/data").await,
            Err(LcdError::PairResponseMalformed(_))
        ));
        assert!(revoked.load(Ordering::SeqCst));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert!(!counters[0].circuit_open);
    }

    fn minimum_head_response(height: &str, chain: &str) -> serde_json::Value {
        json!({"block":{"header":{"height":height,"chain_id":chain,"time":"2026-01-01T00:00:00Z"}}})
    }
    async fn mock_minimum_head(server: &MockServer, height: &str, chain: &str) {
        mock_chain_identity(server, "columbus-5").await;
        Mock::given(path(LATEST_BLOCK_PATH))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(minimum_head_response(height, chain)),
            )
            .mount(server)
            .await;
    }
    fn minimum_head_client(servers: &[&MockServer], policy: ProviderPolicy) -> LcdClient {
        LcdClient::new_with_policy(
            servers.iter().map(|server| server.uri()).collect(),
            1000,
            60_000,
            "columbus-5",
            policy,
        )
        .with_scheduled_observation()
    }

    #[tokio::test]
    async fn recovery1464_minimum_head_uses_qualified_alternative_without_changing_generic_history()
    {
        let old = MockServer::start().await;
        let current = MockServer::start().await;
        mock_minimum_head(&old, "100", "columbus-5").await;
        mock_minimum_head(&current, "102", "columbus-5").await;
        let lcd = minimum_head_client(&[&old, &current], ProviderPolicy::default());
        lcd.inner.latest_height.store(101, Ordering::Relaxed);
        assert_eq!(
            lcd.get_latest_block_height_bounded_at_least(16_384, 101)
                .await
                .unwrap(),
            102
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters.iter().map(|c| c.attempts).sum::<u64>(), 2);
        assert!(counters
            .iter()
            .all(|c| c.successes == 1 && c.failures == 0 && !c.circuit_open && c.in_flight == 0));
        // The caller's floor never changes the established global twenty-block lag policy.
        assert_eq!(
            lcd.get_latest_block_height_bounded(16_384).await.unwrap(),
            100
        );
        assert_eq!(lcd.inner.latest_height.load(Ordering::Relaxed), 102);
        Mock::given(path("/cosmos/base/tendermint/v1beta1/blocks/99"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "99")
                    .set_body_json(minimum_head_response("99", "columbus-5")),
            )
            .mount(&current)
            .await;
        assert_eq!(
            lcd.get_block_at_height(99)
                .await
                .unwrap()
                .block
                .header
                .height,
            "99"
        );
        assert_eq!(lcd.inner.latest_height.load(Ordering::Relaxed), 102);
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_equality_is_eligible() {
        let server = MockServer::start().await;
        mock_minimum_head(&server, "101", "columbus-5").await;
        let lcd = minimum_head_client(&[&server], ProviderPolicy::default());
        assert_eq!(
            lcd.get_latest_block_height_bounded_at_least(16_384, 101)
                .await
                .unwrap(),
            101
        );
        assert_eq!(lcd.provider_counters().await[0].attempts, 1);
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_all_old_respects_single_attempt_and_rate_budget() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;
        let third = MockServer::start().await;
        for server in [&first, &second] {
            mock_minimum_head(server, "100", "columbus-5").await;
        }
        mock_minimum_head(&third, "102", "columbus-5").await;
        let lcd = minimum_head_client(
            &[&first, &second, &third],
            ProviderPolicy {
                max_attempts: 2,
                burst: 2,
                requests_per_second: 1,
                ..Default::default()
            },
        );
        let result = lcd
            .get_latest_block_height_bounded_at_least(16_384, 101)
            .await;
        assert!(matches!(result, Err(LcdError::AllEndpointsFailed(_))));
        assert_eq!(hits(&first, LATEST_BLOCK_PATH).await, 1);
        assert_eq!(hits(&second, LATEST_BLOCK_PATH).await, 1);
        assert_eq!(
            hits(&third, LATEST_BLOCK_PATH).await,
            0,
            "no attempt-budget reset can reach a third endpoint"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters.iter().map(|c| c.attempts).sum::<u64>(), 2);
        assert_eq!(counters.iter().map(|c| c.successes).sum::<u64>(), 2);
        assert!(counters
            .iter()
            .all(|c| c.failures == 0 && !c.circuit_open && c.in_flight == 0));
        // Valid but ineligible I/O spends the usual two-token provider charge.
        for endpoint in &lcd.inner.endpoints.read().await[..2] {
            assert!(matches!(
                endpoint.provider.admit(),
                Err(provider_pool::ProviderDenial::Rate)
            ));
        }
        assert_eq!(
            lcd.inner.latest_height.load(Ordering::Relaxed),
            100,
            "the database floor cannot seed invented global observed height"
        );
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_real_failure_retains_quarantine_and_valid_alternative() {
        let failed = MockServer::start().await;
        let old = MockServer::start().await;
        let current = MockServer::start().await;
        mock_chain_identity(&failed, "columbus-5").await;
        Mock::given(path(LATEST_BLOCK_PATH))
            .respond_with(ResponseTemplate::new(503))
            .mount(&failed)
            .await;
        mock_minimum_head(&old, "100", "columbus-5").await;
        mock_minimum_head(&current, "101", "columbus-5").await;
        let lcd = minimum_head_client(&[&failed, &old, &current], ProviderPolicy::default());
        assert_eq!(
            lcd.get_latest_block_height_bounded_at_least(16_384, 101)
                .await
                .unwrap(),
            101
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 1);
        assert!(counters[0].circuit_open);
        assert_eq!(counters[1].failures, 0);
        assert_eq!(counters[1].successes, 1);
        assert!(!counters[1].circuit_open);
        assert_eq!(counters[2].successes, 1);
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_integrity_is_not_hidden_by_old_heads_or_deadline() {
        for malformed in [false, true] {
            let bad = MockServer::start().await;
            let old = MockServer::start().await;
            mock_minimum_head(
                &bad,
                if malformed { "invalid" } else { "100" },
                if malformed {
                    "columbus-5"
                } else {
                    "localterra"
                },
            )
            .await;
            mock_minimum_head(&old, "100", "columbus-5").await;
            let lcd = minimum_head_client(&[&bad, &old], ProviderPolicy::default());
            let error = lcd
                .get_latest_block_height_bounded_at_least(16_384, 101)
                .await
                .unwrap_err();
            if malformed {
                assert!(matches!(error, LcdError::Deserialize(_)));
            } else {
                assert!(matches!(error, LcdError::ChainIdentity(_)));
            }
            assert_eq!(lcd.provider_counters().await[1].successes, 1);
            assert_eq!(lcd.provider_counters().await[1].failures, 0);
        }
        let wrong = MockServer::start().await;
        let slow = MockServer::start().await;
        mock_minimum_head(&wrong, "100", "localterra").await;
        mock_chain_identity(&slow, "columbus-5").await;
        Mock::given(path(LATEST_BLOCK_PATH))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(minimum_head_response("101", "columbus-5"))
                    .set_delay(Duration::from_millis(500)),
            )
            .mount(&slow)
            .await;
        let lcd = minimum_head_client(
            &[&wrong, &slow],
            ProviderPolicy {
                deadline_ms: 100,
                attempt_timeout_ms: 1000,
                ..Default::default()
            },
        );
        assert!(matches!(
            lcd.get_latest_block_height_bounded_at_least(16_384, 101)
                .await,
            Err(LcdError::ChainIdentity(_))
        ));
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_missing_and_nonpositive_values_are_not_soft_success() {
        for height in ["0", "-1"] {
            let server = MockServer::start().await;
            mock_minimum_head(&server, height, "columbus-5").await;
            let lcd = minimum_head_client(&[&server], ProviderPolicy::default());
            assert!(matches!(
                lcd.get_latest_block_height_bounded_at_least(16_384, 101)
                    .await,
                Err(LcdError::AllEndpointsFailed(_))
            ));
            let counters = lcd.provider_counters().await;
            assert_eq!(counters[0].successes, 0);
            assert_eq!(counters[0].failures, 1);
            assert!(
                counters[0].circuit_open,
                "nonpositive latest heads keep existing rejection/quarantine behavior"
            );
        }
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path(LATEST_BLOCK_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"block":{"header":{"chain_id":"columbus-5","time":"2026-01-01T00:00:00Z"}}}),
            ))
            .mount(&server)
            .await;
        let lcd = minimum_head_client(&[&server], ProviderPolicy::default());
        let error = lcd
            .get_latest_block_height_bounded_at_least(16_384, 101)
            .await
            .unwrap_err();
        assert!(
            matches!(error, LcdError::PairResponseMalformed(_)),
            "missing height retains typed schema rejection"
        );
        assert!(!crate::indexer::block_indexer::is_retryable_availability(
            &error
        ));
    }
    #[tokio::test]
    async fn recovery1464_minimum_head_ineligible_success_cannot_reset_concurrent_real_failure() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        let started = Arc::new(tokio::sync::Notify::new());
        let notified = started.clone();
        Mock::given(path(LATEST_BLOCK_PATH))
            .respond_with(move |_: &wiremock::Request| {
                notified.notify_one();
                ResponseTemplate::new(200)
                    .set_body_json(minimum_head_response("100", "columbus-5"))
                    .set_delay(Duration::from_millis(150))
            })
            .mount(&server)
            .await;
        Mock::given(path("/failed"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let lcd = minimum_head_client(&[&server], ProviderPolicy::default());
        let pending = lcd.clone();
        let head = tokio::spawn(async move {
            pending
                .get_latest_block_height_bounded_at_least(16_384, 101)
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), started.notified())
            .await
            .unwrap();
        assert!(lcd.get::<serde_json::Value>("/failed").await.is_err());
        assert!(matches!(
            head.await.unwrap(),
            Err(LcdError::AllEndpointsFailed(_))
        ));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].attempts, 2);
        assert_eq!(counters[0].failures, 1);
        assert_eq!(counters[0].successes, 1);
        assert!(counters[0].circuit_open);
        assert_eq!(counters[0].in_flight, 0);
    }

    #[tokio::test]
    async fn lagging_live_height_and_unconfirmed_history_are_rejected() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/tip"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "1000")
                    .set_body_json(json!({"ok":true})),
            )
            .mount(&server)
            .await;
        Mock::given(path("/behind"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "900")
                    .set_body_json(json!({"ok":true})),
            )
            .mount(&server)
            .await;
        payload(&server, 200).await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 0, "columbus-5");
        lcd.get::<serde_json::Value>("/tip").await.unwrap();
        assert!(lcd.get::<serde_json::Value>("/behind").await.is_err());
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        assert!(matches!(
            lcd.get_at_height::<serde_json::Value>("/data", 100).await,
            Err(LcdError::HistoricalHeightUnavailable)
        ));
        assert_eq!(lcd.provider_counters().await[0].failures, 2);
        Mock::given(path("/historic"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "100")
                    .set_body_json(json!({"ok":true})),
            )
            .mount(&server)
            .await;
        assert!(lcd
            .get_at_height::<serde_json::Value>("/historic", 100)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn recovery1464_scheduled_pair_budget_does_not_quarantine_healthy_fallback() {
        // The real production budgets: a 2.5s bad attempt leaves only 1.5s
        // of a 4s pair budget for a healthy endpoint needing 2s. Generic caller
        // cancellation still quarantines; scheduled cancellation is neutral.
        for scheduled in [false, true] {
            let bad = MockServer::start().await;
            let healthy = MockServer::start().await;
            for server in [&bad, &healthy] {
                mock_chain_identity(server, "columbus-5").await;
            }
            for (server, delay) in [(&bad, 3_000), (&healthy, 2_000)] {
                Mock::given(path("/data"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .set_body_json(json!({"ok":true}))
                            .set_delay(Duration::from_millis(delay)),
                    )
                    .mount(server)
                    .await;
            }
            let lcd = LcdClient::new_with_policy(
                vec![bad.uri(), healthy.uri()],
                2_500,
                5_000,
                "columbus-5",
                ProviderPolicy {
                    attempt_timeout_ms: 2_500,
                    deadline_ms: 8_000,
                    max_backoff_ms: 5_000,
                    ..Default::default()
                },
            );
            let view = if scheduled {
                lcd.with_scheduled_observation()
            } else {
                lcd.clone()
            };
            assert!(std::sync::Arc::ptr_eq(&view.inner, &lcd.inner));
            assert_eq!(view.chain_cache_namespace(), lcd.chain_cache_namespace());
            assert!(tokio::time::timeout(
                Duration::from_secs(4),
                view.get::<serde_json::Value>("/data")
            )
            .await
            .is_err());
            let counters = lcd.provider_counters().await;
            assert_eq!(counters[0].failures, 1);
            assert!(counters[0].circuit_open);
            assert_eq!(counters[1].in_flight, 0);
            assert_eq!(counters[1].failures, if scheduled { 0 } else { 1 });
            assert_eq!(
                counters[1].caller_cancellations,
                if scheduled { 1 } else { 0 }
            );
            assert_eq!(
                counters[1].successes, 0,
                "cancellation never fabricates provider success"
            );
            if scheduled {
                assert!(!counters[1].circuit_open);
                assert_eq!(
                    view.get::<serde_json::Value>("/data").await.unwrap()["ok"],
                    true
                );
                assert_eq!(lcd.provider_counters().await[1].failures, 0);
            } else {
                assert!(counters[1].circuit_open);
                assert!(view
                    .get::<serde_json::Value>("/data")
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("all provider circuits open"));
            }
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_identity_caller_deadline_is_neutral_but_own_timeout_is_not() {
        let server = MockServer::start().await;
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(Duration::from_millis(300)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            500,
            1_000,
            "columbus-5",
            ProviderPolicy {
                attempt_timeout_ms: 500,
                deadline_ms: 100,
                ..Default::default()
            },
        );
        assert!(lcd
            .with_scheduled_observation()
            .require_verified_chain()
            .await
            .is_err());
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].failures,
                counters[0].caller_cancellations,
                counters[0].in_flight
            ),
            (0, 1, 0)
        );
        assert!(!counters[0].circuit_open);
        assert!(
            lcd.require_verified_chain().await.is_err(),
            "plain caller retains original cancellation quarantine"
        );
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        let slow = MockServer::start().await;
        mock_chain_identity(&slow, "columbus-5").await;
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_millis(300)),
            )
            .mount(&slow)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![slow.uri()],
            100,
            1_000,
            "columbus-5",
            ProviderPolicy {
                attempt_timeout_ms: 100,
                deadline_ms: 1_000,
                ..Default::default()
            },
        );
        assert!(lcd
            .with_scheduled_observation()
            .get::<serde_json::Value>("/data")
            .await
            .is_err());
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].failures,
                counters[0].caller_cancellations,
                counters[0].in_flight
            ),
            (1, 0, 0)
        );
        assert!(counters[0].circuit_open);
    }
    #[tokio::test]
    async fn recovery1464_scheduled_slow_error_body_cannot_hide_completed_http_failure() {
        assert_known_slow_http_error_quarantines(503).await;
    }
    #[tokio::test]
    async fn recovery1464_scheduled_noncontract_500_body_cancellation_retains_quarantine() {
        assert_known_slow_http_error_quarantines(500).await;
    }
    #[tokio::test]
    async fn recovery1464_scheduled_protocol_rejection_headers_survive_slow_body_deadline() {
        for status in [400, 401, 404] {
            assert_known_slow_http_error_quarantines(status).await;
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_completed_protocol_rejections_preserve_wasm_not_found_policy() {
        for (status, path_value, body, local) in [
            (400, "/data", "rejected", false),
            (401, "/data", "rejected", false),
            (404, "/data", "rejected", false),
            (
                404,
                "/cosmwasm/wasm/v1/contract/fixture",
                r#"{"code":5,"message":"contract not found"}"#,
                true,
            ),
        ] {
            let server = MockServer::start().await;
            mock_chain_identity(&server, "columbus-5").await;
            Mock::given(path(path_value))
                .respond_with(ResponseTemplate::new(status).set_body_string(body))
                .mount(&server)
                .await;
            let lcd = LcdClient::new_with_policy(
                vec![server.uri()],
                500,
                1_000,
                "columbus-5",
                ProviderPolicy::default(),
            );
            let error = lcd
                .with_scheduled_observation()
                .get::<serde_json::Value>(path_value)
                .await
                .unwrap_err();
            assert!(if local {
                matches!(error, LcdError::ContractQueryRejected(_))
            } else {
                matches!(error,LcdError::HttpRejected(code) if code==status)
            });
            let counters = lcd.provider_counters().await;
            assert_eq!(
                (
                    counters[0].failures,
                    counters[0].caller_cancellations,
                    counters[0].in_flight
                ),
                (0, 0, 0)
            );
            assert_eq!(counters[0].successes, if local { 1 } else { 0 });
        }
    }
    #[tokio::test]
    async fn recovery1464_fee_safety_callback_contract_404_headers_precede_incomplete_body() {
        assert_known_slow_http_error_quarantines_at(
            404,
            "/cosmwasm/wasm/v1/contract/fixture/smart/fixture",
        )
        .await;
    }
    async fn assert_known_slow_http_error_quarantines(status: u16) {
        assert_known_slow_http_error_quarantines_at(status, "/data").await;
    }
    async fn assert_known_slow_http_error_quarantines_at(status: u16, path_value: &str) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let protocol_rejected =
            !should_cooldown_endpoint(reqwest::StatusCode::from_u16(status).unwrap());
        let unclassified_contract_404 =
            status == 404 && path_value.starts_with("/cosmwasm/wasm/v1/contract/");
        // Raw HTTP deliberately separates completed error headers from a body
        // that exceeds the caller budget. Wiremock delay would delay both.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for identity in [true, false] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut byte = [0u8; 1];
                    stream.read_exact(&mut byte).await.unwrap();
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 8192);
                }
                if identity {
                    let body = r#"{"default_node_info":{"network":"columbus-5"}}"#;
                    stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                } else {
                    stream.write_all(format!("HTTP/1.1 {status} Failure\r\nContent-Length: 4\r\nRetry-After: 1\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let _ = stream.write_all(b"fail").await;
                }
            }
        });
        let lcd = LcdClient::new_with_policy(
            vec![endpoint],
            2_500,
            1_000,
            "columbus-5",
            ProviderPolicy {
                attempt_timeout_ms: 2_500,
                deadline_ms: if protocol_rejected { 100 } else { 4_000 },
                ..Default::default()
            },
        );
        let safety_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let view = lcd.with_scheduled_observation().with_fee_safety_callback({
            let safety_called = safety_called.clone();
            Arc::new(move || {
                safety_called.store(true, Ordering::SeqCst);
            })
        });
        let result = tokio::time::timeout(
            Duration::from_millis(500),
            view.get::<serde_json::Value>(path_value),
        )
        .await;
        if status == 500 {
            assert!(
                result.is_err(),
                "ambiguous500 retains body classification until caller cancellation"
            );
        } else {
            let error = result
                .expect("known endpoint failure returns without awaiting its slow body")
                .unwrap_err();
            assert!(if unclassified_contract_404 {
                matches!(error, LcdError::AllEndpointsFailed(_))
            } else if protocol_rejected {
                matches!(error,LcdError::HttpRejected(code) if code==status)
            } else {
                matches!(error, LcdError::AllEndpointsFailed(_))
            });
        }
        assert_eq!(
            safety_called.load(Ordering::SeqCst),
            protocol_rejected,
            "permanent headers notify before the body arrives; outages do not notify"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].failures,
                counters[0].caller_cancellations,
                counters[0].in_flight
            ),
            (1, 0, 0)
        );
        assert!(counters[0].circuit_open);
        server.abort();
        let _ = server.await;
    }
    #[tokio::test]
    async fn recovery1464_scheduled_completed_nonsmart_500_preserves_legacy_contract_rejection() {
        for (path_value, body) in [
            ("/cosmos/bank/v1beta1/balances/fixture", "pair not found"),
            (
                "/cosmwasm/wasm/v1/contract/fixture",
                "query wasm contract failed: denied",
            ),
        ] {
            let server = MockServer::start().await;
            mock_chain_identity(&server, "columbus-5").await;
            Mock::given(path(path_value))
                .respond_with(ResponseTemplate::new(500).set_body_string(body))
                .mount(&server)
                .await;
            let lcd = LcdClient::new_with_policy(
                vec![server.uri()],
                500,
                1_000,
                "columbus-5",
                ProviderPolicy::default(),
            );
            let error = lcd
                .with_scheduled_observation()
                .get::<serde_json::Value>(path_value)
                .await
                .unwrap_err();
            assert!(matches!(error, LcdError::ContractQueryRejected(_)));
            let counters = lcd.provider_counters().await;
            assert_eq!(
                (
                    counters[0].successes,
                    counters[0].failures,
                    counters[0].caller_cancellations,
                    counters[0].in_flight
                ),
                (1, 0, 0, 0)
            );
            assert!(!counters[0].circuit_open);
        }
    }
    fn identity_http(status: u16, body: &str) -> String {
        format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())
    }
    async fn identity_contention_fixture(
        response: Option<String>,
        data_gate: Option<(tokio::sync::oneshot::Receiver<()>, Option<String>)>,
    ) -> (
        String,
        tokio::task::JoinHandle<()>,
        Arc<tokio::sync::Notify>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let data_started = Arc::new(tokio::sync::Notify::new());
        let started = data_started.clone();
        let server = tokio::spawn(async move {
            let count = if data_gate.is_some() { 2 } else { 1 };
            let mut gate = data_gate;
            for index in 0..count {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut byte = [0u8; 1];
                    stream.read_exact(&mut byte).await.unwrap();
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                    assert!(request.len() < 8192);
                }
                if index == 0 {
                    if let Some(response) = &response {
                        stream.write_all(response.as_bytes()).await.unwrap();
                    }
                } else {
                    started.notify_one();
                    let (release, response) = gate.take().unwrap();
                    let _ = release.await;
                    if let Some(response) = response {
                        stream.write_all(response.as_bytes()).await.unwrap();
                    }
                    // None closes without headers to prove transport failure.
                }
            }
        });
        (endpoint, server, data_started)
    }
    async fn metadata_writer_is_pending(lcd: &LcdClient) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while lcd.inner.endpoints.try_read().is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("metadata writer queues behind retained read guard");
    }
    #[tokio::test]
    async fn recovery1464_scheduled_known_identity_negatives_quarantine_before_metadata_cleanup() {
        for half_open in [false, true] {
            for response in [
                Some(identity_http(503, "fail")),
                Some(identity_http(
                    200,
                    r#"{"default_node_info":{"network":"wrong-chain"}}"#,
                )),
                None,
            ] {
                let (endpoint, server, _) = identity_contention_fixture(response, None).await;
                let lcd = LcdClient::new_with_policy(
                    vec![endpoint],
                    1_000,
                    1_000,
                    "columbus-5",
                    ProviderPolicy {
                        attempt_timeout_ms: 1_000,
                        deadline_ms: 2_000,
                        ..Default::default()
                    },
                );
                let guard = lcd.inner.endpoints.read().await;
                let provider = guard[0].provider.clone();
                if half_open {
                    provider.failed(Duration::ZERO, None);
                }
                let view = lcd.with_scheduled_observation();
                let task = tokio::spawn(async move { view.require_verified_chain().await });
                metadata_writer_is_pending(&lcd).await;
                assert_eq!(provider.counters().in_flight, 1);
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
                let counters = provider.counters();
                assert_eq!(
                    (
                        counters.failures,
                        counters.caller_cancellations,
                        counters.in_flight
                    ),
                    (if half_open { 2 } else { 1 }, 0, 0)
                );
                assert!(counters.circuit_open);
                assert!(!counters.half_open);
                drop(guard);
                server.abort();
                let _ = server.await;
            }
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_known_transport_failure_quarantines_before_metadata_cleanup() {
        for half_open in [false, true] {
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let (endpoint, server, started) = identity_contention_fixture(
                Some(identity_http(
                    200,
                    r#"{"default_node_info":{"network":"columbus-5"}}"#,
                )),
                Some((release_rx, None)),
            )
            .await;
            let lcd = LcdClient::new_with_policy(
                vec![endpoint],
                1_000,
                1_000,
                "columbus-5",
                ProviderPolicy {
                    attempt_timeout_ms: 1_000,
                    deadline_ms: 2_000,
                    ..Default::default()
                },
            );
            if half_open {
                lcd.inner.endpoints.read().await[0]
                    .provider
                    .failed(Duration::ZERO, None);
            }
            let view = lcd.with_scheduled_observation();
            let task = tokio::spawn(async move { view.get::<serde_json::Value>("/data").await });
            tokio::time::timeout(Duration::from_secs(1), started.notified())
                .await
                .unwrap();
            let guard = lcd.inner.endpoints.read().await;
            let provider = guard[0].provider.clone();
            release_tx.send(()).unwrap();
            metadata_writer_is_pending(&lcd).await;
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
            let counters = provider.counters();
            assert_eq!(
                (
                    counters.failures,
                    counters.caller_cancellations,
                    counters.in_flight
                ),
                (if half_open { 2 } else { 1 }, 0, 0)
            );
            assert!(counters.circuit_open);
            assert!(!counters.half_open);
            drop(guard);
            server.abort();
            let _ = server.await;
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_typed_identity_survives_outer_deadline_and_cleanup_contention()
    {
        for half_open in [false, true] {
            for body in [
                r#"{"default_node_info":{"network":"wrong-chain"}}"#,
                "{malformed",
            ] {
                let (endpoint, server, _) =
                    identity_contention_fixture(Some(identity_http(200, body)), None).await;
                let lcd = LcdClient::new_with_policy(
                    vec![endpoint],
                    1_000,
                    1_000,
                    "columbus-5",
                    ProviderPolicy {
                        attempt_timeout_ms: 1_000,
                        deadline_ms: 100,
                        ..Default::default()
                    },
                );
                let guard = lcd.inner.endpoints.read().await;
                let provider = guard[0].provider.clone();
                if half_open {
                    provider.failed(Duration::ZERO, None);
                }
                let view = lcd.with_scheduled_observation();
                let task =
                    tokio::spawn(async move { view.get::<serde_json::Value>("/data").await });
                metadata_writer_is_pending(&lcd).await;
                let error = tokio::time::timeout(Duration::from_secs(1), task)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap_err();
                assert!(
                    matches!(error, LcdError::ChainIdentity(_)),
                    "known integrity cannot become retryable availability at caller deadline"
                );
                let counters = provider.counters();
                assert_eq!(
                    (
                        counters.failures,
                        counters.caller_cancellations,
                        counters.in_flight
                    ),
                    (if half_open { 2 } else { 1 }, 0, 0)
                );
                assert!(counters.circuit_open);
                drop(guard);
                server.abort();
                let _ = server.await;
            }
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_identity_probe_preserves_known_integrity_after_clipped_deadline(
    ) {
        for half_open in [false, true] {
            let (endpoint, server, _) = identity_contention_fixture(
                Some(identity_http(
                    200,
                    r#"{"default_node_info":{"network":"wrong-chain"}}"#,
                )),
                None,
            )
            .await;
            let lcd = LcdClient::new_with_policy(
                vec![endpoint],
                1_000,
                1_000,
                "columbus-5",
                ProviderPolicy {
                    attempt_timeout_ms: 1_000,
                    deadline_ms: 100,
                    ..Default::default()
                },
            );
            let guard = lcd.inner.endpoints.read().await;
            let provider = guard[0].provider.clone();
            if half_open {
                provider.failed(Duration::ZERO, None);
            }
            let view = lcd.with_scheduled_observation();
            let task = tokio::spawn(async move { view.require_verified_chain().await });
            metadata_writer_is_pending(&lcd).await;
            // Let the probe's clipped deadline cancel attestation cleanup. Its
            // typed slot must survive the subsequent failed-attempt recording.
            tokio::time::sleep(Duration::from_millis(130)).await;
            drop(guard);
            let error = tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(matches!(error, LcdError::ChainIdentity(_)));
            let counters = provider.counters();
            assert_eq!(
                (
                    counters.failures,
                    counters.caller_cancellations,
                    counters.in_flight
                ),
                (if half_open { 2 } else { 1 }, 0, 0)
            );
            server.abort();
            let _ = server.await;
        }
    }
    #[tokio::test]
    async fn recovery1464_wrong_block_chain_metadata_updates_once_before_queued_reader() {
        for half_open in [false, true] {
            let (release_tx, release_rx) = tokio::sync::oneshot::channel();
            let block = r#"{"block":{"header":{"chain_id":"wrong-chain","height":"42","time":"2026-10-11T00:00:00Z"}}}"#;
            let (endpoint, server, started) = identity_contention_fixture(
                Some(identity_http(
                    200,
                    r#"{"default_node_info":{"network":"columbus-5"}}"#,
                )),
                Some((release_rx, Some(identity_http(200, block)))),
            )
            .await;
            let lcd = LcdClient::new_with_policy(
                vec![endpoint],
                1_000,
                1_000,
                "columbus-5",
                ProviderPolicy {
                    attempt_timeout_ms: 1_000,
                    deadline_ms: 200,
                    ..Default::default()
                },
            );
            if half_open {
                lcd.inner.endpoints.read().await[0]
                    .provider
                    .failed(Duration::ZERO, None);
            }
            let view = lcd.with_scheduled_observation();
            let task = tokio::spawn(async move {
                view.get::<serde_json::Value>("/cosmos/base/tendermint/v1beta1/blocks/latest")
                    .await
            });
            tokio::time::timeout(Duration::from_secs(1), started.notified())
                .await
                .unwrap();
            let guard = lcd.inner.endpoints.read().await;
            let provider = guard[0].provider.clone();
            release_tx.send(()).unwrap();
            metadata_writer_is_pending(&lcd).await;
            // Queue a reader behind the first failure writer. With two writes,
            // it would acquire between them and block identity-error cleanup
            // until the caller deadline counted an already recorded failure.
            let reader_started = Arc::new(tokio::sync::Notify::new());
            let queued = reader_started.clone();
            let (acquired_tx, acquired_rx) = tokio::sync::oneshot::channel();
            let (unlock_tx, unlock_rx) = tokio::sync::oneshot::channel();
            let reader_lcd = lcd.clone();
            let reader = tokio::spawn(async move {
                queued.notify_one();
                let guard = reader_lcd.inner.endpoints.read().await;
                let _ = acquired_tx.send((guard[0].verified, guard[0].identity_error.is_some()));
                let _ = unlock_rx.await;
                drop(guard);
            });
            reader_started.notified().await;
            drop(guard);
            let metadata = tokio::time::timeout(Duration::from_secs(1), acquired_rx)
                .await
                .unwrap()
                .unwrap();
            let error = tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(matches!(error, LcdError::ChainIdentity(_)));
            let counters = provider.counters();
            assert_eq!((counters.failures,counters.caller_cancellations,counters.in_flight),(if half_open {2} else {1},0,0),"typed wrong-block failure is counted exactly once while a subsequent reader remains held");
            assert_eq!(
                metadata,
                (false, true),
                "failure/verification/identity metadata update together"
            );
            unlock_tx.send(()).unwrap();
            reader.await.unwrap();
            server.abort();
            let _ = server.await;
        }
    }
    #[tokio::test]
    async fn recovery1464_scheduled_own_timeout_recording_stays_armed_until_metadata_cleanup() {
        for half_open in [false, true] {
            let (endpoint, server, _) = identity_contention_fixture(
                Some(identity_http(
                    200,
                    r#"{"default_node_info":{"network":"columbus-5"}}"#,
                )),
                None,
            )
            .await;
            let lcd = LcdClient::new_with_policy(
                vec![endpoint],
                100,
                1_000,
                "columbus-5",
                ProviderPolicy {
                    attempt_timeout_ms: 100,
                    deadline_ms: 180,
                    ..Default::default()
                },
            );
            let guard = lcd.inner.endpoints.read().await;
            let provider = guard[0].provider.clone();
            if half_open {
                provider.failed(Duration::ZERO, None);
            }
            let view = lcd.with_scheduled_observation();
            let task = tokio::spawn(async move { view.get::<serde_json::Value>("/data").await });
            metadata_writer_is_pending(&lcd).await;
            assert!(tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .is_err());
            let counters = provider.counters();
            assert_eq!(
                (
                    counters.failures,
                    counters.caller_cancellations,
                    counters.in_flight
                ),
                (if half_open { 2 } else { 1 }, 0, 0)
            );
            assert!(counters.circuit_open);
            drop(guard);
            server.abort();
            let _ = server.await;
        }
    }
    #[tokio::test]
    async fn total_deadline_cancellation_releases_shared_permit() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_secs(1)),
            )
            .mount(&server)
            .await;
        let policy = ProviderPolicy {
            concurrency: 1,
            deadline_ms: 40,
            attempt_timeout_ms: 500,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(vec![server.uri()], 1000, 0, "columbus-5", policy);
        assert!(lcd
            .get::<serde_json::Value>("/data")
            .await
            .unwrap_err()
            .to_string()
            .contains("total deadline"));
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        server.reset().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 200).await;
        lcd.get::<serde_json::Value>("/data").await.unwrap();
    }
    #[tokio::test]
    async fn recovery1464_historical_block_and_tx_body_heights_cannot_be_substituted() {
        let wrong = MockServer::start().await;
        let good = MockServer::start().await;
        for server in [&wrong, &good] {
            mock_chain_identity(server, "columbus-5").await;
            Mock::given(path("/tip"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("x-cosmos-block-height", "1000")
                        .set_body_json(json!({"ok":true})),
                )
                .mount(server)
                .await;
        }
        for (server, served) in [(&wrong, "42"), (&good, "41")] {
            Mock::given(path("/cosmos/base/tendermint/v1beta1/blocks/41"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("x-cosmos-block-height", served)
                        .set_body_json(json!({
                    "block_id":{"hash":"canonical"}, "block":{"header":{
                        "chain_id":"columbus-5","height":served,"time":"2026-10-10T00:00:00Z"}}})),
                )
                .mount(server)
                .await;
            Mock::given(path("/cosmos/tx/v1beta1/txs"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("x-cosmos-block-height", served)
                        .set_body_json(json!({
                    "tx_responses":[{"height":served,"txhash":"fixture"}],"total":"1"})),
                )
                .mount(server)
                .await;
        }
        let lcd = LcdClient::new(vec![wrong.uri(), good.uri()], 1000, 60_000, "columbus-5");
        assert_eq!(lcd.get_block_hash(41).await.unwrap(), "canonical");
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        let unsafe_lcd = LcdClient::new(vec![wrong.uri()], 1000, 0, "columbus-5");
        unsafe_lcd.get::<serde_json::Value>("/tip").await.unwrap();
        assert!(matches!(
            unsafe_lcd.get_block_hash(41).await,
            Err(LcdError::HistoricalHeightUnavailable)
        ));
        assert!(matches!(
            unsafe_lcd.get_block_txs(41, 100, 2).await,
            Err(LcdError::HistoricalHeightUnavailable)
        ));
        let safe = LcdClient::new(vec![good.uri()], 1000, 0, "columbus-5");
        safe.get::<serde_json::Value>("/tip").await.unwrap();
        assert_eq!(safe.get_block_hash(41).await.unwrap(), "canonical");
        let page = safe.get_block_txs(41, 100, 2).await.unwrap();
        assert_eq!(page.txs.len(), 1);
        assert_eq!(page.txs[0].height, "41");
        assert_eq!(
            safe.provider_counters().await[0].failures,
            0,
            "validated history cannot trip the live-height lag circuit"
        );
    }

    #[tokio::test]
    async fn recovery1464_contract_absence_is_local_and_other_http_rejections_are_permanent() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        Mock::given(path_regex("^/cosmwasm/wasm/v1/contract/"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "code":5,"message":"contract not found: untrusted payload"})))
            .mount(&server)
            .await;
        Mock::given(path("/bad"))
            .respond_with(ResponseTemplate::new(400).set_body_string("untrusted payload"))
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 60_000, "columbus-5");
        let absent = lcd
            .query_contract::<serde_json::Value>("terra1absent", &json!({"pair":{}}))
            .await;
        assert!(
            matches!(absent, Err(LcdError::ContractQueryRejected(ref message)) if message == "contract not found")
        );
        assert!(matches!(
            lcd.get::<serde_json::Value>("/bad").await,
            Err(LcdError::HttpRejected(400))
        ));
        assert_eq!(lcd.provider_counters().await[0].failures, 0);
        assert!(!lcd.provider_counters().await[0].circuit_open);
    }

    #[tokio::test]
    async fn recovery1464_total_deadline_preserves_observed_integrity_failure() {
        let wrong = MockServer::start().await;
        let slow = MockServer::start().await;
        mock_chain_identity(&wrong, "localterra").await;
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(Duration::from_secs(1)),
            )
            .mount(&slow)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![wrong.uri(), slow.uri()],
            1000,
            0,
            "columbus-5",
            ProviderPolicy {
                deadline_ms: 100,
                attempt_timeout_ms: 1000,
                ..Default::default()
            },
        );
        assert!(
            matches!(
                lcd.get::<serde_json::Value>("/data").await,
                Err(LcdError::ChainIdentity(_))
            ),
            "deadline cannot erase wrong-chain evidence"
        );
        assert_eq!(
            lcd.provider_counters()
                .await
                .iter()
                .map(|p| p.in_flight)
                .sum::<usize>(),
            0
        );
        assert_eq!(hits(&wrong, "/data").await, 0);
        assert_eq!(hits(&slow, "/data").await, 0);
    }

    #[tokio::test]
    async fn background_pacing_wait_does_not_hold_or_penalize_provider_capacity() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 200).await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1000,
            60_000,
            "columbus-5",
            ProviderPolicy {
                concurrency: 1,
                ..Default::default()
            },
        );
        let background = lcd.with_ingestion_budget();
        let budget = background.ingestion_budget.as_ref().unwrap().clone();
        *budget.next_request.lock().await = tokio::time::Instant::now() + Duration::from_secs(5);
        let pending =
            tokio::spawn(async move { background.get::<serde_json::Value>("/data").await });
        // Observe an acquired background lane before checking provider state.
        tokio::time::timeout(Duration::from_secs(1), async {
            while budget.permits.available_permits() == 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        assert_eq!(budget.permits.available_permits(), 2);
        let counters = &lcd.provider_counters().await[0];
        assert_eq!(counters.in_flight, 0);
        assert_eq!(counters.failures, 0);
        assert!(!counters.circuit_open);
        assert_eq!(hits(&server, "/data").await, 1);
    }

    #[tokio::test]
    async fn background_pacing_deadline_does_not_open_healthy_circuit() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 200).await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1000,
            60_000,
            "columbus-5",
            ProviderPolicy {
                deadline_ms: 40,
                ..Default::default()
            },
        );
        let background = lcd.with_ingestion_budget();
        let budget = background.ingestion_budget.as_ref().unwrap().clone();
        *budget.next_request.lock().await = tokio::time::Instant::now() + Duration::from_secs(5);
        assert!(matches!(
            background.get::<serde_json::Value>("/data").await,
            Err(LcdError::AdmissionDeferred)
        ));
        assert_eq!(budget.permits.available_permits(), 2);
        let counters = &lcd.provider_counters().await[0];
        assert_eq!(counters.attempts, 0);
        assert_eq!(counters.failures, 0);
        assert!(!counters.circuit_open);
        assert_eq!(hits(&server, "/data").await, 0);
        lcd.get::<serde_json::Value>("/data").await.unwrap();
    }

    #[tokio::test]
    async fn recovery1464_background_capacity_wait_resumes_after_busy_healthy_provider() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 200).await;
        Mock::given(path("/busy"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"ok":true}))
                    .set_delay(Duration::from_millis(300)),
            )
            .mount(&server)
            .await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1000,
            60_000,
            "columbus-5",
            ProviderPolicy {
                concurrency: 1,
                deadline_ms: 1000,
                attempt_timeout_ms: 800,
                ..Default::default()
            },
        );
        let foreground = lcd.clone();
        let busy = tokio::spawn(async move { foreground.get::<serde_json::Value>("/busy").await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while hits(&server, "/busy").await == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let background = lcd.with_ingestion_budget();
        let budget = background.ingestion_budget.as_ref().unwrap().clone();
        let pending = background.clone();
        let task = tokio::spawn(async move {
            pending.require_verified_chain().await?;
            pending.get::<serde_json::Value>("/data").await
        });
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert!(
            !task.is_finished(),
            "scheduled identity must wait instead of instant unavailable"
        );
        assert_eq!(
            budget.permits.available_permits(),
            2,
            "capacity wait stores no ingestion lane permit"
        );
        assert_eq!(lcd.provider_counters().await[0].in_flight, 1);
        busy.await.unwrap().unwrap();
        task.await.unwrap().unwrap();
        assert_eq!(hits(&server, "/data").await, 1);
        let counter = &lcd.provider_counters().await[0];
        assert!(counter.concurrency_deferrals > 0);
        assert_eq!(counter.in_flight, 0);
        assert_eq!(counter.failures, 0);
    }

    #[tokio::test]
    async fn recovery1464_background_capacity_deadline_and_cancel_add_no_attempt_penalty() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 200).await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1000,
            60_000,
            "columbus-5",
            ProviderPolicy {
                concurrency: 1,
                deadline_ms: 120,
                ..Default::default()
            },
        );
        let mut held = lcd.inner.endpoints.read().await[0]
            .provider
            .admit()
            .unwrap();
        held.completed();
        let background = lcd.with_ingestion_budget();
        for identity in [true, false] {
            let result = if identity {
                background.require_verified_chain().await
            } else {
                background
                    .get::<serde_json::Value>("/data")
                    .await
                    .map(|_| ())
            };
            assert!(matches!(result, Err(LcdError::AdmissionDeferred)));
        }
        let pending = background.clone();
        let task = tokio::spawn(async move { pending.get::<serde_json::Value>("/data").await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(
            background
                .ingestion_budget
                .as_ref()
                .unwrap()
                .permits
                .available_permits(),
            2
        );
        assert_eq!(
            lcd.provider_counters().await[0].attempts,
            1,
            "only preexisting holder admitted"
        );
        assert_eq!(lcd.provider_counters().await[0].failures, 0);
        assert_eq!(server.received_requests().await.unwrap().len(), 0);
        drop(held);
        assert_eq!(lcd.provider_counters().await[0].in_flight, 0);
    }

    #[tokio::test]
    async fn recovery1464_background_attempted_failures_never_reset_attempt_budget() {
        let server = MockServer::start().await;
        mock_chain_identity(&server, "columbus-5").await;
        payload(&server, 503).await;
        let lcd = LcdClient::new_with_policy(
            vec![server.uri()],
            1000,
            0,
            "columbus-5",
            ProviderPolicy {
                max_attempts: 1,
                deadline_ms: 1000,
                ..Default::default()
            },
        );
        let result = lcd
            .with_ingestion_budget()
            .get::<serde_json::Value>("/data")
            .await;
        assert!(matches!(result, Err(LcdError::AllEndpointsFailed(_))));
        assert_eq!(hits(&server, "/data").await, 1);
        assert_eq!(lcd.provider_counters().await[0].attempts, 1);
    }

    #[tokio::test]
    async fn recovery1464_circuit_skips_charge_only_actual_background_attempts() {
        let first = MockServer::start().await;
        let second = MockServer::start().await;
        let healthy = MockServer::start().await;
        mock_chain_identity(&healthy, "columbus-5").await;
        payload(&healthy, 200).await;
        let lcd = LcdClient::new_with_policy(
            vec![first.uri(), second.uri(), healthy.uri()],
            1000,
            60_000,
            "columbus-5",
            ProviderPolicy {
                deadline_ms: 400,
                ..Default::default()
            },
        );
        {
            let endpoints = lcd.inner.endpoints.read().await;
            endpoints[0].provider.failed(Duration::from_secs(60), None);
            endpoints[1].provider.failed(Duration::from_secs(60), None);
        }
        let background = lcd.with_ingestion_budget();
        let mut samples = Vec::new();
        for _ in 0..2 {
            let started = std::time::Instant::now();
            background.get::<serde_json::Value>("/data").await.unwrap();
            samples.push(started.elapsed());
        }
        assert!(
            samples
                .iter()
                .all(|elapsed| *elapsed < Duration::from_millis(400)),
            "denied circuits cannot consume the entire provider deadline"
        );
        assert!(
            samples[1] >= Duration::from_millis(200),
            "successful attempts must still obey shared pacing"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].attempts, 0);
        assert_eq!(counters[1].attempts, 0);
        assert_eq!(counters[2].attempts, 2);
        assert_eq!(counters[2].failures, 0);
        assert_eq!(counters[2].in_flight, 0);
        assert_eq!(hits(&healthy, "/data").await, 2);
    }
    #[tokio::test]
    async fn pinned_contract_snapshot_fails_over_and_requires_exact_height() {
        use wiremock::matchers::{header, query_param};
        let unsupported = MockServer::start().await;
        let good = MockServer::start().await;
        for server in [&unsupported, &good] {
            mock_chain_identity(server, "columbus-5").await;
        }
        for (server, response_height) in [(&unsupported, "322"), (&good, "321")] {
            Mock::given(path_regex("^/cosmwasm/"))
                .and(header("x-cosmos-block-height", "321"))
                .and(query_param("height", "321"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .insert_header("x-cosmos-block-height", response_height)
                        .set_body_json(json!({"data":{"reserve":"6000000"}})),
                )
                .mount(server)
                .await;
        }
        let lcd = LcdClient::new(
            vec![unsupported.uri(), good.uri()],
            1000,
            10_000,
            "columbus-5",
        );
        let data: serde_json::Value = lcd
            .query_contract_at_height_bounded("terra1pair", &json!({"pool":{}}), 321, 64 * 1024)
            .await
            .unwrap();
        assert_eq!(data["reserve"], "6000000");
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        assert_eq!(lcd.provider_counters().await[1].successes, 1);
        assert!(lcd
            .query_contract_at_height_bounded::<serde_json::Value>(
                "terra1pair",
                &json!({"pool":{}}),
                0,
                64 * 1024
            )
            .await
            .is_err());
        assert_eq!(lcd.provider_counters().await[1].attempts, 1);
        good.reset().await;
        mock_chain_identity(&good, "columbus-5").await;
        Mock::given(path_regex("^/cosmwasm/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data":{"reserve":"6000000"}})),
            )
            .mount(&good)
            .await;
        assert!(lcd
            .query_contract_at_height_bounded::<serde_json::Value>(
                "terra1pair",
                &json!({"pool":{}}),
                321,
                64 * 1024
            )
            .await
            .is_err());
    }
    #[tokio::test]
    async fn oversized_rate_limit_body_cannot_evade_circuit() {
        let bad = MockServer::start().await;
        let good = MockServer::start().await;
        for server in [&bad, &good] {
            mock_chain_identity(server, "columbus-5").await;
        }
        Mock::given(path_regex("^/cosmwasm/"))
            .respond_with(ResponseTemplate::new(429).set_body_string("x".repeat(65_537)))
            .mount(&bad)
            .await;
        Mock::given(path_regex("^/cosmwasm/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":{"ok":true}})))
            .mount(&good)
            .await;
        let lcd = LcdClient::new(vec![bad.uri(), good.uri()], 1000, 60_000, "columbus-5");
        let value: serde_json::Value = lcd
            .query_contract_bounded("terra1pair", &json!({"pool":{}}), 65_536)
            .await
            .unwrap();
        assert_eq!(value["ok"], true);
        assert_eq!(lcd.provider_counters().await[0].failures, 1);
        assert!(lcd.provider_counters().await[0].circuit_open);
    }
    #[tokio::test]
    async fn attempted_failure_overrides_capacity_and_open_circuits_stay_transient() {
        let healthy = MockServer::start().await;
        let bad = MockServer::start().await;
        for server in [&healthy, &bad] {
            mock_chain_identity(server, "columbus-5").await;
        }
        payload(&healthy, 200).await;
        payload(&bad, 503).await;
        let policy = ProviderPolicy {
            burst: 2,
            requests_per_second: 1,
            ..Default::default()
        };
        let lcd = LcdClient::new_with_policy(
            vec![healthy.uri(), bad.uri()],
            1000,
            60_000,
            "columbus-5",
            policy,
        );
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        assert!(matches!(
            lcd.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].rate_deferrals, 1);
        assert_eq!(counters[1].failures, 1);
        let only_bad = LcdClient::new(vec![bad.uri()], 1000, 60_000, "columbus-5");
        assert!(matches!(
            only_bad.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert!(matches!(
            only_bad.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert!(matches!(
            only_bad.require_verified_chain().await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert_eq!(only_bad.provider_counters().await[0].attempts, 1);
    }
}

#[cfg(test)]
mod deferred_recovery_tests;
#[cfg(test)]
mod parent_deadline_tests;
#[cfg(test)]
mod scheduled_continuation_tests;

#[cfg(test)]
mod local_ingestion_budget_tests {
    use super::*;
    use serde_json::Value;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    #[tokio::test]
    async fn scheduled_quote_ingestion_paces_attempts_and_bounds_response_bytes() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(wiremock::matchers::path("/observation"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"height":1})))
            .mount(&server)
            .await;
        Mock::given(wiremock::matchers::path("/oversized"))
            .respond_with(ResponseTemplate::new(200).set_body_string("x".repeat(65_537)))
            .mount(&server)
            .await;
        let client = LcdClient::new(vec![server.uri()], 1000, 1, "columbus-5");
        client.require_verified_chain().await.unwrap();
        let bounded = client.with_ingestion_budget();
        let start = std::time::Instant::now();
        let (a, b, c) = tokio::join!(
            bounded.get::<Value>("/observation"),
            bounded.get::<Value>("/observation"),
            bounded.get::<Value>("/observation")
        );
        assert!(a.is_ok() && b.is_ok() && c.is_ok());
        assert!(start.elapsed() >= Duration::from_millis(490));
        assert!(bounded.get::<Value>("/oversized").await.is_err());
    }
}
