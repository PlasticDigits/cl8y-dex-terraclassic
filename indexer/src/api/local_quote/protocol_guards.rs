//! Independently observed token-specific negatives survive subsequent ingestion failure.
//! Requests only overlay this bounded memory; they never discover or refresh roles.
use super::*;
use std::collections::BTreeMap;

pub(super) type Pending = HashMap<String, BTreeMap<(String, String), i64>>;
const MAX_NAMESPACES: usize = 32;
const MAX_GUARDS: usize = 2048;

pub(super) fn record(namespace: &str, token: &str, actor: &str, observed_at: i64) -> Option<()> {
    apply(
        &mut store().lock().unwrap(),
        namespace,
        token,
        actor,
        observed_at,
    )
}
fn apply(g: &mut Store, namespace: &str, token: &str, actor: &str, observed_at: i64) -> Option<()> {
    if token.len() > 128 || actor.len() > 128 {
        mark_tax_wallet_capacity_exhausted(g, namespace);
        g.results.clear();
        return None;
    }
    if !g.pending_protocol_guards.contains_key(namespace)
        && g.pending_protocol_guards.len() >= MAX_NAMESPACES
    {
        g.actor_capacity_registry_exhausted = true;
        g.results.clear();
        return None;
    }
    let guards = g
        .pending_protocol_guards
        .entry(namespace.to_owned())
        .or_default();
    let key = (token.to_owned(), actor.to_owned());
    if !guards.contains_key(&key) && guards.len() >= MAX_GUARDS {
        mark_tax_wallet_capacity_exhausted(g, namespace);
        g.results.clear();
        return None;
    }
    let previous = guards.get(&key).copied();
    guards.insert(key, previous.unwrap_or(observed_at).max(observed_at));
    if previous.is_none_or(|at| at < observed_at) {
        // Includes cold observations: an earlier authoritative factory scan cannot
        // reset a newer independently observed token role.
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
        if let Some(previous) = g.snapshots.get(namespace).cloned() {
            let mut snapshot = (*previous).clone();
            snapshot.config_epoch = g.next_config_epoch;
            g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
        }
        g.results.clear();
    }
    Some(())
}

/// Use the exact same ordered overlay for requests, progress and publication checks.
pub(super) fn derive(g: &Store, namespace: &str, source: &Snapshot) -> Snapshot {
    let mut snapshot = source.clone();
    snapshot.protocol_actor_tokens.clear();
    if let Some(guards) = g.pending_protocol_guards.get(namespace) {
        for ((token, actor), observed_at) in guards {
            snapshot
                .protocol_actor_tokens
                .entry(actor.clone())
                .or_default()
                .insert(token.clone());
            if let Some(token) = snapshot.taxes.get_mut(token) {
                // Missing actor context already fails closed. Overlay existing
                // positives without exceeding the token's 2,048-context budget.
                if let Some(actor) = token.actors.get_mut(actor) {
                    actor.protocol = true;
                    actor.manager = false;
                    actor.balance = None;
                    actor.observed_at_ms = *observed_at;
                }
            }
        }
    }
    snapshot.generation = generation(g, namespace, source);
    snapshot
}

pub(super) fn permits_token(snapshot: &Snapshot, token: &str) -> bool {
    snapshot.tax_trader.as_deref().is_none_or(|actor| {
        !snapshot
            .protocol_actor_tokens
            .get(actor)
            .is_some_and(|tokens| tokens.contains(token))
    })
}
pub(super) fn permits_path(snapshot: &Snapshot, path: &[route_paths::PathHop]) -> bool {
    path.iter().all(|(_, offer, ask)| {
        [offer, ask].iter().all(|id| {
            snapshot
                .addresses
                .get(id)
                .is_some_and(|token| permits_token(snapshot, token))
        })
    })
}

pub(super) fn generation(g: &Store, namespace: &str, source: &Snapshot) -> String {
    let base = admission_generation(source);
    match g
        .pending_protocol_guards
        .get(namespace)
        .filter(|guards| !guards.is_empty())
    {
        Some(guards) => format!(
            "{:x}",
            Sha256::digest(format!("{base}|protocol-guards|{guards:?}"))
        ),
        None => base,
    }
}

/// Called only under the same Store lock that publishes a fully admitted snapshot.
/// Reserve copies, omitted actors and failed/partial token observations cannot clear
/// a negative. The original timestamp is checked before any guard is removed.
pub(super) fn clear_admitted_false(
    g: &mut Store,
    namespace: &str,
    admitted: &Snapshot,
    allow_clear: bool,
) {
    if !allow_clear {
        return;
    }
    let Some(guards) = g.pending_protocol_guards.get_mut(namespace) else {
        return;
    };
    let at = now_ms();
    guards.retain(|(token, actor), observed_at| {
        !admitted.taxes.get(token).is_some_and(|token| {
            token.observed_at_ms > 0
                && token.observed_at_ms <= at
                && at - token.observed_at_ms < CONFIG_TTL_MS
                && admitted.pairs.iter().any(|pair| {
                    token.pairs.contains(&pair.contract_address)
                        && admitted
                            .config_expiries
                            .get(&pair.contract_address)
                            .copied()
                            .unwrap_or(admitted.config_expiry_ms)
                            > at
                })
                && token.actors.get(actor).is_some_and(|actor| {
                    !actor.protocol
                        && actor.observed_at_ms > *observed_at
                        && actor.observed_at_ms <= at
                        && at - actor.observed_at_ms < CONFIG_TTL_MS
                })
        })
    });
    // Empty namespaces still count toward the bounded registry, preserving its
    // non-eviction policy under repeated cold initialization and inventory scans.
}

#[cfg(test)]
mod tests {
    use super::*;
    fn positive(namespace: &str, actor_at: i64) -> Snapshot {
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = namespace.into();
        snapshot.taxes.insert(
            "in18".into(),
            tax::TokenObservation {
                policy_identity: "policy".into(),
                code_id: 1,
                buy_bps: 0,
                sell_bps: 0,
                observed_at_ms: now_ms(),
                max_wallet: None,
                pairs: snapshot
                    .pairs
                    .iter()
                    .map(|pair| pair.contract_address.clone())
                    .collect(),
                actors: BTreeMap::from([
                    (
                        "wallet".into(),
                        tax::ActorObservation {
                            observed_at_ms: actor_at,
                            protocol: false,
                            manager: true,
                            balance: Some(0),
                        },
                    ),
                    (
                        "other".into(),
                        tax::ActorObservation {
                            observed_at_ms: actor_at,
                            protocol: false,
                            manager: false,
                            balance: Some(0),
                        },
                    ),
                ]),
            },
        );
        snapshot
    }
    #[test]
    fn pending_token_protocol_negative_rejects_old_ordinary_mirror_without_tax_map() {
        let namespace = "ordinary-before-capability";
        let source = super::super::tests::fixture();
        assert!(source.taxes.is_empty());
        let mut g = Store::default();
        assert!(apply(&mut g, namespace, "in18", "wallet", now_ms()).is_some());
        let mut guarded = derive(&g, namespace, &source);
        let taxed_path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        guarded.tax_trader = Some("wallet".into());
        assert!(score(&guarded, &taxed_path, 10u128.pow(18), SCALE / 20).is_none());
        assert!(!permits_token(&guarded, "in18"));
        let unrelated = vec![("pair7b".into(), 10, 2)];
        // The base corpus uses raw-unit marks because end-to-end amounts have
        // matching scales. Bind this isolated six-to-eighteen-decimal path to
        // independently known token scales while retaining unit USD prices.
        for (id, price) in &mut guarded.references.prices {
            price.decimals = if *id == 1 || *id == 2 { 18 } else { 6 };
        }
        assert!(score(&guarded, &unrelated, 10u128.pow(6), SCALE / 20).is_some());
        guarded.tax_trader = Some("other".into());
        assert!(score(&guarded, &taxed_path, 10u128.pow(18), SCALE / 20).is_some());
    }
    #[test]
    fn known_token_role_survives_failure_after_flags_and_invalidates_warm_inflight_and_cache() {
        let at = now_ms() - 10;
        let namespace = "protocol-after-flags";
        let snapshot = positive(namespace, at - 1);
        let mut g = Store::default();
        g.snapshots
            .insert(namespace.into(), Arc::new(snapshot.clone()));
        g.results
            .insert("warm-approval".into(), json!({"approved":true}));
        let mut reviewed = derive(&g, namespace, &snapshot);
        reviewed.tax_trader = Some("wallet".into());
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        assert!(score(&reviewed, &path, 10u128.pow(18), SCALE / 20).is_some());
        // Models validated protocol:true followed by cancelled/failed balance,
        // config, code or database reads: nothing else publishes afterward.
        assert!(apply(&mut g, namespace, "in18", "wallet", at).is_some());
        assert!(g.results.is_empty());
        let mut guarded = derive(&g, namespace, &g.snapshots[namespace]);
        guarded.tax_trader = Some("wallet".into());
        assert!(score(&guarded, &path, 10u128.pow(18), SCALE / 20).is_none());
        guarded.tax_trader = Some("other".into());
        assert!(score(&guarded, &path, 10u128.pow(18), SCALE / 20).is_some());
        assert!(guarded.taxes["in18"]
            .rates(Some("wallet"), "pair7a")
            .is_none());
        assert_ne!(reviewed.generation, guarded.generation);
        assert!(!reviewed_snapshot_current(&g, namespace, &reviewed, None));
        assert_eq!(
            guarded.taxes["in18"].rates(Some("other"), "pair7a"),
            Some((0, 0))
        );
        // Unrelated ordinary assets are not turned into global wallet blocks.
        assert!(!guarded
            .actors
            .get("wallet")
            .is_some_and(|(_, blocked)| *blocked));
        clear_admitted_false(&mut g, namespace, &snapshot, true);
        assert_eq!(g.pending_protocol_guards[namespace].len(), 1);
        let equal = positive(namespace, at);
        clear_admitted_false(&mut g, namespace, &equal, true);
        assert_eq!(g.pending_protocol_guards[namespace].len(), 1);
        let fresh = positive(namespace, at + 1);
        clear_admitted_false(&mut g, namespace, &fresh, false); // reserve publication
        assert_eq!(g.pending_protocol_guards[namespace].len(), 1);
        clear_admitted_false(&mut g, namespace, &fresh, true);
        assert!(g.pending_protocol_guards[namespace].is_empty());
    }
    #[test]
    fn cold_token_negative_survives_missing_and_future_actor_observations() {
        let namespace = "cold-protocol";
        let at = now_ms() - 10;
        let mut g = Store::default();
        assert!(apply(&mut g, namespace, "in18", "wallet", at).is_some());
        assert!(g.snapshots.is_empty());
        let mut incomplete = positive(namespace, at + 1);
        incomplete.taxes.get_mut("in18").unwrap().actors.clear();
        clear_admitted_false(&mut g, namespace, &incomplete, true);
        let future = positive(namespace, now_ms() + 10_000);
        clear_admitted_false(&mut g, namespace, &future, true);
        assert_eq!(g.pending_protocol_guards[namespace].len(), 1);
        assert!(derive(&g, namespace, &incomplete).taxes["in18"]
            .rates(Some("wallet"), "pair7a")
            .is_none());
    }
    #[test]
    fn protocol_guard_capacity_never_evicts_and_factory_recovery_cannot_clear_tax_quarantine() {
        let mut g = Store::default();
        for i in 0..MAX_GUARDS {
            assert!(apply(&mut g, "capacity", "token", &format!("actor-{i}"), 1).is_some());
        }
        assert!(apply(&mut g, "capacity", "other-token", "new-actor", 2).is_none());
        assert_eq!(g.pending_protocol_guards["capacity"].len(), MAX_GUARDS);
        assert!(g.tax_wallet_capacity_exhausted.contains("capacity"));
        g.actor_capacity_exhausted.remove("capacity");
        assert!(actor_capacity_unavailable(&g, "capacity"));
        for i in 1..MAX_NAMESPACES {
            assert!(apply(&mut g, &format!("namespace-{i}"), "token", "actor", 1).is_some());
        }
        assert!(apply(&mut g, "overflow-namespace", "token", "actor", 1).is_none());
        assert_eq!(g.pending_protocol_guards.len(), MAX_NAMESPACES);
        assert!(g.actor_capacity_registry_exhausted);
    }
}
