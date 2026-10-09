//! Client-owned provider admission and circuit state (#1453). No waiting queue is retained.
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ProviderPolicy {
    pub concurrency: usize,
    pub requests_per_second: u32,
    pub burst: u32,
    pub max_attempts: usize,
    pub attempt_timeout_ms: u64,
    pub deadline_ms: u64,
    pub max_backoff_ms: u64,
    pub max_height_lag: i64,
}
impl Default for ProviderPolicy {
    fn default() -> Self {
        Self {
            concurrency: 8,
            requests_per_second: 64,
            burst: 128,
            max_attempts: 3,
            attempt_timeout_ms: 1000,
            deadline_ms: 4000,
            max_backoff_ms: 60_000,
            max_height_lag: 20,
        }
    }
}
impl ProviderPolicy {
    pub(crate) fn bounded(mut self) -> Self {
        self.concurrency = self.concurrency.clamp(1, 32);
        self.requests_per_second = self.requests_per_second.clamp(1, 1024);
        self.burst = self.burst.clamp(2, 1024);
        self.max_attempts = self.max_attempts.clamp(1, 8);
        self.attempt_timeout_ms = self.attempt_timeout_ms.clamp(10, 5000);
        self.deadline_ms = self.deadline_ms.clamp(10, 30_000);
        self.max_backoff_ms = self.max_backoff_ms.clamp(1, 300_000);
        self.max_height_lag = self.max_height_lag.clamp(1, 1000);
        self
    }
}

#[derive(Clone, Debug, Default, Serialize, utoipa::ToSchema)]
pub struct ProviderCounters {
    pub attempts: u64,
    pub successes: u64,
    pub failures: u64,
    pub rate_deferrals: u64,
    pub concurrency_deferrals: u64,
    pub circuit_deferrals: u64,
    pub recovery_probes: u64,
    pub in_flight: usize,
    pub consecutive_failures: u32,
    pub circuit_open: bool,
    pub half_open: bool,
}

struct State {
    tokens: f64,
    refilled_at: Instant,
    open_until: Option<Instant>,
    half_open: bool,
    counters: ProviderCounters,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProviderDenial {
    CircuitOpen,
    HalfOpenBusy,
    Concurrency,
    Rate,
}
impl ProviderDenial {
    pub fn is_capacity(self) -> bool {
        !matches!(self, Self::CircuitOpen)
    }
}

pub(super) struct ProviderState {
    policy: ProviderPolicy,
    base_backoff: Duration,
    state: Mutex<State>,
}
impl ProviderState {
    pub fn new(policy: ProviderPolicy, base_backoff: Duration) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                tokens: f64::from(policy.burst),
                refilled_at: Instant::now(),
                open_until: None,
                half_open: false,
                counters: ProviderCounters::default(),
            }),
            policy,
            base_backoff,
        })
    }
    pub fn admit(self: &Arc<Self>) -> Result<ProviderPermit, ProviderDenial> {
        self.admit_at(Instant::now())
    }
    fn admit_at(self: &Arc<Self>, now: Instant) -> Result<ProviderPermit, ProviderDenial> {
        let mut state = self.state.lock().unwrap();
        if state.half_open {
            state.counters.circuit_deferrals += 1;
            return Err(ProviderDenial::HalfOpenBusy);
        }
        if state.open_until.is_some_and(|until| now < until) {
            state.counters.circuit_deferrals += 1;
            return Err(ProviderDenial::CircuitOpen);
        }
        if state.counters.in_flight >= self.policy.concurrency {
            state.counters.concurrency_deferrals += 1;
            return Err(ProviderDenial::Concurrency);
        }
        let elapsed = now
            .saturating_duration_since(state.refilled_at)
            .as_secs_f64();
        state.tokens = (state.tokens + elapsed * f64::from(self.policy.requests_per_second))
            .min(f64::from(self.policy.burst));
        state.refilled_at = now;
        // Reserve both the identity HTTP read and the data HTTP read. Startup identity
        // probes reserve the same bound. Thus attestation cannot bypass the rate budget.
        if state.tokens < 2.0 {
            state.counters.rate_deferrals += 1;
            return Err(ProviderDenial::Rate);
        }
        state.tokens -= 2.0;
        let probe = state.open_until.is_some();
        state.half_open = probe;
        if probe {
            state.counters.recovery_probes += 1;
        }
        state.counters.in_flight += 1;
        state.counters.attempts += 1;
        Ok(ProviderPermit {
            provider: Arc::clone(self),
            probe,
            completed: false,
            failure_generation: state.counters.failures,
        })
    }
    pub fn failed(&self, base: Duration, retry_after: Option<Duration>) -> Duration {
        self.failed_at(base, retry_after, Instant::now())
    }
    fn failed_at(&self, base: Duration, retry_after: Option<Duration>, now: Instant) -> Duration {
        let mut state = self.state.lock().unwrap();
        state.counters.failures += 1;
        state.counters.consecutive_failures = state.counters.consecutive_failures.saturating_add(1);
        let cap = Duration::from_millis(self.policy.max_backoff_ms);
        let exponential = base
            .saturating_mul(
                1u32 << state
                    .counters
                    .consecutive_failures
                    .saturating_sub(1)
                    .min(12),
            )
            .min(cap);
        // Positive jitter, with a hard cap. A zero configured test cooldown remains zero.
        let jitter = if exponential.is_zero() {
            Duration::ZERO
        } else {
            Duration::from_millis(rand::random::<u64>() % (exponential.as_millis() as u64 / 4 + 1))
        };
        let delay = exponential
            .saturating_add(jitter)
            .max(retry_after.unwrap_or_default())
            .min(cap);
        let until = now + delay;
        state.open_until = Some(state.open_until.map_or(until, |old| old.max(until)));
        delay
    }
    #[cfg(test)]
    pub fn succeeded(&self) {
        let mut state = self.state.lock().unwrap();
        state.counters.successes += 1;
        state.counters.consecutive_failures = 0;
        state.open_until = None;
    }
    pub fn counters(&self) -> ProviderCounters {
        let state = self.state.lock().unwrap();
        let mut counters = state.counters.clone();
        counters.circuit_open = state.open_until.is_some();
        counters.half_open = state.half_open;
        counters
    }
}
/// Drop releases admission on errors, cancellation, deadline expiry and successful completion.
pub(super) struct ProviderPermit {
    provider: Arc<ProviderState>,
    probe: bool,
    completed: bool,
    failure_generation: u64,
}
impl ProviderPermit {
    pub fn completed(&mut self) {
        self.completed = true;
    }
    pub fn succeeded(&mut self) {
        self.completed = true;
        let mut state = self.provider.state.lock().unwrap();
        state.counters.successes += 1;
        // A response admitted before a concurrent failure cannot close the newly
        // opened circuit. Recovery requires the single admitted half-open probe.
        if self.failure_generation == state.counters.failures {
            state.counters.consecutive_failures = 0;
            state.open_until = None;
        }
    }
}
impl Drop for ProviderPermit {
    fn drop(&mut self) {
        if !self.completed {
            // Outer total deadlines and caller cancellation must not leave a slow provider
            // immediately available for a thundering herd of replacement reads.
            self.provider.failed(self.provider.base_backoff, None);
        }
        let mut state = self.provider.state.lock().unwrap();
        state.counters.in_flight -= 1;
        if self.probe {
            state.half_open = false;
        }
    }
}

pub(super) fn retry_after(headers: &reqwest::header::HeaderMap, cap: Duration) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    // Bound hostile strings before parsing; never retain response headers.
    if value.len() > 128 {
        return Some(cap);
    }
    let delay = if value.bytes().all(|c| c.is_ascii_digit()) {
        Duration::from_secs(value.parse::<u64>().unwrap_or(u64::MAX))
    } else {
        let date = chrono::DateTime::parse_from_rfc2822(value).ok()?;
        Duration::from_millis(
            date.signed_duration_since(chrono::Utc::now())
                .num_milliseconds()
                .max(0) as u64,
        )
    };
    Some(delay.min(cap))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_shared_rate_and_concurrency_release_on_drop() {
        let provider = ProviderState::new(
            ProviderPolicy {
                concurrency: 1,
                burst: 4,
                requests_per_second: 2,
                ..Default::default()
            },
            Duration::ZERO,
        );
        let now = Instant::now();
        let mut first = provider.admit_at(now).unwrap();
        assert!(provider.admit_at(now).is_err());
        first.completed();
        drop(first);
        let mut second = provider.admit_at(now).unwrap();
        second.completed();
        drop(second);
        assert!(provider.admit_at(now).is_err());
        let mut third = provider.admit_at(now + Duration::from_secs(1)).unwrap();
        third.completed();
        drop(third);
        let counters = provider.counters();
        assert_eq!(counters.in_flight, 0);
        assert_eq!(counters.concurrency_deferrals, 1);
        assert_eq!(counters.rate_deferrals, 1);
    }
    #[test]
    fn capped_backoff_single_probe_and_recovery() {
        let provider = ProviderState::new(
            ProviderPolicy {
                max_backoff_ms: 1000,
                ..Default::default()
            },
            Duration::ZERO,
        );
        let now = Instant::now();
        assert_eq!(
            provider.failed_at(
                Duration::from_secs(10),
                Some(Duration::from_secs(9999)),
                now
            ),
            Duration::from_secs(1)
        );
        assert!(provider.admit_at(now).is_err());
        let mut probe = provider.admit_at(now + Duration::from_secs(1)).unwrap();
        assert!(provider.admit_at(now + Duration::from_secs(1)).is_err());
        probe.completed();
        drop(probe);
        let mut probe = provider.admit_at(now + Duration::from_secs(1)).unwrap();
        provider.succeeded();
        probe.completed();
        drop(probe);
        let mut recovered = provider.admit_at(now + Duration::from_secs(1)).unwrap();
        recovered.completed();
        drop(recovered);
        assert!(!provider.counters().circuit_open);
        assert_eq!(provider.counters().recovery_probes, 2);
    }
    #[test]
    fn cancelled_half_open_probe_reopens_bounded_circuit() {
        let provider = ProviderState::new(
            ProviderPolicy {
                max_backoff_ms: 1000,
                ..Default::default()
            },
            Duration::from_secs(1),
        );
        let now = Instant::now();
        provider.failed_at(Duration::ZERO, None, now);
        let probe = provider.admit_at(now + Duration::from_millis(1)).unwrap();
        drop(probe);
        assert!(provider.admit_at(now + Duration::from_millis(2)).is_err());
        assert_eq!(provider.counters().failures, 2);
        assert_eq!(provider.counters().in_flight, 0);
        assert!(provider.counters().circuit_open);
    }
    #[test]
    fn success_admitted_before_failure_cannot_close_circuit() {
        let provider = ProviderState::new(ProviderPolicy::default(), Duration::from_secs(1));
        let now = Instant::now();
        let mut earlier = provider.admit_at(now).unwrap();
        provider.failed_at(Duration::from_secs(1), None, now);
        earlier.succeeded();
        drop(earlier);
        assert!(provider.counters().circuit_open);
        assert!(provider.admit_at(now).is_err());
        let mut recovery = provider.admit_at(now + Duration::from_secs(2)).unwrap();
        recovery.succeeded();
        drop(recovery);
        assert!(!provider.counters().circuit_open);
    }
    #[test]
    fn denial_reason_distinguishes_expired_circuit_rate_pressure() {
        let provider = ProviderState::new(
            ProviderPolicy {
                burst: 2,
                requests_per_second: 1,
                ..Default::default()
            },
            Duration::ZERO,
        );
        let now = Instant::now();
        let mut first = provider.admit_at(now).unwrap();
        first.completed();
        drop(first);
        provider.failed_at(Duration::ZERO, None, now);
        assert!(matches!(provider.admit_at(now), Err(ProviderDenial::Rate)));
        assert!(provider.counters().circuit_open);
        provider.failed_at(Duration::from_secs(1), None, now);
        assert!(matches!(
            provider.admit_at(now),
            Err(ProviderDenial::CircuitOpen)
        ));
    }
    #[test]
    fn retry_after_is_bounded_and_parses_dates() {
        let mut headers = reqwest::header::HeaderMap::new();
        for value in ["999999999999999999999999999", "900"] {
            headers.insert(reqwest::header::RETRY_AFTER, value.parse().unwrap());
            assert_eq!(
                retry_after(&headers, Duration::from_secs(60)),
                Some(Duration::from_secs(60))
            );
        }
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(
            retry_after(&headers, Duration::from_secs(60)),
            Some(Duration::ZERO)
        );
        headers.insert(reqwest::header::RETRY_AFTER, "broken".parse().unwrap());
        assert_eq!(retry_after(&headers, Duration::from_secs(60)), None);
    }
}
