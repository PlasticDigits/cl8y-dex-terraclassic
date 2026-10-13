//! Independent scheduled bounded quote observation workers.
use super::*;
use crate::api::{local_reference_quote, local_wrapper_quote};

pub(super) async fn query(state: &AppState, address: &str, value: Value) -> Option<Value> {
    state
        .lcd
        .query_contract_bounded(address, &value, 32_768)
        .await
        .ok()
}

pub(super) fn actor_context(
    blocked: Option<bool>,
    discount: Option<u16>,
    previous: Option<(u16, bool)>,
) -> Option<(u16, bool)> {
    match blocked {
        Some(true) => Some((0, true)),
        Some(false) => Some((discount.unwrap_or(0), false)),
        None => previous
            .filter(|(_, was_blocked)| *was_blocked)
            .map(|_| (0, true)),
    }
}
pub(super) const MAX_ACTOR_CONTEXTS: usize = 2048;
const RECENT_ACTORS: usize = 120;
const HISTORICAL_BLOCK_REFRESH: usize = 8;
pub(super) const MAX_TAX_ACTORS_PER_PASS: usize = 24;
pub(super) const ENROLLED_ACTORS_PER_PASS: usize = 4;
pub(super) const HOLDER_ACCOUNTS_PER_PASS: usize = 4;

/// Negative observations protect warm quotes before any subsequent database or
/// token enrichment await. A failed enrichment cannot leave the old approval
/// visible, and an older reserve pass/inventory cannot restore it.
pub(super) fn publish_known_block(namespace: &str, address: &str) -> Result<(), sqlx::Error> {
    apply_known_block(&mut store().lock().unwrap(), namespace, address)
}
pub(super) fn clear_known_block(namespace: &str, address: &str) {
    let mut g = store().lock().unwrap();
    if let Some(blocks) = g.early_actor_blocks.get_mut(namespace) {
        blocks.remove(address);
    }
}
/// An authoritative unblock uses conservative fees immediately. Optional fee
/// provider delay/cancellation cannot keep an old blacklist approval or discount.
pub(super) fn publish_known_unblock(namespace: &str, address: &str) -> Result<(), sqlx::Error> {
    apply_known_unblock(&mut store().lock().unwrap(), namespace, address)
}
fn apply_known_unblock(g: &mut Store, namespace: &str, address: &str) -> Result<(), sqlx::Error> {
    let before = g
        .snapshots
        .get(namespace)
        .and_then(|s| s.actors.get(address).copied());
    let was_blocked = g
        .early_actor_blocks
        .get(namespace)
        .is_some_and(|b| b.contains(address));
    let fresh_positive = !was_blocked
        && before.is_some_and(|c| !c.1)
        && g.snapshots
            .get(namespace)
            .is_some_and(|s| s.config_expiry_ms > now_ms())
        && !actor_versions::guard_context(g, namespace, address).is_some_and(|c| c.1);
    let guard_context = if fresh_positive {
        actor_versions::guard_context(g, namespace, address).unwrap_or(before.unwrap())
    } else {
        (0, false)
    };
    actor_versions::observe(
        g,
        namespace,
        address,
        guard_context,
        was_blocked || (!fresh_positive && before.is_some_and(|c| c != (0, false))),
    )?;
    if fresh_positive {
        return Ok(());
    }

    if let Some(blocks) = g.early_actor_blocks.get_mut(namespace) {
        blocks.remove(address);
    }
    let Some(previous) = g.snapshots.get(namespace).cloned() else {
        return Ok(());
    };
    if previous.actors.get(address) == Some(&(0, false)) {
        return Ok(());
    }
    let mut snapshot = (*previous).clone();
    if let Err(error) = insert_actor(&mut snapshot.actors, address.to_owned(), (0, false)) {
        mark_actor_capacity_exhausted(g, namespace);
        return Err(error);
    }
    g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    snapshot.config_epoch = g.next_config_epoch;
    snapshot.actor_revision = actor_versions::revision(g, namespace);
    snapshot.generation = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}|known-unblock|{}",
            snapshot.generation, snapshot.config_epoch
        ))
    );
    g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
    g.results.clear();
    Ok(())
}
pub(super) fn publish_observed_guard(
    namespace: &str,
    address: &str,
    blocked: bool,
    started: &actor_versions::Proof,
) -> Result<Option<u64>, sqlx::Error> {
    let mut g = store().lock().unwrap();
    // A false response may authorize an unblock only if no newer authority
    // replaced the state observed when its query started. True stays conservative.
    if !blocked && !actor_versions::guard_current(&g, namespace, address, started) {
        return Ok(None);
    }
    if blocked {
        apply_known_block(&mut g, namespace, address)?;
    } else {
        apply_known_unblock(&mut g, namespace, address)?;
    }
    Ok(actor_versions::capture(
        &g,
        namespace,
        &HashMap::from([(address.into(), (0, blocked))]),
    )
    .entries_version(address))
}
pub(super) fn publish_actor_guard(
    namespace: &str,
    address: &str,
    blocked: bool,
) -> Result<u64, sqlx::Error> {
    let mut g = store().lock().unwrap();
    if blocked {
        apply_known_block(&mut g, namespace, address)?;
    } else {
        apply_known_unblock(&mut g, namespace, address)?;
    }
    Ok(actor_versions::capture(
        &g,
        namespace,
        &HashMap::from([(address.into(), (0, blocked))]),
    )
    .entries_version(address)
    .expect("completed guard has a proof"))
}
pub(super) fn observed_actor_contexts(namespace: &str) -> HashMap<String, (u16, bool)> {
    observed_actor_contexts_with_proof(namespace).0
}
pub(super) fn observed_actor_contexts_with_proof(
    namespace: &str,
) -> (HashMap<String, (u16, bool)>, actor_versions::Proof) {
    let g = store().lock().unwrap();
    let mut actors = g
        .snapshots
        .get(namespace)
        .map(|s| s.actors.clone())
        .unwrap_or_default();
    if let Some(blocks) = g.early_actor_blocks.get(namespace) {
        actors.extend(blocks.iter().map(|actor| (actor.clone(), (0, true))));
    }
    let proof = actor_versions::capture(&g, namespace, &actors);
    (actors, proof)
}
fn apply_known_block(g: &mut Store, namespace: &str, address: &str) -> Result<(), sqlx::Error> {
    if g.early_actor_blocks.len() >= 32 && !g.early_actor_blocks.contains_key(namespace) {
        g.actor_capacity_registry_exhausted = true;
        return Err(actor_capacity_error());
    }
    let blocks = g
        .early_actor_blocks
        .entry(namespace.to_owned())
        .or_default();
    if blocks.len() >= MAX_ACTOR_CONTEXTS && !blocks.contains(address) {
        mark_actor_capacity_exhausted(g, namespace);
        g.results.clear();
        return Err(actor_capacity_error());
    }
    let newly_observed = blocks.insert(address.to_owned());
    let before = g
        .snapshots
        .get(namespace)
        .and_then(|s| s.actors.get(address).copied());
    actor_versions::observe(
        g,
        namespace,
        address,
        (0, true),
        newly_observed || before.is_some_and(|c| !c.1),
    )?;

    if newly_observed {
        // Includes cold observations, so an older complete inventory scan can
        // never clear a guard merely because initialization has not finished.
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    }
    let Some(previous) = g.snapshots.get(namespace).cloned() else {
        // Keep the cold negative in the bounded ledger until a fresh explicit
        // unblock or complete epoch-checked inventory clears it.
        return Ok(());
    };
    if previous
        .actors
        .get(address)
        .is_some_and(|(_, blocked)| *blocked)
    {
        return Ok(());
    }
    let mut snapshot = (*previous).clone();
    if let Err(error) = insert_actor(&mut snapshot.actors, address.to_owned(), (0, true)) {
        mark_actor_capacity_exhausted(g, namespace);
        g.results.clear();
        return Err(error);
    }
    if !newly_observed {
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    }
    snapshot.config_epoch = g.next_config_epoch;
    snapshot.actor_revision = actor_versions::revision(g, namespace);
    snapshot.generation = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}|known-block|{address}|{}",
            snapshot.generation, snapshot.config_epoch
        ))
    );
    g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
    // Old deeper jobs may only replace an extant cache entry. Clearing it also
    // prevents progress polling from returning an earlier executable approval.
    g.results.clear();
    Ok(())
}

/// Operator-configured future buyers are ingested on the independent schedule,
/// including wallets that have never traded or owned a community token. Invalid
/// or oversized configuration fails admission instead of silently truncating it.
pub(super) fn configured_wallets(raw: Option<&str>) -> Result<Vec<String>, sqlx::Error> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    if raw.len() > MAX_ACTOR_CONTEXTS * 128 {
        return Err(sqlx::Error::Protocol(
            "Local wallet enrollment exceeds capacity".into(),
        ));
    }
    let mut wallets = std::collections::BTreeSet::new();
    for entry in raw
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        if entry.len() > 128
            || route_solver::parse_quote_trader(Some(entry.to_owned()), None).is_err()
        {
            return Err(sqlx::Error::Protocol(
                "Invalid local wallet enrollment address".into(),
            ));
        }
        wallets.insert(entry.to_owned());
        if wallets.len() > MAX_ACTOR_CONTEXTS {
            return Err(sqlx::Error::Protocol(
                "Local wallet enrollment exceeds capacity".into(),
            ));
        }
    }
    Ok(wallets.into_iter().collect())
}
/// CW20 enumeration is a background discovery source, never evidence of tax or
/// blacklist eligibility. Failed/malformed pages preserve the previous cursor.
pub(super) async fn holder_page(
    state: &AppState,
    token: &str,
    cursor: Option<&str>,
) -> Option<(Vec<String>, Option<String>)> {
    let page = query(
        state,
        token,
        json!({"all_accounts": {
            "start_after": cursor, "limit": HOLDER_ACCOUNTS_PER_PASS
        }}),
    )
    .await?;
    parse_holder_page(&page, cursor)
}
fn parse_holder_page(page: &Value, cursor: Option<&str>) -> Option<(Vec<String>, Option<String>)> {
    let rows = page["accounts"].as_array()?;
    if rows.len() > HOLDER_ACCOUNTS_PER_PASS {
        return None;
    }
    let mut wallets = Vec::new();
    let mut previous = cursor;
    for row in rows {
        let address = row.as_str()?;
        if address.len() > 128
            || previous.is_some_and(|before| address <= before)
            || route_solver::parse_quote_trader(Some(address.to_owned()), None).is_err()
        {
            return None;
        }
        wallets.push(address.to_owned());
        previous = Some(address);
    }
    let next = (wallets.len() == HOLDER_ACCOUNTS_PER_PASS).then(|| wallets.last().unwrap().clone());
    Some((wallets, next))
}
pub(super) fn retained_blocks(
    previous: &HashMap<String, (u16, bool)>,
) -> Result<HashMap<String, (u16, bool)>, sqlx::Error> {
    let blocks = previous
        .iter()
        .filter(|(_, (_, blocked))| *blocked)
        .map(|(address, _)| (address.clone(), (0, true)))
        .collect::<HashMap<_, _>>();
    if blocks.len() > MAX_ACTOR_CONTEXTS {
        return Err(actor_capacity_error());
    }
    Ok(blocks)
}
pub(super) fn actor_capacity_error() -> sqlx::Error {
    sqlx::Error::Protocol("Local actor observation capacity exhausted".into())
}
/// Fees can fall back conservatively; an observed block is never a capacity victim.
pub(super) fn insert_actor(
    actors: &mut HashMap<String, (u16, bool)>,
    address: String,
    context: (u16, bool),
) -> Result<(), sqlx::Error> {
    if !actors.contains_key(&address) && actors.len() >= MAX_ACTOR_CONTEXTS {
        let victim = actors
            .iter()
            .filter(|(_, (_, blocked))| !*blocked)
            .map(|(address, _)| address)
            .min()
            .cloned();
        if let Some(victim) = victim {
            actors.remove(&victim);
        } else if context.1 {
            return Err(actor_capacity_error());
        } else {
            return Ok(());
        }
    }
    actors.insert(address, context);
    Ok(())
}
/// Reserve eight observations for forgotten-but-still-blocked wallets. Rotation
/// is per namespace and advances independently of request traffic or snapshot epochs.
pub(super) fn actor_refresh_addresses(
    recent: Vec<String>,
    previous: &HashMap<String, (u16, bool)>,
    cursor: usize,
) -> (Vec<String>, usize) {
    let mut selected = recent.into_iter().take(RECENT_ACTORS).collect::<Vec<_>>();
    let mut historical = previous
        .iter()
        .filter(|(address, (_, blocked))| *blocked && !selected.contains(address))
        .map(|(address, _)| address.clone())
        .collect::<Vec<_>>();
    historical.sort();
    if historical.is_empty() {
        return (selected, 0);
    }
    let count = historical.len().min(HISTORICAL_BLOCK_REFRESH);
    for offset in 0..count {
        selected.push(historical[(cursor % historical.len() + offset) % historical.len()].clone());
    }
    (
        selected,
        (cursor % historical.len() + count) % historical.len(),
    )
}
/// Refresh immutable reserves independently of slow configuration/network admission.
/// Only scheduled workers and test setup call this function.
pub async fn refresh_reserves(state: &AppState) -> Result<(), sqlx::Error> {
    let ns = state.lcd.chain_cache_namespace();
    let snapshot = store().lock().unwrap().snapshots.get(&ns).cloned();
    let Some(s) = snapshot else {
        return Ok(());
    };
    if s.config_expiry_ms <= now_ms() {
        return Ok(());
    }
    let mut mirrors = HashMap::new();
    let mut pool_mirrors = HashMap::new();
    let mut config_expiries = s.config_expiries.clone();
    for pair in &s.pairs {
        if s.config_expiries
            .get(&pair.contract_address)
            .copied()
            .unwrap_or(s.config_expiry_ms)
            <= now_ms()
        {
            continue;
        }
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&pair.contract_address) {
            continue;
        }
        let context = s
            .pair_contexts
            .get(&pair.contract_address)
            .map(String::as_str);
        if let Some((views, retained_expiry)) =
            load_qualified_mirror(state, pair, &s.addresses, s.max_age_ms, context).await
        {
            if let Some(expiry) = retained_expiry {
                config_expiries
                    .entry(pair.contract_address.clone())
                    .and_modify(|at| *at = (*at).min(expiry))
                    .or_insert(expiry);
            }
            if let Some(mirror) = views.hybrid {
                mirrors.insert(pair.contract_address.clone(), mirror);
            }
            if let Some(mirror) = views.pool {
                pool_mirrors.insert(pair.contract_address.clone(), mirror);
            }
        }
    }
    publication(
        ns,
        s.pairs.clone(),
        s.addresses.clone(),
        mirrors,
        pool_mirrors,
        s.max_age_ms,
        s.hooks.clone(),
        s.config_expiry_ms,
        config_expiries,
        s.pair_contexts.clone(),
        local_reference_quote::ReferencePrices::observe(
            &state.pool,
            &sqlx::query_as::<_, assets::AssetRow>("SELECT * FROM assets ORDER BY id LIMIT 2048")
                .fetch_all(&state.pool)
                .await?,
            &state.hub_usd,
        )
        .await?,
        s.actors.clone(),
        s.discounted_pairs.clone(),
        s.policy_identity.clone(),
        s.taxes.clone(),
        Some(s.config_epoch),
    )
    .scheduled()
    .await?;
    Ok(())
}

/// Shared by reserve and configuration publication. Only the explicit fee
/// generation error permits retention; SQL, malformed/zero data and negatives do not.
pub(super) struct QualifiedViews {
    pub hybrid: Option<Arc<sim::HopMirror>>,
    pub pool: Option<Arc<sim::HopMirror>>,
}
pub(super) async fn load_qualified_mirror(
    state: &AppState,
    pair: &pairs::PairRow,
    addresses: &HashMap<i32, String>,
    max_age_ms: u64,
    context: Option<&str>,
) -> Option<(QualifiedViews, Option<i64>)> {
    let namespace = state.lcd.chain_cache_namespace();
    match sim::load_local_views_for_quote(&state.pool, pair, addresses, max_age_ms, &namespace)
        .await
    {
        Ok(views) => Some((
            QualifiedViews {
                hybrid: views.hybrid.map(Arc::new),
                pool: views.pool.map(Arc::new),
            },
            None,
        )),
        Err(sim::DbSimError::IncompleteFeeGeneration {
            height,
            observed_at,
            reserve_0,
            reserve_1,
        }) => {
            let views = crate::indexer::fee_phase::retained_views(
                &namespace,
                pair.id,
                context?,
                height,
                observed_at,
                (reserve_0, reserve_1),
            )?;
            // Retention never extends the original qualification of either view.
            let expiry = views
                .hybrid
                .iter()
                .chain(views.pool.iter())
                .map(|(_, expiry)| *expiry)
                .min()?;
            Some((
                QualifiedViews {
                    hybrid: views.hybrid.map(|(mirror, _)| mirror),
                    pool: views.pool.map(|(mirror, _)| mirror),
                },
                Some(expiry),
            ))
        }
        Err(sim::DbSimError::FeeGenerationRejected) => None,
        Err(_) => {
            crate::indexer::fee_phase::reject(&namespace, pair.id, None, Utc::now());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn issue1489_expired_positive_fee_cannot_survive_or_renew_on_same_guard() {
        let ns = "expired-positive-fee1489";
        let mut g = Store::default();
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = ns.into();
        snapshot.config_expiry_ms = now_ms() - 1;
        snapshot.actors.insert("actor".into(), (800, false));
        g.snapshots.insert(ns.into(), Arc::new(snapshot.clone()));
        let old_version = actor_versions::observe(&mut g, ns, "actor", (0, false), false).unwrap();
        let old = actor_versions::capture(&g, ns, &snapshot.actors);
        apply_known_unblock(&mut g, ns, "actor").unwrap();
        assert_eq!(g.snapshots[ns].actors["actor"], (0, false));
        assert_eq!(g.snapshots[ns].config_expiry_ms, snapshot.config_expiry_ms);
        assert_ne!(
            actor_versions::capture(&g, ns, &snapshot.actors).entries_version("actor"),
            Some(old_version)
        );
        actor_versions::reconcile(&mut g, &mut snapshot, &old).unwrap();
        assert_eq!(snapshot.actors["actor"], (0, false));
    }
    #[test]
    fn issue1489_same_unblocked_observation_keeps_verified_discount_and_proof() {
        let namespace = "unchanged-actor-refresh1489";
        let actor = "actor";
        let mut g = Store::default();
        let mut snapshot = super::super::tests::fixture();
        snapshot.namespace = namespace.into();
        snapshot.actors.insert(actor.into(), (800, false));
        let original = Arc::new(snapshot);
        g.snapshots.insert(namespace.into(), Arc::clone(&original));
        let version = actor_versions::observe(&mut g, namespace, actor, (0, false), false).unwrap();
        apply_known_unblock(&mut g, namespace, actor).unwrap();
        let current = &g.snapshots[namespace];
        assert_eq!(
            current.actors[actor],
            (800, false),
            "same positive guard must not clear verified fees while the optional read awaits"
        );
        assert!(Arc::ptr_eq(&original, current));
        assert_eq!(
            actor_versions::capture(&g, namespace, &current.actors).entries_version(actor),
            Some(version)
        );
    }

    #[test]
    fn known_block_publication_is_synchronous_and_invalidates_warm_approval_without_enrichment() {
        let s = super::super::tests::fixture();
        let old_generation = s.generation.clone();
        let original_expiry = s.config_expiry_ms;
        let mut g = Store::default();
        g.snapshots.insert("negative-publish".into(), Arc::new(s));
        let mut reviewed = (*g.snapshots["negative-publish"]).clone();
        reviewed.generation = admission_generation(&reviewed);
        assert!(reviewed_snapshot_current(
            &g,
            "negative-publish",
            &reviewed,
            None
        ));
        g.results.insert(
            "warm-progress".into(),
            json!({"estimated_amount_out":"100"}),
        );
        apply_known_block(&mut g, "negative-publish", "wallet").unwrap();
        let current = &g.snapshots["negative-publish"];
        assert_eq!(current.actors["wallet"], (0, true));
        assert_ne!(current.generation, old_generation);
        assert!(current.config_epoch > 0);
        assert_eq!(current.config_expiry_ms, original_expiry);
        assert!(g.results.is_empty());
        assert!(
            !reviewed_snapshot_current(&g, "negative-publish", &reviewed, None),
            "in-flight quote cannot republish an approval computed before the block"
        );
        // There are no DB/network calls in this publication; subsequent worker
        // failure/cancellation leaves the negative in the initialized snapshot.
        let epoch = current.config_epoch;
        apply_known_block(&mut g, "negative-publish", "wallet").unwrap();
        assert_eq!(g.snapshots["negative-publish"].config_epoch, epoch);
    }
    #[test]
    fn immediate_negative_capacity_exhaustion_quarantines_without_evicting_known_blocks() {
        let mut s = super::super::tests::fixture();
        s.actors = (0..MAX_ACTOR_CONTEXTS)
            .map(|i| (format!("blocked-{i}"), (0, true)))
            .collect();
        let original = s.actors.clone();
        let mut g = Store::default();
        g.snapshots.insert("negative-overflow".into(), Arc::new(s));
        g.results.insert("warm".into(), json!({}));
        assert!(apply_known_block(&mut g, "negative-overflow", "new-block").is_err());
        assert_eq!(g.snapshots["negative-overflow"].actors, original);
        assert!(actor_capacity_unavailable(&g, "negative-overflow"));
        assert!(g.results.is_empty());
    }
    #[test]
    fn cold_negative_survives_cancelled_initialization_and_is_bounded() {
        let mut g = Store::default();
        apply_known_block(&mut g, "cold-negative", "wallet").unwrap();
        assert!(g.snapshots.is_empty());
        assert!(g.early_actor_blocks["cold-negative"].contains("wallet"));
        assert!(g.next_config_epoch > 0);
        g.early_actor_blocks.insert(
            "full-cold".into(),
            (0..MAX_ACTOR_CONTEXTS)
                .map(|i| format!("blocked-{i}"))
                .collect(),
        );
        assert!(apply_known_block(&mut g, "full-cold", "one-more").is_err());
        assert_eq!(g.early_actor_blocks["full-cold"].len(), MAX_ACTOR_CONTEXTS);
        assert!(actor_capacity_unavailable(&g, "full-cold"));
    }

    #[test]
    fn configured_uncensused_wallets_are_bounded_validated_and_deduplicated() {
        let first = "terra1000000000000000000000000000000000001";
        let second = "terra1000000000000000000000000000000000002";
        assert_eq!(configured_wallets(None).unwrap(), Vec::<String>::new());
        assert_eq!(
            configured_wallets(Some(&format!(" {second}, {first},{first}, "))).unwrap(),
            vec![first, second]
        );
        assert!(configured_wallets(Some("untrusted-wallet")).is_err());
        let oversized = (0..=MAX_ACTOR_CONTEXTS)
            .map(|i| format!("terra1{i:038}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(configured_wallets(Some(&oversized)).is_err());
    }
    #[test]
    fn holder_pagination_requires_bounded_strict_progress_and_restarts_completed_crawl() {
        let accounts = (1..=4)
            .map(|i| format!("terra1{i:038}"))
            .collect::<Vec<_>>();
        let (selected, next) = parse_holder_page(&json!({"accounts":accounts}), None).unwrap();
        assert_eq!(selected, accounts);
        assert_eq!(next.as_deref(), Some(accounts[3].as_str()));
        assert_eq!(
            parse_holder_page(&json!({"accounts":[]}), next.as_deref()),
            Some((Vec::new(), None))
        );
        assert!(parse_holder_page(&json!({"accounts":[accounts[3]]}), next.as_deref()).is_none());
        assert!(parse_holder_page(&json!({"accounts":[accounts[1], accounts[0]]}), None).is_none());
        assert!(parse_holder_page(&json!({"accounts":[accounts[0], accounts[0]]}), None).is_none());
        assert!(parse_holder_page(&json!({"accounts":["bad-address"]}), None).is_none());
        assert!(parse_holder_page(
            &json!({"accounts":[accounts[0], accounts[1], accounts[2], accounts[3], "extra"]}),
            None
        )
        .is_none());
        assert!(parse_holder_page(&json!({}), None).is_none());
    }
    #[test]
    fn known_wallet_block_survives_missing_fee_and_failed_blacklist_refresh() {
        assert_eq!(actor_context(Some(true), None, None), Some((0, true)));
        assert_eq!(
            actor_context(None, None, Some((250, true))),
            Some((0, true))
        );
        assert_eq!(
            actor_context(Some(false), Some(250), Some((0, true))),
            Some((250, false))
        );
        assert_eq!(actor_context(None, None, Some((250, false))), None);
        assert_eq!(actor_context(Some(false), None, None), Some((0, false)));
    }
    #[test]
    fn previous_block_absent_from_recent_census_is_retained_and_fresh_unblock_needs_no_fee() {
        let previous = HashMap::from([
            ("historical-block".into(), (250, true)),
            ("old-fee".into(), (500, false)),
        ]);
        let mut actors = retained_blocks(&previous).unwrap();
        assert_eq!(actors.get("historical-block"), Some(&(0, true)));
        assert!(!actors.contains_key("old-fee"));
        let context = actor_context(None, None, previous.get("historical-block").copied()).unwrap();
        insert_actor(&mut actors, "historical-block".into(), context).unwrap();
        assert!(actors["historical-block"].1);
        let cleared =
            actor_context(Some(false), None, previous.get("historical-block").copied()).unwrap();
        insert_actor(&mut actors, "historical-block".into(), cleared).unwrap();
        assert_eq!(actors["historical-block"], (0, false));
    }
    #[test]
    fn historical_blocks_rotate_outside_recent_census_with_fixed_observation_budget() {
        let previous = (0..20)
            .map(|i| (format!("historical-{i:02}"), (0, true)))
            .collect::<HashMap<_, _>>();
        let recent = (0..120)
            .map(|i| format!("recent-{i:03}"))
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        let mut cursor = 0;
        for _ in 0..3 {
            let (selected, next) = actor_refresh_addresses(recent.clone(), &previous, cursor);
            assert_eq!(selected.len(), 128);
            assert_eq!(&selected[..120], recent.as_slice());
            seen.extend(selected[120..].iter().cloned());
            cursor = next;
        }
        assert_eq!(seen.len(), 20);
    }
    #[test]
    fn actor_capacity_never_evicts_blocks_and_new_block_overflow_declines_publication() {
        let mut actors = (0..MAX_ACTOR_CONTEXTS)
            .map(|i| (format!("blocked-{i:04}"), (0, true)))
            .collect::<HashMap<_, _>>();
        let original = actors.clone();
        insert_actor(&mut actors, "unknown-fee".into(), (500, false)).unwrap();
        assert_eq!(actors, original);
        assert!(insert_actor(&mut actors, "new-block".into(), (0, true)).is_err());
        assert_eq!(actors, original);
        actors.insert("blocked-0000".into(), (0, false));
        insert_actor(&mut actors, "new-block".into(), (0, true)).unwrap();
        assert_eq!(actors.len(), MAX_ACTOR_CONTEXTS);
        assert!(!actors.contains_key("blocked-0000"));
        assert!(actors["new-block"].1);
    }
}
/// Scheduled wrapper observation pass; requests never call it.
pub async fn refresh_wrapper(state: &AppState) -> bool {
    if let Some(w) = local_wrapper_quote::observe(state).await {
        publish_wrapper(state.lcd.chain_cache_namespace(), w);
        true
    } else {
        false
    }
}
/// Counters contain no actor/input information and make cache/coalescing evidence testable.
pub fn cache_counters() -> (u64, u64, usize, usize) {
    let g = store().lock().unwrap();
    (g.hits, g.started, g.jobs.len(), g.results.len())
}
/// Independent scheduled workers, each with one outstanding pass. Missed ticks skip;
/// requests never enqueue, retry or accelerate ingestion.
pub fn start_ingestion(state: AppState) {
    let mut workers =
        start_ingestion_with_cancel(state, tokio_util::sync::CancellationToken::new());
    // Compatibility for callers explicitly owning process-lifetime ingestion.
    workers.detach_all();
}

/// The API owns and joins these producers. Fatal indexing cancels in-flight passes
/// through the health notification; transient provider availability does not.
pub fn start_ingestion_with_cancel(
    mut state: AppState,
    cancel: tokio_util::sync::CancellationToken,
) -> tokio::task::JoinSet<()> {
    let halted = state.indexer_health.halt_token();
    let mut workers = tokio::task::JoinSet::new();
    state.lcd = state.lcd.with_ingestion_budget();
    let wrapper_state = state.clone();
    spawn_ingestion_worker(&mut workers, &cancel, &halted, async move {
        let mut timer = tokio::time::interval(Duration::from_secs(10));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Some(w) = local_wrapper_quote::observe(&wrapper_state).await {
                publish_wrapper(wrapper_state.lcd.chain_cache_namespace(), w);
            }
        }
    });
    let reserves = state.clone();
    spawn_ingestion_worker(&mut workers, &cancel, &halted, async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            match tokio::time::timeout(Duration::from_secs(2), refresh_reserves(&reserves)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("Local reserve ingestion unavailable: {e}"),
                Err(_) => tracing::warn!("Local reserve ingestion exceeded deadline"),
            }
        }
    });
    spawn_ingestion_worker(&mut workers, &cancel, &halted, async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            match tokio::time::timeout(
                Duration::from_secs(60),
                super::admission::ingest_once(&state),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("Local quote admission unavailable: {e}"),
                Err(_) => tracing::warn!("Local quote admission exhausted pass budget"),
            }
        }
    });
    workers
}

fn spawn_ingestion_worker<F>(
    workers: &mut tokio::task::JoinSet<()>,
    cancel: &tokio_util::sync::CancellationToken,
    halted: &tokio_util::sync::CancellationToken,
    worker: F,
) where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let cancel = cancel.clone();
    let halted = halted.clone();
    workers.spawn(async move {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => {},
            _ = halted.cancelled() => {},
            _ = worker => {},
        }
    });
}
