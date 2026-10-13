//! FIFO deeper discovery with fixed workers and bounded, coalesced pending intents.
//! Queued work retains its original cohort/expiry. Completion or cancellation
//! releases a slot and drains the queue without another client request or RPC.
use super::*;
const MAX_PENDING_BYTES: usize = 16 * 1024 * 1024;

pub(super) struct Work {
    pub key: String,
    namespace: String,
    snapshot: Arc<Snapshot>,
    request: route_solver::SolveRouteParams,
    native: Option<NativeContext>,
    start: i32,
    goal: i32,
    spread: u128,
    amount: u128,
    expires_at_ms: i64,
    observed_at_ms: i64,
}
impl Work {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: String,
        namespace: &str,
        snapshot: &Arc<Snapshot>,
        request: &route_solver::SolveRouteParams,
        native: Option<&NativeContext>,
        start: i32,
        goal: i32,
        spread: u128,
        body: &Value,
    ) -> Self {
        Self {
            key,
            namespace: namespace.into(),
            snapshot: Arc::clone(snapshot),
            request: request.clone(),
            native: native.cloned(),
            start,
            goal,
            spread,
            amount: body["execution_metadata"]["wrapper"]["route_input_amount"]
                .as_str()
                .or_else(|| body["execution_metadata"]["input_amount"].as_str())
                .and_then(|raw| raw.parse().ok())
                .unwrap_or(0),
            observed_at_ms: body["execution_metadata"]["observed_at_ms"]
                .as_i64()
                .unwrap_or(0),
            expires_at_ms: body["execution_metadata"]["expires_at_ms"]
                .as_i64()
                .unwrap_or(0),
        }
    }
    fn retained_bytes(&self) -> usize {
        // Conservative bounded-accounting estimate includes owned contexts and
        // pinned book observations, even when Arcs happen to share them.
        let s = &self.snapshot;
        4096usize
            .saturating_mul(s.pairs.len())
            .saturating_add(1024usize.saturating_mul(
                s.addresses.len()
                    + s.references.prices.len()
                    + s.references.policy_exclusions.len(),
            ))
            .saturating_add(512usize.saturating_mul(s.actors.len()))
            .saturating_add(
                s.protocol_actor_tokens
                    .values()
                    .map(|tokens| 512usize.saturating_mul(tokens.len()))
                    .fold(0usize, usize::saturating_add),
            )
            .saturating_add(
                s.taxes
                    .values()
                    .map(|tax| 1024usize.saturating_add(512usize.saturating_mul(tax.actors.len())))
                    .fold(0usize, usize::saturating_add),
            )
            .saturating_add(
                s.mirrors
                    .values()
                    .chain(s.pool_mirrors.values())
                    .map(|mirror| 512usize.saturating_mul(mirror.bids.len() + mirror.asks.len()))
                    .fold(0usize, usize::saturating_add),
            )
            .saturating_add(4096)
    }
    fn current(&self, g: &Store) -> bool {
        self.amount > 0
            && now_ms() < self.expires_at_ms
            && g.results.contains_key(&self.key)
            && reviewed_snapshot_current(g, &self.namespace, &self.snapshot, self.native.as_ref())
    }
    fn run(self) {
        let _job = JobGuard(self.key.clone());
        let discovery = skeletons::discovery(&self.snapshot, self.start, self.goal, 4);
        let book_allowed = self.native.is_none()
            && self.request.pool_only != Some(true)
            && self.request.hybrid_optimize != Some(false);
        let maker_fills =
            crate::hybrid_limits::clamp_max_maker_fills(self.request.max_maker_fills.unwrap_or(8));
        let selected = select_detailed(
            &self.snapshot,
            &discovery.paths,
            self.amount,
            self.spread,
            book_allowed,
            maker_fills,
            None,
            self.native.as_ref(),
        )
        .executable;
        let mut g = store().lock().unwrap();
        if !self.current(&g) {
            tracing::info!(target: "route_discovery", stage = "discarded", reason = "cohort_or_expiry", "bounded route discovery");
            return;
        }
        let mut promoted = false;
        // Mark completion even when no improvement exists; the original quote
        // remains usable. Compare to the current cache, never an old baseline.
        let previous = g.results.get_mut(&self.key).unwrap();
        discovery.annotate(previous, true);
        previous["paths_considered"] = json!(discovery.paths.len());
        let best_output = previous["estimated_amount_out_net"]
            .as_str()
            .and_then(|raw| raw.parse::<u128>().ok())
            .unwrap_or(0);
        if let Some(candidate) = selected {
            if let Some(mut body) = final_response(
                &self.snapshot,
                &self.request,
                &candidate,
                self.amount,
                self.spread,
                discovery.paths.len(),
                self.native.as_ref(),
            ) {
                let output = body["estimated_amount_out_net"]
                    .as_str()
                    .and_then(|raw| raw.parse::<u128>().ok())
                    .unwrap_or(0);
                if output > best_output
                    && body["execution_metadata"]["expires_at_ms"]
                        .as_i64()
                        .is_some_and(|expiry| expiry > now_ms())
                {
                    // A fresher alternative cannot renew the first quote's
                    // original cohort deadline. Review stays pinned externally;
                    // all advisory attribution timestamps follow this binding.
                    let observed = body["execution_metadata"]["observed_at_ms"]
                        .as_i64()
                        .unwrap_or(0)
                        .min(self.observed_at_ms);
                    let expiry = body["execution_metadata"]["expires_at_ms"]
                        .as_i64()
                        .unwrap_or(0)
                        .min(self.expires_at_ms);
                    body["snapshot_age_ms"] = json!(now_ms().saturating_sub(observed));
                    body["execution_metadata"]["observed_at_ms"] = json!(observed);
                    body["execution_metadata"]["expires_at_ms"] = json!(expiry);
                    if body["economic_attribution"]["status"] == "available" {
                        body["economic_attribution"]["observed_at_ms"] = json!(observed);
                        body["economic_attribution"]["expires_at_ms"] = json!(expiry);
                    }
                    discovery.annotate(&mut body, true);
                    g.results.insert(self.key.clone(), body);
                    promoted = true;
                }
            }
        }
        tracing::info!(target: "route_discovery", stage = if promoted { "promoted" } else { "evaluated" }, paths = discovery.paths.len(), truncated = discovery.truncated, "bounded route discovery");
    }
}

pub(super) fn needs_work(g: &Store, key: &str) -> bool {
    !g.jobs.contains(key) && !g.deferred_jobs.iter().any(|work| work.key == key)
}
pub(super) fn enqueue(g: &mut Store, work: Work) {
    // Purge expired/removed work cheaply before counting pending capacity.
    // Full generation validation occurs before each launch and publication;
    // unrelated hot requests never hash every queued economic cohort.
    // Coalescing prevents a hot intent from displacing older waiting intents.
    let pending = std::mem::take(&mut g.deferred_jobs);
    let at = now_ms();
    g.deferred_jobs = pending
        .into_iter()
        .filter(|work| work.expires_at_ms > at && g.results.contains_key(&work.key))
        .collect();
    // The memory budget constrains waiting work. Existing workers may consume
    // a qualified large snapshot without reserving a new waiting slot. Drain
    // older FIFO entries first, then admit directly if original capacity is free.
    drain(g);
    if needs_work(g, &work.key) && g.jobs.len() < MAX_JOBS && work.current(g) {
        launch(g, work);
        return;
    }
    let pending_bytes = g
        .deferred_jobs
        .iter()
        .map(Work::retained_bytes)
        .fold(0usize, usize::saturating_add);
    if needs_work(g, &work.key)
        && g.deferred_jobs.len() < MAX_RESULTS
        && pending_bytes.saturating_add(work.retained_bytes()) <= MAX_PENDING_BYTES
        && work.current(g)
    {
        g.deferred_jobs.push_back(work);
        tracing::info!(target: "route_discovery", stage = "queued", active = g.jobs.len(), pending = g.deferred_jobs.len(), "bounded route discovery");
    }
    drain(g);
}
pub(super) fn drain(g: &mut Store) {
    while g.jobs.len() < MAX_JOBS {
        let Some(work) = g.deferred_jobs.pop_front() else {
            break;
        };
        if !work.current(g) {
            continue;
        }
        launch(g, work);
    }
}
fn launch(g: &mut Store, work: Work) {
    if !g.jobs.insert(work.key.clone()) {
        return;
    }
    g.started += 1;
    tokio::task::spawn_blocking(move || work.run());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1489_fifo_coalesces_distinct_intents_and_culls_expired_or_changed_cohorts() {
        let mut source = super::super::tests::fixture();
        source.namespace = "queue1489".into();
        let mut g = Store::default();
        g.snapshots
            .insert(source.namespace.clone(), Arc::new(source.clone()));
        g.jobs
            .extend((0..MAX_JOBS).map(|index| format!("occupied-{index}")));
        let reviewed = Arc::new(protocol_guards::derive(&g, &source.namespace, &source));
        let q = super::super::tests::request(10u128.pow(18));
        let body = json!({"execution_metadata":{"input_amount":q.amount_in,"expires_at_ms":now_ms()+10_000}});
        for key in ["first", "second", "first"] {
            g.results.insert(key.into(), body.clone());
            enqueue(
                &mut g,
                Work::new(
                    key.into(),
                    &source.namespace,
                    &reviewed,
                    &q,
                    None,
                    1,
                    2,
                    SCALE / 20,
                    &body,
                ),
            );
        }
        assert_eq!(
            g.deferred_jobs
                .iter()
                .map(|work| work.key.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(g.jobs.len(), MAX_JOBS);
        // Ingestion ownership alone is not an economic change.
        let mut refreshed = source.clone();
        refreshed.config_epoch += 1;
        g.snapshots
            .insert(source.namespace.clone(), Arc::new(refreshed.clone()));
        assert!(g.deferred_jobs.iter().all(|work| work.current(&g)));
        // FIFO capacity does not retain expired jobs or removed cache entries.
        g.deferred_jobs.front_mut().unwrap().expires_at_ms = now_ms() - 1;
        g.results.insert("third".into(), body.clone());
        enqueue(
            &mut g,
            Work::new(
                "third".into(),
                &source.namespace,
                &reviewed,
                &q,
                None,
                1,
                2,
                SCALE / 20,
                &body,
            ),
        );
        assert_eq!(
            g.deferred_jobs
                .iter()
                .map(|work| work.key.as_str())
                .collect::<Vec<_>>(),
            vec!["second", "third"]
        );
        g.results.remove("second");
        assert!(!g.deferred_jobs.front().unwrap().current(&g));
        // A changed policy/reference/reserve generation invalidates every old
        // job. Its result can never be rebranded with the newer generation.
        refreshed.generation.push_str("-changed-economics");
        g.snapshots
            .insert(source.namespace.clone(), Arc::new(refreshed));
        assert!(g.deferred_jobs.iter().all(|work| !work.current(&g)));
        drain(&mut g); // no free slot, remains bounded and does not spawn
        assert_eq!(g.jobs.len(), MAX_JOBS);
    }
}

#[cfg(test)]
mod expiry_tests {
    use super::*;
    #[tokio::test]
    async fn issue1489_expired_active_job_never_promotes_or_renews_original_quote() {
        let namespace = "expired-active1489";
        let mut source = super::super::tests::fixture();
        source.namespace = namespace.into();
        let q = super::super::tests::request(10u128.pow(18));
        let key = "expired-active1489-key";
        let body = json!({"estimated_amount_out_net":"1","search_complete":false,"execution_metadata":{"input_amount":q.amount_in,"observed_at_ms":now_ms()-20_000,"expires_at_ms":now_ms()-1}});
        let reviewed = {
            let mut g = store().lock().unwrap();
            g.snapshots
                .insert(namespace.into(), Arc::new(source.clone()));
            g.results.insert(key.into(), body.clone());
            g.jobs.insert(key.into());
            Arc::new(protocol_guards::derive(&g, namespace, &source))
        };
        let work = Work::new(
            key.into(),
            namespace,
            &reviewed,
            &q,
            None,
            1,
            2,
            SCALE / 20,
            &body,
        );
        work.run();
        let mut g = store().lock().unwrap();
        assert_eq!(
            g.results[key], body,
            "expired work cannot publish fresher candidate"
        );
        assert!(!g.jobs.contains(key));
        g.results.remove(key);
        g.snapshots.remove(namespace);
    }
}

#[cfg(test)]
mod capacity_tests {
    use super::*;
    #[test]
    fn issue1489_pending_count_and_snapshot_byte_budgets_remain_fixed() {
        for heavy in [false, true] {
            let mut source = super::super::tests::fixture();
            source.namespace = if heavy {
                "queue1489-byte"
            } else {
                "queue1489-count"
            }
            .into();
            if heavy {
                source.pairs.resize(MAX_PAIRS, source.pairs[0].clone());
            } else {
                source.pairs.clear();
                source.addresses.clear();
                source.references.prices.clear();
                source.mirrors.clear();
            }
            let mut g = Store::default();
            g.snapshots
                .insert(source.namespace.clone(), Arc::new(source.clone()));
            g.jobs
                .extend((0..MAX_JOBS).map(|index| format!("occupied-{index}")));
            let reviewed = Arc::new(protocol_guards::derive(&g, &source.namespace, &source));
            let q = super::super::tests::request(1);
            let body =
                json!({"execution_metadata":{"input_amount":"1","expires_at_ms":now_ms()+20_000}});
            for index in 0..=MAX_RESULTS {
                let key = format!("intent-{index}");
                g.results.insert(key.clone(), body.clone());
                enqueue(
                    &mut g,
                    Work::new(
                        key,
                        &source.namespace,
                        &reviewed,
                        &q,
                        None,
                        1,
                        2,
                        SCALE / 20,
                        &body,
                    ),
                );
            }
            assert_eq!(g.jobs.len(), MAX_JOBS);
            assert!(g.deferred_jobs.len() <= MAX_RESULTS);
            assert!(
                g.deferred_jobs
                    .iter()
                    .map(Work::retained_bytes)
                    .sum::<usize>()
                    <= MAX_PENDING_BYTES
            );
            if heavy {
                assert!(
                    g.deferred_jobs.len() < MAX_RESULTS,
                    "heavy original cohorts must hit fixed memory budget first"
                );
            } else {
                assert_eq!(g.deferred_jobs.len(), MAX_RESULTS);
            }
            assert!(!g
                .deferred_jobs
                .iter()
                .any(|work| work.key == format!("intent-{MAX_RESULTS}")));
        }
    }
}

#[cfg(test)]
mod direct_admission_tests {
    use super::*;
    #[tokio::test]
    async fn issue1489_large_qualified_context_uses_free_worker_without_pending_budget() {
        let namespace = "large-context1489";
        let mut source = super::super::tests::fixture();
        source.namespace = namespace.into();
        // Each of these original token observations stays below the existing
        // 2048 actor bound. Their aggregate conservative accounting exceeds the
        // waiting budget, while the exact quoted ordinary path remains supported.
        let observed = now_ms();
        let actors = (0..512)
            .map(|index| {
                (
                    format!("synthetic-{index}"),
                    tax::ActorObservation {
                        observed_at_ms: observed,
                        protocol: false,
                        manager: false,
                        balance: Some(0),
                    },
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for index in 0..80 {
            source.taxes.insert(
                format!("unrelated-tax-{index}"),
                tax::TokenObservation {
                    policy_identity: "synthetic-large-context".into(),
                    code_id: 1,
                    buy_bps: 0,
                    sell_bps: 0,
                    observed_at_ms: observed,
                    max_wallet: None,
                    pairs: source
                        .pairs
                        .iter()
                        .map(|pair| pair.contract_address.clone())
                        .collect(),
                    actors: actors.clone(),
                },
            );
        }
        let amount = 10u128.pow(16);
        let mut q = super::super::tests::request(amount);
        q.pool_only = Some(true);
        let key = "large-context1489-key";
        let reviewed = {
            let mut g = store().lock().unwrap();
            g.snapshots
                .insert(namespace.into(), Arc::new(source.clone()));
            Arc::new(protocol_guards::derive(&g, namespace, &source))
        };
        let eligible = select(
            &reviewed,
            &paths(&reviewed, 1, 2, 4),
            amount,
            SCALE / 20,
            false,
            8,
        )
        .expect("original ordinary pool path is eligible despite unrelated actor contexts");
        let body = response(&reviewed, &q, &eligible, amount, SCALE / 20, 1);
        let work = Work::new(
            key.into(),
            namespace,
            &reviewed,
            &q,
            None,
            1,
            2,
            SCALE / 20,
            &body,
        );
        assert!(work.retained_bytes() > MAX_PENDING_BYTES);
        {
            let mut g = store().lock().unwrap();
            g.results.insert(key.into(), body);
            assert!(g.jobs.len() < MAX_JOBS);
            enqueue(&mut g, work);
            assert!(
                g.jobs.contains(key),
                "pending memory cap cannot strand supported work with a free original worker"
            );
            assert!(!g.deferred_jobs.iter().any(|work| work.key == key));
        }
        let started = std::time::Instant::now();
        loop {
            let complete = {
                let g = store().lock().unwrap();
                g.results[key]["search_complete"] == true && !g.jobs.contains(key)
            };
            if complete {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(2));
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        let mut g = store().lock().unwrap();
        g.results.remove(key);
        g.snapshots.remove(namespace);
    }
}
