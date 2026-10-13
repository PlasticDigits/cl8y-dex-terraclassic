//! Optional scheduled fee reads retain only unexpired previously verified fees.
//! Latest-started observations own completion; cancellation invalidates that
//! observation conservatively without revoking a newer observation.
use super::*;

pub(super) struct Observation {
    namespace: String,
    actor: String,
    version: u64,
    pub query: u64,
    completed: bool,
}
impl Observation {
    pub(super) fn begin(namespace: &str, actor: &str, version: u64) -> Option<Self> {
        let query =
            actor_versions::begin_fee(&mut store().lock().unwrap(), namespace, actor, version)?;
        Some(Self {
            namespace: namespace.into(),
            actor: actor.into(),
            version,
            query,
            completed: false,
        })
    }
    pub(super) fn complete(mut self, discount: Option<u16>) -> Result<Option<u64>, sqlx::Error> {
        self.completed = true;
        complete(&self, discount)
    }
}
impl Drop for Observation {
    fn drop(&mut self) {
        if !self.completed {
            // No network work on cancellation; only the still-current read may
            // revoke its cached fee. Superseded reads cannot disturb newer fees.
            let _ = complete(self, None);
        }
    }
}
fn complete(observation: &Observation, discount: Option<u16>) -> Result<Option<u64>, sqlx::Error> {
    let mut g = store().lock().unwrap();
    let ns = &observation.namespace;
    let actor = &observation.actor;
    if !actor_versions::fee_current(&g, ns, actor, observation.version, observation.query) {
        return Ok(None);
    }
    let context = (discount.unwrap_or(0), false);
    let previous = g.snapshots.get(ns).cloned();
    let changed = previous.as_ref().and_then(|s| s.actors.get(actor)).copied() != Some(context);
    let version = actor_versions::accept_fee(&mut g, ns, actor, context, changed)?;
    if changed {
        if let Some(previous) = previous {
            let mut snapshot = (*previous).clone();
            if let Err(error) =
                ingestion::insert_actor(&mut snapshot.actors, actor.clone(), context)
            {
                mark_actor_capacity_exhausted(&mut g, ns);
                g.results.clear();
                return Err(error);
            }
            g.next_config_epoch = g.next_config_epoch.saturating_add(1);
            snapshot.config_epoch = g.next_config_epoch;
            snapshot.actor_revision = actor_versions::revision(&g, ns);
            snapshot.generation = format!(
                "{:x}",
                Sha256::digest(format!(
                    "{}|actor-fee|{}",
                    snapshot.generation, snapshot.actor_revision
                ))
            );
            // Guard/fee completion cannot renew old observation/configuration deadlines.
            g.snapshots.insert(ns.clone(), Arc::new(snapshot));
            g.results.clear();
        }
    }
    Ok(Some(version))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(ns: &str) -> (Snapshot, String, u64) {
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = ns.into();
        let actor = super::super::publication_tests::address(33100);
        snapshot.actors.insert(actor.clone(), (800, false));
        store()
            .lock()
            .unwrap()
            .snapshots
            .insert(ns.into(), Arc::new(snapshot.clone()));
        let version = ingestion::publish_actor_guard(ns, &actor, false).unwrap();
        (snapshot, actor, version)
    }
    fn cleanup(ns: &str) {
        let mut g = store().lock().unwrap();
        g.snapshots.remove(ns);
        g.actor_versions.remove(ns);
        g.early_actor_blocks.remove(ns);
    }
    #[test]
    #[serial_test::serial]
    fn issue1489_failed_or_cancelled_current_fee_revokes_without_renewing_deadlines() {
        for cancelled in [false, true] {
            let ns = format!("fee-failure1489-{cancelled}");
            let (snapshot, actor, version) = setup(&ns);
            let read = Observation::begin(&ns, &actor, version).unwrap();
            if cancelled {
                drop(read);
            } else {
                read.complete(None).unwrap();
            }
            let actual = store().lock().unwrap().snapshots[&ns].clone();
            cleanup(&ns);
            assert_eq!(actual.actors[&actor], (0, false));
            assert_eq!(actual.config_expiry_ms, snapshot.config_expiry_ms);
            assert_eq!(actual.config_expiries, snapshot.config_expiries);
            assert_ne!(actual.generation, snapshot.generation);
        }
    }
    #[test]
    #[serial_test::serial]
    fn issue1489_cancelled_older_fee_cannot_revoke_newer_verified_context() {
        let ns = "fee-older-cancel1489";
        let (_, actor, version) = setup(ns);
        let old = Observation::begin(ns, &actor, version).unwrap();
        let current = Observation::begin(ns, &actor, version).unwrap();
        current.complete(Some(400)).unwrap();
        let before = store().lock().unwrap().snapshots[ns].clone();
        drop(old);
        let after = store().lock().unwrap().snapshots[ns].clone();
        cleanup(ns);
        assert!(Arc::ptr_eq(&before, &after));
        assert_eq!(after.actors[&actor], (400, false));
    }
    #[test]
    #[serial_test::serial]
    fn issue1489_held_old_fee_publication_cannot_restore_or_extend_newer_context() {
        let ns = "fee-held-publication1489";
        let (mut snapshot, actor, version) = setup(ns);
        let old = Observation::begin(ns, &actor, version).unwrap();
        let query = old.query;
        let version = old.complete(Some(800)).unwrap().unwrap();
        let mut proof = actor_versions::capture(&store().lock().unwrap(), ns, &snapshot.actors);
        proof.observed(&actor, version);
        proof.observed_fee(&actor, query);
        snapshot.config_expiry_ms += 60_000;
        for expiry in snapshot.config_expiries.values_mut() {
            *expiry += 60_000;
        }
        let mut pending = super::super::publication_tests::pending(snapshot);
        pending.actor_proof = proof;
        let prepared = pending.prepare();
        Observation::begin(ns, &actor, version)
            .unwrap()
            .complete(Some(400))
            .unwrap();
        let before = store().lock().unwrap().snapshots[ns].clone();
        prepared.commit();
        let after = store().lock().unwrap().snapshots[ns].clone();
        let capacity_failed = store()
            .lock()
            .unwrap()
            .actor_capacity_exhausted
            .contains(ns);
        cleanup(ns);
        assert!(Arc::ptr_eq(&before, &after));
        assert_eq!(after.actors[&actor], (400, false));
        assert!(
            !capacity_failed,
            "obsolete observations do not mean capacity exhaustion"
        );
    }
}
