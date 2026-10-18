//! Immutable, bounded local execution quotes (#1452). Network handles are confined to
//! scheduled ingestion; solve, references, refresh and deeper search only see this snapshot.
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use axum::{http::StatusCode, Json};
use bigdecimal::num_bigint::BigInt;
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::prerequisite_trace::MissingStage;
use super::{db_orderbook_sim as sim, route_paths, route_solver, AppState};
use crate::db::queries::{assets, pairs};

const MAX_PAIRS: usize = 512;
const MAX_RESULTS: usize = 256;
const MAX_JOBS: usize = 4;
/// Reviewed local quote signing window. This is deliberately long so that
/// disabled users and assistive-technology users are never forced to race a
/// wallet popup. On-chain floors (`minimum_receive` / `max_spread`) remain the
/// economic protection if prices or fees move during the window.
///
/// Accessibility policy: 30 minutes, not 20 seconds.
pub(crate) const LOCAL_QUOTE_SIGNING_WINDOW_MS: i64 = 30 * 60 * 1000;
/// Producer-side observation validity for cached pair/config/tax rows. The
/// scheduled poller refreshes these well inside this window; it is not the
/// user's signing deadline.
const CONFIG_TTL_MS: i64 = LOCAL_QUOTE_SIGNING_WINDOW_MS;
const MAX_WORK: u64 = 16_384;
const SCALE: u128 = 1_000_000_000_000_000_000;
type Error = (StatusCode, String);

#[derive(Clone)]
struct Snapshot {
    namespace: String,
    generation: String,
    pairs: Vec<pairs::PairRow>,
    addresses: HashMap<i32, String>,
    ids: HashMap<String, i32>,
    mirrors: HashMap<String, Arc<sim::HopMirror>>,
    // Independently complete pool tuples; never relabel or splice an old book.
    pool_mirrors: HashMap<String, Arc<sim::HopMirror>>,
    // Prehashed immutable views keep scheduled publication off the quote lock.
    view_hashes: Arc<generation::ViewHashes>,
    pair_contexts: HashMap<String, String>,
    max_age_ms: u64,
    hooks: HashMap<String, super::local_hook_pricing::CachedHookPrices>,
    config_expiry_ms: i64,
    config_expiries: HashMap<String, i64>,
    config_epoch: u64,
    // Monotone invalidation for shortened validity windows; ordinary refreshes
    // retain it, so a later identical refresh cannot resurrect an old cache key.
    validity_revision: u64,
    actor_revision: u64,
    references: super::local_reference_quote::ReferencePrices,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
    policy_identity: String,
    quote_relative_protection: bool,
    taxes: HashMap<String, tax::TokenObservation>,
    tax_trader: Option<String>,
    protocol_actor_tokens: HashMap<String, HashSet<String>>,
}
#[derive(Default)]
struct Store {
    skeletons: skeletons::SkeletonCache,
    snapshots: HashMap<String, Arc<Snapshot>>,
    admission_inventory: HashMap<String, diagnostics::Inventory>,
    wrappers: HashMap<String, super::local_wrapper_quote::WrapperSnapshot>,
    // Jointly bounded with positive views; known negatives survive unavailable reads.
    wrapper_revocations: HashMap<String, super::local_wrapper_quote::WrapperRevocation>,
    results: HashMap<String, Value>,
    jobs: HashSet<String>,
    deferred_jobs: std::collections::VecDeque<jobs::Work>,
    hits: u64,
    started: u64,
    next_config_epoch: u64,
    next_actor_version: u64,
    actor_versions: HashMap<String, actor_versions::Namespace>,
    actor_refresh_cursors: HashMap<String, usize>,
    actor_capacity_exhausted: HashSet<String>,
    actor_capacity_registry_exhausted: bool,
    early_actor_blocks: HashMap<String, HashSet<String>>,
    tax_wallet_capacity_exhausted: HashSet<String>,
    pending_protocol_guards: protocol_guards::Pending,
}
fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}
fn actor_capacity_unavailable(g: &Store, namespace: &str) -> bool {
    g.actor_capacity_registry_exhausted
        || g.actor_capacity_exhausted.contains(namespace)
        || g.tax_wallet_capacity_exhausted.contains(namespace)
}
fn mark_actor_capacity_exhausted(g: &mut Store, namespace: &str) {
    if g.actor_capacity_exhausted.len() < 32 || g.actor_capacity_exhausted.contains(namespace) {
        g.actor_capacity_exhausted.insert(namespace.to_owned());
    } else {
        g.actor_capacity_registry_exhausted = true;
    }
}
fn mark_tax_wallet_capacity_exhausted(g: &mut Store, namespace: &str) {
    if g.tax_wallet_capacity_exhausted.len() < 32
        || g.tax_wallet_capacity_exhausted.contains(namespace)
    {
        g.tax_wallet_capacity_exhausted.insert(namespace.to_owned());
    } else {
        g.actor_capacity_registry_exhausted = true;
    }
}
fn unavailable() -> Error {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        json!({"error":"missing_observations"}).to_string(),
    )
}
fn invalid(_message: &str) -> Error {
    (
        StatusCode::BAD_REQUEST,
        json!({"error":"invalid_request"}).to_string(),
    )
}
fn eligible_error() -> Error {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({"error":"protection"}).to_string(),
    )
}
/// HTTP transport for the reviewed public reason contract. Internal strings are
/// never echoed; older bounded admission/parameter failures are classified here.
pub(super) fn public_error((status, body): Error) -> (StatusCode, Json<Value>) {
    let parsed = (body.len() <= 2048)
        .then(|| serde_json::from_str::<Value>(&body).ok())
        .flatten()
        .filter(|v| {
            v["error"].as_str().is_some_and(|code| {
                matches!(
                    code,
                    "protection"
                        | "price_impact_too_high"
                        | "amount_too_large"
                        | "amount_too_small"
                        | "insufficient_liquidity"
                        | "unsupported_semantics"
                        | "missing_observations"
                        | "stale_observations"
                        | "accounting_unavailable"
                        | "trading_blocked"
                        | "no_route"
                        | "invalid_request"
                        | "reference_policy_excluded"
                )
            })
        });
    let fallback = match status {
        StatusCode::BAD_REQUEST => "invalid_request",
        StatusCode::UNPROCESSABLE_ENTITY => "protection",
        _ => "missing_observations",
    };
    let mut public = json!({"error":fallback});
    if let Some(parsed) = parsed {
        public["error"] = parsed["error"].clone();
        if parsed["generation"].as_str().is_some_and(|s| {
            !s.is_empty()
                && s.len() <= 256
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b":_-".contains(&b))
        }) {
            public["generation"] = parsed["generation"].clone();
        }
        if parsed["error"] == "protection"
            && parsed["execution_supported"] == false
            && parsed["indicative_amount_out"].as_str().is_some_and(|s| {
                s.len() <= 39
                    && !s.starts_with('0')
                    && s.bytes().all(|b| b.is_ascii_digit())
                    && s.parse::<u128>().is_ok()
            })
            && parsed["observed_at_ms"]
                .as_i64()
                .is_some_and(|at| at > 0 && at <= now_ms())
            && parsed["expires_at_ms"].as_i64().is_some_and(|at| {
                at > now_ms()
                    && at <= now_ms() + LOCAL_QUOTE_SIGNING_WINDOW_MS
                    && parsed["observed_at_ms"].as_i64().is_some_and(|observed| {
                        at > observed && at.saturating_sub(observed) <= LOCAL_QUOTE_SIGNING_WINDOW_MS
                    })
            })
            && !public["generation"].is_null()
        {
            for field in [
                "indicative_amount_out",
                "execution_supported",
                "observed_at_ms",
                "expires_at_ms",
            ] {
                public[field] = parsed[field].clone();
            }
        }
    }
    if status == StatusCode::SERVICE_UNAVAILABLE {
        super::prerequisite_trace::emit();
    }
    tracing::warn!(
        status = status.as_u16(),
        reason = public["error"].as_str().unwrap_or(fallback),
        "Local quote rejected"
    );
    (status, Json(public))
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}
fn wide_floor(a: u128, b: u128, d: u128) -> Option<u128> {
    if d == 0 {
        return None;
    }
    (BigInt::from(a) * BigInt::from(b) / BigInt::from(d))
        .to_string()
        .parse()
        .ok()
}
fn spread_atomics(raw: Option<&str>) -> Result<u128, Error> {
    let raw = raw.unwrap_or("0.05");
    // Decimal uses up to 18 fractional places, never float-derived executable amounts.
    let (integer, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if integer != "0" || fraction.len() > 18 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("max_spread must be a decimal between 0 and 0.5"));
    }
    let value = format!("{fraction:0<18}")
        .parse::<u128>()
        .map_err(|_| invalid("Invalid max_spread"))?;
    if value > SCALE / 2 {
        return Err(invalid("max_spread must be at most 0.5"));
    }
    Ok(value)
}
fn key(namespace: &str, s: &Snapshot, q: &route_solver::SolveRouteParams, spread: u128) -> String {
    format!(
        "{namespace}|{}|{}|{}|{:?}|{:?}|{spread}|{:?}|{:?}|{:?}",
        s.generation,
        q.token_in.trim(),
        q.token_out.trim(),
        q.amount_in,
        q.amount_out,
        q.trader,
        q.sender,
        (
            q.pool_only,
            &q.protection_policy,
            q.max_maker_fills,
            q.hybrid_optimize,
            q.discount_bps
        )
    )
}
/// Preserve only an already qualified newer complete tuple in the same binding.
/// Raw numeric/SQL errors never call this helper with an arbitrary fallback.
fn preserve_newer_mirrors(snapshot: &mut Snapshot, current: &Snapshot) {
    for (pool_view, incoming_views, previous_views) in [
        (false, &mut snapshot.mirrors, &current.mirrors),
        (true, &mut snapshot.pool_mirrors, &current.pool_mirrors),
    ] {
        for (address, incoming) in incoming_views {
            if snapshot.pair_contexts.get(address).is_none()
                || snapshot.pair_contexts.get(address) != current.pair_contexts.get(address)
            {
                continue;
            }
            let Some(previous) = previous_views.get(address) else {
                continue;
            };
            let non_regressing_height = match (incoming.block_height, previous.block_height) {
                (Some(new), Some(old)) => new >= old,
                (_, None) => true,
                (None, Some(_)) => false,
            };
            if (incoming.snapshot_at < previous.snapshot_at || !non_regressing_height)
                && !sim::is_snapshot_stale(previous.snapshot_at, snapshot.max_age_ms)
                && crate::indexer::fee_phase::permits_view(
                    &snapshot.namespace,
                    previous.pair_id,
                    previous.block_height,
                    previous.snapshot_at,
                    pool_view,
                )
            {
                *incoming = previous.clone();
                let hashes = Arc::make_mut(&mut snapshot.view_hashes);
                let key = (address.clone(), pool_view);
                if let Some(previous_hash) = current.view_hashes.get(&key) {
                    hashes.insert(key, previous_hash.clone());
                } else {
                    hashes.remove(&key);
                }

                let expiry = current
                    .config_expiries
                    .get(address)
                    .copied()
                    .unwrap_or(current.config_expiry_ms);
                snapshot
                    .config_expiries
                    .entry(address.clone())
                    .and_modify(|at| *at = (*at).min(expiry))
                    .or_insert(expiry);
            }
        }
    }
}

fn publish(
    namespace: String,
    pairs: Vec<pairs::PairRow>,
    addresses: HashMap<i32, String>,
    mirrors: HashMap<String, Arc<sim::HopMirror>>,
    pool_mirrors: HashMap<String, Arc<sim::HopMirror>>,
    max_age_ms: u64,
    hooks: HashMap<String, super::local_hook_pricing::CachedHookPrices>,
    config_expiry_ms: i64,
    config_expiries: HashMap<String, i64>,
    pair_contexts: HashMap<String, String>,
    references: super::local_reference_quote::ReferencePrices,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
    policy_identity: String,
    taxes: HashMap<String, tax::TokenObservation>,
    expected_config_epoch: Option<u64>,
) {
    publication(
        namespace,
        pairs,
        addresses,
        mirrors,
        pool_mirrors,
        max_age_ms,
        hooks,
        config_expiry_ms,
        config_expiries,
        pair_contexts,
        references,
        actors,
        discounted_pairs,
        policy_identity,
        taxes,
        expected_config_epoch,
    )
    .prepare()
    .commit();
}

fn publication(
    namespace: String,
    pairs: Vec<pairs::PairRow>,
    addresses: HashMap<i32, String>,
    mirrors: HashMap<String, Arc<sim::HopMirror>>,
    pool_mirrors: HashMap<String, Arc<sim::HopMirror>>,
    max_age_ms: u64,
    hooks: HashMap<String, super::local_hook_pricing::CachedHookPrices>,
    config_expiry_ms: i64,
    config_expiries: HashMap<String, i64>,
    pair_contexts: HashMap<String, String>,
    references: super::local_reference_quote::ReferencePrices,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
    policy_identity: String,
    taxes: HashMap<String, tax::TokenObservation>,
    expected_config_epoch: Option<u64>,
) -> publication::Pending {
    let ids = addresses.iter().map(|(id, a)| (a.clone(), *id)).collect();
    let snapshot = Snapshot {
        namespace: namespace.clone(),
        generation: String::new(),
        pairs,
        addresses,
        ids,
        mirrors,
        pool_mirrors,
        view_hashes: Arc::default(),
        max_age_ms,
        hooks,
        config_expiry_ms,
        config_expiries,
        pair_contexts,
        config_epoch: expected_config_epoch.unwrap_or(0),
        validity_revision: 0,
        actor_revision: 0,
        references,
        actors,
        discounted_pairs,
        policy_identity,
        quote_relative_protection: true,
        taxes,
        tax_trader: None,
        protocol_actor_tokens: HashMap::new(),
    };
    let actor_proof =
        actor_versions::capture(&store().lock().unwrap(), &namespace, &snapshot.actors);
    publication::Pending {
        snapshot,
        expected_config_epoch,
        actor_proof,
        #[cfg(test)]
        start_barrier: None,
    }
}

/// Called by a scheduled producer before failure cleanup awaits. Existing CPU
/// work finishes bounded computation but cannot republish the obsolete epoch.
pub(crate) fn invalidate_fee_generation(namespace: &str, pair_id: i32) {
    let mut g = store().lock().unwrap();
    if let Some(previous) = g.snapshots.get(namespace).cloned() {
        let mut snapshot = (*previous).clone();
        snapshot
            .mirrors
            .retain(|_, mirror| mirror.pair_id != pair_id);
        snapshot
            .pool_mirrors
            .retain(|_, mirror| mirror.pair_id != pair_id);
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
        snapshot.config_epoch = g.next_config_epoch;
        snapshot.generation = format!(
            "{:x}",
            Sha256::digest(format!(
                "{}|fee-negative|{}",
                snapshot.generation, snapshot.config_epoch
            ))
        );
        g.snapshots.insert(namespace.to_owned(), Arc::new(snapshot));
    }
    g.results.clear();
}

/// Deterministic synthetic fixture admission. Production uses independently attested ingestion.
#[doc(hidden)]
pub fn initialize_for_test(
    namespace: String,
    pairs: Vec<pairs::PairRow>,
    addresses: HashMap<i32, String>,
    mirrors: HashMap<String, sim::HopMirror>,
    max_staleness_ms: u64,
) {
    publish(
        namespace,
        pairs.into_iter().filter(|p| p.hooks.is_empty()).collect(),
        addresses.clone(),
        mirrors
            .into_iter()
            .map(|(address, mirror)| (address, Arc::new(mirror)))
            .collect(),
        HashMap::new(),
        max_staleness_ms,
        HashMap::new(),
        now_ms() + CONFIG_TTL_MS,
        HashMap::new(),
        HashMap::new(),
        super::local_reference_quote::ReferencePrices::synthetic(addresses.keys().copied()),
        HashMap::new(),
        HashSet::new(),
        "synthetic".into(),
        HashMap::new(),
        None,
    );
}

/// Synthetic fixture policy only; explicit setup never models production attestation.
#[doc(hidden)]
pub fn initialize_fixture_contexts(
    namespace: &str,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
) {
    let mut registry = store().lock().unwrap();
    let mut snapshot = (**registry
        .snapshots
        .get(namespace)
        .expect("initialized fixture"))
    .clone();
    assert_eq!(snapshot.policy_identity, "synthetic");
    assert!(actors.values().all(|(bps, _)| *bps <= 10_000));
    snapshot.actors = actors;
    snapshot.discounted_pairs = discounted_pairs;
    let mut actor_fields = snapshot.actors.iter().collect::<Vec<_>>();
    actor_fields.sort_by_key(|(a, _)| *a);
    let mut discounted = snapshot.discounted_pairs.iter().collect::<Vec<_>>();
    discounted.sort();
    let mut hash = Sha256::new();
    hash.update(format!(
        "{}|{actor_fields:?}|{discounted:?}",
        snapshot.generation
    ));
    snapshot.generation = format!("{:x}", hash.finalize());
    registry
        .snapshots
        .insert(namespace.to_owned(), Arc::new(snapshot));
}

fn observation_max_age(s: &Snapshot) -> u64 {
    s.max_age_ms.min(20_000)
}
fn quote_mirror<'a>(s: &'a Snapshot, pair: &str, book: bool) -> Option<&'a Arc<sim::HopMirror>> {
    let eligible = |mirror: &&Arc<sim::HopMirror>, pool_view| {
        !sim::is_snapshot_stale(mirror.snapshot_at, observation_max_age(s))
            && matches!(
                mirror.freshness,
                sim::MirrorFreshness::Fresh | sim::MirrorFreshness::EmptyPool
            )
            && crate::indexer::fee_phase::permits_view(
                &s.namespace,
                mirror.pair_id,
                mirror.block_height,
                mirror.snapshot_at,
                pool_view,
            )
    };
    if !book {
        if let Some(pool) = s
            .pool_mirrors
            .get(pair)
            .filter(|mirror| eligible(mirror, true))
        {
            return Some(pool);
        }
    }
    s.mirrors.get(pair).filter(|mirror| eligible(mirror, false))
}

#[derive(Clone)]
struct Candidate {
    path: Vec<route_paths::PathHop>,
    output: u128,
    spot: u128,
    hops: Vec<Value>,
    operations: Vec<Value>,
    oldest: i64,
    block: Option<i64>,
    mirrors: Vec<Arc<sim::HopMirror>>,
    protection_blocked: bool,
    execution_market_spread: u128,
    economics: Vec<attribution::HopEconomics>,
}
#[cfg(test)]
fn score(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    input: u128,
    spread: u128,
) -> Option<Candidate> {
    score_with_book(s, path, input, spread, 0, 8)
}
fn score_with_book(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    input: u128,
    spread: u128,
    book_input: u128,
    maker_fills: u32,
) -> Option<Candidate> {
    score_with_plan(s, path, input, spread, book_input, maker_fills, None)
}
fn score_with_plan(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    input: u128,
    spread: u128,
    book_input: u128,
    maker_fills: u32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Option<Candidate> {
    evaluate_with_plan(s, path, input, spread, book_input, maker_fills, plan)
        .ok()
        .filter(|candidate| !candidate.protection_blocked)
}
fn evaluate_with_plan(
    s: &Snapshot,
    path: &[route_paths::PathHop],
    input: u128,
    spread: u128,
    book_input: u128,
    maker_fills: u32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Result<Candidate, diagnostics::Rejection> {
    if plan.is_some_and(|p| p.len() != path.len()) {
        return Err(diagnostics::Rejection::InvalidRequest);
    }
    if path.is_empty() {
        return Err(diagnostics::Rejection::NoRoute);
    }
    if input == 0 {
        return Err(diagnostics::Rejection::Dust);
    }
    if !protocol_guards::permits_path(s, path) {
        return Err(diagnostics::Rejection::TradingBlocked);
    }
    let mut visited = HashSet::new();
    for id in std::iter::once(path[0].1).chain(path.iter().map(|hop| hop.2)) {
        let asset = s
            .addresses
            .get(&id)
            .ok_or_else(|| diagnostics::missing(MissingStage::AssetIdentity))?;
        if s.taxes.contains_key(asset) && !visited.insert(asset) {
            return Err(diagnostics::Rejection::TradingBlocked);
        }
    }
    let mut protection_blocked = false;
    let mut execution_market_spread = spread;
    let mut output = input;
    let mut hops = Vec::new();
    let mut operations = Vec::new();
    let mut oldest = now_ms();
    let mut block = None;
    let mut used_mirrors = Vec::with_capacity(path.len());
    let mut economics = Vec::with_capacity(path.len());
    for (hop_index, (pair, offer_id, ask_id)) in path.iter().enumerate() {
        if s.config_expiries
            .get(pair)
            .copied()
            .unwrap_or(s.config_expiry_ms)
            <= now_ms()
        {
            super::prerequisite_trace::record(
                super::prerequisite_trace::Component::Config,
                None,
                Some(
                    s.config_expiries
                        .get(pair)
                        .copied()
                        .unwrap_or(s.config_expiry_ms),
                ),
            );
            return Err(diagnostics::Rejection::Stale);
        }
        let needs_book = plan.map_or(hop_index == 0 && book_input > 0, |plan| {
            plan[hop_index].as_ref().is_some_and(|hop| {
                hop.book_input
                    .parse::<u128>()
                    .is_ok_and(|amount| amount > 0)
            })
        });
        let mirror = quote_mirror(s, pair, needs_book)
            .ok_or_else(|| diagnostics::mirror_rejection(s, pair, needs_book))?;
        used_mirrors.push(mirror.clone());
        let offer_token = s
            .addresses
            .get(offer_id)
            .ok_or_else(|| diagnostics::missing(MissingStage::AssetIdentity))?;
        let ask_token = s
            .addresses
            .get(ask_id)
            .ok_or_else(|| diagnostics::missing(MissingStage::AssetIdentity))?;
        let (rin, rout) =
            if offer_token == &mirror.asset_0_addr && ask_token == &mirror.asset_1_addr {
                (mirror.reserve_0, mirror.reserve_1)
            } else if offer_token == &mirror.asset_1_addr && ask_token == &mirror.asset_0_addr {
                (mirror.reserve_1, mirror.reserve_0)
            } else {
                return Err(diagnostics::Rejection::Accounting);
            };
        if mirror.fee_bps > 10_000 {
            return Err(diagnostics::Rejection::Accounting);
        }
        let offer_tax = s.taxes.get(offer_token);
        let ask_tax = s.taxes.get(ask_token);
        let sell_bps = offer_tax
            .map_or(Some(0), |tax| {
                tax.rates(s.tax_trader.as_deref(), pair)
                    .map(|rates| rates.0)
            })
            .ok_or_else(|| {
                offer_tax.map_or(diagnostics::Rejection::Accounting, |tax| {
                    diagnostics::tax_rejection(tax, s.tax_trader.as_deref(), pair)
                })
            })?;
        let buy_bps = ask_tax
            .map_or(Some(0), |tax| {
                tax.rates(s.tax_trader.as_deref(), pair)
                    .map(|rates| rates.1)
            })
            .ok_or_else(|| {
                ask_tax.map_or(diagnostics::Rejection::Accounting, |tax| {
                    diagnostics::tax_rejection(tax, s.tax_trader.as_deref(), pair)
                })
            })?;
        let (economic_input, sell_tax, sell_residue) =
            tax::sell(output, sell_bps).ok_or(diagnostics::Rejection::Accounting)?;
        let (book_input, maker_fills) = if let Some(plan) = plan {
            // Router accepts either exact economic budgets or the authorized gross
            // ratio; rescale the latter identically to funded_hybrid.
            modes::declared_split(plan[hop_index].as_ref(), economic_input)
                .or_else(|| {
                    modes::declared_split(plan[hop_index].as_ref(), output).and_then(
                        |(book, fills)| Some((wide_floor(economic_input, book, output)?, fills)),
                    )
                })
                .ok_or(diagnostics::Rejection::InvalidRequest)?
        } else {
            (
                if hop_index == 0 {
                    wide_floor(economic_input, book_input, output)
                        .ok_or(diagnostics::Rejection::Accounting)?
                } else {
                    0
                },
                maker_fills,
            )
        };
        if book_input > 0
            && economic_input > book_input
            && economic_input - book_input < (economic_input / 10).max(1)
        {
            return Err(diagnostics::Rejection::InvalidRequest);
        }
        let leg =
            sim::quote_local_leg(mirror, offer_token, economic_input, book_input, maker_fills)
                .map_err(diagnostics::simulation_rejection)?;
        let pool_net = leg.pool_return_amount;
        let commission = leg.commission_amount;
        let post_hook = s
            .hooks
            .get(pair)
            .map_or(Some(leg.return_amount), |hooks| {
                hooks.net_return(ask_token, leg.return_amount).ok()
            })
            .ok_or(diagnostics::Rejection::Unsupported)?;
        let (net, buy_tax) =
            tax::buy(post_hook, buy_bps).ok_or(diagnostics::Rejection::Accounting)?;
        // Bought intermediate proceeds stay in the authenticated router; only
        // purchased rounding residue and final delivery credit the payer wallet.
        if (hop_index + 1 == path.len()
            && ask_tax.is_some_and(|tax| !tax.permits_credit(s.tax_trader.as_deref(), net)))
            || (hop_index > 0
                && sell_residue > 0
                && offer_tax
                    .is_some_and(|tax| !tax.permits_credit(s.tax_trader.as_deref(), sell_residue)))
        {
            let credit_tax = if hop_index + 1 == path.len()
                && ask_tax.is_some_and(|tax| !tax.permits_credit(s.tax_trader.as_deref(), net))
            {
                ask_tax
            } else {
                offer_tax
            };
            return Err(
                credit_tax.map_or(diagnostics::Rejection::TradingBlocked, |tax| {
                    diagnostics::tax_rejection(tax, s.tax_trader.as_deref(), pair)
                }),
            );
        }
        let gross = pool_net
            .checked_add(commission)
            .ok_or(diagnostics::Rejection::Accounting)?;
        let market_spread = leg.spread_amount;
        let denominator = gross
            .checked_add(leg.book_return_net)
            .ok_or(diagnostics::Rejection::Accounting)?;
        let consumed = economic_input
            .checked_sub(leg.pool_input_actual)
            .ok_or(diagnostics::Rejection::Accounting)?;
        let book_shortfall = if leg.pool_input_actual > 0 && consumed > 0 {
            wide_floor(pool_net, consumed, leg.pool_input_actual)
                .ok_or(diagnostics::Rejection::Accounting)?
                .saturating_sub(leg.book_return_net)
        } else {
            0
        };
        let spread_total = market_spread
            .min(gross)
            .checked_add(book_shortfall)
            .ok_or(diagnostics::Rejection::Accounting)?;
        // L9 market impact remains an independently checked contract metric.
        // Quote-relative protection anchors the market ceiling to this amount's
        // pinned economics; positive net floors still bind the exact UI setting.
        if s.quote_relative_protection {
            execution_market_spread = execution_market_spread.max(quote_relative_market_spread(
                spread_total,
                denominator,
                spread,
            )?);
        } else if BigInt::from(spread_total) * BigInt::from(SCALE)
            > BigInt::from(denominator) * BigInt::from(spread)
        {
            protection_blocked = true;
        }
        if book_input > 0 && economic_input > book_input && pool_net == 0 {
            return Err(diagnostics::Rejection::Dust);
        }
        let floor = net_floor(s, net, spread).ok_or(diagnostics::Rejection::Accounting)?;
        // Escrow-backed book output is independent of AMM reserves. Apply pool
        // plausibility only to the amount actually exchanged against those reserves.
        if net == 0 {
            return Err(diagnostics::Rejection::Dust);
        }
        if sim::hop_sim_implausible(leg.pool_input_actual, rin, rout, pool_net) {
            return Err(diagnostics::Rejection::Liquidity);
        }
        economics.push(attribution::HopEconomics {
            reserve_in: rin,
            reserve_out: rout,
            input: output,
            economic_input,
            pre_commission: leg
                .return_amount
                .checked_add(commission)
                .and_then(|amount| amount.checked_add(leg.book_commission_amount)),
            post_commission: leg.return_amount,
            post_hook,
            net,
        });
        hops.push(json!({"pair":pair,"offer_token":offer_token,"ask_token":ask_token,"offer_amount":output.to_string(),"economic_offer_amount":economic_input.to_string(),"sell_tax_amount":sell_tax.to_string(),"sell_residue":sell_residue.to_string(),"sell_tax_bps":sell_bps,"buy_tax_bps":buy_bps,"buy_tax_amount":buy_tax.to_string(),"post_hook_return_amount":post_hook.to_string(),"return_amount":net.to_string(),"pool_return_amount":pool_net.to_string(),"book_return_net":leg.book_return_net.to_string(),"pool_input_actual":leg.pool_input_actual.to_string(),"book_offer_consumed":consumed.to_string(),"book_input_actual":consumed.to_string(),"spread_amount":market_spread.to_string(),"commission_amount":commission.to_string(),"min_return":floor.to_string()}));
        operations.push(json!({"terra_swap":{"offer_asset_info":{"token":{"contract_addr":offer_token}},"ask_asset_info":{"token":{"contract_addr":ask_token}},"hybrid":if book_input==0 && plan.is_none_or(|plan| plan[hop_index].is_none()){Value::Null}else{json!({"pool_input":(economic_input-book_input).to_string(),"book_input":book_input.to_string(),"max_maker_fills":maker_fills,"book_start_hint":null})},"min_return":floor.to_string()}}));
        hops.last_mut().ok_or(diagnostics::Rejection::Accounting)?["declared_hybrid"] = operations
            .last()
            .ok_or(diagnostics::Rejection::Accounting)?["terra_swap"]["hybrid"]
            .clone();
        hops.last_mut().ok_or(diagnostics::Rejection::Accounting)?["book_commission_amount"] =
            json!(leg.book_commission_amount.to_string());
        hops.last_mut().ok_or(diagnostics::Rejection::Accounting)?["reserve_in"] =
            json!(rin.to_string());
        hops.last_mut().ok_or(diagnostics::Rejection::Accounting)?["reserve_out"] =
            json!(rout.to_string());
        hops.last_mut().ok_or(diagnostics::Rejection::Accounting)?["book_side"] =
            json!(if offer_token == &mirror.asset_0_addr {
                "bid"
            } else {
                "ask"
            });
        output = net;
        oldest = oldest.min(mirror.snapshot_at.timestamp_millis());
        if let Some(height) = mirror.block_height {
            block = Some(block.map_or(height, |old: i64| old.min(height)));
        }
    }
    let (fair, reference_at) = s
        .references
        .fair_output(
            path[0].1,
            path.last().ok_or(diagnostics::Rejection::NoRoute)?.2,
            input,
            Utc::now(),
        )
        .ok_or_else(|| diagnostics::reference_rejection(s, path, input))?;
    let spot = fair;
    oldest = oldest.min(reference_at);
    if BigInt::from(fair.abs_diff(output)) * BigInt::from(100u8)
        >= BigInt::from(fair.max(output)) * BigInt::from(99u8)
    {
        return Err(diagnostics::Rejection::FairDeviation);
    }
    Ok(Candidate {
        path: path.to_vec(),
        output,
        spot,
        hops,
        operations,
        oldest,
        block,
        mirrors: used_mirrors,
        protection_blocked,
        execution_market_spread,
        economics,
    })
}
fn better(a: &Candidate, b: &Candidate) -> bool {
    a.output > b.output || (a.output == b.output && a.path < b.path)
}
fn paths(s: &Snapshot, start: i32, goal: i32, depth: usize) -> Vec<Vec<route_paths::PathHop>> {
    skeletons::paths(s, start, goal, depth)
}
#[derive(Default)]
struct Selection {
    executable: Option<Candidate>,
    indicative: Option<Candidate>,
    rejected: Option<diagnostics::Rejection>,
}
#[allow(clippy::too_many_arguments)]
fn select_detailed(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    amount: u128,
    spread: u128,
    book_allowed: bool,
    maker_fills: u32,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
    native: Option<&NativeContext>,
) -> Selection {
    let splits = if book_allowed && plan.is_none() {
        vec![0, amount / 2, amount]
    } else {
        vec![0]
    };
    let mut selected = Selection::default();
    for path in paths {
        for book in &splits {
            let evaluated = evaluate_with_plan(s, path, amount, spread, *book, maker_fills, plan)
                .and_then(|candidate| {
                    if let Some(reason) = native.and_then(|n| {
                        native_candidate_rejection(n, &candidate, amount)
                            .or_else(|| native_fair_rejection(s, n, &candidate, amount))
                    }) {
                        Err(reason)
                    } else {
                        Ok(candidate)
                    }
                });
            let reason = match evaluated {
                Ok(candidate) if !candidate.protection_blocked => {
                    if selected
                        .executable
                        .as_ref()
                        .is_none_or(|old| better(&candidate, old))
                    {
                        selected.executable = Some(candidate);
                    }
                    continue;
                }
                Ok(candidate) => {
                    if selected
                        .indicative
                        .as_ref()
                        .is_none_or(|old| better(&candidate, old))
                    {
                        selected.indicative = Some(candidate);
                    }
                    diagnostics::Rejection::Protection
                }
                Err(reason) => reason,
            };
            if selected
                .rejected
                .is_none_or(|old| reason.priority() < old.priority())
            {
                selected.rejected = Some(reason);
            }
        }
    }
    selected
}
fn select(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    amount: u128,
    spread: u128,
    book_allowed: bool,
    maker_fills: u32,
) -> Option<Candidate> {
    select_detailed(
        s,
        paths,
        amount,
        spread,
        book_allowed,
        maker_fills,
        None,
        None,
    )
    .executable
}

fn response(
    s: &Snapshot,
    q: &route_solver::SolveRouteParams,
    c: &Candidate,
    input: u128,
    spread: u128,
    count: usize,
) -> Value {
    let mut candidate = c.clone();
    if let Some(target) = q.amount_out.as_ref().and_then(|v| v.parse::<u128>().ok()) {
        let last = candidate.hops.last_mut().unwrap();
        let floor = last["min_return"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            .max(target);
        last["min_return"] = json!(floor.to_string());
        candidate.operations.last_mut().unwrap()["terra_swap"]["min_return"] =
            json!(floor.to_string());
    }
    let c = &candidate;
    let route_tax = c.path.iter().any(|(_, offer, ask)| {
        [offer, ask]
            .iter()
            .any(|id| s.taxes.contains_key(&s.addresses[id]))
    });
    let sell_bps = c.hops[0]["sell_tax_bps"].as_u64().unwrap_or(0);
    let buy_bps = c.hops.last().unwrap()["buy_tax_bps"].as_u64().unwrap_or(0);
    let mut tokens = vec![q.token_in.clone()];
    let route_hops: Vec<_> = c
        .path
        .iter()
        .map(|(pair, offer, ask)| {
            tokens.push(s.addresses[ask].clone());
            json!({"pair":pair,"offer_token":s.addresses[offer],"ask_token":s.addresses[ask]})
        })
        .collect();
    let spread_text = decimal_text(spread);
    let floor = c.hops.last().unwrap()["min_return"].as_str().unwrap();
    let mut expires = c
        .oldest
        .saturating_add(LOCAL_QUOTE_SIGNING_WINDOW_MS)
        .min(s.config_expiry_ms);
    for (pair, offer, ask) in &c.path {
        expires = expires.min(
            s.config_expiries
                .get(pair)
                .copied()
                .unwrap_or(s.config_expiry_ms),
        );
        for id in [offer, ask] {
            if let Some(tax) = s.taxes.get(&s.addresses[id]) {
                expires = expires.min(tax.observed_at_ms.saturating_add(CONFIG_TTL_MS));
                if let Some(actor_expiry) = tax.actor_expiry(s.tax_trader.as_deref()) {
                    expires = expires.min(actor_expiry);
                }
            }
        }
    }
    for id in [c.path[0].1, c.path.last().unwrap().2] {
        if let Some(price) = s.references.prices.get(&id) {
            expires = expires.min(price.as_of.timestamp_millis() + LOCAL_QUOTE_SIGNING_WINDOW_MS);
        }
    }
    for (index, operation) in c.operations.iter().enumerate() {
        if operation["terra_swap"]["hybrid"].is_null() {
            continue;
        }
        let mirror = &c.mirrors[index];
        let book = if s.addresses[&c.path[index].1] == mirror.asset_0_addr {
            &mirror.bids
        } else {
            &mirror.asks
        };
        if let Some(expiry) = book
            .iter()
            .filter_map(|o| o.expires_at)
            .filter(|e| *e > now_ms() / 1000)
            .min()
        {
            expires = expires.min(expiry.saturating_mul(1000));
        }
    }
    let impact = if c.spot == 0 {
        None
    } else {
        Some(format!(
            "{:.4}",
            (c.spot.abs_diff(c.output) as f64 / c.spot.max(c.output) as f64) * 100.0
        ))
    };
    let mut body = json!({"token_in":q.token_in,"token_out":q.token_out,"hops":route_hops,"intermediate_tokens":tokens,
        "quote_kind":if c.operations.iter().all(|op|op["terra_swap"]["hybrid"].is_null()){"indexer_pool_db"}else{"indexer_hybrid_db"},"router_operations":c.operations,"estimated_amount_out":c.output.to_string(),"estimated_amount_out_net":c.output.to_string(),
        "tax_kind":if route_tax {"route_local_v1"} else {"none"},"buy_tax_bps":buy_bps,"sell_tax_bps":sell_bps,"router_hops_tax":route_tax,"tax_funding_version":if route_tax {json!(1)} else {Value::Null},"solver_version":"local_v1","paths_considered":count,
        "optimality_scope":"Best net output among bounded eligible local pool paths","lcd_hybrid_queries":0,"db_hybrid_queries":0,"fidelity_check":"skipped",
        "snapshot_age_ms":now_ms().saturating_sub(c.oldest),"snapshot_block_height":c.block,"spot_amount_out":c.spot.to_string(),"slippage_percent":impact,
        "required_amount_in":q.amount_out.as_ref().map(|_|input.to_string()),"target_amount_out":q.amount_out,"sizing_mode":q.amount_out.as_ref().map(|_|"local_pool_exact_output"),
        "execution_metadata":{"version":1,"tax_funding_version":if route_tax {json!(1)} else {Value::Null},"generation":s.generation,"input_amount":input.to_string(),"max_spread":spread_text,"trader":q.trader,"sender":q.sender,
            "observed_at_ms":c.oldest,"expires_at_ms":expires,"policy":if c.operations[0]["terra_swap"]["hybrid"].is_null(){"pool_only"}else{"declared_hybrid_hop0"},"hops":c.hops,"minimum_receive":floor}});
    if s.quote_relative_protection {
        body["execution_metadata"]["protection_policy"] = json!("quote_relative_v1");
        body["execution_metadata"]["execution_market_spread"] =
            json!(decimal_text(c.execution_market_spread));
    }
    order_economics::attach_original_book_terms(c, &mut body);
    body["economic_attribution"] = attribution::route(c, input, input, c.output, &body);
    body["order_economics"] = order_economics::route(c, input, &body);
    body
}

fn validity_tightened(current: &Snapshot, reviewed: &Snapshot) -> bool {
    observation_max_age(current) < observation_max_age(reviewed)
        || reviewed.pairs.iter().any(|pair| {
            current
                .config_expiries
                .get(&pair.contract_address)
                .copied()
                .unwrap_or(current.config_expiry_ms)
                < reviewed
                    .config_expiries
                    .get(&pair.contract_address)
                    .copied()
                    .unwrap_or(reviewed.config_expiry_ms)
        })
}
fn admission_generation(s: &Snapshot) -> String {
    let frozen = crate::indexer::asset_code_id_freeze::snapshot_frozen_pair_addresses();
    let mut relevant = s
        .pairs
        .iter()
        .filter(|p| frozen.contains(&p.contract_address))
        .map(|p| p.contract_address.as_str())
        .collect::<Vec<_>>();
    relevant.sort();
    let mut h = Sha256::new();
    h.update(format!(
        "{}|{relevant:?}|fee:{}|protection:{}|validity:{}|max_age:{}",
        s.generation,
        crate::indexer::fee_phase::revision(&s.namespace),
        s.quote_relative_protection,
        s.validity_revision,
        observation_max_age(s)
    ));
    format!("{:x}", h.finalize())
}
fn wrapper_generation(market: &str, w: &super::local_wrapper_quote::WrapperSnapshot) -> String {
    let mut mappings = w.mappings.iter().collect::<Vec<_>>();
    mappings.sort_by_key(|(k, _)| *k);
    let mut hash = Sha256::new();
    hash.update(format!(
        "{market}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{mappings:?}",
        w.mapper,
        w.treasury,
        w.observed_at_ms,
        w.fee_wrap_bps,
        w.fee_unwrap_bps,
        w.tax_atomics,
        w.router_unwrap_supported,
        w.mapper_floor_version,
        w.treasury_floor_version
    ));
    format!("{:x}", hash.finalize())
}
fn wrapper_namespace_admitted(g: &Store, namespace: &str) -> bool {
    g.wrappers.contains_key(namespace)
        || g.wrapper_revocations.contains_key(namespace)
        || g.wrappers.len() + g.wrapper_revocations.len() < 32
}
fn current_wrapper<'a>(
    g: &'a Store,
    namespace: &str,
) -> Result<&'a super::local_wrapper_quote::WrapperSnapshot, Error> {
    if g.wrapper_revocations.contains_key(namespace) {
        return Err(diagnostics::Rejection::Unsupported.error());
    }
    g.wrappers.get(namespace).ok_or_else(unavailable)
}
fn publish_wrapper(namespace: String, w: super::local_wrapper_quote::WrapperSnapshot) {
    let mut g = store().lock().unwrap();
    if wrapper_namespace_admitted(&g, &namespace) {
        g.wrapper_revocations.remove(&namespace);
        g.wrappers.insert(namespace, w);
    }
}
/// One owned scheduled observation publishes a whole view or revokes it. A
/// transient failure retains only the previous view's original expiry; a known
/// pause cannot leave that view executable while awaiting its next deadline.
fn apply_wrapper_observation(
    namespace: String,
    observation: super::local_wrapper_quote::WrapperObservation,
) -> bool {
    use super::local_wrapper_quote::WrapperObservation;
    match observation {
        WrapperObservation::Available(wrapper) => {
            publish_wrapper(namespace, wrapper);
            true
        }
        WrapperObservation::Revoked(reason) => {
            let mut g = store().lock().unwrap();
            if wrapper_namespace_admitted(&g, &namespace) {
                g.wrappers.remove(&namespace);
                g.wrapper_revocations.insert(namespace, reason);
            }
            drop(g);
            tracing::warn!(
                reason = reason.as_str(),
                "Native wrapper observation revoked"
            );
            false
        }
        WrapperObservation::Unavailable => false,
    }
}

#[derive(Clone)]
struct NativeContext {
    request: route_solver::SolveRouteParams,
    wrapper: super::local_wrapper_quote::WrapperSnapshot,
    wrap: bool,
    unwrap: bool,
}
/// Revalidation may race an independently observed pause after the native view
/// was cloned. Preserve its semantic rejection instead of inviting client
/// retention through a recoverable missing-observation response.
fn invalidated_snapshot_error(g: &Store, namespace: &str, native: bool) -> Error {
    if native && g.wrapper_revocations.contains_key(namespace) {
        diagnostics::Rejection::Unsupported.error()
    } else {
        unavailable()
    }
}
/// Linearize result publication against independently observed guards and market
/// generations. A request that started earlier cannot recreate a cleared cache.
fn reviewed_snapshot_current(
    g: &Store,
    namespace: &str,
    reviewed: &Snapshot,
    native: Option<&NativeContext>,
) -> bool {
    if actor_capacity_unavailable(g, namespace) {
        return false;
    }
    let Some(current) = g.snapshots.get(namespace) else {
        return false;
    };
    if validity_tightened(current, reviewed) {
        return false;
    }
    // A routine configuration refresh can advance its ingestion epoch while
    // retaining exactly the same economics. The full published generation binds
    // policies/references/tuples; negatives change that generation immediately.
    let mut generation = protocol_guards::generation(g, namespace, current);
    if native.is_some() {
        let Some(wrapper) = g.wrappers.get(namespace) else {
            return false;
        };
        generation = wrapper_generation(&generation, wrapper);
    }
    generation == reviewed.generation
}
fn native_candidate_rejection(
    n: &NativeContext,
    c: &Candidate,
    input: u128,
) -> Option<diagnostics::Rejection> {
    if c.operations
        .iter()
        .any(|op| !op["terra_swap"]["hybrid"].is_null())
    {
        return Some(diagnostics::Rejection::Unsupported);
    }
    if !n.wrapper.is_fresh(now_ms()) {
        super::prerequisite_trace::record(
            super::prerequisite_trace::Component::Wrapper,
            Some(n.wrapper.observed_at_ms),
            Some(n.wrapper.observed_at_ms.saturating_add(20_000)),
        );
        return Some(diagnostics::Rejection::Stale);
    }
    for (enabled, denom, amount) in [
        (
            n.wrap,
            &n.request.token_in,
            if n.wrap {
                n.request
                    .amount_in
                    .as_deref()
                    .and_then(|raw| raw.parse().ok())
                    .or_else(|| n.wrapper.inverse_wrap(input))
            } else {
                None
            },
        ),
        (n.unwrap, &n.request.token_out, Some(c.output)),
    ] {
        if !enabled {
            continue;
        }
        let Some(amount) = amount else {
            return Some(diagnostics::Rejection::AmountLimit);
        };
        let Some(mapping) = n.wrapper.mappings.get(denom) else {
            return Some(diagnostics::missing(MissingStage::NativeMapping));
        };
        if mapping.maximum.is_some_and(|limit| {
            mapping
                .used
                .checked_add(amount)
                .is_none_or(|used| used > limit)
        }) {
            return Some(diagnostics::Rejection::AmountLimit);
        }
    }
    if n.wrap && n.wrapper.fee_wrap_bps >= 10_000 {
        return Some(diagnostics::Rejection::Unsupported);
    }
    if n.unwrap {
        match n.wrapper.router_unwrap_supported {
            Some(true) => {}
            Some(false) => return Some(diagnostics::Rejection::Unsupported),
            None => return Some(diagnostics::missing(MissingStage::NativeRouter)),
        }
        if n.wrapper.tax_atomics.is_none() {
            return Some(diagnostics::missing(MissingStage::NativeTaxRate));
        }
        if n.wrapper
            .mappings
            .get(&n.request.token_out)
            .is_none_or(|m| m.tax_cap.is_none())
        {
            return Some(diagnostics::missing(MissingStage::NativeTaxCap));
        }
        if n.wrapper
            .unwrap_output(&n.request.token_out, c.output, now_ms())
            .is_none()
        {
            return Some(diagnostics::Rejection::Dust);
        }
    }
    None
}
fn native_fair_rejection(
    s: &Snapshot,
    n: &NativeContext,
    c: &Candidate,
    input: u128,
) -> Option<diagnostics::Rejection> {
    let gross = if n.wrap {
        n.request
            .amount_in
            .as_deref()
            .and_then(|v| v.parse().ok())
            .or_else(|| n.wrapper.inverse_wrap(input))?
    } else {
        input
    };
    let output = if n.unwrap {
        n.wrapper
            .unwrap_output(&n.request.token_out, c.output, now_ms())?
            .1
    } else {
        c.output
    };
    let (fair, _) =
        s.references
            .fair_output(c.path.first()?.1, c.path.last()?.2, gross, Utc::now())?;
    (BigInt::from(fair.abs_diff(output)) * BigInt::from(100u8)
        >= BigInt::from(fair.max(output)) * BigInt::from(99u8))
    .then_some(diagnostics::Rejection::FairDeviation)
}
fn final_response(
    s: &Snapshot,
    q: &route_solver::SolveRouteParams,
    c: &Candidate,
    input: u128,
    spread: u128,
    count: usize,
    native: Option<&NativeContext>,
) -> Option<Value> {
    let mut body = response(s, q, c, input, spread, count);
    let Some(n) = native else {
        return Some(body);
    };
    if native_candidate_rejection(n, c, input).is_some() {
        return None;
    }
    let gross = if n.wrap {
        if let Some(raw) = n.request.amount_in.as_ref() {
            raw.parse::<u128>().ok()?
        } else {
            n.wrapper.inverse_wrap(input)?
        }
    } else {
        input
    };
    let route_floor = body["execution_metadata"]["minimum_receive"]
        .as_str()?
        .parse::<u128>()
        .ok()?;
    let (output, wallet_floor, minimum_base) = if n.unwrap {
        let (_, output, _) = n
            .wrapper
            .unwrap_output(&n.request.token_out, c.output, now_ms())?;
        let (_, route_net_floor, _) =
            n.wrapper
                .unwrap_output(&n.request.token_out, route_floor, now_ms())?;
        let floor = route_net_floor.max(net_floor(s, output, spread)?).max(
            n.request
                .amount_out
                .as_ref()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        );
        let wrapped_floor = n.wrapper.inverse_unwrap(&n.request.token_out, floor)?;
        let (_, _, base) =
            n.wrapper
                .unwrap_output(&n.request.token_out, wrapped_floor, now_ms())?;
        (output, floor, base)
    } else {
        (c.output, route_floor, route_floor)
    };
    body["token_in"] = json!(n.request.token_in);
    body["token_out"] = json!(n.request.token_out);
    body["estimated_amount_out"] = json!(output.to_string());
    body["estimated_amount_out_net"] = json!(output.to_string());
    if n.request.amount_out.is_some() {
        body["required_amount_in"] = json!(gross.to_string());
        body["target_amount_out"] = json!(n.request.amount_out);
    }
    // Fair wallet value uses the authorized gross input and independent marks.
    // Comparing fee-reduced input/output to an equally reduced reference would
    // erase wrapper costs and could waive the nonwaivable >=99% risk guard.
    let (fair, _) =
        s.references
            .fair_output(c.path.first()?.1, c.path.last()?.2, gross, Utc::now())?;
    if BigInt::from(fair.abs_diff(output)) * BigInt::from(100u8)
        >= BigInt::from(fair.max(output)) * BigInt::from(99u8)
    {
        return None;
    }
    body["spot_amount_out"] = json!(fair.to_string());
    body["slippage_percent"] = json!(format!(
        "{:.4}",
        fair.abs_diff(output) as f64 / fair.max(output) as f64 * 100.0
    ));
    body["execution_metadata"]["observed_at_ms"] = json!(c.oldest.min(n.wrapper.observed_at_ms));
    body["execution_metadata"]["input_amount"] = json!(gross.to_string());
    body["execution_metadata"]["minimum_receive"] = json!(wallet_floor.to_string());
    body["execution_metadata"]["expires_at_ms"] = json!(body["execution_metadata"]
        ["expires_at_ms"]
        .as_i64()?
        .min(n.wrapper.observed_at_ms + LOCAL_QUOTE_SIGNING_WINDOW_MS));
    body["execution_metadata"]["wrapper"] = json!({"input_token":n.request.token_in,"output_token":n.request.token_out,"input_amount":gross.to_string(),"route_input_amount":input.to_string(),"route_output_amount":c.output.to_string(),"minimum_receive_base":minimum_base.to_string(),"wrap_input":n.wrap,"unwrap_output":n.unwrap,"treasury":n.wrapper.treasury,"mapper":n.wrapper.mapper});
    body["economic_attribution"] = attribution::route(c, gross, input, output, &body);
    body["order_economics"] = order_economics::route(c, gross, &body);
    Some(body)
}
struct JobGuard(String);
impl Drop for JobGuard {
    fn drop(&mut self) {
        if let Ok(mut g) = store().lock() {
            g.jobs.remove(&self.0);
            jobs::drain(&mut g);
        }
    }
}
/// Request handler. Deliberately cannot access LCD, SQL or recursively solved reference prices.
pub async fn solve(
    state: &AppState,
    q: &route_solver::SolveRouteParams,
) -> Result<Json<Value>, Error> {
    require_safe_indexing(state)?;
    let namespace = state.lcd.chain_cache_namespace();
    let quote = solve_local(&namespace, q).await;
    require_safe_indexing(state)?;
    quote
}
fn require_safe_indexing(state: &AppState) -> Result<(), Error> {
    use crate::indexer::lifecycle::IndexerStatus;
    if matches!(
        state.indexer_health.snapshot().status,
        IndexerStatus::Halted | IndexerStatus::Stopped
    ) {
        return Err(unavailable());
    }
    Ok(())
}
fn topology(namespace: &str, q: &route_solver::SolveRouteParams) -> Result<Json<Value>, Error> {
    let g = store().lock().unwrap();
    if actor_capacity_unavailable(&g, namespace) {
        return Err(unavailable());
    }
    let s = g
        .snapshots
        .get(namespace)
        .cloned()
        .ok_or_else(unavailable)?;
    let resolve = |token: &str| -> Option<i32> {
        let mapped = g
            .wrappers
            .get(namespace)
            .and_then(|w| w.mappings.get(token))
            .map(|m| m.cw20.as_str())
            .unwrap_or(token);
        s.ids.get(mapped).copied()
    };
    let start = resolve(&q.token_in).ok_or_else(|| invalid("Unknown cached input identity"))?;
    let goal = resolve(&q.token_out).ok_or_else(|| invalid("Unknown cached output identity"))?;
    // Structural caching locks Store independently. Never enumerate while holding
    // the registry guard; otherwise one topology request deadlocks every quote.
    drop(g);
    let mut s = (*s).clone();
    s.pairs.retain(|pair| {
        s.config_expiries
            .get(&pair.contract_address)
            .copied()
            .unwrap_or(s.config_expiry_ms)
            > now_ms()
    });
    let path = paths(&s, start, goal, 4)
        .into_iter()
        .next()
        .ok_or_else(|| diagnostics::candidate_error(namespace, &s, &[], start, goal, None))?;
    let hops=path.iter().map(|(pair,offer,ask)|json!({"pair":pair,"offer_token":s.addresses[offer],"ask_token":s.addresses[ask]})).collect::<Vec<_>>();
    let ops=path.iter().map(|(_,offer,ask)|json!({"terra_swap":{"offer_asset_info":{"token":{"contract_addr":s.addresses[offer]}},"ask_asset_info":{"token":{"contract_addr":s.addresses[ask]}},"hybrid":null}})).collect::<Vec<_>>();
    let mut tokens = vec![q.token_in.clone()];
    tokens.extend(path.iter().map(|(_, _, ask)| s.addresses[ask].clone()));
    if actor_capacity_unavailable(&store().lock().unwrap(), namespace) {
        return Err(unavailable());
    }
    Ok(Json(
        json!({"token_in":q.token_in,"token_out":q.token_out,"hops":hops,"intermediate_tokens":tokens,"router_operations":ops,"quote_kind":"indexer_route_only","estimated_amount_out":null,"solver_version":"local_v1"}),
    ))
}
fn bind_error_generation(
    namespace: &str,
    q: &route_solver::SolveRouteParams,
    mut error: Error,
) -> Error {
    let g = store().lock().unwrap();
    if let (Some(snapshot), Ok(mut body)) = (
        g.snapshots.get(namespace),
        serde_json::from_str::<Value>(&error.1),
    ) {
        let s = protocol_guards::derive(&g, namespace, snapshot);
        let generation = if matches!(q.token_in.as_str(), "uluna" | "uusd")
            || matches!(q.token_out.as_str(), "uluna" | "uusd")
        {
            g.wrappers
                .get(namespace)
                .map(|w| wrapper_generation(&s.generation, w))
        } else {
            Some(s.generation)
        };
        if let Some(generation) = generation {
            // Keep priced evidence bound to the generation which produced it.
            if body["generation"].is_null() {
                body["generation"] = json!(generation);
            }
            error.1 = body.to_string();
        }
    }
    error
}
async fn solve_local(
    namespace: &str,
    request: &route_solver::SolveRouteParams,
) -> Result<Json<Value>, Error> {
    solve_local_with_plan(namespace, request, None)
        .await
        .map_err(|error| bind_error_generation(namespace, request, error))
}
pub async fn solve_post(
    state: &AppState,
    request: &route_solver::SolveRouteParams,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Result<Json<Value>, Error> {
    if request.amount_in.is_none() || request.amount_out.is_some() {
        return Err(invalid("POST requires amount_in; receive sizing uses GET"));
    }
    if plan.is_some_and(|p| p.is_empty() || p.len() > 4) {
        return Err(invalid("hybrid_by_hop must declare one to four hops"));
    }
    require_safe_indexing(state)?;
    let namespace = state.lcd.chain_cache_namespace();
    let quote = solve_local_with_plan(&namespace, request, plan)
        .await
        .map_err(|error| bind_error_generation(&namespace, request, error));
    require_safe_indexing(state)?;
    quote
}
async fn solve_local_with_plan(
    namespace: &str,
    request: &route_solver::SolveRouteParams,
    plan: Option<&[Option<route_solver::HybridHopJson>]>,
) -> Result<Json<Value>, Error> {
    if request
        .protection_policy
        .as_deref()
        .is_some_and(|policy| policy != "quote_relative_v1")
    {
        return Err(invalid("Unknown protection policy"));
    }
    if actor_capacity_unavailable(&store().lock().unwrap(), namespace) {
        return Err(unavailable());
    }
    if request.amount_in.is_none() && request.amount_out.is_none() {
        return topology(namespace, request);
    }
    for raw in [request.amount_in.as_deref(), request.amount_out.as_deref()]
        .into_iter()
        .flatten()
    {
        if raw.len() > 39
            || raw.starts_with('0')
            || !raw.bytes().all(|b| b.is_ascii_digit())
            || raw.parse::<u128>().is_err()
        {
            return Err(invalid("Amount must be canonical positive Uint128"));
        }
    }
    let mut q = request.clone();
    let wrap = matches!(q.token_in.as_str(), "uluna" | "uusd");
    let unwrap = matches!(q.token_out.as_str(), "uluna" | "uusd");
    if (wrap || unwrap) && plan.is_some_and(|plan| plan.iter().any(Option::is_some)) {
        return Err(diagnostics::Rejection::Unsupported.error());
    }
    let native = if wrap || unwrap {
        let w = current_wrapper(&store().lock().unwrap(), namespace)?.clone();
        if !w.is_fresh(now_ms()) {
            super::prerequisite_trace::record(
                super::prerequisite_trace::Component::Wrapper,
                Some(w.observed_at_ms),
                Some(w.observed_at_ms.saturating_add(20_000)),
            );
            return Err(unavailable());
        }
        if wrap {
            let mapping = w.mappings.get(&q.token_in).ok_or_else(unavailable)?;
            if let Some(raw) = q.amount_in.as_ref() {
                let amount = raw
                    .parse::<u128>()
                    .map_err(|_| invalid("Invalid native offer"))?;
                if mapping.maximum.is_some_and(|limit| {
                    mapping
                        .used
                        .checked_add(amount)
                        .is_none_or(|used| used > limit)
                }) {
                    return Err(diagnostics::Rejection::AmountLimit.error());
                }
                let (_, credit) = w
                    .wrap_input(&q.token_in, amount, now_ms())
                    .ok_or_else(|| diagnostics::Rejection::Dust.error())?;
                q.amount_in = Some(credit.to_string());
            }
            q.token_in = mapping.cw20.clone();
        }
        if unwrap {
            let mapping = w.mappings.get(&q.token_out).ok_or_else(unavailable)?;
            if q.token_in != mapping.cw20 {
                match w.router_unwrap_supported {
                    Some(true) => {}
                    Some(false) => return Err(diagnostics::Rejection::Unsupported.error()),
                    None => return Err(diagnostics::Rejection::Missing.error()),
                }
            }
            if w.tax_atomics.is_none() || mapping.tax_cap.is_none() {
                return Err(diagnostics::Rejection::Missing.error());
            }
            if w.tax_atomics.is_some_and(|rate| rate >= SCALE) || w.fee_unwrap_bps >= 10_000 {
                return Err(diagnostics::Rejection::Unsupported.error());
            }
            if let Some(raw) = q.amount_out.as_ref() {
                let target = raw
                    .parse::<u128>()
                    .map_err(|_| invalid("Invalid native target"))?;
                q.amount_out = Some(
                    w.inverse_unwrap(&q.token_out, target)
                        .ok_or_else(|| diagnostics::Rejection::AmountLimit.error())?
                        .to_string(),
                );
            }
            q.token_out = mapping.cw20.clone();
        }
        Some(NativeContext {
            request: request.clone(),
            wrapper: w,
            wrap,
            unwrap,
        })
    } else {
        None
    };
    let q = &q;
    route_solver::parse_quote_trader(q.trader.clone(), q.sender.clone())?;
    if q.amount_in.is_some() == q.amount_out.is_some() {
        return Err(invalid(
            "Provide exactly one positive amount_in or amount_out",
        ));
    }
    if q.amount_out.is_some() && (q.hybrid_optimize == Some(true) || q.pool_only == Some(false)) {
        return Err(invalid("Receive sizing requires pool-only local execution"));
    }
    let spread = spread_atomics(q.max_spread.as_deref())?;
    let mut derived = {
        let g = store().lock().unwrap();
        let snapshot = g.snapshots.get(namespace).ok_or_else(unavailable)?;
        protocol_guards::derive(&g, namespace, snapshot)
    };
    derived.quote_relative_protection =
        request.protection_policy.as_deref() == Some("quote_relative_v1");
    for actor in [request.trader.as_deref(), request.sender.as_deref()]
        .into_iter()
        .flatten()
    {
        if derived
            .actors
            .get(actor)
            .is_some_and(|(_, blocked)| *blocked)
        {
            return Err((
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":"trading_blocked","generation":derived.generation}).to_string(),
            ));
        }
    }
    let subject = request.trader.as_deref().or(request.sender.as_deref());
    derived.tax_trader = subject.map(str::to_owned);
    let discount =
        if request.trader.is_some() && request.sender.is_some() && request.trader != request.sender
        {
            0
        } else {
            subject
                .and_then(|a| derived.actors.get(a))
                .map_or(0, |(bps, _)| *bps)
        };
    for (address, mirror) in derived
        .mirrors
        .iter_mut()
        .chain(derived.pool_mirrors.iter_mut())
    {
        if !derived.discounted_pairs.contains(address) {
            continue;
        }
        let mirror = Arc::make_mut(mirror);
        mirror.fee_bps = sim::effective_fee_bps_from_discount_bps(mirror.fee_bps, discount);
    }
    let mut snapshot = Arc::new(derived);
    if let Some(n) = native.as_ref() {
        let mut derived = (*snapshot).clone();
        derived.generation = wrapper_generation(&derived.generation, &n.wrapper);
        snapshot = Arc::new(derived);
    }
    if snapshot.config_expiry_ms <= now_ms() {
        return Err(unavailable());
    }
    if q.token_in == q.token_out {
        if plan.is_some_and(|p| !p.is_empty()) {
            return Err(invalid("Direct wrapper quotes have no pair split"));
        }
        let native = native.as_ref().ok_or_else(eligible_error)?;
        if !protocol_guards::permits_token(&snapshot, q.token_in.trim()) {
            return Err(eligible_error());
        }
        // Identity equality still requires an independent fresh reference. Its
        // absence is an availability failure, not evidence about offer size.
        let id = snapshot.ids.get(&q.token_in).ok_or_else(unavailable)?;
        if snapshot
            .references
            .fair_output(*id, *id, 1, Utc::now())
            .is_none()
        {
            return Err(unavailable());
        }
        let gross = native
            .request
            .amount_in
            .as_deref()
            .and_then(|v| v.parse::<u128>().ok())
            .or_else(|| {
                let target = native.request.amount_out.as_deref()?.parse::<u128>().ok()?;
                if native.wrap {
                    native.wrapper.inverse_wrap(target)
                } else {
                    native
                        .wrapper
                        .inverse_unwrap(&native.request.token_out, target)
                }
            })
            .ok_or_else(|| diagnostics::Rejection::AmountLimit.error())?;
        let denom = if native.wrap {
            &native.request.token_in
        } else {
            &native.request.token_out
        };
        let mapping = native.wrapper.mappings.get(denom).ok_or_else(unavailable)?;
        if mapping.maximum.is_some_and(|limit| {
            mapping
                .used
                .checked_add(gross)
                .is_none_or(|used| used > limit)
        }) {
            return Err(diagnostics::Rejection::AmountLimit.error());
        }
        let wallet_output = if native.wrap {
            native
                .wrapper
                .wrap_input(denom, gross, now_ms())
                .map(|(_, net)| net)
        } else {
            native
                .wrapper
                .unwrap_output_direct(denom, gross, now_ms())
                .map(|(_, net, _)| net)
        }
        .ok_or_else(|| diagnostics::Rejection::Dust.error())?;
        if BigInt::from(gross.abs_diff(wallet_output)) * BigInt::from(100u8)
            >= BigInt::from(gross.max(wallet_output)) * BigInt::from(99u8)
        {
            return Err(diagnostics::Rejection::FairDeviation.error());
        }
        let body = modes::wrapper_only(&snapshot, q, native, spread).ok_or_else(eligible_error)?;
        let g = store().lock().unwrap();
        if !reviewed_snapshot_current(&g, namespace, &snapshot, Some(native)) {
            return Err(invalidated_snapshot_error(&g, namespace, true));
        }
        return Ok(Json(body));
    }
    let start = *snapshot
        .ids
        .get(q.token_in.trim())
        .ok_or_else(|| invalid("token_in has no eligible cached identity"))?;
    let goal = *snapshot
        .ids
        .get(q.token_out.trim())
        .ok_or_else(|| invalid("token_out has no eligible cached identity"))?;
    let mut ck = key(namespace, &snapshot, request, spread);
    if let Some(plan) = plan {
        ck.push_str(&format!(
            "|manual:{}",
            serde_json::to_string(plan).map_err(|_| invalid("Invalid declared split"))?
        ));
    }
    {
        let mut g = store().lock().unwrap();
        if !reviewed_snapshot_current(&g, namespace, &snapshot, native.as_ref()) {
            return Err(invalidated_snapshot_error(&g, namespace, native.is_some()));
        }
        if let Some(body) = g
            .results
            .get(&ck)
            .filter(|body| {
                body["execution_metadata"]["expires_at_ms"]
                    .as_i64()
                    .is_some_and(|expiry| expiry > now_ms())
                    && body["hops"].as_array().is_some_and(|hops| {
                        hops.iter().all(|hop| {
                            hop["pair"].as_str().is_some_and(|pair| {
                                !crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(pair)
                            })
                        })
                    })
            })
            .cloned()
        {
            g.hits += 1;
            // A cache hit must not strand a short result rejected by a full
            // queue. Retry/coalesce using the original immutable cohort.
            if plan.is_none()
                && q.amount_in.is_some()
                && body["search_complete"] != true
                && jobs::needs_work(&g, &ck)
            {
                jobs::enqueue(
                    &mut g,
                    jobs::Work::new(
                        ck.clone(),
                        namespace,
                        &snapshot,
                        q,
                        native.as_ref(),
                        start,
                        goal,
                        spread,
                        &body,
                    ),
                );
            }
            return Ok(Json(body));
        }
    }
    let book_allowed = native.is_none()
        && q.pool_only != Some(true)
        && q.hybrid_optimize != Some(false)
        && q.amount_out.is_none();
    let maker_fills = crate::hybrid_limits::clamp_max_maker_fills(q.max_maker_fills.unwrap_or(8));
    let select_amount = |paths: &[Vec<route_paths::PathHop>], amount| {
        select_detailed(
            &snapshot,
            paths,
            amount,
            spread,
            book_allowed,
            maker_fills,
            plan,
            native.as_ref(),
        )
    };
    let mut discovery = skeletons::discovery(
        &snapshot,
        start,
        goal,
        if q.amount_out.is_some() { 4 } else { 2 },
    );
    let mut initial = discovery.paths.clone();
    let mut selected = q
        .amount_in
        .as_deref()
        .and_then(|raw| raw.parse::<u128>().ok())
        .map(|amount| select_amount(&initial, amount));
    // Keep the short result and its bounded evidence. Searching a longer
    // corridor consumes the existing pass; classification never re-simulates.
    if initial.is_empty()
        || selected
            .as_ref()
            .is_some_and(|result| result.executable.is_none())
    {
        discovery = skeletons::discovery(&snapshot, start, goal, 4);
        initial = discovery.paths.clone();
        selected = q
            .amount_in
            .as_deref()
            .and_then(|raw| raw.parse::<u128>().ok())
            .map(|amount| select_amount(&initial, amount));
    }
    let mut considered = initial.len();
    let (amount, candidate) = if let Some(raw) = q.amount_in.as_ref() {
        let amount = raw
            .parse::<u128>()
            .ok()
            .filter(|a| *a > 0)
            .ok_or_else(|| invalid("amount_in must be a positive Uint128"))?;
        let mut selected = selected.unwrap_or_default();
        let candidate = selected.executable.take().ok_or_else(|| {
            diagnostics::selection_error(
                namespace,
                &snapshot,
                &initial,
                start,
                goal,
                plan,
                q,
                amount,
                spread,
                native.as_ref(),
                selected,
            )
        })?;
        (amount, candidate)
    } else {
        let target = q
            .amount_out
            .as_ref()
            .unwrap()
            .parse::<u128>()
            .ok()
            .filter(|a| *a > 0)
            .ok_or_else(|| invalid("amount_out must be a positive Uint128"))?;
        // Invert constant-product and integer commission locally. Unlike power-of-two
        // bracketing, this cannot skip a feasible target near the spread boundary.
        considered = initial.len().min(64);
        let (mut best, mut rejected) =
            reverse::select_inverse_detailed(&snapshot, &initial, target, spread);
        // Preserve the supported conditional longer-path pass when the first
        // 64 bounded inversions contain only failing short alternatives.
        if best.is_none() {
            let longer = initial
                .iter()
                .skip(64)
                .filter(|path| path.len() > 2)
                .cloned()
                .collect::<Vec<_>>();
            considered += longer.len().min(64);
            let (candidate, reason) =
                reverse::select_inverse_detailed(&snapshot, &longer, target, spread);
            best = candidate;
            if let Some(reason) = reason {
                if rejected.is_none_or(|old| reason.priority() < old.priority()) {
                    rejected = Some(reason);
                }
            }
        }
        best.ok_or_else(|| {
            rejected
                .map(|reason| diagnostics::bound_error(&snapshot, reason.error()))
                .unwrap_or_else(|| {
                    diagnostics::candidate_error(namespace, &snapshot, &initial, start, goal, plan)
                })
        })?
    };
    let mut body = final_response(
        &snapshot,
        q,
        &candidate,
        amount,
        spread,
        considered,
        native.as_ref(),
    )
    .ok_or_else(|| {
        if let Some(reason) = native.as_ref().and_then(|n| {
            native_candidate_rejection(n, &candidate, amount)
                .or_else(|| native_fair_rejection(&snapshot, n, &candidate, amount))
        }) {
            diagnostics::bound_error(&snapshot, reason.error())
        } else {
            diagnostics::candidate_error(
                namespace,
                &snapshot,
                std::slice::from_ref(&candidate.path),
                start,
                goal,
                plan,
            )
        }
    })?;
    let complete =
        q.amount_out.is_some() || plan.is_some() || initial.iter().any(|path| path.len() > 2);
    if q.amount_out.is_some() && initial.len() > 64 {
        discovery.truncated = true;
    }
    discovery.annotate(&mut body, complete);
    if plan.is_some() {
        body["execution_metadata"]["policy"] = json!("declared_hybrid_manual");
        if body["order_economics"]["status"] == "available" {
            body["order_economics"]["policy"] = json!("declared_hybrid_manual");
        }
        body["optimality_scope"] = json!(
            "Best net output among bounded eligible paths matching the exact declared splits"
        );
    }
    {
        let mut g = store().lock().unwrap();
        if !reviewed_snapshot_current(&g, namespace, &snapshot, native.as_ref()) {
            return Err(invalidated_snapshot_error(&g, namespace, native.is_some()));
        }
        if g.results.len() >= MAX_RESULTS {
            g.results.clear();
        }
        g.results.insert(ck.clone(), body.clone());
        if plan.is_none()
            && q.amount_in.is_some()
            && body["search_complete"] != true
            && jobs::needs_work(&g, &ck)
        {
            jobs::enqueue(
                &mut g,
                jobs::Work::new(
                    ck.clone(),
                    namespace,
                    &snapshot,
                    q,
                    native.as_ref(),
                    start,
                    goal,
                    spread,
                    &body,
                ),
            );
        }
    }
    Ok(Json(body))
}

pub fn progress(
    state: &AppState,
    q: &route_solver::SolveRouteParams,
) -> Result<super::route_solve_progress::SolveProgress, Error> {
    if q.amount_in.is_some() == q.amount_out.is_some()
        || q.amount_in
            .as_deref()
            .or(q.amount_out.as_deref())
            .and_then(|a| a.parse::<u128>().ok())
            .is_none_or(|a| a == 0)
    {
        return Err(invalid(
            "Progress requires exactly one positive amount_in or amount_out",
        ));
    }
    require_safe_indexing(state)?;
    route_solver::parse_quote_trader(q.trader.clone(), q.sender.clone())?;
    progress_local(&state.lcd.chain_cache_namespace(), q)
}
fn progress_local(
    namespace: &str,
    q: &route_solver::SolveRouteParams,
) -> Result<super::route_solve_progress::SolveProgress, Error> {
    let ns = namespace.to_string();
    let g = store().lock().unwrap();
    if actor_capacity_unavailable(&g, &ns) {
        return Err(unavailable());
    }
    let mut s = protocol_guards::derive(&g, &ns, g.snapshots.get(&ns).ok_or_else(unavailable)?);
    s.quote_relative_protection = q.protection_policy.as_deref() == Some("quote_relative_v1");
    if matches!(q.token_in.as_str(), "uluna" | "uusd")
        || matches!(q.token_out.as_str(), "uluna" | "uusd")
    {
        let w = current_wrapper(&g, &ns)?;
        s.generation = wrapper_generation(&s.generation, w);
    }
    let ck = key(&ns, &s, q, spread_atomics(q.max_spread.as_deref())?);
    let searching = g.jobs.contains(&ck) || g.deferred_jobs.iter().any(|work| work.key == ck);
    let search_complete = g
        .results
        .get(&ck)
        .is_some_and(|body| body["search_complete"] == true);
    let resolve_id = |address: &str| {
        let wrapped = g
            .wrappers
            .get(&ns)
            .and_then(|w| w.mappings.get(address))
            .map(|mapping| mapping.cw20.as_str())
            .unwrap_or(address);
        s.ids.get(wrapped).copied()
    };
    let reference_blocker = [q.token_in.as_str(), q.token_out.as_str()]
        .into_iter()
        .find_map(|address| {
            resolve_id(address).map_or(Some("missing_observations"), |id| {
                s.references.unavailable_reason(id, Utc::now())
            })
        });
    let cached = reference_blocker.is_none()
        && g.results.get(&ck).is_some_and(|body| {
            body["hops"].as_array().is_some_and(|hops| {
                !hops.is_empty()
                    && hops.iter().all(|hop| {
                        hop["pair"].as_str().is_some_and(|pair| {
                            !crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(pair)
                                && s.pairs.iter().any(|p| p.contract_address == pair)
                                && s.config_expiries
                                    .get(pair)
                                    .copied()
                                    .unwrap_or(s.config_expiry_ms)
                                    > now_ms()
                        })
                    })
            }) && (!matches!(q.token_in.as_str(), "uluna" | "uusd")
                && !matches!(q.token_out.as_str(), "uluna" | "uusd")
                || g.wrappers
                    .get(&ns)
                    .is_some_and(|wrapper| wrapper.is_fresh(now_ms())))
                && body["execution_metadata"]["expires_at_ms"]
                    .as_i64()
                    .is_some_and(|expires| expires > now_ms())
                && body["execution_metadata"]["execution_supported"].as_bool() != Some(false)
                && body["execution_metadata"]["minimum_receive"]
                    .as_str()
                    .and_then(|raw| raw.parse::<u128>().ok())
                    .is_some_and(|floor| floor > 0)
                && body["router_operations"]
                    .as_array()
                    .is_some_and(|operations| !operations.is_empty())
        });
    let eligibility = if cached {
        "executable_quote"
    } else if reference_blocker == Some("reference_policy_excluded") {
        "policy_excluded"
    } else if reference_blocker.is_some() {
        "waiting_for_observations"
    } else {
        "unverified"
    };
    Ok(super::route_solve_progress::SolveProgress {
        stage: if searching {
            "evaluating"
        } else if cached {
            "cached"
        } else {
            "idle"
        }
        .into(),
        done: u32::from(cached && !searching && search_complete),
        total: 1,
        label: if searching {
            "Improving local route"
        } else if cached {
            "Local quote cached"
        } else {
            "Local search idle"
        }
        .into(),
        cache_hit: cached,
        updated_at_ms: now_ms().max(0) as u64,
        quote_generation: Some(s.generation.clone()),
        quote_eligibility: Some(eligibility.into()),
        quote_blocker: reference_blocker.map(str::to_owned),
        best_amount_out: cached
            .then(|| {
                g.results[&ck]["estimated_amount_out_net"]
                    .as_str()
                    .map(str::to_owned)
            })
            .flatten(),
        search_complete: Some(search_complete),
        search_truncated: g
            .results
            .get(&ck)
            .and_then(|body| body["search_truncated"].as_bool()),
    })
}

mod actor_fees;
mod actor_inventory;
mod actor_versions;
mod admission;
mod attribution;
mod diagnostics;
#[cfg(test)]
mod discovery_tests;
mod generation;
mod ingestion;
mod jobs;
#[cfg(test)]
mod missing_trace_tests;
mod modes;
mod order_economics;
mod pair_admission;
mod publication;
#[cfg(test)]
mod publication_tests;
mod reverse;
#[cfg(test)]
mod small_market_tests;
#[cfg(test)]
use reverse::inverse_pool_path;
#[cfg(test)]
use reverse::select_inverse;
mod protection;
mod protocol_guards;
use protection::{decimal_text, net_floor, quote_relative_market_spread};
mod skeletons;
mod tax;
pub use admission::{ingest_fixture, ingest_once};
pub use ingestion::{
    cache_counters, refresh_reserves, refresh_wrapper, start_ingestion, start_ingestion_with_cancel,
};

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    fn initialize_for_test(
        namespace: String,
        pairs: Vec<pairs::PairRow>,
        addresses: HashMap<i32, String>,
        mirrors: HashMap<String, Arc<sim::HopMirror>>,
        max_age_ms: u64,
    ) {
        super::initialize_for_test(
            namespace,
            pairs,
            addresses,
            mirrors
                .into_iter()
                .map(|(address, mirror)| (address, (*mirror).clone()))
                .collect(),
            max_age_ms,
        );
    }

    pub(super) fn fixture() -> Snapshot {
        let now = Utc::now();
        let mut addresses = HashMap::new();
        addresses.insert(1, "in18".into());
        addresses.insert(2, "out18".into());
        let mut pairs = Vec::new();
        let mut mirrors = HashMap::new();
        for i in 0..8 {
            let mid = 3 + i;
            let token = format!("middle{i}6");
            addresses.insert(mid, token.clone());
            let depth = if i == 7 { 1_000_000u128 } else { 10 };
            for (a, b, rin, rout, suffix) in [
                (1, mid, depth * 10u128.pow(18), depth * 10u128.pow(6), "a"),
                (mid, 2, depth * 10u128.pow(6), depth * 10u128.pow(18), "b"),
            ] {
                let addr = format!("pair{i}{suffix}");
                let id = pairs.len() as i32;
                pairs.push(pairs::PairRow {
                    id,
                    contract_address: addr.clone(),
                    asset_0_id: a,
                    asset_1_id: b,
                    lp_token: None,
                    fee_bps: Some(30),
                    hooks: vec![],
                    created_at_block: Some(1),
                    created_at: now,
                    updated_at: now,
                });
                mirrors.insert(
                    addr,
                    Arc::new(sim::HopMirror {
                        pair_id: id,
                        asset_0_addr: addresses[&a].clone(),
                        asset_1_addr: addresses[&b].clone(),
                        reserve_0: rin,
                        reserve_1: rout,
                        fee_bps: 30,
                        block_height: Some(10),
                        snapshot_at: now,
                        freshness: sim::MirrorFreshness::Fresh,
                        bids: vec![],
                        asks: vec![],
                    }),
                );
            }
        }
        Snapshot {
            namespace: "unit1452".into(),
            generation: "fixture".into(),
            config_epoch: 0,
            validity_revision: 0,
            actor_revision: 0,
            ids: addresses.iter().map(|(id, a)| (a.clone(), *id)).collect(),
            addresses: addresses.clone(),
            pairs,
            mirrors,
            pool_mirrors: HashMap::new(),
            view_hashes: Arc::default(),
            max_age_ms: 20_000,
            hooks: HashMap::new(),
            config_expiry_ms: now_ms() + 180_000,
            config_expiries: HashMap::new(),
            pair_contexts: HashMap::new(),
            references: super::super::local_reference_quote::ReferencePrices::synthetic(
                addresses.keys().copied(),
            ),
            actors: HashMap::new(),
            discounted_pairs: HashSet::new(),
            policy_identity: "synthetic".into(),
            quote_relative_protection: false,
            taxes: HashMap::new(),
            tax_trader: None,
            protocol_actor_tokens: HashMap::new(),
        }
    }
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_http_stale_origins_are_bounded_private_and_correlated() {
        use axum::{
            body::{to_bytes, Body},
            http::Request,
            response::IntoResponse,
            routing::get,
            Router,
        };
        use tower::ServiceExt;
        let app = Router::new()
            .route(
                "/api/v1/route/solve",
                get(|| async {
                    let mut snapshot = fixture();
                    snapshot.references.prices.get_mut(&1).unwrap().as_of =
                        Utc::now() - chrono::Duration::seconds(21);
                    let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
                    for _ in 0..64 {
                        assert_eq!(
                            evaluate_with_plan(
                                &snapshot,
                                &path,
                                10u128.pow(18),
                                SCALE / 20,
                                0,
                                8,
                                None
                            )
                            .err(),
                            Some(diagnostics::Rejection::Stale)
                        );
                    }
                    public_error(diagnostics::Rejection::Stale.error())
                }),
            )
            .route(
                "/api/v1/route/solve/best",
                get(|| async {
                    // An unavailable alternative must not log a failed user request
                    // when another candidate succeeds.
                    super::super::prerequisite_trace::record(
                        super::super::prerequisite_trace::Component::Reference,
                        Some(now_ms() - 21_000),
                        Some(now_ms() - 1_000),
                    );
                    Json(json!({"useful":true})).into_response()
                }),
            )
            .layer(
                tower_http::trace::TraceLayer::new_for_http()
                    .make_span_with(super::super::quote_trace::http_span),
            )
            .layer(axum::middleware::from_fn(
                super::super::quote_trace::correlate,
            ));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/route/solve?trader=private-stale-actor&amount_in=private-amount")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let id = response.headers()["x-quote-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            serde_json::from_slice::<Value>(&to_bytes(response.into_body(), 2048).await.unwrap())
                .unwrap(),
            json!({"error":"stale_observations"})
        );
        assert_eq!(
            app.oneshot(
                Request::builder()
                    .uri("/api/v1/route/solve/best")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
            StatusCode::OK
        );
        assert!(logs_contain(&id));
        assert!(logs_contain("reference"));
        assert!(logs_contain("age_known=true"));
        assert!(!logs_contain("private-stale-actor"));
        assert!(!logs_contain("private-amount"));
        assert!(!logs_contain("pair7a"));
        logs_assert(|lines: &[&str]| {
            let count = lines
                .iter()
                .filter(|line| line.contains("Local quote prerequisite stale"))
                .count();
            if count == 1 {
                Ok(())
            } else {
                Err(format!(
                    "expected one stale witness only for rejected request, got {count}"
                ))
            }
        });
    }

    pub(super) fn request(amount: u128) -> route_solver::SolveRouteParams {
        route_solver::SolveRouteParams {
            max_spread: Some("0.05".into()),
            protection_policy: None,
            token_in: "in18".into(),
            token_out: "out18".into(),
            amount_in: Some(amount.to_string()),
            amount_out: None,
            hybrid_optimize: None,
            pool_only: None,
            max_maker_fills: None,
            trader: None,
            sender: None,
            discount_bps: None,
        }
    }

    /// Shared public corpus, pinned synthetic reserves and independent integer
    /// oracle. Native labels exercise decimal economics here; socket tests own
    /// real wrapper admission and protocol capability evidence.
    #[test]
    fn issue1473_bidirectional_shared_matrix_pins_net_output_and_protection() {
        let corpus: Value =
            serde_json::from_str(include_str!("../../../docs/qa/issue-1473/matrix.json")).unwrap();
        let mut count = 0;
        for pair in corpus["pairs"].as_array().unwrap() {
            for reverse in [false, true] {
                let first = pair[usize::from(reverse)].as_str().unwrap();
                let second = pair[usize::from(!reverse)].as_str().unwrap();
                let da = corpus["assets"][first]["decimals"].as_u64().unwrap() as u32;
                let db = corpus["assets"][second]["decimals"].as_u64().unwrap() as u32;
                let mut s = fixture();
                s.quote_relative_protection = true;
                let mut m = (*s.mirrors["pair0a"]).clone();
                m.asset_0_addr = first.into();
                m.asset_1_addr = second.into();
                m.reserve_0 = 1_000 * 10u128.pow(da);
                m.reserve_1 = 1_000 * 10u128.pow(db);
                s.addresses.insert(1, first.into());
                s.addresses.insert(2, second.into());
                s.mirrors.insert("matrix".into(), Arc::new(m.clone()));
                for (id, decimals) in [(1, da), (2, db)] {
                    s.references.prices.insert(
                        id,
                        super::super::local_reference_quote::ReferencePrice {
                            price: BigDecimal::from(1),
                            decimals,
                            as_of: Utc::now(),
                        },
                    );
                }
                let path = vec![("matrix".into(), 1, 2)];
                let mut amounts = vec![1];
                for human in corpus["human_amounts"].as_array().unwrap() {
                    let raw = (human.as_str().unwrap().parse::<BigDecimal>().unwrap()
                        * BigDecimal::from(10u128.pow(da)))
                    .with_scale(0)
                    .to_string()
                    .parse::<u128>()
                    .unwrap();
                    amounts.push(raw);
                }
                for input in amounts {
                    let gross = (BigInt::from(m.reserve_1) * BigInt::from(input)
                        / BigInt::from(m.reserve_0 + input))
                    .to_string()
                    .parse::<u128>()
                    .unwrap();
                    let commission = gross * 30 / 10_000;
                    let expected = gross - commission;
                    let ideal = (BigInt::from(input) * BigInt::from(m.reserve_1)
                        / BigInt::from(m.reserve_0))
                    .to_string()
                    .parse::<u128>()
                    .unwrap();
                    for spread_text in corpus["max_spreads"].as_array().unwrap() {
                        let spread = spread_atomics(spread_text.as_str()).unwrap();
                        let result = evaluate_with_plan(&s, &path, input, spread, 0, 8, None);
                        if expected == 0 {
                            assert!(
                                matches!(result, Err(diagnostics::Rejection::Dust)),
                                "{first}->{second} input {input}"
                            );
                            assert!(score(&s, &path, input, spread).is_none());
                        } else {
                            let c = result.unwrap_or_else(|r| {
                                panic!("{first}->{second} input {input}: {r:?}")
                            });
                            assert_eq!(c.output, expected, "{first}->{second} input {input}");
                            let blocked = BigInt::from(ideal - gross) * BigInt::from(SCALE)
                                > BigInt::from(gross) * BigInt::from(spread);
                            assert!(!c.protection_blocked);
                            assert!(score(&s, &path, input, spread).is_some());
                            assert!(c.execution_market_spread >= spread);
                            assert_eq!(
                                c.execution_market_spread,
                                quote_relative_market_spread(
                                    (ideal - gross).min(gross),
                                    gross,
                                    spread
                                )
                                .unwrap()
                            );
                            if blocked {
                                assert!(c.execution_market_spread > spread);
                            }
                            assert!(
                                c.hops[0]["min_return"]
                                    .as_str()
                                    .unwrap()
                                    .parse::<u128>()
                                    .unwrap()
                                    > 0
                            );
                        }
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 420);
    }
    #[test]
    fn issue1473_price_only_error_strips_signing_material_and_keeps_original_expiry() {
        let mut s = fixture();
        let path = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        let q = request(10u128.pow(18));
        let error = diagnostics::amount_error(
            "unit",
            &s,
            std::slice::from_ref(&path),
            1,
            2,
            None,
            &q,
            10u128.pow(18),
            SCALE / 100,
            false,
            8,
            None,
        );
        assert_eq!(error.0, StatusCode::UNPROCESSABLE_ENTITY);
        let body: Value = serde_json::from_str(&error.1).unwrap();
        assert_eq!(body["error"], "protection");
        assert_eq!(body["execution_supported"], false);
        assert!(
            body["indicative_amount_out"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        assert_eq!(body["generation"], s.generation);
        assert!(
            body["expires_at_ms"].as_i64().unwrap()
                <= s.mirrors["pair0a"].snapshot_at.timestamp_millis() + LOCAL_QUOTE_SIGNING_WINDOW_MS
        );
        for absent in [
            "router_operations",
            "execution_metadata",
            "hops",
            "minimum_receive",
        ] {
            assert!(body[absent].is_null());
        }
        Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap()).reserve_0 = 0;
        let failure = evaluate_with_plan(&s, &path, 10u128.pow(18), SCALE / 100, 0, 8, None);
        assert!(matches!(failure, Err(diagnostics::Rejection::Liquidity)));
        let error = diagnostics::amount_error(
            "unit",
            &s,
            &[path],
            1,
            2,
            None,
            &q,
            10u128.pow(18),
            SCALE / 100,
            false,
            8,
            None,
        );
        let body: Value = serde_json::from_str(&error.1).unwrap();
        assert_eq!(body["error"], "insufficient_liquidity");
        assert!(body["indicative_amount_out"].is_null());
    }

    #[test]
    fn issue1473_arbitrary_identities_boundaries_and_failure_classes() {
        let mut s = fixture();
        s.addresses.insert(1, "arbitrary-offer".into());
        s.addresses.insert(2, "unrelated-ask".into());
        let mut m = (*s.mirrors["pair0a"]).clone();
        m.asset_0_addr = "arbitrary-offer".into();
        m.asset_1_addr = "unrelated-ask".into();
        m.reserve_0 = 100_000_000;
        m.reserve_1 = 100_000_000;
        s.mirrors
            .insert("arbitrary-pair".into(), Arc::new(m.clone()));
        let path = vec![("arbitrary-pair".into(), 1, 2)];
        for reverse in [false, true] {
            let path = if reverse {
                vec![("arbitrary-pair".into(), 2, 1)]
            } else {
                path.clone()
            };
            for spread in [SCALE / 100, SCALE / 20, SCALE * 30 / 100] {
                let boundary = wide_floor(m.reserve_0, spread, SCALE).unwrap();
                for input in [boundary - 1, boundary, boundary + 1] {
                    let gross = m.reserve_1 * input / (m.reserve_0 + input);
                    let ideal = input * m.reserve_1 / m.reserve_0;
                    let blocked = BigInt::from(ideal - gross) * BigInt::from(SCALE)
                        > BigInt::from(gross) * BigInt::from(spread);
                    let c = evaluate_with_plan(&s, &path, input, spread, 0, 8, None).unwrap();
                    assert_eq!(c.output, gross - gross * 30 / 10_000);
                    assert_eq!(c.protection_blocked, blocked);
                    assert_eq!(score(&s, &path, input, spread).is_some(), !blocked);
                }
            }
        }
        assert!(matches!(
            evaluate_with_plan(&s, &path, u128::MAX, SCALE / 20, 0, 8, None),
            Err(diagnostics::Rejection::Accounting)
        ));
        assert!(matches!(
            evaluate_with_plan(&s, &[], 1, SCALE / 20, 0, 8, None),
            Err(diagnostics::Rejection::NoRoute)
        ));
        s.references.prices.remove(&2);
        assert!(matches!(
            evaluate_with_plan(&s, &path, 100_000, SCALE / 20, 0, 8, None),
            Err(diagnostics::Rejection::Missing)
        ));
        s.references.prices.insert(
            2,
            super::super::local_reference_quote::ReferencePrice {
                price: BigDecimal::from(1),
                decimals: 39,
                as_of: Utc::now(),
            },
        );
        assert!(matches!(
            evaluate_with_plan(&s, &path, 100_000, SCALE / 20, 0, 8, None),
            Err(diagnostics::Rejection::Unsupported)
        ));
        Arc::make_mut(s.mirrors.get_mut("arbitrary-pair").unwrap()).snapshot_at -=
            chrono::Duration::seconds(21);
        assert!(matches!(
            evaluate_with_plan(&s, &path, 100_000, SCALE / 20, 0, 8, None),
            Err(diagnostics::Rejection::Stale)
        ));
    }

    #[test]
    fn issue1473_http_reason_contract_allowlists_and_bounds_private_payloads() {
        let now = now_ms();
        let supplied = json!({"error":"protection","private_actor":"secret","router_operations":[{"arbitrary":"transaction"}],
            "indicative_amount_out":"123","execution_supported":false,"generation":"test-generation","observed_at_ms":now,"expires_at_ms":now+19_000});
        let (_, Json(clean)) =
            public_error((StatusCode::UNPROCESSABLE_ENTITY, supplied.to_string()));
        assert!(clean["private_actor"].is_null());
        assert!(clean["router_operations"].is_null());
        assert_eq!(clean["indicative_amount_out"], "123");
        for bad in [
            "+123",
            "0123",
            "0",
            "1e5",
            "340282366920938463463374607431768211456",
        ] {
            let mut supplied = supplied.clone();
            supplied["indicative_amount_out"] = json!(bad);
            let (_, Json(clean)) =
                public_error((StatusCode::UNPROCESSABLE_ENTITY, supplied.to_string()));
            assert!(clean["indicative_amount_out"].is_null());
        }
        let (_, Json(clean)) = public_error((
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"error":"<script>private</script>"}).to_string(),
        ));
        assert_eq!(clean, json!({"error":"protection"}));
        let mut missing_generation = supplied.clone();
        missing_generation["generation"] = json!("");
        let (_, Json(clean)) = public_error((
            StatusCode::UNPROCESSABLE_ENTITY,
            missing_generation.to_string(),
        ));
        assert!(clean["generation"].is_null());
        assert!(clean["indicative_amount_out"].is_null());
        let (_, Json(clean)) =
            public_error((StatusCode::UNPROCESSABLE_ENTITY, "sensitive".repeat(1_000)));
        assert_eq!(clean, json!({"error":"protection"}));
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn issue1473_request_amounts_reject_noncanonical_and_overflow_without_discovery() {
        for raw in [
            "0",
            "-1",
            "+1",
            "01",
            "1e6",
            "0.01",
            "340282366920938463463374607431768211456",
            &"1".repeat(1_000),
        ] {
            let mut q = request(1);
            q.amount_in = Some(raw.into());
            let error = solve_local("uninitialized-malformed1473", &q)
                .await
                .err()
                .unwrap();
            assert_eq!(error.0, StatusCode::BAD_REQUEST, "{raw}");
            let body: Value = serde_json::from_str(&error.1).unwrap();
            assert_eq!(body["error"], "invalid_request");
        }
    }
    #[test]
    fn issue1473_quote_relative_metadata_keeps_exact_ui_tolerance_and_positive_floors() {
        let mut s = fixture();
        s.quote_relative_protection = true;
        let path = vec![("pair0a".into(), 1, 3), ("pair0b".into(), 3, 2)];
        let input = 10u128.pow(18);
        let mut q = request(input);
        q.protection_policy = Some("quote_relative_v1".into());
        let c = score(&s, &path, input, SCALE / 100).unwrap();
        let body = response(&s, &q, &c, input, SCALE / 100, 1);
        assert_eq!(body["execution_metadata"]["max_spread"], "0.01");
        assert_eq!(
            body["execution_metadata"]["protection_policy"],
            "quote_relative_v1"
        );
        assert_ne!(
            body["execution_metadata"]["execution_market_spread"],
            "0.01"
        );
        for hop in &c.hops {
            let net = hop["return_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap();
            assert_eq!(
                hop["min_return"].as_str().unwrap().parse::<u128>().unwrap(),
                ((net * 99 + 99) / 100).max(1)
            );
        }
        assert_eq!(
            body["execution_metadata"]["minimum_receive"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            ((c.output * 99 + 99) / 100).max(1)
        );
        s.quote_relative_protection = false;
        assert!(score(&s, &path, input, SCALE / 100).is_none());
    }

    #[test]
    fn issue1473_native_pricing_capability_and_rate_caps_are_distinct_from_ui_protection() {
        use super::super::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
        let s = fixture();
        let input = 10u128.pow(18);
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let c = score(&s, &path, input, SCALE / 20).unwrap();
        let mut q = request(input);
        q.token_out = "uusd".into();
        let mut n = NativeContext {
            request: q,
            wrap: false,
            unwrap: true,
            wrapper: WrapperSnapshot {
                mapper: "mapper".into(),
                treasury: "treasury".into(),
                observed_at_ms: now_ms(),
                mappings: HashMap::from([(
                    "uusd".into(),
                    NativeMapping {
                        cw20: "out18".into(),
                        maximum: None,
                        used: 0,
                        tax_cap: Some(0),
                    },
                )]),
                fee_wrap_bps: 0,
                fee_unwrap_bps: 0,
                tax_atomics: Some(0),
                router_unwrap_supported: Some(false),
                mapper_floor_version: Some(1),
                treasury_floor_version: Some(1),
            },
        };
        assert_eq!(
            native_candidate_rejection(&n, &c, input),
            Some(diagnostics::Rejection::Unsupported)
        );
        n.wrapper.router_unwrap_supported = None;
        assert_eq!(
            native_candidate_rejection(&n, &c, input),
            Some(diagnostics::Rejection::Missing),
            "an unavailable scheduled router read is not proof of unsupported semantics"
        );
        n.wrapper.router_unwrap_supported = Some(true);
        n.wrapper.tax_atomics = None;
        assert_eq!(
            native_candidate_rejection(&n, &c, input),
            Some(diagnostics::Rejection::Missing)
        );
        n.wrapper.tax_atomics = Some(0);
        n.wrapper.mappings.get_mut("uusd").unwrap().maximum = Some(c.output - 1);
        assert_eq!(
            native_candidate_rejection(&n, &c, input),
            Some(diagnostics::Rejection::AmountLimit)
        );
        n.wrapper.mappings.get_mut("uusd").unwrap().maximum = None;
        assert_eq!(native_candidate_rejection(&n, &c, input), None);
        assert!(final_response(&s, &request(input), &c, input, SCALE / 20, 1, Some(&n)).is_some());
        n.wrapper.observed_at_ms -= 21_000;
        assert_eq!(
            native_candidate_rejection(&n, &c, input),
            Some(diagnostics::Rejection::Stale)
        );
    }

    #[test]
    #[serial_test::serial]
    fn topology_releases_registry_before_structural_cache_and_leaves_it_usable() {
        let namespace = "1452-topology-registry-reentry";
        let s = fixture();
        initialize_for_test(
            namespace.into(),
            s.pairs,
            s.addresses,
            s.mirrors,
            s.max_age_ms,
        );
        let mut discovery = request(1);
        discovery.amount_in = None;
        let first = topology(namespace, &discovery).unwrap();
        let repeated = topology(namespace, &discovery).unwrap();
        assert_eq!(
            first.0["router_operations"],
            repeated.0["router_operations"]
        );
        assert!(!first.0["hops"].as_array().unwrap().is_empty());
        assert_eq!(first.0["quote_kind"], "indexer_route_only");
        // Another parallel fixture can legitimately acquire the global registry
        // after topology returns. Reentry must work; immediate vacancy is not an
        // ownership guarantee. The repeated requests above also exercise reentry
        // through the structural cache rather than only acquiring this mutex.
        assert!(store().lock().is_ok());
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn actor_capacity_quarantine_blocks_warm_quotes_topology_and_progress() {
        let namespace = "actor-capacity-quarantine-fixture";
        let s = fixture();
        initialize_for_test(
            namespace.into(),
            s.pairs,
            s.addresses,
            s.mirrors,
            s.max_age_ms,
        );
        let q = request(10u128.pow(18));
        assert!(solve_local(namespace, &q).await.is_ok());
        {
            let mut g = store().lock().unwrap();
            mark_actor_capacity_exhausted(&mut g, namespace);
        }
        assert_eq!(
            solve_local(namespace, &q).await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            topology(namespace, &q).unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            progress_local(namespace, &q).unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        let mut discovery = q.clone();
        discovery.amount_in = None;
        assert_eq!(
            solve_local(namespace, &discovery).await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    #[test]
    fn actor_quarantine_namespaces_are_bounded_and_registry_overflow_fails_closed() {
        let mut local = Store::default();
        for i in 0..32 {
            mark_actor_capacity_exhausted(&mut local, &format!("namespace-{i}"));
        }
        assert_eq!(local.actor_capacity_exhausted.len(), 32);
        assert!(!local.actor_capacity_registry_exhausted);
        mark_actor_capacity_exhausted(&mut local, "overflow-namespace");
        assert_eq!(local.actor_capacity_exhausted.len(), 32);
        assert!(actor_capacity_unavailable(&local, "overflow-namespace"));
    }
    #[test]
    fn quality_admission_compares_all_short_corridors_and_rejects_thin_protection() {
        let s = fixture();
        let paths = paths(&s, 1, 2, 2);
        assert_eq!(paths.len(), 8);
        for amount in [
            10u128.pow(16),
            10u128.pow(17),
            10u128.pow(18),
            5 * 10u128.pow(18),
        ] {
            let c = select(&s, &paths, amount, SCALE / 20, true, 8).unwrap();
            assert_eq!(c.path[0].0, "pair7a");
            assert!(c.output > 0);
        }
        let mut shuffled = s.clone();
        shuffled.pairs.reverse();
        assert_eq!(
            select(&shuffled, &paths, 10u128.pow(18), SCALE / 20, true, 8)
                .unwrap()
                .path,
            select(&s, &paths, 10u128.pow(18), SCALE / 20, true, 8)
                .unwrap()
                .path
        );
    }
    #[test]
    fn inverse_is_minimal_across_power_of_two_spread_boundary_and_mixed_decimals() {
        let s = fixture();
        let p = paths(&s, 1, 2, 2).pop().unwrap();
        for target in [1, 40 * 10u128.pow(18), 1_000 * 10u128.pow(18)] {
            let offer = inverse_pool_path(&s, &p, target).unwrap();
            let c = score(&s, &p, offer, SCALE / 20).unwrap();
            assert!(c.output >= target);
            assert!(score(&s, &p, offer - 1, SCALE / 20).is_none_or(|c| c.output < target));
        }
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn inverse_uses_eligible_longer_baseline_when_all_short_targets_fail_protection() {
        let mut s = fixture();
        for (name, m) in s.mirrors.iter_mut() {
            if name.starts_with("pair7") {
                let m = Arc::make_mut(m);
                m.reserve_0 /= 100_000;
                m.reserve_1 /= 100_000;
            }
        }
        for (id, address) in [(100, "deep6"), (101, "deep18")] {
            s.addresses.insert(id, address.into());
        }
        for (index, (a, b, rin, rout)) in [
            (1, 100, 10u128.pow(24), 10u128.pow(12)),
            (100, 101, 10u128.pow(12), 10u128.pow(24)),
            (101, 2, 10u128.pow(24), 10u128.pow(24)),
        ]
        .into_iter()
        .enumerate()
        {
            let pair = format!("long-inverse-{index}");
            s.pairs.push(pairs::PairRow {
                id: 100 + index as i32,
                contract_address: pair.clone(),
                asset_0_id: a,
                asset_1_id: b,
                lp_token: None,
                fee_bps: Some(30),
                hooks: vec![],
                created_at_block: Some(1),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            });
            s.mirrors.insert(
                pair,
                Arc::new(sim::HopMirror {
                    pair_id: 100 + index as i32,
                    asset_0_addr: s.addresses[&a].clone(),
                    asset_1_addr: s.addresses[&b].clone(),
                    reserve_0: rin,
                    reserve_1: rout,
                    fee_bps: 30,
                    block_height: Some(10),
                    snapshot_at: Utc::now(),
                    freshness: sim::MirrorFreshness::Fresh,
                    bids: vec![],
                    asks: vec![],
                }),
            );
        }
        initialize_for_test(
            "inverse-longer-fixture".into(),
            s.pairs,
            s.addresses,
            s.mirrors,
            s.max_age_ms,
        );
        let mut q = request(1);
        q.amount_in = None;
        q.amount_out = Some(10u128.pow(18).to_string());
        q.pool_only = Some(true);
        let body = solve_local("inverse-longer-fixture", &q).await.unwrap().0;
        assert_eq!(body["hops"].as_array().unwrap().len(), 3);
        assert!(
            body["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                >= 10u128.pow(18)
        );
        assert!(
            body["execution_metadata"]["minimum_receive"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                >= 10u128.pow(18)
        );
    }
    #[test]
    fn stale_future_identity_overflow_and_dust_are_not_executable() {
        let mut s = fixture();
        let p = paths(&s, 1, 2, 2).pop().unwrap();
        assert!(score(&s, &p, u128::MAX, SCALE / 20).is_none());
        assert!(score(&s, &p, 1, SCALE / 20).is_none());
        Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap()).snapshot_at =
            Utc::now() + chrono::Duration::seconds(1);
        assert!(score(&s, &p, 10u128.pow(18), SCALE / 20).is_none());
        Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap()).snapshot_at =
            Utc::now() - chrono::Duration::seconds(21);
        assert!(score(&s, &p, 10u128.pow(18), SCALE / 20).is_none());
    }
    #[test]
    fn better_small_book_stays_eligible_and_emitted_plan_has_hop0_partition() {
        let mut s = fixture();
        let p = paths(&s, 1, 2, 2)[0].clone();
        let m = Arc::make_mut(s.mirrors.get_mut(&p[0].0).unwrap());
        m.bids
            .push(crate::db::queries::resting_orders::RestingOrderRow {
                pair_id: m.pair_id,
                order_id: 1,
                side: "bid".into(),
                price: "0.000000000002".parse().unwrap(),
                remaining: BigDecimal::from(1_000_000u128),
                owner: None,
                expires_at: None,
            });
        let amount = 10u128.pow(16);
        let pool = score(&s, &p, amount, SCALE / 20).unwrap();
        let book = select(&s, &[p], amount, SCALE / 20, true, 8).unwrap();
        assert!(book.output > pool.output);
        assert!(!book.operations[0]["terra_swap"]["hybrid"].is_null());
        assert!(book.operations[1]["terra_swap"]["hybrid"].is_null());
        let split = &book.operations[0]["terra_swap"]["hybrid"];
        assert_eq!(
            split["pool_input"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                + split["book_input"]
                    .as_str()
                    .unwrap()
                    .parse::<u128>()
                    .unwrap(),
            amount
        );
    }
    fn direct_book_fixture(
        reserve_0: u128,
        reserve_1: u128,
        escrow: u128,
    ) -> (Snapshot, Vec<route_paths::PathHop>) {
        let mut s = fixture();
        let pair = "direct-book-leg".to_string();
        let mut m = (*s.mirrors["pair7a"]).clone();
        m.asset_0_addr = "in18".into();
        m.asset_1_addr = "out18".into();
        m.reserve_0 = reserve_0;
        m.reserve_1 = reserve_1;
        m.freshness = if reserve_0 == 0 || reserve_1 == 0 {
            sim::MirrorFreshness::EmptyPool
        } else {
            sim::MirrorFreshness::Fresh
        };
        m.bids = vec![crate::db::queries::resting_orders::RestingOrderRow {
            pair_id: m.pair_id,
            order_id: 1,
            side: "bid".into(),
            price: BigDecimal::from(1),
            remaining: BigDecimal::from(escrow),
            owner: None,
            expires_at: None,
        }];
        s.mirrors.insert(pair.clone(), Arc::new(m));
        (s, vec![(pair, 1, 2)])
    }
    #[test]
    fn valid_book_escrow_can_deliver_more_than_the_pool_ask_reserve() {
        let unit = 10u128.pow(18);
        let input = 2 * unit;
        let (s, path) = direct_book_fixture(10 * unit, unit, input);
        // The AMM leg returns less than its reserve, while real maker escrow supplies
        // most of the output. Treating the combined output as an AMM drain is wrong.
        let mixed = score_with_book(&s, &path, input, SCALE / 20, unit, 8).unwrap();
        assert!(mixed.output > unit);
        assert!(
            mixed.hops[0]["pool_return_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                < unit
        );
        assert_eq!(mixed.hops[0]["pool_input_actual"], json!(unit.to_string()));
        let winner = select(&s, &[path], input, SCALE / 20, true, 8).unwrap();
        assert!(winner.output > mixed.output);
        assert_eq!(winner.hops[0]["pool_input_actual"], json!("0"));
        assert_eq!(
            winner.operations[0]["terra_swap"]["hybrid"]["book_input"],
            json!(input.to_string())
        );
        assert!(
            winner.hops[0]["min_return"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
    }
    #[test]
    fn fully_filled_pure_book_works_with_empty_pool_in_both_directions() {
        let unit = 10u128.pow(18);
        let input = 2 * unit;
        let (s, path) = direct_book_fixture(0, 0, input);
        let forward = score_with_book(&s, &path, input, SCALE / 20, input, 8).unwrap();
        assert!(forward.output > 0);
        assert_eq!(forward.hops[0]["pool_return_amount"], json!("0"));
        assert_eq!(forward.hops[0]["spread_amount"], json!("0"));
        let mut reverse = s.clone();
        let m = Arc::make_mut(reverse.mirrors.get_mut(&path[0].0).unwrap());
        m.asks = m.bids.clone();
        m.asks[0].side = "ask".into();
        m.bids.clear();
        let reverse_path = vec![(path[0].0.clone(), 2, 1)];
        let backward =
            score_with_book(&reverse, &reverse_path, input, SCALE / 20, input, 8).unwrap();
        assert_eq!(backward.output, forward.output);
        assert!(score(&s, &path, input, SCALE / 20).is_none());
    }
    #[test]
    fn empty_pool_book_remainders_expired_escrow_and_zero_fill_budget_fail_safely() {
        let unit = 10u128.pow(18);
        let input = 2 * unit;
        let (s, path) = direct_book_fixture(0, 0, unit);
        assert!(score_with_book(&s, &path, input, SCALE / 20, input, 8).is_none());
        assert!(matches!(
            sim::quote_local_leg(&s.mirrors[&path[0].0], "in18", input, input, 8),
            Err(sim::DbSimError::InsufficientLiquidity)
        ));
        let (mut filled, path) = direct_book_fixture(0, 0, input);
        assert!(score_with_book(&filled, &path, input, SCALE / 20, input, 0).is_none());
        assert!(score_with_book(&filled, &path, input, SCALE / 20, input / 2, 8).is_none());
        Arc::make_mut(filled.mirrors.get_mut(&path[0].0).unwrap()).bids[0].expires_at =
            Some(Utc::now().timestamp() - 1);
        assert!(score_with_book(&filled, &path, input, SCALE / 20, input, 8).is_none());
        Arc::make_mut(filled.mirrors.get_mut(&path[0].0).unwrap()).snapshot_at =
            Utc::now() - chrono::Duration::seconds(21);
        assert!(score_with_book(&filled, &path, input, SCALE / 20, input, 8).is_none());
    }
    #[test]
    fn cached_hooks_deduct_before_positive_floors_and_inverse_targets_delivered_net() {
        let mut s = fixture();
        let p = paths(&s, 1, 2, 2).pop().unwrap();
        let amount = 10u128.pow(18);
        let before = score(&s, &p, amount, SCALE / 20).unwrap();
        let cfg = json!({"recipient":"recipient","tax_token":"out18","tax_percentage_bps":333});
        s.hooks.insert(
            "pair7b".into(),
            super::super::local_hook_pricing::CachedHookPrices::from_observation(&[(
                "hook".into(),
                cfg,
            )])
            .unwrap(),
        );
        let after = score(&s, &p, amount, SCALE / 20).unwrap();
        assert_eq!(
            after.output,
            before.output - wide_floor(before.output, 333, 10_000).unwrap()
        );
        assert!(
            after.hops[1]["min_return"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        let required = inverse_pool_path(&s, &p, 100).unwrap();
        assert!(score(&s, &p, required, SCALE / 20).unwrap().output >= 100);
        assert!(score(&s, &p, required - 1, SCALE / 20).is_none_or(|c| c.output < 100));
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn fixed_snapshot_refresh_reuses_exact_amount_cache_without_reenrichment() {
        let s = fixture();
        initialize_for_test(
            "local-unit-cache".into(),
            s.pairs,
            s.addresses,
            s.mirrors,
            s.max_age_ms,
        );
        let q = request(10u128.pow(18));
        let first = solve_local("local-unit-cache", &q).await.unwrap().0;
        let hits = cache_counters().0;
        for _ in 0..8 {
            let next = solve_local("local-unit-cache", &q).await.unwrap().0;
            assert_eq!(
                next["estimated_amount_out_net"],
                first["estimated_amount_out_net"]
            );
            assert_eq!(next["router_operations"], first["router_operations"]);
            assert_eq!(next["slippage_percent"], first["slippage_percent"]);
        }
        assert!(cache_counters().0 >= hits + 8);
        let g = store().lock().unwrap();
        let snap = &g.snapshots["local-unit-cache"];
        assert_ne!(
            key("local-unit-cache", snap, &q, SCALE / 20),
            key(
                "local-unit-cache",
                snap,
                &request(10u128.pow(18) + 1),
                SCALE / 20
            )
        );
    }
    #[test]
    #[serial_test::serial]
    fn delayed_reserve_publication_cannot_restore_previous_admission_epoch() {
        let namespace = "1452-policy-publication-regression".to_owned();
        let publish_fixture = |s: Snapshot, expected| {
            publish(
                namespace.clone(),
                s.pairs,
                s.addresses,
                s.mirrors,
                s.pool_mirrors,
                s.max_age_ms,
                s.hooks,
                s.config_expiry_ms,
                s.config_expiries.clone(),
                s.pair_contexts.clone(),
                s.references,
                s.actors,
                s.discounted_pairs,
                s.policy_identity,
                s.taxes,
                expected,
            );
        };
        publish_fixture(fixture(), None);
        let old = (*store().lock().unwrap().snapshots[&namespace]).clone();
        let mut new = old.clone();
        // Policy publication is independent of clock changes or identical timestamps.
        new.actors.insert("known-wallet".into(), (0, true));
        publish_fixture(new.clone(), None);
        publish_fixture(old.clone(), Some(old.config_epoch));
        let current = store().lock().unwrap().snapshots[&namespace].clone();
        assert!(current.config_epoch > old.config_epoch);
        assert_eq!(current.config_expiry_ms, old.config_expiry_ms);
        assert_eq!(current.actors["known-wallet"], (0, true));
    }

    #[test]
    fn native_wrapper_costs_cannot_define_their_own_fair_reference_or_waive_99_guard() {
        use super::super::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
        let s = fixture();
        let credit = 10u128.pow(18);
        let q = request(credit);
        let selected = select(&s, &paths(&s, 1, 2, 2), credit, SCALE / 20, false, 8).unwrap();
        let mut original = request(credit * 10_000);
        original.token_in = "uluna".into();
        let mut context = NativeContext {
            request: original,
            wrapper: WrapperSnapshot {
                mapper: "mapper".into(),
                treasury: "treasury".into(),
                observed_at_ms: now_ms(),
                mappings: HashMap::from([(
                    "uluna".into(),
                    NativeMapping {
                        cw20: "in18".into(),
                        maximum: None,
                        used: 0,
                        tax_cap: None,
                    },
                )]),
                fee_wrap_bps: 9_999,
                fee_unwrap_bps: 0,
                tax_atomics: None,
                router_unwrap_supported: Some(false),
                mapper_floor_version: None,
                treasury_floor_version: None,
            },
            wrap: true,
            unwrap: false,
        };
        assert!(final_response(&s, &q, &selected, credit, SCALE / 20, 1, Some(&context)).is_none());
        context.wrapper.fee_wrap_bps = 200;
        let gross = context.wrapper.inverse_wrap(credit).unwrap();
        context.request.amount_in = Some(gross.to_string());
        let body =
            final_response(&s, &q, &selected, credit, SCALE / 20, 1, Some(&context)).unwrap();
        assert_eq!(body["spot_amount_out"], gross.to_string());
        assert!(
            body["slippage_percent"]
                .as_str()
                .unwrap()
                .parse::<f64>()
                .unwrap()
                > 2.0
        );
    }

    #[test]
    fn spread_is_checked_decimal_not_float_and_response_floors_are_positive() {
        assert_eq!(spread_atomics(Some("0.05")).unwrap(), SCALE / 20);
        assert_eq!(spread_atomics(Some("0")).unwrap(), 0);
        for bad in ["NaN", "1", "0.51", "-0.1", "0.0000000000000000001", "0.1e0"] {
            assert!(spread_atomics(Some(bad)).is_err());
        }
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn manual_split_post_prices_exact_controls_and_binds_cache_identity() {
        let mut s = fixture();
        let mut direct = s
            .pairs
            .iter()
            .find(|p| p.contract_address == "pair7a")
            .unwrap()
            .clone();
        direct.contract_address = "manual-direct".into();
        direct.asset_1_id = 2;
        let mut mirror = (*s.mirrors["pair7a"]).clone();
        mirror.asset_1_addr = "out18".into();
        mirror.reserve_1 *= 10u128.pow(12);
        s.mirrors
            .insert(direct.contract_address.clone(), Arc::new(mirror));
        s.pairs.push(direct);
        let path = paths(&s, 1, 2, 1).pop().unwrap();
        let amount = 1_000_000_000_000_000_000u128;
        initialize_for_test(
            "manual1452".into(),
            s.pairs.clone(),
            s.addresses.clone(),
            s.mirrors.clone(),
            20_000,
        );
        let mut q = request(amount);
        q.pool_only = Some(true);
        let declared = route_solver::HybridHopJson {
            pool_input: (amount / 2).to_string(),
            book_input: (amount / 2).to_string(),
            max_maker_fills: 17,
            book_start_hint: None,
        };
        let body = solve_local_with_plan("manual1452", &q, Some(&[Some(declared.clone())]))
            .await
            .unwrap()
            .0;
        assert_eq!(
            body["execution_metadata"]["policy"],
            "declared_hybrid_manual"
        );
        assert_eq!(body["order_economics"]["status"], "available");
        assert_eq!(
            body["order_economics"]["policy"],
            body["execution_metadata"]["policy"]
        );
        assert_eq!(body["hops"][0]["pair"], path[0].0);
        assert_eq!(
            body["router_operations"][0]["terra_swap"]["hybrid"]["book_input"],
            declared.book_input
        );
        assert_eq!(
            body["router_operations"][0]["terra_swap"]["hybrid"]["max_maker_fills"],
            17
        );
        // An empty book returns its unspent allocation to the pool exactly.
        assert_eq!(
            body["execution_metadata"]["hops"][0]["pool_input_actual"],
            amount.to_string()
        );
        let mut other = declared.clone();
        other.max_maker_fills = 18;
        let changed = solve_local_with_plan("manual1452", &q, Some(&[Some(other)]))
            .await
            .unwrap()
            .0;
        assert_eq!(
            changed["router_operations"][0]["terra_swap"]["hybrid"]["max_maker_fills"],
            18
        );
        let mut invalid = declared;
        invalid.pool_input = "1".into();
        assert!(
            solve_local_with_plan("manual1452", &q, Some(&[Some(invalid)]))
                .await
                .is_err()
        );
        assert!(
            solve_local_with_plan("manual1452", &q, Some(&[None, None, None, None]))
                .await
                .is_err()
        );
        assert!(!store()
            .lock()
            .unwrap()
            .jobs
            .iter()
            .any(|key| key.starts_with("manual1452")));
    }
    #[test]
    fn manual_later_hop_split_matches_exact_previous_delivered_amount() {
        let s = fixture();
        let path = paths(&s, 1, 2, 2)
            .into_iter()
            .find(|p| p.len() == 2 && p[0].0 == "pair7a")
            .unwrap();
        let amount = 1_000_000_000_000_000_000u128;
        let first = score(&s, &path, amount, SCALE / 20).unwrap();
        let next = first.hops[0]["return_amount"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap();
        let plan = [
            None,
            Some(route_solver::HybridHopJson {
                pool_input: (next / 2).to_string(),
                book_input: (next - next / 2).to_string(),
                max_maker_fills: 9,
                book_start_hint: None,
            }),
        ];
        let c = score_with_plan(&s, &path, amount, SCALE / 20, 0, 8, Some(&plan)).unwrap();
        assert!(c.operations[0]["terra_swap"]["hybrid"].is_null());
        assert_eq!(
            c.operations[1]["terra_swap"]["hybrid"]["max_maker_fills"],
            9
        );
        assert_eq!(c.output, first.output);
        assert!(score_with_plan(
            &s,
            &path,
            amount + 1_000_000_000_000_000,
            SCALE / 20,
            0,
            8,
            Some(&plan)
        )
        .is_none());
    }
    #[tokio::test]
    #[serial_test::serial]
    async fn issue1499_native_router_observation_loss_is_missing_and_positive_recovery_is_local() {
        use super::super::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
        let namespace = "1499-router-observation";
        let s = fixture();
        initialize_for_test(namespace.into(), s.pairs, s.addresses, s.mirrors, 20_000);
        let mut wrapper = WrapperSnapshot {
            mapper: "mapper".into(),
            treasury: "treasury".into(),
            observed_at_ms: now_ms(),
            mappings: HashMap::from([(
                "uluna".into(),
                NativeMapping {
                    cw20: "out18".into(),
                    maximum: None,
                    used: 0,
                    tax_cap: Some(0),
                },
            )]),
            fee_wrap_bps: 0,
            fee_unwrap_bps: 0,
            tax_atomics: Some(0),
            router_unwrap_supported: Some(true),
            mapper_floor_version: Some(1),
            treasury_floor_version: Some(1),
        };
        let original_clock = wrapper.observed_at_ms;
        let mut q = request(10u128.pow(18));
        q.token_out = "uluna".into();
        q.pool_only = Some(true);
        publish_wrapper(namespace.into(), wrapper.clone());
        let original = solve_local(namespace, &q).await.unwrap().0;
        wrapper.router_unwrap_supported = None;
        publish_wrapper(namespace.into(), wrapper.clone());
        let missing = solve_local(namespace, &q).await.unwrap_err();
        assert_eq!(missing.0, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            serde_json::from_str::<Value>(&missing.1).unwrap()["error"],
            "missing_observations"
        );
        wrapper.router_unwrap_supported = Some(false);
        publish_wrapper(namespace.into(), wrapper.clone());
        let unsupported = solve_local(namespace, &q).await.unwrap_err();
        assert_eq!(unsupported.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            serde_json::from_str::<Value>(&unsupported.1).unwrap()["error"],
            "unsupported_semantics"
        );
        wrapper.router_unwrap_supported = Some(true);
        publish_wrapper(namespace.into(), wrapper);
        let recovered = solve_local(namespace, &q).await.unwrap().0;
        assert_eq!(
            recovered["estimated_amount_out_net"],
            original["estimated_amount_out_net"]
        );
        assert_eq!(
            store().lock().unwrap().wrappers[namespace].observed_at_ms,
            original_clock
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn issue1505_observed_pause_revokes_cached_native_quotes_without_renewing_old_clocks() {
        use super::super::local_wrapper_quote::{
            NativeMapping, WrapperObservation, WrapperRevocation, WrapperSnapshot,
        };
        let namespace = "1505-pause-revocation";
        let s = fixture();
        initialize_for_test(namespace.into(), s.pairs, s.addresses, s.mirrors, 20_000);
        let wrapper = WrapperSnapshot {
            mapper: "mapper".into(),
            treasury: "treasury".into(),
            observed_at_ms: now_ms(),
            mappings: HashMap::from([(
                "uluna".into(),
                NativeMapping {
                    cw20: "out18".into(),
                    maximum: None,
                    used: 0,
                    tax_cap: Some(0),
                },
            )]),
            fee_wrap_bps: 0,
            fee_unwrap_bps: 0,
            tax_atomics: Some(0),
            router_unwrap_supported: Some(true),
            mapper_floor_version: Some(1),
            treasury_floor_version: Some(1),
        };
        let original_clock = wrapper.observed_at_ms;
        let mut q = request(10u128.pow(18));
        q.token_out = "uluna".into();
        q.pool_only = Some(true);
        for reason in [
            WrapperRevocation::MapperPaused,
            WrapperRevocation::TreasuryWrappingPaused,
        ] {
            assert!(apply_wrapper_observation(
                namespace.into(),
                WrapperObservation::Available(wrapper.clone())
            ));
            let original = solve_local(namespace, &q).await.unwrap().0;
            // Populate and then hit the exact native quote cache before revocation.
            assert_eq!(
                solve_local(namespace, &q).await.unwrap().0["estimated_amount_out_net"],
                original["estimated_amount_out_net"]
            );
            assert!(!apply_wrapper_observation(
                namespace.into(),
                WrapperObservation::Unavailable
            ));
            assert_eq!(
                store().lock().unwrap().wrappers[namespace].observed_at_ms,
                original_clock
            );
            assert!(solve_local(namespace, &q).await.is_ok());
            let reviewed = {
                let g = store().lock().unwrap();
                let mut reviewed = protocol_guards::derive(&g, namespace, &g.snapshots[namespace]);
                reviewed.generation = wrapper_generation(&reviewed.generation, &wrapper);
                reviewed
            };
            let native = NativeContext {
                request: q.clone(),
                wrapper: wrapper.clone(),
                wrap: false,
                unwrap: true,
            };
            assert!(!apply_wrapper_observation(
                namespace.into(),
                WrapperObservation::Revoked(reason)
            ));
            {
                let g = store().lock().unwrap();
                // Deterministic interleaving: the quote captured a valid wrapper,
                // then the independent producer revoked it before final review.
                assert!(!reviewed_snapshot_current(
                    &g,
                    namespace,
                    &reviewed,
                    Some(&native)
                ));
                assert_eq!(
                    invalidated_snapshot_error(&g, namespace, true).0,
                    StatusCode::UNPROCESSABLE_ENTITY
                );
                assert_eq!(
                    invalidated_snapshot_error(&g, namespace, false).0,
                    StatusCode::SERVICE_UNAVAILABLE
                );
            }
            assert!(!store().lock().unwrap().wrappers.contains_key(namespace));
            // The local consumer must reject even though its executable quote was cached.
            let missing = solve_local(namespace, &q).await.unwrap_err();
            assert_eq!(missing.0, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                serde_json::from_str::<Value>(&missing.1).unwrap()["error"],
                "unsupported_semantics"
            );
            assert!(!apply_wrapper_observation(
                namespace.into(),
                WrapperObservation::Unavailable
            ));
            assert_eq!(
                solve_local(namespace, &q).await.unwrap_err().0,
                StatusCode::UNPROCESSABLE_ENTITY
            );
            assert_eq!(
                progress_local(namespace, &q).unwrap_err().0,
                StatusCode::UNPROCESSABLE_ENTITY
            );
        }
        assert!(apply_wrapper_observation(
            namespace.into(),
            WrapperObservation::Available(wrapper)
        ));
        assert!(solve_local(namespace, &q).await.is_ok());
        assert_eq!(
            store().lock().unwrap().wrappers[namespace].observed_at_ms,
            original_clock
        );
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn zero_hop_wrapper_sizing_is_local_positive_and_explicitly_cannot_sign() {
        use super::super::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
        let namespace = "zero-wrapper1452";
        let s = fixture();
        initialize_for_test(namespace.into(), s.pairs, s.addresses, s.mirrors, 20_000);
        publish_wrapper(
            namespace.into(),
            WrapperSnapshot {
                mapper: "mapper".into(),
                treasury: "treasury".into(),
                observed_at_ms: now_ms(),
                mappings: HashMap::from([(
                    "uluna".into(),
                    NativeMapping {
                        cw20: "in18".into(),
                        maximum: None,
                        used: 0,
                        tax_cap: Some(100),
                    },
                )]),
                fee_wrap_bps: 200,
                fee_unwrap_bps: 50,
                tax_atomics: Some(SCALE / 100),
                router_unwrap_supported: Some(true),
                mapper_floor_version: None,
                treasury_floor_version: None,
            },
        );
        for (input, output) in [("uluna", "in18"), ("in18", "uluna")] {
            let mut q = request(10_000);
            q.token_in = input.into();
            q.token_out = output.into();
            let body = solve_local(namespace, &q).await.unwrap().0;
            assert_eq!(body["hops"], json!([]));
            assert_eq!(body["router_operations"], json!([]));
            assert_eq!(body["execution_metadata"]["execution_supported"], false);
            assert!(
                body["execution_metadata"]["minimum_receive"]
                    .as_str()
                    .unwrap()
                    .parse::<u128>()
                    .unwrap()
                    > 0
            );
            q.amount_in = None;
            q.amount_out = Some("9800".into());
            q.pool_only = Some(true);
            let inverse = solve_local(namespace, &q).await.unwrap().0;
            assert!(
                inverse["estimated_amount_out"]
                    .as_str()
                    .unwrap()
                    .parse::<u128>()
                    .unwrap()
                    >= 9800
            );
            assert_eq!(inverse["execution_metadata"]["execution_supported"], false);
        }
        let mut w = store().lock().unwrap().wrappers[namespace].clone();
        // Direct unwrap uses the mapper, so unrelated router readiness cannot
        // remove useful arithmetic or waive protocol capability admission.
        w.router_unwrap_supported = Some(false);
        w.mapper_floor_version = Some(1);
        publish_wrapper(namespace.into(), w.clone());
        let mut q = request(10_000);
        q.token_in = "in18".into();
        q.token_out = "uluna".into();
        let supported = solve_local(namespace, &q).await.unwrap().0;
        assert_eq!(supported["execution_metadata"]["execution_supported"], true);
        assert_eq!(
            supported["execution_metadata"]["wrapper"]["floor_protocol_version"],
            1
        );
        assert!(supported["execution_blocker"].is_null());
        q.token_in = "uluna".into();
        q.token_out = "in18".into();
        let missing_treasury = solve_local(namespace, &q).await.unwrap().0;
        assert_eq!(
            missing_treasury["execution_metadata"]["execution_supported"],
            false
        );
        w.treasury_floor_version = Some(1);
        publish_wrapper(namespace.into(), w.clone());
        let enabled = solve_local(namespace, &q).await.unwrap().0;
        assert_eq!(enabled["execution_metadata"]["execution_supported"], true);
        assert_ne!(
            enabled["execution_metadata"]["generation"],
            missing_treasury["execution_metadata"]["generation"]
        );
        w.mapper_floor_version = None;
        publish_wrapper(namespace.into(), w.clone());
        let withdrawn = solve_local(namespace, &q).await.unwrap().0;
        assert_eq!(
            withdrawn["execution_metadata"]["execution_supported"],
            false
        );
        assert!(withdrawn["execution_metadata"]["wrapper"]["floor_protocol_version"].is_null());
        assert_ne!(
            withdrawn["execution_metadata"]["generation"],
            enabled["execution_metadata"]["generation"]
        );
        w.mapper_floor_version = Some(1);
        w.observed_at_ms = now_ms() - 20_001;
        publish_wrapper(namespace.into(), w.clone());
        assert_eq!(
            solve_local(namespace, &q).await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        w.observed_at_ms = now_ms() + 60_000;
        publish_wrapper(namespace.into(), w.clone());
        assert_eq!(
            solve_local(namespace, &q).await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        w.observed_at_ms = now_ms();
        w.mappings.get_mut("uluna").unwrap().maximum = Some(9_999);
        publish_wrapper(namespace.into(), w.clone());
        assert_eq!(
            solve_local(namespace, &q).await.unwrap_err().0,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a fresh observed rate limit remains a protection failure"
        );
        w.mappings.get_mut("uluna").unwrap().maximum = None;
        publish_wrapper(namespace.into(), w);
        {
            let mut g = store().lock().unwrap();
            let mut missing = (*g.snapshots[namespace]).clone();
            missing.references.prices.clear();
            g.snapshots.insert(namespace.into(), Arc::new(missing));
        }
        assert_eq!(
            solve_local(namespace, &q).await.unwrap_err().0,
            StatusCode::SERVICE_UNAVAILABLE,
            "a missing independent reference is not offer-size protection"
        );
    }

    #[test]
    fn inverse_refines_when_mixed_decimal_pool_jumps_over_hook_qualified_residue() {
        let mut s = fixture();
        let m = Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap());
        m.asset_1_addr = "out18".into();
        m.reserve_0 = 1000;
        m.reserve_1 = 5000;
        m.fee_bps = 0;
        s.hooks.insert(
            "pair7a".into(),
            super::super::local_hook_pricing::CachedHookPrices::from_observation(&[
                (
                    "tax".into(),
                    json!({"recipient":"r","tax_token":"out18","tax_percentage_bps":2500}),
                ),
                (
                    "burn".into(),
                    json!({"burn_token":"out18","burn_percentage_bps":2500}),
                ),
            ])
            .unwrap(),
        );
        let path = vec![("pair7a".into(), 1, 2)];
        // First hook-qualified original3 is skipped by the pool: offer1 returns4,
        // whose two coincident floors deliver2. Next reachable offer2 returns9.
        assert_eq!(score(&s, &path, 1, SCALE / 2).unwrap().output, 2);
        let (required, candidate) = select_inverse(&s, &[path.clone()], 3, SCALE / 2).unwrap();
        assert_eq!(required, 2);
        assert_eq!(candidate.output, 5);
        assert!(score(&s, &path, required - 1, SCALE / 2).unwrap().output < 3);
    }

    #[test]
    fn composed_hook_inverse_refines_insufficient_candidate_and_beats_healthy_alternate() {
        let mut s = fixture();
        {
            let m = Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap());
            m.reserve_0 = 1000;
            m.reserve_1 = 6000;
            m.fee_bps = 0;
        }
        {
            let m = Arc::make_mut(s.mirrors.get_mut("pair7b").unwrap());
            m.reserve_0 = 1000;
            m.reserve_1 = 1002;
            m.fee_bps = 0;
        }
        s.hooks.insert(
            "pair7b".into(),
            super::super::local_hook_pricing::CachedHookPrices::from_observation(&[
                (
                    "tax".into(),
                    json!({"recipient":"r","tax_token":"out18","tax_percentage_bps":2500}),
                ),
                (
                    "burn".into(),
                    json!({"burn_token":"out18","burn_percentage_bps":2500}),
                ),
            ])
            .unwrap(),
        );
        let composed = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        // The downstream minimum offer4 delivers3, but the upstream minimum
        // input1 jumps to offer5, whose downstream coincident floors deliver2.
        let inverse = inverse_pool_path(&s, &composed, 3).unwrap();
        assert_eq!(inverse, 1);
        assert_eq!(score(&s, &composed, inverse, SCALE / 2).unwrap().output, 2);
        let (required, candidate) = select_inverse(&s, &[composed.clone()], 3, SCALE / 2).unwrap();
        assert_eq!(required, 2);
        assert!(candidate.output >= 3);
        assert!(
            score(&s, &composed, required - 1, SCALE / 2)
                .unwrap()
                .output
                < 3
        );
        let mut alternate = (*s.mirrors["pair7b"]).clone();
        alternate.asset_0_addr = "in18".into();
        s.mirrors
            .insert("healthy-direct".into(), Arc::new(alternate));
        let healthy = vec![("healthy-direct".into(), 1, 2)];
        let (required, candidate) =
            select_inverse(&s, &[composed.clone(), healthy], 3, SCALE / 2).unwrap();
        assert_eq!(candidate.path, composed);
        assert_eq!(required, 2);
        assert!(candidate.output >= 3);
    }
    #[test]
    fn recovery1464_publication_preserves_newer_complete_only_with_unchanged_binding() {
        let mut current = fixture();
        current.namespace = "publication-order-fee".into();
        current
            .pair_contexts
            .insert("pair7a".into(), "qualified".into());
        current
            .config_expiries
            .insert("pair7a".into(), now_ms() + 10_000);
        let mut pool = (*current.mirrors["pair7a"]).clone();
        pool.block_height = Some(11);
        pool.reserve_0 += 10;
        current.pool_mirrors.insert("pair7a".into(), Arc::new(pool));
        let mut delayed = current.clone();
        let delayed_pool = Arc::make_mut(delayed.pool_mirrors.get_mut("pair7a").unwrap());
        delayed_pool.block_height = Some(10);
        delayed_pool.reserve_0 /= 2;
        delayed
            .config_expiries
            .insert("pair7a".into(), now_ms() + 20_000);
        let old = Arc::make_mut(delayed.mirrors.get_mut("pair7a").unwrap());
        old.block_height = Some(9);
        old.snapshot_at -= chrono::Duration::seconds(1);
        old.reserve_0 /= 2;
        preserve_newer_mirrors(&mut delayed, &current);
        assert!(Arc::ptr_eq(
            &delayed.mirrors["pair7a"],
            &current.mirrors["pair7a"]
        ));
        assert!(Arc::ptr_eq(
            &delayed.pool_mirrors["pair7a"],
            &current.pool_mirrors["pair7a"]
        ));
        assert_eq!(delayed.mirrors["pair7a"].block_height, Some(10));
        assert_eq!(delayed.pool_mirrors["pair7a"].block_height, Some(11));
        assert_eq!(
            delayed.config_expiries["pair7a"],
            current.config_expiries["pair7a"]
        );
        let mut changed = current.clone();
        changed
            .pair_contexts
            .insert("pair7a".into(), "changed-policy".into());
        Arc::make_mut(changed.mirrors.get_mut("pair7a").unwrap()).block_height = Some(9);
        Arc::make_mut(changed.pool_mirrors.get_mut("pair7a").unwrap()).block_height = Some(10);
        preserve_newer_mirrors(&mut changed, &current);
        assert_eq!(changed.mirrors["pair7a"].block_height, Some(9));
        assert_eq!(changed.pool_mirrors["pair7a"].block_height, Some(10));
    }
    #[test]
    #[serial_test::serial]
    fn recovery1464_pool_companion_changes_cache_and_reviewed_generation() {
        let namespace = "pool-companion-generation";
        let publish_fixture = |s: Snapshot, expected| {
            publish(
                namespace.into(),
                s.pairs,
                s.addresses,
                s.mirrors,
                s.pool_mirrors,
                s.max_age_ms,
                s.hooks,
                s.config_expiry_ms,
                s.config_expiries,
                s.pair_contexts,
                s.references,
                s.actors,
                s.discounted_pairs,
                s.policy_identity,
                s.taxes,
                expected,
            );
        };
        let mut first = fixture();
        first
            .pool_mirrors
            .insert("pair7a".into(), first.mirrors["pair7a"].clone());
        publish_fixture(first, None);
        let (previous, reviewed) = {
            let g = store().lock().unwrap();
            let previous = g.snapshots[namespace].clone();
            let reviewed = protocol_guards::derive(&g, namespace, &previous);
            assert!(reviewed_snapshot_current(&g, namespace, &reviewed, None));
            (previous, reviewed)
        };
        let mut changed = (*previous).clone();
        // The original hybrid remains byte-for-byte unchanged; only the whole
        // complete pool advances in the same configuration epoch.
        let pool = Arc::make_mut(changed.pool_mirrors.get_mut("pair7a").unwrap());
        pool.reserve_0 += 1;
        pool.reserve_1 += 1;
        publish_fixture(changed, Some(previous.config_epoch));
        let g = store().lock().unwrap();
        let current = &g.snapshots[namespace];
        assert_eq!(current.config_epoch, previous.config_epoch);
        assert!(Arc::ptr_eq(
            &current.mirrors["pair7a"],
            &previous.mirrors["pair7a"]
        ));
        assert_ne!(current.generation, previous.generation);
        assert_ne!(
            key(namespace, current, &request(1), SCALE / 20),
            key(namespace, &previous, &request(1), SCALE / 20)
        );
        assert!(!reviewed_snapshot_current(&g, namespace, &reviewed, None));
    }
}
