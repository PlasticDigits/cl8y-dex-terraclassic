//! Bounded authoritative actor observations prevent stale admission from restoring
//! blacklist approvals or discounts after independently completed guard updates.
use super::*;
#[derive(Clone)]
struct Entry {
    version: u64,
    context: (u16, bool),
    fee_query: u64,
}
#[derive(Default)]
pub(super) struct Namespace {
    revision: u64,
    entries: HashMap<String, Entry>,
}
#[derive(Clone, Default)]
pub(super) struct Proof {
    revision: u64,
    entries: HashMap<String, Option<u64>>,
    fee_queries: HashMap<String, u64>,
}
impl Proof {
    pub(super) fn entries_version(&self, actor: &str) -> Option<u64> {
        self.entries.get(actor).copied().flatten()
    }
    pub(super) fn observed_fee(&mut self, actor: &str, query: u64) {
        self.fee_queries.insert(actor.into(), query);
    }
    pub(super) fn observed(&mut self, actor: &str, version: u64) {
        self.entries.insert(actor.into(), Some(version));
    }
}
pub(super) fn capture(g: &Store, ns: &str, actors: &HashMap<String, (u16, bool)>) -> Proof {
    let current = g.actor_versions.get(ns);
    Proof {
        revision: current.map_or(0, |n| n.revision),
        fee_queries: HashMap::new(),
        entries: actors
            .keys()
            .map(|actor| {
                (
                    actor.clone(),
                    current
                        .and_then(|n| n.entries.get(actor))
                        .map(|e| e.version),
                )
            })
            .collect(),
    }
}
pub(super) fn revision(g: &Store, ns: &str) -> u64 {
    g.actor_versions.get(ns).map_or(0, |n| n.revision)
}
pub(super) fn observe(
    g: &mut Store,
    ns: &str,
    actor: &str,
    context: (u16, bool),
    changed: bool,
) -> Result<u64, sqlx::Error> {
    if g.actor_versions.len() >= 32 && !g.actor_versions.contains_key(ns) {
        g.actor_capacity_registry_exhausted = true;
        return Err(ingestion::actor_capacity_error());
    }
    let active = g.snapshots.get(ns).map(|s| &s.actors);
    let blocks = g.early_actor_blocks.get(ns);
    let n = g.actor_versions.entry(ns.into()).or_default();
    if n.entries.len() >= ingestion::MAX_ACTOR_CONTEXTS && !n.entries.contains_key(actor) {
        n.entries.retain(|actor, e| {
            e.context.1
                || active.is_some_and(|a| a.contains_key(actor))
                || blocks.is_some_and(|b| b.contains(actor))
        });
    }
    if n.entries.len() >= ingestion::MAX_ACTOR_CONTEXTS && !n.entries.contains_key(actor) {
        mark_actor_capacity_exhausted(g, ns);
        return Err(ingestion::actor_capacity_error());
    }
    let changed = changed || n.entries.get(actor).is_some_and(|e| e.context != context);
    let new = !n.entries.contains_key(actor);
    let version = if changed || new {
        g.next_actor_version = g
            .next_actor_version
            .checked_add(1)
            .expect("actor guard epoch exhausted");
        g.next_actor_version
    } else {
        n.entries[actor].version
    };
    if changed {
        n.revision = version;
    }
    let fee_query = n.entries.get(actor).map_or(0, |entry| entry.fee_query);
    n.entries.insert(
        actor.into(),
        Entry {
            version,
            context,
            fee_query,
        },
    );
    Ok(version)
}
pub(super) fn guard_current(g: &Store, ns: &str, actor: &str, proof: &Proof) -> bool {
    let expected = proof.entries_version(actor);
    let current = g.actor_versions.get(ns);
    current
        .and_then(|n| n.entries.get(actor))
        .map(|e| e.version)
        == expected
        && (expected.is_some() || proof.revision == current.map_or(0, |n| n.revision))
}
pub(super) fn guard_context(g: &Store, ns: &str, actor: &str) -> Option<(u16, bool)> {
    g.actor_versions
        .get(ns)?
        .entries
        .get(actor)
        .map(|e| e.context)
}
pub(super) fn begin_fee(g: &mut Store, ns: &str, actor: &str, version: u64) -> Option<u64> {
    let entry = g.actor_versions.get_mut(ns)?.entries.get_mut(actor)?;
    if entry.version != version || entry.context.1 {
        return None;
    }
    g.next_actor_version = g
        .next_actor_version
        .checked_add(1)
        .expect("actor fee epoch exhausted");
    entry.fee_query = g.next_actor_version;
    Some(entry.fee_query)
}
pub(super) fn fee_current(g: &Store, ns: &str, actor: &str, version: u64, query: u64) -> bool {
    g.actor_versions
        .get(ns)
        .and_then(|n| n.entries.get(actor))
        .is_some_and(|e| e.version == version && e.fee_query == query && !e.context.1)
}
pub(super) fn publication_fees_current(g: &Store, ns: &str, proof: &Proof) -> bool {
    proof.fee_queries.iter().all(|(actor, query)| {
        g.actor_versions
            .get(ns)
            .and_then(|n| n.entries.get(actor))
            .is_some_and(|e| e.fee_query == *query)
    })
}
pub(super) fn accept_fee(
    g: &mut Store,
    ns: &str,
    actor: &str,
    context: (u16, bool),
    changed: bool,
) -> Result<u64, sqlx::Error> {
    if changed {
        return observe(g, ns, actor, context, true);
    }
    let entry = g
        .actor_versions
        .get_mut(ns)
        .and_then(|n| n.entries.get_mut(actor))
        .expect("fee completion requires a current bounded guard record");
    entry.context = context;
    Ok(entry.version)
}
pub(super) fn reconcile(g: &mut Store, s: &mut Snapshot, proof: &Proof) -> Result<(), sqlx::Error> {
    let current = g.actor_versions.get(&s.namespace);
    let current_revision = current.map_or(0, |n| n.revision);
    let keys = s.actors.keys().cloned().collect::<Vec<_>>();
    for actor in keys {
        let expected = proof.entries.get(&actor).copied().flatten();
        let observed = current.and_then(|n| n.entries.get(&actor));
        let same = observed.map(|e| e.version) == expected
            && (expected.is_some() || proof.revision == current_revision);
        if !same {
            if let Some(context) = observed.map(|e| e.context).or_else(|| {
                g.snapshots
                    .get(&s.namespace)
                    .and_then(|s| s.actors.get(&actor).copied())
            }) {
                s.actors.insert(actor, context);
            } else {
                s.actors.remove(&actor);
            }
        }
    }
    if let Some(blocks) = g.early_actor_blocks.get(&s.namespace) {
        for actor in blocks {
            ingestion::insert_actor(&mut s.actors, actor.clone(), (0, true))?;
        }
    }
    s.actor_revision = current_revision;
    Ok(())
}

pub(super) fn authoritative_inventory(
    g: &mut Store,
    ns: &str,
    actors: &HashMap<String, (u16, bool)>,
) -> Result<(), sqlx::Error> {
    if actors.len() > ingestion::MAX_ACTOR_CONTEXTS
        || (g.actor_versions.len() >= 32 && !g.actor_versions.contains_key(ns))
    {
        mark_actor_capacity_exhausted(g, ns);
        return Err(ingestion::actor_capacity_error());
    }
    let before = g.snapshots.get(ns).map(|s| &s.actors);
    let old = g.actor_versions.remove(ns).unwrap_or_default();
    let changed = before != Some(actors)
        || old
            .entries
            .iter()
            .any(|(actor, e)| e.context != (0, false) && !actors.contains_key(actor));
    let revision = if changed {
        g.next_actor_version = g
            .next_actor_version
            .checked_add(1)
            .expect("actor guard epoch exhausted");
        g.next_actor_version
    } else {
        old.revision
    };
    let mut entries = HashMap::new();
    for (actor, context) in actors {
        let version = if let Some(entry) = old
            .entries
            .get(actor)
            .filter(|entry| entry.context == *context)
        {
            entry.version
        } else {
            g.next_actor_version = g
                .next_actor_version
                .checked_add(1)
                .expect("actor guard epoch exhausted");
            g.next_actor_version
        };
        entries.insert(
            actor.clone(),
            Entry {
                version,
                context: *context,
                fee_query: 0,
            },
        );
    }
    // Retain conservative unblock tombstones only within the existing actor cap.
    // A reset revokes per-actor proofs too: proofs with an observed actor version
    // do not depend on the namespace revision. Unchanged inventories stay stable.
    for (actor, entry) in old.entries {
        if entries.len() >= ingestion::MAX_ACTOR_CONTEXTS {
            break;
        }
        entries.entry(actor).or_insert(Entry {
            version: if changed { revision } else { entry.version },
            context: (0, false),
            fee_query: 0,
        });
    }
    g.actor_versions
        .insert(ns.into(), Namespace { revision, entries });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1489_authoritative_inventory_revokes_omitted_actor_discount_proof() {
        let mut g = Store::default();
        let ns = "inventory-discount1489";
        let mut pending = super::super::tests::fixture();
        pending.namespace = ns.into();
        pending.actors.insert("actor".into(), (800, false));
        g.snapshots.insert(ns.into(), Arc::new(pending.clone()));
        observe(&mut g, ns, "actor", (0, false), false).unwrap();
        let old = capture(&g, ns, &pending.actors);
        authoritative_inventory(&mut g, ns, &HashMap::new()).unwrap();
        let mut recovered = pending.clone();
        recovered.actors.clear();
        g.snapshots.insert(ns.into(), Arc::new(recovered));
        reconcile(&mut g, &mut pending, &old).unwrap();
        assert_eq!(
            pending.actors.get("actor").copied().unwrap_or((0, false)),
            (0, false),
            "a complete inventory reset cannot restore a pending old discount"
        );
        let fresh = capture(&g, ns, &pending.actors);
        let version = fresh.entries_version("actor");
        let current_revision = revision(&g, ns);
        authoritative_inventory(&mut g, ns, &HashMap::new()).unwrap();
        assert_eq!(revision(&g, ns), current_revision);
        assert_eq!(
            capture(&g, ns, &pending.actors).entries_version("actor"),
            version
        );
    }
    #[test]
    fn issue1489_actor_guard_versions_are_bounded_and_authoritative_replacement_is_atomic() {
        let mut g = Store::default();
        let ns = "actor-version-budget1489";
        for number in 0..ingestion::MAX_ACTOR_CONTEXTS {
            observe(&mut g, ns, &format!("blocked-{number}"), (0, true), true).unwrap();
        }
        assert!(observe(&mut g, ns, "overflow", (0, true), true).is_err());
        assert_eq!(g.actor_versions[ns].entries.len(), 2048);
        let replacement = (0..2048)
            .map(|number| (format!("replacement-{number}"), (0, true)))
            .collect();
        authoritative_inventory(&mut g, ns, &replacement).unwrap();
        assert_eq!(g.actor_versions[ns].entries.len(), 2048);
        assert!(g.actor_versions[ns].entries.contains_key("replacement-0"));
        assert!(!g.actor_versions[ns].entries.contains_key("blocked-0"));
        let mut unchanged = Store::default();
        let first = observe(&mut unchanged, ns, "actor", (0, false), false).unwrap();
        let generation = revision(&unchanged, ns);
        assert_eq!(
            observe(&mut unchanged, ns, "actor", (0, false), false).unwrap(),
            first
        );
        assert_eq!(
            revision(&unchanged, ns),
            generation,
            "unchanged positive guard evidence cannot churn identity"
        );
    }
    #[test]
    fn issue1489_gc_reintroduced_actor_never_reuses_old_discount_proof() {
        let mut g = Store::default();
        let ns = "actor-gc1489";
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = ns.into();
        snapshot.actors.insert("actor".into(), (800, false));
        g.snapshots.insert(ns.into(), Arc::new(snapshot.clone()));
        observe(&mut g, ns, "actor", (0, false), false).unwrap();
        let old = capture(&g, ns, &snapshot.actors);
        let version = old.entries_version("actor").unwrap();
        g.snapshots.remove(ns);
        for number in 0..2048 {
            observe(&mut g, ns, &format!("inactive-{number}"), (0, false), false).unwrap();
        }
        assert!(!g.actor_versions[ns].entries.contains_key("actor"));
        let fresh = observe(&mut g, ns, "actor", (0, false), false).unwrap();
        assert_ne!(fresh, version);
        reconcile(&mut g, &mut snapshot, &old).unwrap();
        assert_eq!(
            snapshot.actors["actor"],
            (0, false),
            "old discount cannot survive GC and fresh conservative admission"
        );
    }
}
