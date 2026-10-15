//! Scheduled publication owns at most two CPU jobs; requests never start one.
//! Original reserve/admission producers await one pass each. A cancelled pass
//! retains its permit until CPU work finishes and cannot publish afterward.
use super::*;

pub(super) struct Pending {
    pub snapshot: Snapshot,
    pub expected_config_epoch: Option<u64>,
    pub actor_proof: actor_versions::Proof,
    #[cfg(test)]
    pub start_barrier: Option<Arc<std::sync::Barrier>>,
}
fn slots() -> Arc<tokio::sync::Semaphore> {
    static SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
    Arc::clone(SLOTS.get_or_init(|| Arc::new(tokio::sync::Semaphore::new(2))))
}
fn previous_view_hashes(namespace: &str) -> Option<Arc<generation::ViewHashes>> {
    let g = store().lock().unwrap();
    g.snapshots
        .get(namespace)
        .map(|current| Arc::clone(&current.view_hashes))
}
impl Pending {
    pub(super) async fn scheduled(self) -> Result<(), sqlx::Error> {
        let permit = slots()
            .acquire_owned()
            .await
            .map_err(|_| sqlx::Error::Protocol("publication capacity unavailable".into()))?;

        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            self.prepare()
        })
        .await;

        match result {
            Ok(prepared) => {
                prepared.commit();
                Ok(())
            }
            Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
            Err(_) => Err(sqlx::Error::Protocol("publication worker cancelled".into())),
        }
    }
    pub(super) fn prepare(mut self) -> Prepared {
        #[cfg(test)]
        if let Some(barrier) = &self.start_barrier {
            barrier.wait();
        }

        // Capture only this namespace's bounded digest map. Order traversal and
        // all hashing stay off Store, within the existing two CPU-job permits.
        let previous = previous_view_hashes(&self.snapshot.namespace);
        let identity = generation::prepare_reusing(&mut self.snapshot, previous.as_deref());
        Prepared {
            pending: self,
            identity,
        }
    }
}
pub(super) struct Prepared {
    pending: Pending,
    identity: Sha256,
}
impl Prepared {
    pub(super) fn commit(self) {
        let mut snapshot = self.pending.snapshot;
        let namespace = snapshot.namespace.clone();
        let expected_config_epoch = self.pending.expected_config_epoch;
        let prepared_identity = self.identity;
        let mut g = store().lock().unwrap();

        // A reserve pass may finish after admission publishes a new policy epoch.
        // Compare and replace under one lock so old reads cannot restore old guards.
        if expected_config_epoch.is_some_and(|expected| {
            !g.snapshots
                .get(&namespace)
                .is_some_and(|current| current.config_epoch == expected)
        }) {
            return;
        }
        if g.snapshots.len() >= 32 && !g.snapshots.contains_key(&namespace) {
            return;
        }
        if !actor_versions::publication_fees_current(&g, &namespace, &self.pending.actor_proof) {
            tracing::info!(target: "route_discovery", stage = "discarded", reason = "fee_observation_superseded", "bounded route discovery");
            return;
        }
        if actor_versions::reconcile(&mut g, &mut snapshot, &self.pending.actor_proof).is_err() {
            mark_actor_capacity_exhausted(&mut g, &namespace);
            g.results.clear();
            return;
        }
        if let Some(current) = g.snapshots.get(&namespace) {
            preserve_newer_mirrors(&mut snapshot, current);
            snapshot.validity_revision = current
                .validity_revision
                .saturating_add(u64::from(validity_tightened(&snapshot, current)));
        }
        let qualifications = snapshot
            .pairs
            .iter()
            .filter_map(|pair| {
                snapshot
                    .pair_contexts
                    .get(&pair.contract_address)
                    .map(|context| crate::indexer::fee_phase::Qualification {
                        pair_id: pair.id,
                        assets: (pair.asset_0_id, pair.asset_1_id),
                        context: context.clone(),
                        expiry_ms: snapshot
                            .config_expiries
                            .get(&pair.contract_address)
                            .copied()
                            .unwrap_or(snapshot.config_expiry_ms),
                        max_age_ms: snapshot.max_age_ms,
                        mirror: snapshot.mirrors.get(&pair.contract_address).cloned(),
                        pool_mirror: snapshot.pool_mirrors.get(&pair.contract_address).cloned(),
                    })
            })
            .collect();
        if !snapshot.pair_contexts.is_empty()
            && !crate::indexer::fee_phase::qualify(&namespace, qualifications)
        {
            g.actor_capacity_registry_exhausted = true;
            g.results.clear();
            return;
        }
        for (pool_view, views) in [
            (false, &mut snapshot.mirrors),
            (true, &mut snapshot.pool_mirrors),
        ] {
            views.retain(|address, mirror| {
                crate::indexer::fee_phase::permits_view(
                    &namespace,
                    mirror.pair_id,
                    mirror.block_height,
                    mirror.snapshot_at,
                    pool_view,
                ) && snapshot
                    .pairs
                    .iter()
                    .any(|pair| &pair.contract_address == address)
            });
        }
        let Some(identity) = generation::finish(&snapshot, prepared_identity) else {
            // An unbound view cannot publish under an identity for different data.
            g.snapshots.remove(&namespace);
            g.results.clear();
            tracing::warn!(target: "route_discovery", reason = "view_identity_unbound", "qualified publication unavailable");
            return;
        };
        snapshot.generation = identity;
        protocol_guards::clear_admitted_false(
            &mut g,
            &namespace,
            &snapshot,
            expected_config_epoch.is_none(),
        );
        if expected_config_epoch.is_none() {
            g.next_config_epoch = g.next_config_epoch.saturating_add(1);
            snapshot.config_epoch = g.next_config_epoch;
        }
        g.snapshots.insert(namespace, Arc::new(snapshot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[serial_test::serial]
    fn issue1505_preparation_captures_only_the_current_namespace_hashes() {
        let mut current = super::super::tests::fixture();
        current.namespace = "hash-reuse1505-current".into();
        generation::prepare(&mut current);
        let namespace = current.namespace.clone();
        let expected = Arc::clone(&current.view_hashes);
        store()
            .lock()
            .unwrap()
            .snapshots
            .insert(namespace.clone(), Arc::new(current));
        let captured = previous_view_hashes(&namespace).unwrap();
        let unrelated = previous_view_hashes("hash-reuse1505-unpublished");
        store().lock().unwrap().snapshots.remove(&namespace);
        assert!(
            Arc::ptr_eq(&captured, &expected),
            "capture clones only the existing digest-map Arc"
        );
        assert!(
            unrelated.is_none(),
            "a different namespace cannot borrow another market's cache"
        );
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn issue1489_cancelled_preparation_keeps_two_cpu_slots_and_never_commits() {
        let slots = slots();
        assert_eq!(slots.available_permits(), 2);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let mut jobs = Vec::new();
        for number in 0..3 {
            let mut snapshot = super::super::tests::fixture();
            snapshot.namespace = format!("cancelled-preparation1489-{number}");
            let pending = Pending {
                snapshot,
                expected_config_epoch: None,
                actor_proof: actor_versions::Proof::default(),
                start_barrier: (number < 2).then(|| Arc::clone(&barrier)),
            };
            jobs.push(tokio::spawn(pending.scheduled()));
        }
        tokio::time::timeout(Duration::from_secs(2), async {
            while slots.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        for job in &jobs {
            assert!(!job.is_finished());
            job.abort();
        }
        for job in jobs {
            assert!(job.await.unwrap_err().is_cancelled());
        }
        assert_eq!(
            slots.available_permits(),
            0,
            "cancelled awaiters cannot release running CPU ownership"
        );
        barrier.wait();
        tokio::time::timeout(Duration::from_secs(2), async {
            while slots.available_permits() != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let g = store().lock().unwrap();
        for number in 0..3 {
            assert!(!g
                .snapshots
                .contains_key(&format!("cancelled-preparation1489-{number}")));
        }
    }
}
