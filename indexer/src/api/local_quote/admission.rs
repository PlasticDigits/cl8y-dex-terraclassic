//! Incremental scheduled configuration admission. Each completed observation can
//! initialize quotes without waiting for a complete factory census.
use super::ingestion::{
    actor_context, actor_refresh_addresses, configured_wallets, holder_page, insert_actor, query,
    retained_blocks, ENROLLED_ACTORS_PER_PASS,
};
use super::*;
use crate::api::{
    community_tax_rank, local_hook_pricing, local_reference_quote, trading_tax_discovery,
};

const PAIRS_PER_PASS: usize = 8;
const ACTORS_PER_PASS: usize = 8;
#[derive(Clone)]
struct PairObservation {
    pair: pairs::PairRow,
    asset_addresses: (String, String),
    expiry_ms: i64,
    hooks: local_hook_pricing::CachedHookPrices,
    discounted: bool,
    identity: String,
    taxes: HashMap<String, tax::TokenObservation>,
}
#[derive(Default)]
struct Admission {
    policy_identity: String,
    cursor: usize,
    actor_cursor: usize,
    enrollment_cursor: usize,
    holder_cursors: HashMap<String, Option<String>>,
    tax_wallets: HashMap<String, tax::TokenObservation>,
    pairs: HashMap<String, PairObservation>,
}
type Admissions = HashMap<String, Arc<tokio::sync::Mutex<Admission>>>;
fn admissions() -> &'static Mutex<Admissions> {
    static ADMISSIONS: OnceLock<Mutex<Admissions>> = OnceLock::new();
    ADMISSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
const MAX_TAX_WALLET_LEDGERS: usize = 2048;
fn retain_wallet_ledgers(
    ledgers: &mut HashMap<String, tax::TokenObservation>,
    live_assets: &HashSet<&String>,
) {
    ledgers.retain(|token, observation| {
        live_assets.contains(token)
            || observation.observed_at_ms == 0
            || observation.actors.values().any(|actor| actor.protocol)
    });
}
fn record_wallet_ledger(
    ledgers: &mut HashMap<String, tax::TokenObservation>,
    token: &str,
    observation: &mut tax::TokenObservation,
) -> bool {
    if !ledgers.contains_key(token) && ledgers.len() >= MAX_TAX_WALLET_LEDGERS {
        return false;
    }
    if let Some(previous) = ledgers.get(token) {
        observation.inherit_wallet_contexts(previous);
    }
    let mut wallet_ledger = observation.clone();
    wallet_ledger.pairs.clear();
    ledgers.insert(token.to_owned(), wallet_ledger);
    true
}
/// Failed required asset semantics invalidate every related admission slice,
/// including unselected pairs. Pool/book refresh failures use their own lanes.
fn invalidate_asset_observations(admission: &mut Admission, asset: &str) {
    admission.pairs.retain(|_, observation| {
        observation.asset_addresses.0 != asset && observation.asset_addresses.1 != asset
    });
}
fn invalidate_cached_asset(g: &mut Store, namespace: &str, asset: &str) {
    let Some(previous) = g.snapshots.get(namespace).cloned() else {
        return;
    };
    let mut snapshot = (*previous).clone();
    snapshot.pairs.retain(|pair| {
        snapshot
            .addresses
            .get(&pair.asset_0_id)
            .is_none_or(|address| address != asset)
            && snapshot
                .addresses
                .get(&pair.asset_1_id)
                .is_none_or(|address| address != asset)
    });
    if snapshot.pairs.len() == previous.pairs.len() {
        return;
    }
    let retained = snapshot
        .pairs
        .iter()
        .map(|pair| pair.contract_address.clone())
        .collect::<HashSet<_>>();
    snapshot.mirrors.retain(|pair, _| retained.contains(pair));
    snapshot.hooks.retain(|pair, _| retained.contains(pair));
    snapshot
        .config_expiries
        .retain(|pair, _| retained.contains(pair));
    snapshot
        .discounted_pairs
        .retain(|pair| retained.contains(pair));
    g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    snapshot.config_epoch = g.next_config_epoch;
    snapshot.generation = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}|required-asset-unavailable|{asset}|{}",
            snapshot.generation, snapshot.config_epoch
        ))
    );
    g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
    g.results.clear();
}
fn selected_indices(len: usize, cursor: usize, budget: usize) -> (Vec<usize>, usize) {
    if len == 0 {
        return (Vec::new(), 0);
    }
    let count = budget.min(len);
    (
        (0..count).map(|i| (cursor % len + i) % len).collect(),
        (cursor % len + count) % len,
    )
}
/// A bounded slice, invoked only by scheduled ingestion and controlled fixtures.
pub async fn ingest_once(state: &AppState) -> Result<(), sqlx::Error> {
    let namespace = state.lcd.chain_cache_namespace();
    let admission = {
        let mut all = admissions().lock().unwrap();
        if all.len() >= 32 && !all.contains_key(&namespace) {
            return Err(sqlx::Error::Protocol(
                "Local admission namespace capacity exhausted".into(),
            ));
        }
        all.entry(namespace.clone()).or_default().clone()
    };
    // Serialize even explicit fixture calls; reserve publication has independent epoch protection.
    let mut admission = admission.lock().await;
    if !super::actor_inventory::recover(state).await {
        return Err(sqlx::Error::Protocol(
            "Authoritative actor inventory unavailable".into(),
        ));
    }

    let assets: Vec<assets::AssetRow> =
        sqlx::query_as("SELECT * FROM assets ORDER BY id LIMIT 2048")
            .fetch_all(&state.pool)
            .await?;
    let (addresses, _) = route_solver::build_id_to_addr_map(&assets);
    let all_pairs: Vec<pairs::PairRow> =
        sqlx::query_as("SELECT * FROM pairs ORDER BY contract_address LIMIT 512")
            .fetch_all(&state.pool)
            .await?;
    let policy = community_tax_rank::RouterHopTaxPolicy::from_catalog(&state.community_tax);
    let policy_identity = format!(
        "{:?}|{:?}|{policy:?}",
        state.factory_address, state.router_address
    );
    if admission.policy_identity != policy_identity {
        admission.pairs.clear();
        admission.holder_cursors.clear();
        admission.policy_identity = policy_identity;
    }
    let live = all_pairs
        .iter()
        .map(|p| (p.contract_address.as_str(), p))
        .collect::<HashMap<_, _>>();
    admission.pairs.retain(|address, obs| {
        live.get(address.as_str()).is_some_and(|current| {
            current.asset_0_id == obs.pair.asset_0_id && current.asset_1_id == obs.pair.asset_1_id
        }) && addresses.get(&obs.pair.asset_0_id) == Some(&obs.asset_addresses.0)
            && addresses.get(&obs.pair.asset_1_id) == Some(&obs.asset_addresses.1)
            && !crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(address)
            && obs.expiry_ms > now_ms()
    });
    let (selected, next) = selected_indices(all_pairs.len(), admission.cursor, PAIRS_PER_PASS);
    admission.cursor = next;
    let recent: Vec<String> = sqlx::query_scalar(
        "SELECT address FROM traders ORDER BY last_trade_at DESC NULLS LAST,address LIMIT 120",
    )
    .fetch_all(&state.pool)
    .await?;
    let (indices, next) = selected_indices(recent.len(), admission.actor_cursor, ACTORS_PER_PASS);
    admission.actor_cursor = next;
    let mut recent = indices
        .into_iter()
        .map(|i| recent[i].clone())
        .collect::<Vec<_>>();
    let enrollment_raw = std::env::var("LOCAL_QUOTE_WALLETS").ok();
    let enrolled = configured_wallets(enrollment_raw.as_deref())?;
    let (indices, next) = selected_indices(
        enrolled.len(),
        admission.enrollment_cursor,
        ENROLLED_ACTORS_PER_PASS,
    );
    admission.enrollment_cursor = next;
    for i in indices {
        if !recent.contains(&enrolled[i]) {
            recent.push(enrolled[i].clone());
        }
    }
    // Cursor memory follows the capped live asset inventory, independent of requests.
    let live_assets = addresses.values().collect::<HashSet<_>>();
    admission
        .holder_cursors
        .retain(|token, _| live_assets.contains(token));
    retain_wallet_ledgers(&mut admission.tax_wallets, &live_assets);
    let previous = ingestion::observed_actor_contexts(&namespace);
    let mut actors = retained_blocks(&previous).map_err(|e| {
        mark_actor_capacity_exhausted(&mut store().lock().unwrap(), &namespace);
        e
    })?;
    let actor_addresses = {
        let mut g = store().lock().unwrap();
        let cursor = g
            .actor_refresh_cursors
            .get(&namespace)
            .copied()
            .unwrap_or(0);
        let (selected, next) = actor_refresh_addresses(recent, &previous, cursor);
        g.actor_refresh_cursors.insert(namespace.clone(), next);
        selected
    };
    let mut actor_observed_at = HashMap::new();
    if let Some(factory) = state.factory_address.as_deref() {
        for address in &actor_addresses {
            let observed_at = now_ms();
            let blocked = query(
                state,
                factory,
                json!({"blacklist_check":{"wallet":address,"tokens":[],"pairs":[],"pair":null}}),
            )
            .await
            .and_then(|v| v["wallet_blacklisted"].as_bool());
            if blocked == Some(true) {
                ingestion::publish_known_block(&namespace, address)?;
            } else if blocked == Some(false) {
                ingestion::clear_known_block(&namespace, address);
            }
            let discount = if blocked == Some(false) {
                if let Some(registry) = state.fee_discount_address.as_deref() {
                    query(
                        state,
                        registry,
                        json!({"get_discount":{"trader":address,"sender":address}}),
                    )
                    .await
                    .and_then(|v| v["discount_bps"].as_u64())
                    .filter(|d| *d <= 10_000)
                    .map(|d| d as u16)
                } else {
                    Some(0)
                }
            } else {
                None
            };
            if let Some(context) = actor_context(blocked, discount, previous.get(address).copied())
            {
                if blocked == Some(false) {
                    actor_observed_at.insert(address.clone(), observed_at);
                }
                if let Err(error) = insert_actor(&mut actors, address.clone(), context) {
                    mark_actor_capacity_exhausted(&mut store().lock().unwrap(), &namespace);
                    return Err(error);
                }
            }
        }
    }
    // Publish newly observed wallet guards before slow token/pair discovery;
    // cancellation of that work must not leave a known block in a warm cache.
    publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await?;
    let mut tokens: HashMap<String, (u64, i64, Option<tax::TokenObservation>)> = HashMap::new();
    for i in selected.iter().copied() {
        let pair = &all_pairs[i];
        // Refresh replaces the old observation immediately; failed admission cannot extend it.
        admission.pairs.remove(&pair.contract_address);
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&pair.contract_address) {
            continue;
        }
        let (Some(a), Some(b)) = (
            addresses.get(&pair.asset_0_id),
            addresses.get(&pair.asset_1_id),
        ) else {
            continue;
        };
        if crate::indexer::defillama::is_columbus5_gem_address(a)
            || crate::indexer::defillama::is_columbus5_gem_address(b)
        {
            continue;
        }
        for address in [a, b] {
            if tokens.contains_key(address) {
                continue;
            }
            let observed = now_ms();
            let verdict = trading_tax_discovery::discover(state, address, &policy).await;
            let observation = match verdict {
                trading_tax_discovery::TradingTax::Ordinary {
                    code_id: Some(code),
                } => Some((code as u64, observed + CONFIG_TTL_MS, None)),
                trading_tax_discovery::TradingTax::Community(token)
                    if token.route_funding_version == Some(1) =>
                {
                    let related = selected
                        .iter()
                        .map(|i| &all_pairs[*i])
                        .filter(|p| {
                            addresses.get(&p.asset_0_id) == Some(address)
                                || addresses.get(&p.asset_1_id) == Some(address)
                        })
                        .map(|p| p.contract_address.clone())
                        .collect::<Vec<_>>();
                    let mut tax_actors = actor_addresses
                        .iter()
                        .filter(|actor| actors.get(*actor).is_some_and(|(_, blocked)| !blocked))
                        .filter_map(|actor| {
                            actor_observed_at.get(actor).map(|at| (actor.clone(), *at))
                        })
                        .collect::<Vec<_>>();
                    // Discover holders outside the trader census on the same scheduled
                    // worker. Enumeration alone never authorizes a wallet: the factory
                    // blacklist read and per-token identity/balance must also succeed.
                    let cursor = admission
                        .holder_cursors
                        .get(address)
                        .and_then(|v| v.as_deref());
                    if let Some((holders, next)) = holder_page(state, address, cursor).await {
                        admission.holder_cursors.insert(address.clone(), next);
                        if let Some(factory) = state.factory_address.as_deref() {
                            for actor in holders {
                                let observed_at = now_ms();
                                let blocked = query(state, factory,
                                    json!({"blacklist_check":{"wallet":actor,"tokens":[],"pairs":[],"pair":null}}))
                                    .await.and_then(|v| v["wallet_blacklisted"].as_bool());
                                if blocked == Some(true) {
                                    ingestion::publish_known_block(&namespace, &actor)?;
                                } else if blocked == Some(false) {
                                    ingestion::clear_known_block(&namespace, &actor);
                                }
                                if let Some(context) =
                                    actor_context(blocked, None, previous.get(&actor).copied())
                                {
                                    if let Err(error) =
                                        insert_actor(&mut actors, actor.clone(), context)
                                    {
                                        mark_actor_capacity_exhausted(
                                            &mut store().lock().unwrap(),
                                            &namespace,
                                        );
                                        return Err(error);
                                    }
                                    if blocked == Some(false) {
                                        let at = actor_observed_at
                                            .entry(actor.clone())
                                            .or_insert(observed_at);
                                        *at = (*at).min(observed_at);
                                        if let Some((_, previous_at)) = tax_actors
                                            .iter_mut()
                                            .find(|(wallet, _)| wallet == &actor)
                                        {
                                            *previous_at = (*previous_at).min(*at);
                                        } else {
                                            tax_actors.push((actor, *at));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Preserve newly observed negatives even if subsequent token reads
                    // fail or the pass deadline cancels this admission slice.
                    publish_admission(state, &namespace, &admission, &assets, &addresses, &actors)
                        .await?;
                    if let Some(mut obs) = tax::observe(state, &token, &related, &tax_actors).await
                    {
                        if !record_wallet_ledger(&mut admission.tax_wallets, address, &mut obs) {
                            // Never evict a known protocol negative to admit a new asset.
                            mark_tax_wallet_capacity_exhausted(
                                &mut store().lock().unwrap(),
                                &namespace,
                            );
                            return Err(sqlx::Error::Protocol(
                                "Local tax wallet ledger capacity exhausted".into(),
                            ));
                        }
                        Some((obs.code_id, observed + CONFIG_TTL_MS, Some(obs)))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(obs) = observation {
                tokens.insert(address.clone(), obs);
            } else {
                invalidate_asset_observations(&mut admission, address);
                // Synchronous invalidation precedes subsequent provider/database
                // awaits, so a known unsupported mode cannot survive cancellation
                // through another pair's old static approval or a warm cache.
                invalidate_cached_asset(&mut store().lock().unwrap(), &namespace, address);
            }
        }
        let (Some((code_a, expiry_a, tax_a)), Some((code_b, expiry_b, tax_b))) =
            (tokens.get(a), tokens.get(b))
        else {
            continue;
        };
        let Some(factory) = state.factory_address.as_deref() else {
            continue;
        };
        let started = now_ms();
        let checked = tokio::time::timeout(Duration::from_secs(6),async {
            let paused=query(state,&pair.contract_address,json!({"is_paused":{}})).await?;
            let hooks=query(state,&pair.contract_address,json!({"get_hooks":{}})).await?;
            let pins=query(state,&pair.contract_address,json!({"get_asset_code_ids":{}})).await?;
            let blacklist=query(state,factory,json!({"blacklist_check":{"tokens":[a,b],"pairs":[pair.contract_address],"wallet":null,"pair":null}})).await?;
            let wa=query(state,factory,json!({"is_code_id_whitelisted":{"code_id":code_a}})).await?;
            let wb=query(state,factory,json!({"is_code_id_whitelisted":{"code_id":code_b}})).await?;
            if paused["paused"].as_bool()!=Some(false) || pins["code_ids"]!=json!([code_a,code_b]) || !blacklist["blacklisted_tokens"].as_array().is_some_and(|v|v.is_empty()) || !blacklist["blacklisted_pairs"].as_array().is_some_and(|v|v.is_empty()) || wa["whitelisted"].as_bool()!=Some(true) || wb["whitelisted"].as_bool()!=Some(true) {return None;}
            let hook_list=hooks["hooks"].as_array()?;
            if hook_list.len()>8 {return None;}
            let mut hook_obs=Vec::new();
            for hook in hook_list {let addr=hook.as_str()?;hook_obs.push((addr.to_string(),query(state,addr,json!({"get_config":{}})).await?));}
            let prices=local_hook_pricing::CachedHookPrices::from_observation(&hook_obs).ok()?;
            let registry=query(state,&pair.contract_address,json!({"get_discount_registry":{}})).await;
            let discounted=state.fee_discount_address.as_deref().is_some_and(|configured|registry.as_ref().and_then(|v|v["registry"].as_str())==Some(configured));
            Some((prices,discounted,format!("{}|{pins}|{registry:?}",pair.contract_address)))
        }).await.ok().flatten();
        if let Some((hooks, discounted, identity)) = checked {
            let taxes = [(a, tax_a), (b, tax_b)]
                .into_iter()
                .filter_map(|(a, t)| t.as_ref().map(|t| (a.clone(), t.clone())))
                .collect();
            admission.pairs.insert(
                pair.contract_address.clone(),
                PairObservation {
                    pair: pair.clone(),
                    asset_addresses: (a.clone(), b.clone()),
                    expiry_ms: (started + CONFIG_TTL_MS).min(*expiry_a).min(*expiry_b),
                    hooks,
                    discounted,
                    identity,
                    taxes,
                },
            );
        }
        publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await?;
    }
    publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await
}
fn fresh_observations(admission: &Admission, observed_at: i64) -> Vec<&PairObservation> {
    let mut fresh = admission
        .pairs
        .values()
        .filter(|p| p.expiry_ms > observed_at)
        .collect::<Vec<_>>();
    fresh.sort_by(|a, b| a.pair.contract_address.cmp(&b.pair.contract_address));
    fresh
}
fn fresh_config_expiries(admission: &Admission, at: i64) -> HashMap<String, i64> {
    fresh_observations(admission, at)
        .into_iter()
        .map(|obs| (obs.pair.contract_address.clone(), obs.expiry_ms))
        .collect()
}
async fn publish_admission(
    state: &AppState,
    namespace: &str,
    admission: &Admission,
    assets: &[assets::AssetRow],
    addresses: &HashMap<i32, String>,
    actors: &HashMap<String, (u16, bool)>,
) -> Result<(), sqlx::Error> {
    let fresh = fresh_observations(admission, now_ms());
    let mut pairs = Vec::new();
    let mut mirrors = HashMap::new();
    let mut hooks = HashMap::new();
    let mut discounted = HashSet::new();
    let mut taxes = HashMap::new();
    let mut identity = Vec::new();
    let mut tax_observations = Vec::new();
    let expiry = fresh
        .iter()
        .map(|p| p.expiry_ms)
        .max()
        .unwrap_or_else(|| now_ms() + CONFIG_TTL_MS);
    for obs in fresh {
        pairs.push(obs.pair.clone());
        hooks.insert(obs.pair.contract_address.clone(), obs.hooks.clone());
        if obs.discounted {
            discounted.insert(obs.pair.contract_address.clone());
        }
        tax_observations.extend(obs.taxes.iter());
        identity.push(obs.identity.clone());
        if let Ok(m) = sim::load_local_mirror(
            &state.pool,
            &obs.pair,
            addresses,
            state.book_snapshot_max_staleness_ms,
        )
        .await
        {
            mirrors.insert(obs.pair.contract_address.clone(), m);
        }
    }
    // Merge in chronological order: the shared oldest timestamp is conservative,
    // while the newest actor observation must win independent of HashMap order.
    tax_observations.sort_by_key(|(token, obs)| (obs.observed_at_ms, *token));
    for (token, observation) in tax_observations {
        taxes
            .entry(token.clone())
            .and_modify(|current: &mut tax::TokenObservation| current.merge(observation.clone()))
            .or_insert_with(|| observation.clone());
    }
    pairs.sort_by(|a, b| a.contract_address.cmp(&b.contract_address));
    identity.sort();
    publish(
        namespace.to_string(),
        pairs,
        addresses.clone(),
        mirrors,
        state.book_snapshot_max_staleness_ms,
        hooks,
        expiry,
        fresh_config_expiries(admission, now_ms()),
        local_reference_quote::ReferencePrices::observe(&state.pool, assets, &state.hub_usd)
            .await?,
        actors.clone(),
        discounted,
        format!(
            "{:?}|{:?}|{identity:?}",
            state.factory_address, state.router_address
        ),
        taxes,
        None,
    );
    Ok(())
}
/// Test setup drains the capped inventory; production never calls this helper.
#[doc(hidden)]
pub async fn ingest_fixture(state: &AppState) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT LEAST(COUNT(*),512) FROM pairs")
        .fetch_one(&state.pool)
        .await?;
    for _ in 0..(count as usize).div_ceil(PAIRS_PER_PASS).max(1) {
        ingest_once(state).await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn required_asset_failure_removes_unselected_shared_pairs_and_preserves_other_corridors() {
        let snapshot = super::super::tests::fixture();
        let mut admission = Admission::default();
        for pair in &snapshot.pairs {
            admission.pairs.insert(
                pair.contract_address.clone(),
                PairObservation {
                    pair: pair.clone(),
                    asset_addresses: (
                        snapshot.addresses[&pair.asset_0_id].clone(),
                        snapshot.addresses[&pair.asset_1_id].clone(),
                    ),
                    expiry_ms: now_ms() + CONFIG_TTL_MS,
                    hooks: local_hook_pricing::CachedHookPrices::from_observation(&[]).unwrap(),
                    discounted: false,
                    identity: "fixture".into(),
                    taxes: HashMap::new(),
                },
            );
        }
        admission.pairs.remove("pair7a"); // Selected pair was removed before its asset read.
        assert!(admission.pairs.contains_key("pair7b")); // Unselected shared-token pair.
        invalidate_asset_observations(&mut admission, "middle76");
        assert!(!admission.pairs.contains_key("pair7b"));
        assert_eq!(admission.pairs.len(), 14);
        assert!(admission.pairs.contains_key("pair0a"));
        let namespace = "required-asset-failure";
        let mut g = Store::default();
        let reviewed = protocol_guards::derive(&g, namespace, &snapshot);
        g.snapshots.insert(namespace.into(), Arc::new(snapshot));
        g.results.insert(
            "cached-unsupported-approval".into(),
            json!({"approved":true}),
        );
        invalidate_cached_asset(&mut g, namespace, "middle76");
        assert!(g.results.is_empty());
        assert!(!reviewed_snapshot_current(&g, namespace, &reviewed, None));
        let current = &g.snapshots[namespace];
        assert_eq!(current.pairs.len(), 14);
        assert!(!current.mirrors.contains_key("pair7b"));
        let unrelated = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        assert!(score(current, &unrelated, 10u128.pow(16), SCALE / 20).is_some());
    }
    #[test]
    fn wallet_ledger_registry_retains_inactive_negatives_and_refuses_capacity_eviction() {
        let empty = tax::TokenObservation {
            policy_identity: "policy".into(),
            code_id: 1,
            buy_bps: 0,
            sell_bps: 0,
            observed_at_ms: now_ms(),
            max_wallet: None,
            pairs: HashSet::from(["pair".into()]),
            actors: Default::default(),
        };
        let mut negative = empty.clone();
        negative.actors.insert(
            "protocol".into(),
            tax::ActorObservation {
                observed_at_ms: now_ms(),
                protocol: true,
                manager: false,
                balance: None,
            },
        );
        let mut quarantine = empty.clone();
        quarantine.observed_at_ms = 0;
        let live = "live".to_owned();
        let mut ledgers = HashMap::from([
            (live.clone(), empty.clone()),
            ("removed-positive".into(), empty.clone()),
            ("removed-negative".into(), negative),
            ("removed-quarantine".into(), quarantine),
        ]);
        retain_wallet_ledgers(&mut ledgers, &HashSet::from([&live]));
        assert_eq!(ledgers.len(), 3);
        assert!(!ledgers.contains_key("removed-positive"));
        assert!(ledgers.contains_key("removed-negative"));
        assert_eq!(ledgers["removed-quarantine"].observed_at_ms, 0);
        let mut full = (0..MAX_TAX_WALLET_LEDGERS)
            .map(|i| (format!("token-{i}"), empty.clone()))
            .collect();
        assert!(!record_wallet_ledger(
            &mut full,
            "new-token",
            &mut empty.clone()
        ));
        assert_eq!(full.len(), MAX_TAX_WALLET_LEDGERS);
        assert!(!full.contains_key("new-token"));
        assert!(record_wallet_ledger(
            &mut full,
            "token-0",
            &mut empty.clone()
        ));
        assert!(full["token-0"].pairs.is_empty());
    }
    #[test]
    fn configuration_slices_cover_large_inventory_with_constant_pass_work() {
        let mut seen = HashSet::new();
        let mut cursor = 0;
        for _ in 0..64 {
            let (indices, next) = selected_indices(512, cursor, PAIRS_PER_PASS);
            assert_eq!(indices.len(), 8);
            seen.extend(indices);
            cursor = next;
        }
        assert_eq!(seen.len(), 512);
        assert_eq!(cursor, 0);
        assert_eq!(selected_indices(0, 999, 8), (Vec::new(), 0));
        assert_eq!(selected_indices(3, 4, 8), (vec![1, 2, 0], 1));
    }
    #[test]
    fn partial_publication_retains_original_expiry_and_excludes_expired_pairs() {
        let snapshot = super::super::tests::fixture();
        let pair = snapshot.pairs[0].clone();
        let observation = PairObservation {
            pair: pair.clone(),
            asset_addresses: ("in18".into(), "out18".into()),
            expiry_ms: 180_000,
            hooks: local_hook_pricing::CachedHookPrices::from_observation(&[]).unwrap(),
            discounted: false,
            identity: "fixture".into(),
            taxes: HashMap::new(),
        };
        let mut admission = Admission::default();
        admission
            .pairs
            .insert(pair.contract_address.clone(), observation);
        assert_eq!(fresh_observations(&admission, 179_999).len(), 1);
        assert_eq!(
            fresh_observations(&admission, 179_999)[0].expiry_ms,
            180_000
        );
        assert!(fresh_observations(&admission, 180_000).is_empty());
        assert_eq!(admission.pairs[&pair.contract_address].expiry_ms, 180_000);
    }
    #[test]
    fn expired_unrelated_pair_does_not_disable_fresh_route_and_route_floor_pins_its_expiry() {
        let mut snapshot = super::super::tests::fixture();
        let now = now_ms();
        snapshot.config_expiry_ms = now + 180_000;
        snapshot.config_expiries.insert("pair0a".into(), now - 1);
        snapshot.config_expiries.insert("pair7a".into(), now + 1000);
        snapshot.config_expiries.insert("pair7b".into(), now + 2000);
        let paths = super::super::paths(&snapshot, 1, 2, 2);
        let candidate =
            super::super::select(&snapshot, &paths, 10u128.pow(18), SCALE / 20, true, 8).unwrap();
        assert_eq!(candidate.path[0].0, "pair7a");
        let body = super::super::response(
            &snapshot,
            &super::super::tests::request(10u128.pow(18)),
            &candidate,
            10u128.pow(18),
            SCALE / 20,
            8,
        );
        assert_eq!(
            body["execution_metadata"]["expires_at_ms"].as_i64(),
            Some(now + 1000)
        );
        snapshot.config_expiries.insert("pair7a".into(), now - 1);
        assert!(
            super::super::score(&snapshot, &candidate.path, 10u128.pow(18), SCALE / 20).is_none()
        );
    }
}
