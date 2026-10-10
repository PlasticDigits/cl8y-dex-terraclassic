//! Incremental scheduled configuration admission. Each completed observation can
//! initialize quotes without waiting for a complete factory census.
use super::ingestion::{
    actor_context, actor_refresh_addresses, configured_wallets, holder_page, insert_actor, query,
    retained_blocks, ENROLLED_ACTORS_PER_PASS,
};
use super::pair_admission::{observe_pair, AdmissionFailure};
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
    asset_code_ids: (u64, u64),
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
    wallet_actor_cursors: HashMap<String, usize>,
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
fn asset_code_changed(admission: &Admission, asset: &str, code: u64) -> bool {
    admission.pairs.values().any(|pair| {
        (pair.asset_addresses.0 == asset && pair.asset_code_ids.0 != code)
            || (pair.asset_addresses.1 == asset && pair.asset_code_ids.1 != code)
    })
}
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
fn invalidate_pair_observation(admission: &mut Admission, namespace: &str, pair: &str) {
    admission.pairs.remove(pair);
    let mut g = store().lock().unwrap();
    let Some(previous) = g.snapshots.get(namespace).cloned() else {
        return;
    };
    if !previous.pairs.iter().any(|p| p.contract_address == pair) {
        return;
    }
    let mut snapshot = (*previous).clone();
    snapshot.pairs.retain(|p| p.contract_address != pair);
    snapshot.mirrors.remove(pair);
    snapshot.hooks.remove(pair);
    snapshot.config_expiries.remove(pair);
    snapshot.discounted_pairs.remove(pair);
    g.next_config_epoch = g.next_config_epoch.saturating_add(1);
    snapshot.config_epoch = g.next_config_epoch;
    snapshot.generation = format!(
        "{:x}",
        Sha256::digest(format!(
            "{}|pair-invalid|{}",
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
    diagnostics::inventory(&namespace, &all_pairs);
    let policy = community_tax_rank::RouterHopTaxPolicy::from_catalog(&state.community_tax);
    let policy_identity = format!(
        "{:?}|{:?}|{policy:?}",
        state.factory_address, state.router_address
    );
    if admission.policy_identity != policy_identity {
        admission.pairs.clear();
        admission.holder_cursors.clear();
        admission.wallet_actor_cursors.clear();
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
    admission
        .wallet_actor_cursors
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
    let actors_started = std::time::Instant::now();
    let actor_work = async {
        if let Some(factory) = state.factory_address.as_deref() {
            for address in &actor_addresses {
                refresh_actor_context(
                    &namespace, address, &mut actors, &mut actor_observed_at,
                    previous.get(address).copied(),
                    async { query(state, factory, json!({"blacklist_check":{"wallet":address,"tokens":[],"pairs":[],"pair":null}})).await.and_then(|v| v["wallet_blacklisted"].as_bool()) },
                    async {
                        if let Some(registry) = state.fee_discount_address.as_deref() {
                            query(state, registry, json!({"get_discount":{"trader":address,"sender":address}})).await.and_then(|v| v["discount_bps"].as_u64()).filter(|d| *d<=10_000).map(|d| d as u16)
                        } else { Some(0) }
                    },
                ).await?;
            }
        }
        Ok::<_, sqlx::Error>(())
    };
    let completed = actor_slice(actor_work, Duration::from_secs(10)).await?;
    tracing::debug!(
        phase = "actors",
        completed,
        elapsed_ms = actors_started.elapsed().as_millis() as u64,
        observed = actor_observed_at.len(),
        "Local quote admission stage"
    );
    // Publish newly observed wallet guards before slow token/pair discovery;
    // cancellation of that work must not leave a known block in a warm cache.
    publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await?;
    let mut tokens: HashMap<String, (u64, i64, Option<tax::TokenObservation>)> = HashMap::new();
    let mut attempted_tokens = HashSet::new();
    let mut pending_wallet_tokens = HashSet::new();
    for i in selected.iter().copied() {
        let pair = &all_pairs[i];
        // A transient refresh failure does not erase still-fresh qualification.
        // Original observation times remain unchanged; successful negative reads
        // invalidate synchronously, before the next provider/database await.
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&pair.contract_address) {
            diagnostics::record(
                &namespace,
                &pair.contract_address,
                Some(AdmissionFailure::Protected),
            );
            invalidate_pair_observation(&mut admission, &namespace, &pair.contract_address);
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
            diagnostics::record(
                &namespace,
                &pair.contract_address,
                Some(AdmissionFailure::Unsupported),
            );
            invalidate_pair_observation(&mut admission, &namespace, &pair.contract_address);
            continue;
        }
        for address in [a, b] {
            if !attempted_tokens.insert(address.clone()) {
                continue;
            }
            let observed = now_ms();
            let previous_code = admission.pairs.values().find_map(|pair| {
                if pair.asset_addresses.0 == *address {
                    Some(pair.asset_code_ids.0)
                } else if pair.asset_addresses.1 == *address {
                    Some(pair.asset_code_ids.1)
                } else {
                    None
                }
            });
            let token_started = std::time::Instant::now();
            let verdict =
                trading_tax_discovery::discover_scheduled(state, address, &policy, previous_code)
                    .await;
            let outcome = match &verdict {
                Ok(trading_tax_discovery::TradingTax::Ordinary { .. }) => "ordinary",
                Ok(trading_tax_discovery::TradingTax::Community(_)) => "community",
                Ok(trading_tax_discovery::TradingTax::Unsupported) => "unsupported",
                Ok(trading_tax_discovery::TradingTax::Unavailable) => "unavailable",
                Err(error) if crate::indexer::block_indexer::is_retryable_availability(error) => {
                    "provider_unavailable"
                }
                Err(crate::lcd::LcdError::PairResponseMalformed(_)) => "malformed_or_incomplete",
                Err(crate::lcd::LcdError::ChainIdentity(_)) => "chain_identity",
                Err(_) => "required_semantics_rejected",
            };
            tracing::debug!(
                phase = "token_discovery",
                outcome,
                elapsed_ms = token_started.elapsed().as_millis() as u64,
                admitted_pairs = admission.pairs.len(),
                "Local quote admission stage"
            );
            let mut unavailable = verdict.as_ref().err().is_some_and(|error| {
                crate::indexer::block_indexer::is_retryable_availability(error)
            });
            let observation = match verdict
                .unwrap_or(trading_tax_discovery::TradingTax::Unavailable)
            {
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
                    let previous_policy = admission
                        .pairs
                        .values()
                        .filter(|pair| pair.expiry_ms > now_ms())
                        .find_map(|pair| pair.taxes.get(address))
                        .cloned();
                    let required = bounded_pair_observation(
                        tax::observe(state, &token, &related, previous_policy.as_ref()),
                        Duration::from_secs(8),
                    )
                    .await;
                    match required {
                        Ok(mut obs) => {
                            if !record_wallet_ledger(&mut admission.tax_wallets, address, &mut obs)
                            {
                                mark_tax_wallet_capacity_exhausted(
                                    &mut store().lock().unwrap(),
                                    &namespace,
                                );
                                return Err(sqlx::Error::Protocol(
                                    "Local tax wallet ledger capacity exhausted".into(),
                                ));
                            }
                            pending_wallet_tokens.insert(address.clone());
                            Some((obs.code_id, observed + CONFIG_TTL_MS, Some(obs)))
                        }
                        Err(reason) => {
                            unavailable = reason == AdmissionFailure::Missing;
                            tracing::debug!(
                                phase = "token_policy",
                                outcome = reason.label(),
                                "Local quote admission stage"
                            );
                            None
                        }
                    }
                }
                _ => None,
            };
            if let Some(obs) = observation {
                if asset_code_changed(&admission, address, obs.0) {
                    invalidate_asset_observations(&mut admission, address);
                    invalidate_cached_asset(&mut store().lock().unwrap(), &namespace, address);
                }
                tokens.insert(address.clone(), obs);
            } else if !unavailable {
                for related in all_pairs.iter().filter(|p| {
                    addresses.get(&p.asset_0_id) == Some(address)
                        || addresses.get(&p.asset_1_id) == Some(address)
                }) {
                    diagnostics::record(
                        &namespace,
                        &related.contract_address,
                        Some(AdmissionFailure::Unsupported),
                    );
                }
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
            diagnostics::record(
                &namespace,
                &pair.contract_address,
                Some(AdmissionFailure::Missing),
            );
            continue;
        };
        if [tax_a, tax_b]
            .into_iter()
            .flatten()
            .any(|tax| !tax.pairs.contains(&pair.contract_address))
        {
            // A sibling's successful preview cannot renew this pair's missing
            // required registration. Retain only any original fresh approval.
            diagnostics::record(
                &namespace,
                &pair.contract_address,
                Some(AdmissionFailure::Missing),
            );
            continue;
        }
        let Some(factory) = state.factory_address.as_deref() else {
            continue;
        };
        let started = now_ms();
        let checked = bounded_pair_observation(
            observe_pair(state, pair, a, b, *code_a, *code_b, factory),
            Duration::from_secs(6),
        )
        .await;
        diagnostics::record(
            &namespace,
            &pair.contract_address,
            checked.as_ref().err().copied(),
        );
        if let Err(reason) = checked.as_ref() {
            tracing::debug!(
                reason = reason.label(),
                "Local quote pair admission deferred"
            );
            apply_pair_failure(&mut admission, &namespace, &pair.contract_address, *reason);
        }
        if let Ok((hooks, discounted, identity)) = checked {
            let taxes = [(a, tax_a), (b, tax_b)]
                .into_iter()
                .filter_map(|(a, t)| t.as_ref().map(|t| (a.clone(), t.clone())))
                .collect();
            admission.pairs.insert(
                pair.contract_address.clone(),
                PairObservation {
                    pair: pair.clone(),
                    asset_addresses: (a.clone(), b.clone()),
                    asset_code_ids: (*code_a, *code_b),
                    expiry_ms: (started + CONFIG_TTL_MS).min(*expiry_a).min(*expiry_b),
                    hooks,
                    discounted,
                    identity,
                    taxes,
                },
            );
        }
        // Required static policy is useful before optional wallet work begins.
        publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await?;
        for address in [a, b] {
            if !pending_wallet_tokens.remove(address) {
                continue;
            }
            let Some((_, _, Some(static_observation))) = tokens.get(address).cloned() else {
                continue;
            };
            let retained = enrich_wallets(
                state,
                &namespace,
                &mut admission,
                address,
                &static_observation,
                &actor_addresses,
                &mut actor_observed_at,
                &mut actors,
                &previous,
            )
            .await?;
            if !retained {
                tokens.remove(address);
                // Later pairs in this same slice cannot restore a policy whose
                // authoritative enrichment recheck already invalidated it.
                continue;
            }
            if let Some(current) = admission.tax_wallets.get(address) {
                if let Some((_, _, Some(observation))) = tokens.get_mut(address) {
                    observation.inherit_wallet_contexts(current);
                }
            }
            publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await?;
        }
    }
    publish_admission(state, &namespace, &admission, &assets, &addresses, &actors).await
}
/// Revoke only an existing positive manager approval on completed negative flags.
/// This runs before the actor's balance await, so cancellation cannot restore it.
fn revoke_manager_context(
    admission: &mut Admission,
    namespace: &str,
    token: &str,
    actor: &str,
    protocol: bool,
    manager: bool,
) -> bool {
    if manager && !protocol {
        return false;
    }
    let was_manager = admission
        .tax_wallets
        .get(token)
        .and_then(|observation| observation.actors.get(actor))
        .is_some_and(|observation| observation.manager && !observation.protocol)
        || admission.pairs.values().any(|pair| {
            pair.taxes
                .get(token)
                .and_then(|observation| observation.actors.get(actor))
                .is_some_and(|observation| observation.manager && !observation.protocol)
        });
    if !was_manager {
        return false;
    }
    if let Some(observation) = admission.tax_wallets.get_mut(token) {
        observation.actors.remove(actor);
    }
    for pair in admission.pairs.values_mut() {
        if let Some(observation) = pair.taxes.get_mut(token) {
            observation.actors.remove(actor);
        }
    }
    invalidate_asset_observations(admission, token);
    invalidate_cached_asset(&mut store().lock().unwrap(), namespace, token);
    true
}

async fn enrich_wallets(
    state: &AppState,
    namespace: &str,
    admission: &mut Admission,
    address: &str,
    static_observation: &tax::TokenObservation,
    actor_addresses: &[String],
    actor_observed_at: &mut HashMap<String, i64>,
    actors: &mut HashMap<String, (u16, bool)>,
    previous: &HashMap<String, (u16, bool)>,
) -> Result<bool, sqlx::Error> {
    let started = std::time::Instant::now();
    let mut enriched = static_observation.clone();
    let Some(sinks) = tax::configured_sinks(&enriched) else {
        invalidate_asset_observations(admission, address);
        invalidate_cached_asset(&mut store().lock().unwrap(), namespace, address);
        return Ok(false);
    };
    let mut tax_actors = actor_addresses
        .iter()
        .filter(|actor| actors.get(*actor).is_some_and(|(_, blocked)| !blocked))
        .filter_map(|actor| actor_observed_at.get(actor).map(|at| (actor.clone(), *at)))
        .collect::<Vec<_>>();
    let cursor = admission
        .wallet_actor_cursors
        .get(address)
        .copied()
        .unwrap_or(0);
    // Reserve four of the eight bounded observations for uncensused holders.
    let (chosen, next) = selected_indices(tax_actors.len(), cursor, 4);
    admission
        .wallet_actor_cursors
        .insert(address.to_owned(), next);
    tax_actors = chosen
        .into_iter()
        .map(|index| tax_actors[index].clone())
        .collect();
    let mut revoked = false;
    let work = async {
        let cursor = admission
            .holder_cursors
            .get(address)
            .and_then(|v| v.as_deref());
        if let Some((holders, next)) = holder_page(state, address, cursor).await {
            admission.holder_cursors.insert(address.to_owned(), next);
            if let Some(factory) = state.factory_address.as_deref() {
                for actor in holders {
                    refresh_actor_context(namespace,&actor,actors,actor_observed_at,previous.get(&actor).copied(),
                        async {query(state,factory,json!({"blacklist_check":{"wallet":actor,"tokens":[],"pairs":[],"pair":null}})).await.and_then(|v|v["wallet_blacklisted"].as_bool())},
                        async {Some(0)}).await?;
                    if actors.get(&actor).is_some_and(|(_, blocked)| !blocked) {
                        if let Some(at) = actor_observed_at.get(&actor) {
                            if let Some((_, old)) =
                                tax_actors.iter_mut().find(|(wallet, _)| wallet == &actor)
                            {
                                *old = (*old).min(*at);
                            } else {
                                tax_actors.push((actor, *at));
                            }
                        }
                    }
                }
            }
        }
        for (actor, factory_at) in tax_actors.iter().take(8) {
            let observed = tax::observe_actor(state, address, actor, |protocol, manager| {
                if revoke_manager_context(admission, namespace, address, actor, protocol, manager) {
                    enriched.actors.remove(actor);
                    revoked = true;
                }
            })
            .await;
            if let Some(mut observed) = observed {
                observed.observed_at_ms = observed.observed_at_ms.min(*factory_at);
                if sinks.contains(actor) {
                    observed.protocol = true;
                }
                enriched.actors.insert(actor.clone(), observed);
            }
        }
        Ok(())
    };
    let completed = actor_slice(work, Duration::from_secs(10)).await?;
    if revoked {
        // The current pass's static clone still carries the old qualification.
        // Its caller must discard it; a fresh scheduled pass may recover anonymously.
        tracing::debug!(
            phase = "token_wallets",
            completed,
            elapsed_ms = started.elapsed().as_millis() as u64,
            outcome = "exemption_revoked",
            "Local quote admission stage"
        );
        return Ok(false);
    }
    let checked = bounded_pair_observation(
        tax::validate_static(state, address, static_observation),
        Duration::from_secs(6),
    )
    .await;
    tracing::debug!(
        phase = "token_wallets",
        completed,
        elapsed_ms = started.elapsed().as_millis() as u64,
        outcome = checked
            .as_ref()
            .map_or_else(|reason| reason.label(), |()| "supported"),
        "Local quote admission stage"
    );
    match checked {
        Ok(()) => {
            if !record_wallet_ledger(&mut admission.tax_wallets, address, &mut enriched) {
                mark_tax_wallet_capacity_exhausted(&mut store().lock().unwrap(), namespace);
                return Err(sqlx::Error::Protocol(
                    "Local tax wallet ledger capacity exhausted".into(),
                ));
            }
            // Wallet timestamps enrich compatible policies without renewing any static time.
            for pair in admission.pairs.values_mut() {
                if let Some(observation) = pair.taxes.get_mut(address) {
                    observation.inherit_wallet_contexts(&enriched);
                }
            }
        }
        Err(AdmissionFailure::Missing) => {} // New positive wallet approvals remain unpublished.
        Err(_) => {
            invalidate_asset_observations(admission, address);
            invalidate_cached_asset(&mut store().lock().unwrap(), namespace, address);
            return Ok(false);
        }
    }
    Ok(true)
}
async fn actor_slice(
    work: impl std::future::Future<Output = Result<(), sqlx::Error>>,
    budget: Duration,
) -> Result<bool, sqlx::Error> {
    match tokio::time::timeout(budget, work).await {
        Ok(result) => result.map(|()| true),
        Err(_) => Ok(false),
    }
}
async fn refresh_actor_context(
    namespace: &str,
    address: &str,
    actors: &mut HashMap<String, (u16, bool)>,
    observed: &mut HashMap<String, i64>,
    previous: Option<(u16, bool)>,
    blacklist: impl std::future::Future<Output = Option<bool>>,
    discount: impl std::future::Future<Output = Option<u16>>,
) -> Result<(), sqlx::Error> {
    let observed_at = now_ms();
    let blocked = blacklist.await;
    if blocked == Some(true) {
        ingestion::publish_known_block(namespace, address)?;
    } else if blocked == Some(false) {
        ingestion::publish_known_unblock(namespace, address)?;
        // Preserve this completed authoritative guard before optional fees await.
        observed.insert(address.to_owned(), observed_at);
        insert_actor(actors, address.to_owned(), (0, false))?;
    }
    let fee = if blocked == Some(false) {
        discount.await
    } else {
        None
    };
    if let Some(context) = actor_context(blocked, fee, previous) {
        if let Err(error) = insert_actor(actors, address.to_owned(), context) {
            mark_actor_capacity_exhausted(&mut store().lock().unwrap(), namespace);
            return Err(error);
        }
    }
    Ok(())
}
async fn bounded_pair_observation<T>(
    observation: impl std::future::Future<Output = Result<T, AdmissionFailure>>,
    budget: Duration,
) -> Result<T, AdmissionFailure> {
    // Cancellation can hide already observed malformed/integrity evidence
    // inside a later provider failover. Completed typed availability alone
    // permits retention; an incomplete observation must invalidate.
    tokio::time::timeout(budget, observation)
        .await
        .unwrap_or(Err(AdmissionFailure::Unsupported))
}
fn apply_pair_failure(
    admission: &mut Admission,
    namespace: &str,
    pair: &str,
    reason: AdmissionFailure,
) {
    if reason != AdmissionFailure::Missing {
        invalidate_pair_observation(admission, namespace, pair);
    }
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
    #[tokio::test(start_paused = true)]
    async fn recovery1464_bounded_actor_slice_preserves_guards_and_pair_progress() {
        let namespace = "actor-prelude-budget1464";
        let mut snapshot = super::super::tests::fixture();
        snapshot.actors.insert("cleared".into(), (0, true));
        let expiry = snapshot.config_expiry_ms;
        store()
            .lock()
            .unwrap()
            .snapshots
            .insert(namespace.into(), Arc::new(snapshot));
        ingestion::publish_known_block(namespace, "cleared").unwrap();
        let mut actors = HashMap::from([("cleared".into(), (0, true))]);
        let mut observed = HashMap::new();
        let started = tokio::time::Instant::now();
        let completed = actor_slice(
            async {
                refresh_actor_context(
                    namespace,
                    "blocked",
                    &mut actors,
                    &mut observed,
                    None,
                    async { Some(true) },
                    async { panic!("blocked wallet cannot query discount") },
                )
                .await?;
                refresh_actor_context(
                    namespace,
                    "cleared",
                    &mut actors,
                    &mut observed,
                    Some((0, true)),
                    async { Some(false) },
                    async {
                        tokio::time::sleep(Duration::from_secs(64)).await;
                        Some(500)
                    },
                )
                .await?;
                Ok(())
            },
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert!(!completed);
        assert_eq!(started.elapsed(), Duration::from_secs(10));
        assert_eq!(actors["blocked"], (0, true));
        assert_eq!(actors["cleared"], (0, false));
        assert!(observed.contains_key("cleared"));
        let g = store().lock().unwrap();
        let current = &g.snapshots[namespace];
        assert_eq!(current.actors["blocked"], (0, true));
        assert_eq!(current.actors["cleared"], (0, false));
        assert!(g.early_actor_blocks[namespace].contains("blocked"));
        assert!(!g.early_actor_blocks[namespace].contains("cleared"));
        assert_eq!(
            current.config_expiry_ms, expiry,
            "actor observations cannot renew configuration"
        );
        assert!(
            score(
                current,
                &[("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)],
                10u128.pow(16),
                SCALE / 20
            )
            .is_some(),
            "slow optional fees must leave time for useful pair quotes"
        );
    }
    #[tokio::test(start_paused = true)]
    async fn recovery1464_incomplete_pair_observation_invalidates_warm_qualification() {
        let snapshot = super::super::tests::fixture();
        let pair = snapshot.pairs[0].clone();
        let namespace = "incomplete-pair-observation1464";
        let original_generation = snapshot.generation.clone();
        let mut admission = Admission::default();
        admission.pairs.insert(
            pair.contract_address.clone(),
            PairObservation {
                pair: pair.clone(),
                asset_addresses: (
                    snapshot.addresses[&pair.asset_0_id].clone(),
                    snapshot.addresses[&pair.asset_1_id].clone(),
                ),
                asset_code_ids: (1, 1),
                expiry_ms: now_ms() + CONFIG_TTL_MS,
                hooks: local_hook_pricing::CachedHookPrices::from_observation(&[]).unwrap(),
                discounted: false,
                identity: "fixture".into(),
                taxes: HashMap::new(),
            },
        );
        store()
            .lock()
            .unwrap()
            .snapshots
            .insert(namespace.into(), Arc::new(snapshot));
        let failure = bounded_pair_observation(
            async {
                tokio::time::sleep(Duration::from_secs(7)).await;
                Ok::<(), AdmissionFailure>(())
            },
            Duration::from_secs(6),
        )
        .await
        .unwrap_err();
        assert_eq!(failure, AdmissionFailure::Unsupported);
        apply_pair_failure(&mut admission, namespace, &pair.contract_address, failure);
        assert!(admission.pairs.is_empty());
        let g = store().lock().unwrap();
        let current = &g.snapshots[namespace];
        assert!(!current
            .pairs
            .iter()
            .any(|p| p.contract_address == pair.contract_address));
        assert!(!current.mirrors.contains_key(&pair.contract_address));
        assert_ne!(current.generation, original_generation);
        assert!(
            current.pairs.iter().any(|p| p.contract_address == "pair7a"),
            "unrelated fresh corridors remain available"
        );
        drop(g);
        assert_eq!(
            bounded_pair_observation(
                async { Err::<(), _>(AdmissionFailure::Missing) },
                Duration::from_secs(6)
            )
            .await,
            Err(AdmissionFailure::Missing)
        );
    }
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
                    asset_code_ids: (1, 1),
                    expiry_ms: now_ms() + CONFIG_TTL_MS,
                    hooks: local_hook_pricing::CachedHookPrices::from_observation(&[]).unwrap(),
                    discounted: false,
                    identity: "fixture".into(),
                    taxes: HashMap::new(),
                },
            );
        }
        assert!(!asset_code_changed(&admission, "middle76", 1));
        assert!(asset_code_changed(&admission, "middle76", 2));
        assert!(!asset_code_changed(&admission, "unknown", 2));
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
            asset_code_ids: (1, 1),
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
