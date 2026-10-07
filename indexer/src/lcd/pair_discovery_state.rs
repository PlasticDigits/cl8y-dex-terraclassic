//! Client-owned, factory-scoped pair-discovery admission and negative evidence.
//!
//! Every operation uses bounded state and a single short synchronous lock. A cache
//! lookup checks only its key. FIFO tracks slot admission, not expiry order: an
//! expired resident entry can refresh in place and be evicted early under pressure.
//! This keeps exactly one queue record per key without scans or tombstone growth.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub(crate) const MAX_FACTORIES: usize = 16;
pub(crate) const MAX_REJECTIONS_PER_FACTORY: usize = 512;
pub(crate) const MAX_KEY_BYTES: usize = 128;
pub(crate) const REJECTION_TTL: Duration = Duration::from_secs(3600);
const DISCOVERY_BURST: f64 = 32.0;
const DISCOVERY_REFILL_PER_SECOND: f64 = 8.0;
const MAX_IN_FLIGHT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    Unlisted,
    NotAPair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BeginError {
    Cached(RejectReason),
    Deferred(&'static str),
}

/// Outcomes of admitted work, separate from cache hits and admission deferrals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiscoveryOutcome {
    Unlisted,
    NotAPair,
    Transient,
    Listed,
}

/// Fixed-cardinality counters: addresses and factories never become metric labels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiscoveryCounters {
    pub admitted: u64,
    pub cached_unlisted: u64,
    pub cached_not_pair: u64,
    pub rejected_unlisted: u64,
    pub rejected_not_pair: u64,
    pub transient_failures: u64,
    pub discovered: u64,
    pub deferred_invalid_key: u64,
    pub deferred_factory_capacity: u64,
    pub deferred_in_flight: u64,
    pub deferred_coalesced: u64,
    pub deferred_rate_limited: u64,
}

struct Rejection {
    reason: RejectReason,
    inserted_at: Instant,
}

struct FactoryState {
    rejections: HashMap<String, Rejection>,
    insertion_fifo: VecDeque<String>,
    in_flight: HashSet<String>,
    tokens: f64,
    last_refill: Instant,
}

impl FactoryState {
    fn new(now: Instant) -> Self {
        Self {
            rejections: HashMap::new(),
            insertion_fifo: VecDeque::new(),
            in_flight: HashSet::new(),
            tokens: DISCOVERY_BURST,
            last_refill: now,
        }
    }

    fn cached(&self, pair: &str, now: Instant) -> Option<RejectReason> {
        self.rejections.get(pair).and_then(|entry| {
            (now.saturating_duration_since(entry.inserted_at) < REJECTION_TTL)
                .then_some(entry.reason)
        })
    }

    fn insert_rejection(&mut self, pair: &str, reason: RejectReason, now: Instant) {
        if let Some(entry) = self.rejections.get_mut(pair) {
            // Duplicate evidence does not extend a live rejection's fixed TTL.
            // Expired entries retain their FIFO slot when refreshed.
            if now.saturating_duration_since(entry.inserted_at) >= REJECTION_TTL {
                *entry = Rejection {
                    reason,
                    inserted_at: now,
                };
            }
            return;
        }
        if self.rejections.len() == MAX_REJECTIONS_PER_FACTORY {
            if let Some(oldest) = self.insertion_fifo.pop_front() {
                self.rejections.remove(&oldest);
            }
        }
        self.insertion_fifo.push_back(pair.to_owned());
        self.rejections.insert(
            pair.to_owned(),
            Rejection {
                reason,
                inserted_at: now,
            },
        );
    }

    fn refill(&mut self, now: Instant) {
        self.tokens = (self.tokens
            + now
                .saturating_duration_since(self.last_refill)
                .as_secs_f64()
                * DISCOVERY_REFILL_PER_SECOND)
            .min(DISCOVERY_BURST);
        self.last_refill = self.last_refill.max(now);
    }
}

#[derive(Default)]
struct State {
    // Registered factories are never evicted: address churn cannot reset a budget.
    factories: HashMap<String, FactoryState>,
    counters: DiscoveryCounters,
}

pub(crate) struct PairDiscoveryState {
    state: Mutex<State>,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

impl Default for PairDiscoveryState {
    fn default() -> Self {
        Self::with_clock(Arc::new(Instant::now))
    }
}

impl PairDiscoveryState {
    fn with_clock(clock: Arc<dyn Fn() -> Instant + Send + Sync>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            clock,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn counters(&self) -> DiscoveryCounters {
        self.lock().counters
    }

    pub(crate) fn record_outcome(&self, outcome: DiscoveryOutcome) {
        let mut state = self.lock();
        let counter = match outcome {
            DiscoveryOutcome::Unlisted => &mut state.counters.rejected_unlisted,
            DiscoveryOutcome::NotAPair => &mut state.counters.rejected_not_pair,
            DiscoveryOutcome::Transient => &mut state.counters.transient_failures,
            DiscoveryOutcome::Listed => &mut state.counters.discovered,
        };
        *counter = counter.saturating_add(1);
    }

    /// Atomically check negative evidence and reserve all discovery budgets.
    pub(crate) fn begin(
        self: &Arc<Self>,
        factory: &str,
        pair: &str,
    ) -> Result<DiscoveryPermit, BeginError> {
        let mut state = self.lock();
        let now = (self.clock)();
        if factory.is_empty()
            || pair.is_empty()
            || factory.len() > MAX_KEY_BYTES
            || pair.len() > MAX_KEY_BYTES
        {
            state.counters.deferred_invalid_key =
                state.counters.deferred_invalid_key.saturating_add(1);
            return Err(BeginError::Deferred("invalid_key"));
        }
        if !state.factories.contains_key(factory) {
            if state.factories.len() == MAX_FACTORIES {
                state.counters.deferred_factory_capacity =
                    state.counters.deferred_factory_capacity.saturating_add(1);
                return Err(BeginError::Deferred("factory_capacity"));
            }
            state
                .factories
                .insert(factory.to_owned(), FactoryState::new(now));
        }
        let factory_state = state
            .factories
            .get_mut(factory)
            .expect("registered factory");
        if let Some(reason) = factory_state.cached(pair, now) {
            let counter = match reason {
                RejectReason::Unlisted => &mut state.counters.cached_unlisted,
                RejectReason::NotAPair => &mut state.counters.cached_not_pair,
            };
            *counter = counter.saturating_add(1);
            return Err(BeginError::Cached(reason));
        }
        if factory_state.in_flight.contains(pair) {
            state.counters.deferred_coalesced = state.counters.deferred_coalesced.saturating_add(1);
            return Err(BeginError::Deferred("coalesced"));
        }
        if factory_state.in_flight.len() == MAX_IN_FLIGHT {
            state.counters.deferred_in_flight = state.counters.deferred_in_flight.saturating_add(1);
            return Err(BeginError::Deferred("in_flight"));
        }
        factory_state.refill(now);
        if factory_state.tokens < 1.0 {
            state.counters.deferred_rate_limited =
                state.counters.deferred_rate_limited.saturating_add(1);
            return Err(BeginError::Deferred("rate_limited"));
        }
        factory_state.tokens -= 1.0;
        factory_state.in_flight.insert(pair.to_owned());
        state.counters.admitted = state.counters.admitted.saturating_add(1);
        Ok(DiscoveryPermit {
            state: Arc::clone(self),
            factory: factory.to_owned(),
            pair: pair.to_owned(),
        })
    }

    /// Only an admitted operation can insert evidence, and only for its context.
    pub(crate) fn insert_rejection(&self, permit: &DiscoveryPermit, reason: RejectReason) {
        if !std::ptr::eq(self, Arc::as_ptr(&permit.state)) {
            return;
        }
        let mut state = self.lock();
        let now = (self.clock)();
        if let Some(factory) = state.factories.get_mut(&permit.factory) {
            factory.insert_rejection(&permit.pair, reason, now);
        }
    }
}

/// Releases in-flight capacity on success, error, panic, or async cancellation.
pub(crate) struct DiscoveryPermit {
    state: Arc<PairDiscoveryState>,
    factory: String,
    pair: String,
}

impl Drop for DiscoveryPermit {
    fn drop(&mut self) {
        if let Some(factory) = self.state.lock().factories.get_mut(&self.factory) {
            factory.in_flight.remove(&self.pair);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct ManualClock {
        start: Instant,
        millis: AtomicU64,
    }

    impl ManualClock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                start: Instant::now(),
                millis: AtomicU64::new(0),
            })
        }

        fn advance(&self, duration: Duration) {
            self.millis
                .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        }

        fn now(&self) -> Instant {
            self.start + Duration::from_millis(self.millis.load(Ordering::Relaxed))
        }

        fn state(self: &Arc<Self>) -> Arc<PairDiscoveryState> {
            let clock = Arc::clone(self);
            Arc::new(PairDiscoveryState::with_clock(Arc::new(move || {
                clock.now()
            })))
        }
    }

    fn assert_queue_consistent(factory: &FactoryState) {
        assert!(factory.rejections.len() <= MAX_REJECTIONS_PER_FACTORY);
        assert_eq!(factory.rejections.len(), factory.insertion_fifo.len());
        let queued: HashSet<_> = factory.insertion_fifo.iter().collect();
        assert_eq!(queued.len(), factory.insertion_fifo.len());
        assert!(queued
            .iter()
            .all(|key| factory.rejections.contains_key(*key)));
    }

    #[test]
    fn fixed_ttl_expiry_is_exact_and_refresh_keeps_one_fifo_slot() {
        let clock = ManualClock::new();
        let state = clock.state();
        let permit = state.begin("factory", "pair").unwrap();
        state.insert_rejection(&permit, RejectReason::Unlisted);
        clock.advance(REJECTION_TTL - Duration::from_millis(1));
        state.insert_rejection(&permit, RejectReason::NotAPair);
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Cached(RejectReason::Unlisted))
        ));
        drop(permit);
        clock.advance(Duration::from_millis(1));
        let refreshed = state.begin("factory", "pair").unwrap();
        state.insert_rejection(&refreshed, RejectReason::NotAPair);
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Cached(RejectReason::NotAPair))
        ));
        let inner = state.lock();
        let factory = &inner.factories["factory"];
        assert_queue_consistent(factory);
        assert_eq!(factory.insertion_fifo.len(), 1);
    }

    #[test]
    fn targeted_expiry_leaves_other_entries_and_queue_untouched() {
        let clock = ManualClock::new();
        let state = clock.state();
        for pair in ["old", "other"] {
            let permit = state.begin("factory", pair).unwrap();
            state.insert_rejection(&permit, RejectReason::Unlisted);
        }
        clock.advance(REJECTION_TTL);
        let permit = state.begin("factory", "old").unwrap();
        let inner = state.lock();
        assert_eq!(inner.factories["factory"].rejections.len(), 2);
        assert_queue_consistent(&inner.factories["factory"]);
        drop(inner);
        drop(permit);
    }

    #[test]
    fn duplicate_inserts_never_grow_queue_or_refresh_live_ttl() {
        let clock = ManualClock::new();
        let state = clock.state();
        let permit = state.begin("factory", "pair").unwrap();
        for _ in 0..10_000 {
            state.insert_rejection(&permit, RejectReason::Unlisted);
        }
        let inner = state.lock();
        assert_queue_consistent(&inner.factories["factory"]);
        assert_eq!(inner.factories["factory"].rejections.len(), 1);
    }

    #[test]
    fn capacity_churn_100k_is_bounded_and_fifo_evicts_oldest() {
        let clock = ManualClock::new();
        let state = clock.state();
        for i in 0..100_000 {
            clock.advance(Duration::from_millis(125));
            let factory = format!("factory-{}", i % MAX_FACTORIES);
            let pair = format!("pair-{i}");
            let permit = state.begin(&factory, &pair).unwrap();
            state.insert_rejection(&permit, RejectReason::Unlisted);
            if i % 1_000 == 0 {
                let inner = state.lock();
                for factory in inner.factories.values() {
                    assert_queue_consistent(factory);
                }
            }
        }
        let inner = state.lock();
        assert_eq!(inner.factories.len(), MAX_FACTORIES);
        assert_eq!(
            inner
                .factories
                .values()
                .map(|factory| factory.rejections.len())
                .sum::<usize>(),
            8192
        );
        for factory in inner.factories.values() {
            assert_queue_consistent(factory);
            assert!(!factory.rejections.contains_key("pair-0"));
        }
        assert!(inner.factories["factory-15"]
            .rejections
            .contains_key("pair-99999"));
    }

    #[test]
    fn factory_slots_and_key_lengths_are_bounded_without_budget_reset() {
        let clock = ManualClock::new();
        let state = clock.state();
        assert!(matches!(
            state.begin(&"f".repeat(MAX_KEY_BYTES + 1), "pair"),
            Err(BeginError::Deferred("invalid_key"))
        ));
        assert!(matches!(
            state.begin("factory", &"p".repeat(MAX_KEY_BYTES + 1)),
            Err(BeginError::Deferred("invalid_key"))
        ));
        assert!(matches!(
            state.begin("", "pair"),
            Err(BeginError::Deferred("invalid_key"))
        ));
        assert!(state.lock().factories.is_empty());
        for i in 0..MAX_FACTORIES {
            drop(state.begin(&format!("factory-{i}"), "pair").unwrap());
        }
        for i in 0..10_000 {
            assert!(matches!(
                state.begin(&format!("unknown-{i}"), "pair"),
                Err(BeginError::Deferred("factory_capacity"))
            ));
        }
        for _ in 1..32 {
            drop(state.begin("factory-0", "uncached").unwrap());
        }
        assert!(matches!(
            state.begin("factory-0", "uncached"),
            Err(BeginError::Deferred("rate_limited"))
        ));
        assert_eq!(state.lock().factories.len(), MAX_FACTORIES);
    }

    #[test]
    fn admission_coalesces_limits_in_flight_and_drop_releases_capacity() {
        let clock = ManualClock::new();
        let state = clock.state();
        let mut permits = Vec::new();
        for i in 0..MAX_IN_FLIGHT {
            permits.push(state.begin("factory", &format!("pair-{i}")).unwrap());
        }
        assert!(matches!(
            state.begin("factory", "pair-0"),
            Err(BeginError::Deferred("coalesced"))
        ));
        assert!(matches!(
            state.begin("factory", "another"),
            Err(BeginError::Deferred("in_flight"))
        ));
        drop(state.begin("another-factory", "pair-0").unwrap());
        permits.pop();
        let replacement = state.begin("factory", "another").unwrap();
        drop(replacement);
        drop(permits);
        assert!(state.lock().factories["factory"].in_flight.is_empty());
    }

    #[tokio::test]
    async fn async_cancellation_releases_permit_without_refunding_rate_budget() {
        let clock = ManualClock::new();
        let state = clock.state();
        let worker_state = Arc::clone(&state);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _permit = worker_state.begin("factory", "pair").unwrap();
            ready_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready_rx.await.unwrap();
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Deferred("coalesced"))
        ));
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        drop(state.begin("factory", "pair").unwrap());
        let inner = state.lock();
        assert!(inner.factories["factory"].in_flight.is_empty());
        assert_eq!(inner.factories["factory"].tokens, 30.0);
    }

    #[test]
    fn tokens_replenish_at_eight_per_second_and_cap_at_burst() {
        let clock = ManualClock::new();
        let state = clock.state();
        for _ in 0..32 {
            drop(state.begin("factory", "pair").unwrap());
        }
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Deferred("rate_limited"))
        ));
        clock.advance(Duration::from_millis(124));
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Deferred("rate_limited"))
        ));
        clock.advance(Duration::from_millis(1));
        drop(state.begin("factory", "pair").unwrap());
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Deferred("rate_limited"))
        ));
        clock.advance(Duration::from_secs(60));
        for _ in 0..32 {
            drop(state.begin("factory", "pair").unwrap());
        }
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Deferred("rate_limited"))
        ));
    }

    #[test]
    fn factories_isolate_cache_rate_and_in_flight_budgets() {
        let clock = ManualClock::new();
        let state = clock.state();
        let permit = state.begin("busy", "pair").unwrap();
        state.insert_rejection(&permit, RejectReason::Unlisted);
        drop(permit);
        for _ in 1..32 {
            drop(state.begin("busy", "other").unwrap());
        }
        assert!(matches!(
            state.begin("busy", "other"),
            Err(BeginError::Deferred("rate_limited"))
        ));
        let permit = state.begin("quiet", "pair").unwrap();
        state.insert_rejection(&permit, RejectReason::NotAPair);
        assert!(matches!(
            state.begin("busy", "pair"),
            Err(BeginError::Cached(RejectReason::Unlisted))
        ));
        assert!(matches!(
            state.begin("quiet", "pair"),
            Err(BeginError::Cached(RejectReason::NotAPair))
        ));
    }

    #[test]
    fn permit_cannot_insert_evidence_into_another_client_state() {
        let clock = ManualClock::new();
        let first = clock.state();
        let second = clock.state();
        let permit = first.begin("factory", "pair").unwrap();
        second.insert_rejection(&permit, RejectReason::Unlisted);
        assert!(second.lock().factories.is_empty());
    }

    #[test]
    fn client_clones_share_state_new_clients_start_fresh() {
        use crate::lcd::LcdClient;
        let first = LcdClient::new(vec!["http://127.0.0.1:1".into()], 10, 10, "columbus-5");
        let clone = first.clone();
        let second = LcdClient::new(vec!["http://127.0.0.1:1".into()], 10, 10, "columbus-5");
        let changed = LcdClient::new(vec!["http://127.0.0.1:2".into()], 10, 10, "columbus-5");
        assert!(Arc::ptr_eq(
            &first.pair_discovery_state(),
            &clone.pair_discovery_state()
        ));
        for client in [second, changed] {
            assert!(!Arc::ptr_eq(
                &first.pair_discovery_state(),
                &client.pair_discovery_state()
            ));
        }
        let state = first.pair_discovery_state();
        let permit = state.begin("factory", "pair").unwrap();
        state.insert_rejection(&permit, RejectReason::Unlisted);
        assert!(matches!(
            clone.pair_discovery_state().begin("factory", "pair"),
            Err(BeginError::Cached(RejectReason::Unlisted))
        ));
    }

    #[test]
    fn concurrent_inserts_and_lookups_keep_bounded_consistent_state() {
        let start = Instant::now();
        let ticks = AtomicU64::new(0);
        let state = Arc::new(PairDiscoveryState::with_clock(Arc::new(move || {
            start + Duration::from_millis(ticks.fetch_add(125, Ordering::Relaxed))
        })));
        let threads: Vec<_> = (0..8)
            .map(|worker| {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    let factory = format!("factory-{worker}");
                    // All clock ticks across threads remain below one TTL,
                    // even if a worker is descheduled until the others finish.
                    for i in 0..1_000 {
                        let pair = format!("pair-{i}");
                        let permit = state.begin(&factory, &pair).unwrap();
                        state.insert_rejection(&permit, RejectReason::Unlisted);
                        assert!(matches!(
                            state.begin(&factory, &pair),
                            Err(BeginError::Cached(RejectReason::Unlisted))
                        ));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let inner = state.lock();
        assert_eq!(inner.factories.len(), 8);
        for factory in inner.factories.values() {
            assert_queue_consistent(factory);
            assert!(factory.in_flight.is_empty());
        }
        assert_eq!(inner.counters.admitted, 8_000);
        assert_eq!(inner.counters.cached_unlisted, 8_000);
    }

    #[test]
    fn counters_have_only_fixed_outcome_fields() {
        let clock = ManualClock::new();
        let state = clock.state();
        let permit = state.begin("factory", "pair").unwrap();
        state.insert_rejection(&permit, RejectReason::Unlisted);
        assert!(matches!(
            state.begin("factory", "pair"),
            Err(BeginError::Cached(RejectReason::Unlisted))
        ));
        assert!(matches!(
            state.begin("", "pair"),
            Err(BeginError::Deferred("invalid_key"))
        ));
        assert_eq!(
            state.counters(),
            DiscoveryCounters {
                admitted: 1,
                cached_unlisted: 1,
                deferred_invalid_key: 1,
                ..DiscoveryCounters::default()
            }
        );
    }

    #[test]
    fn admitted_outcomes_have_separate_fixed_saturating_counters() {
        let clock = ManualClock::new();
        let state = clock.state();
        for outcome in [
            DiscoveryOutcome::Unlisted,
            DiscoveryOutcome::NotAPair,
            DiscoveryOutcome::Transient,
            DiscoveryOutcome::Listed,
        ] {
            state.record_outcome(outcome);
        }
        assert_eq!(
            state.counters(),
            DiscoveryCounters {
                rejected_unlisted: 1,
                rejected_not_pair: 1,
                transient_failures: 1,
                discovered: 1,
                ..DiscoveryCounters::default()
            }
        );
        state.lock().counters.transient_failures = u64::MAX;
        state.record_outcome(DiscoveryOutcome::Transient);
        assert_eq!(state.counters().transient_failures, u64::MAX);
    }

    /// Reproducible baseline/saturated-cache measurements; no timing threshold.
    #[test]
    #[ignore = "manual cache benchmark"]
    fn pair_discovery_cache_benchmark() {
        let now = Instant::now();
        for entries in [1, MAX_REJECTIONS_PER_FACTORY] {
            let mut factory = FactoryState::new(now);
            for i in 0..entries {
                factory.insert_rejection(&format!("pair-{i}"), RejectReason::Unlisted, now);
            }
            let started = Instant::now();
            for _ in 0..1_000_000 {
                assert!(std::hint::black_box(factory.cached("pair-0", now)).is_some());
                assert!(std::hint::black_box(factory.cached("missing", now)).is_none());
            }
            eprintln!(
                "pair-discovery cache: entries={entries} targeted_lookups=2000000 elapsed={:?}",
                started.elapsed()
            );
            assert_queue_consistent(&factory);
        }
        let mut factory = FactoryState::new(now);
        let started = Instant::now();
        for i in 0..100_000 {
            factory.insert_rejection(&format!("pair-{i}"), RejectReason::Unlisted, now);
        }
        eprintln!(
            "pair-discovery cache: capacity_churn=100000 retained={} elapsed={:?}",
            factory.rejections.len(),
            started.elapsed()
        );
        assert_queue_consistent(&factory);
        assert_eq!(factory.rejections.len(), MAX_REJECTIONS_PER_FACTORY);
    }
}
