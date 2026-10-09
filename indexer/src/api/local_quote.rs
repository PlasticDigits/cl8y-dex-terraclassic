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

use super::{db_orderbook_sim as sim, route_paths, route_solver, AppState};
use crate::db::queries::{assets, pairs};

const MAX_PAIRS: usize = 512;
const MAX_RESULTS: usize = 256;
const MAX_JOBS: usize = 4;
const CONFIG_TTL_MS: i64 = 180_000;
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
    mirrors: HashMap<String, sim::HopMirror>,
    max_age_ms: u64,
    hooks: HashMap<String, super::local_hook_pricing::CachedHookPrices>,
    config_expiry_ms: i64,
    config_epoch: u64,
    references: super::local_reference_quote::ReferencePrices,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
    policy_identity: String,
}
#[derive(Default)]
struct Store {
    skeletons: skeletons::SkeletonCache,
    snapshots: HashMap<String, Arc<Snapshot>>,
    wrappers: HashMap<String, super::local_wrapper_quote::WrapperSnapshot>,
    results: HashMap<String, Value>,
    jobs: HashSet<String>,
    hits: u64,
    started: u64,
    next_config_epoch: u64,
}
fn store() -> &'static Mutex<Store> {
    static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Store::default()))
}
fn unavailable() -> Error {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Local execution quote unavailable; refresh after indexed data recovers".into(),
    )
}
fn invalid(message: &str) -> Error {
    (StatusCode::BAD_REQUEST, message.into())
}
fn eligible_error() -> Error {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        "No eligible route within the selected protection".into(),
    )
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
            q.max_maker_fills,
            q.hybrid_optimize,
            q.discount_bps
        )
    )
}
fn publish(
    namespace: String,
    pairs: Vec<pairs::PairRow>,
    addresses: HashMap<i32, String>,
    mirrors: HashMap<String, sim::HopMirror>,
    max_age_ms: u64,
    hooks: HashMap<String, super::local_hook_pricing::CachedHookPrices>,
    config_expiry_ms: i64,
    references: super::local_reference_quote::ReferencePrices,
    actors: HashMap<String, (u16, bool)>,
    discounted_pairs: HashSet<String>,
    policy_identity: String,
    expected_config_epoch: Option<u64>,
) {
    let mut digest = Sha256::new();
    let mut ordered: Vec<_> = mirrors.iter().collect();
    ordered.sort_by_key(|(address, _)| *address);
    for (address, m) in ordered {
        digest.update(format!(
            "{address}|{}|{}|{}|{:?}|{}|{}|{}",
            m.asset_0_addr,
            m.asset_1_addr,
            m.fee_bps,
            m.block_height,
            m.snapshot_at,
            m.reserve_0,
            m.reserve_1
        ));
        for order in m.bids.iter().chain(&m.asks) {
            digest.update(format!(
                "{}|{}|{}|{}|{:?}",
                order.order_id, order.side, order.price, order.remaining, order.expires_at
            ));
        }
    }
    // Admission changes invalidate quotes even if reserve amounts did not change.
    let mut admitted: Vec<_> = pairs
        .iter()
        .map(|p| (&p.contract_address, p.asset_0_id, p.asset_1_id))
        .collect();
    admitted.sort();
    digest.update(format!("{admitted:?}"));
    let mut hook_keys = hooks.iter().collect::<Vec<_>>();
    hook_keys.sort_by_key(|(k, _)| *k);
    for (address, prices) in hook_keys {
        digest.update(format!("{address}|{:?}", prices.observation_identity()));
    }
    let mut refs = references.prices.iter().collect::<Vec<_>>();
    refs.sort_by_key(|(id, _)| **id);
    digest.update(format!("{refs:?}"));
    let mut actor_fields = actors.iter().collect::<Vec<_>>();
    actor_fields.sort_by_key(|(address, _)| *address);
    digest.update(format!("{actor_fields:?}"));
    let mut discounted = discounted_pairs.iter().collect::<Vec<_>>();
    discounted.sort();
    digest.update(format!("{discounted:?}"));
    digest.update(&policy_identity);
    let generation = format!("{:x}", digest.finalize());
    let ids = addresses.iter().map(|(id, a)| (a.clone(), *id)).collect();
    let mut snapshot = Snapshot {
        namespace: namespace.clone(),
        generation,
        pairs,
        addresses,
        ids,
        mirrors,
        max_age_ms,
        hooks,
        config_expiry_ms,
        config_epoch: expected_config_epoch.unwrap_or(0),
        references,
        actors,
        discounted_pairs,
        policy_identity,
    };
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
    if expected_config_epoch.is_none() {
        g.next_config_epoch = g.next_config_epoch.saturating_add(1);
        snapshot.config_epoch = g.next_config_epoch;
    }
    g.snapshots.insert(namespace, Arc::new(snapshot));
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
        mirrors,
        max_staleness_ms,
        HashMap::new(),
        now_ms() + CONFIG_TTL_MS,
        super::local_reference_quote::ReferencePrices::synthetic(addresses.keys().copied()),
        HashMap::new(),
        HashSet::new(),
        "synthetic".into(),
        None,
    );
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
}
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
    if path.is_empty() || input == 0 {
        return None;
    }
    let mut output = input;
    let mut hops = Vec::new();
    let mut operations = Vec::new();
    let mut oldest = now_ms();
    let mut block = None;
    for (hop_index, (pair, offer_id, ask_id)) in path.iter().enumerate() {
        let mirror = s.mirrors.get(pair)?;
        if sim::is_snapshot_stale(mirror.snapshot_at, s.max_age_ms)
            || mirror.freshness != sim::MirrorFreshness::Fresh
        {
            return None;
        }
        let offer_token = s.addresses.get(offer_id)?;
        let ask_token = s.addresses.get(ask_id)?;
        let (rin, rout) =
            if offer_token == &mirror.asset_0_addr && ask_token == &mirror.asset_1_addr {
                (mirror.reserve_0, mirror.reserve_1)
            } else if offer_token == &mirror.asset_1_addr && ask_token == &mirror.asset_0_addr {
                (mirror.reserve_1, mirror.reserve_0)
            } else {
                return None;
            };
        if rin == 0 || rout == 0 || mirror.fee_bps > 10_000 {
            return None;
        }
        let book_input = if hop_index == 0 { book_input } else { 0 };
        let leg =
            sim::quote_local_leg(mirror, offer_token, output, book_input, maker_fills).ok()?;
        let pool_net = leg.pool_return_amount;
        let commission = leg.commission_amount;
        let net = s.hooks.get(pair).map_or(Some(leg.return_amount), |hooks| {
            hooks.net_return(ask_token, leg.return_amount).ok()
        })?;
        let gross = pool_net.checked_add(commission)?;
        let market_spread = leg.spread_amount;
        let denominator = gross.checked_add(leg.book_return_net)?;
        let consumed = output.checked_sub(leg.pool_input_actual)?;
        let book_shortfall = if leg.pool_input_actual > 0 && consumed > 0 {
            wide_floor(pool_net, consumed, leg.pool_input_actual)?
                .saturating_sub(leg.book_return_net)
        } else {
            0
        };
        let spread_total = market_spread.min(gross).checked_add(book_shortfall)?;
        // Exact L9 pool+book denominator, separate from post-hook receive floors.
        if BigInt::from(spread_total) * BigInt::from(SCALE)
            > BigInt::from(denominator) * BigInt::from(spread)
        {
            return None;
        }
        if book_input > 0 && output > book_input && pool_net == 0 {
            return None;
        }
        let floor = wide_floor(net, SCALE - spread, SCALE)?.max(1);
        if net == 0 || floor == 0 || sim::hop_sim_implausible(output, rin, rout, net) {
            return None;
        }
        hops.push(json!({"pair":pair,"offer_token":offer_token,"ask_token":ask_token,"offer_amount":output.to_string(),"return_amount":net.to_string(),"pool_return_amount":pool_net.to_string(),"book_return_net":leg.book_return_net.to_string(),"pool_input_actual":leg.pool_input_actual.to_string(),"book_offer_consumed":consumed.to_string(),"book_input_actual":consumed.to_string(),"spread_amount":market_spread.to_string(),"commission_amount":commission.to_string(),"min_return":floor.to_string()}));
        operations.push(json!({"terra_swap":{"offer_asset_info":{"token":{"contract_addr":offer_token}},"ask_asset_info":{"token":{"contract_addr":ask_token}},"hybrid":if book_input==0{Value::Null}else{json!({"pool_input":(output-book_input).to_string(),"book_input":book_input.to_string(),"max_maker_fills":maker_fills,"book_start_hint":null})},"min_return":floor.to_string()}}));
        output = net;
        oldest = oldest.min(mirror.snapshot_at.timestamp_millis());
        if let Some(height) = mirror.block_height {
            block = Some(block.map_or(height, |old: i64| old.min(height)));
        }
    }
    let (fair, reference_at) =
        s.references
            .fair_output(path[0].1, path.last()?.2, input, Utc::now())?;
    let spot = fair;
    oldest = oldest.min(reference_at);
    if BigInt::from(fair.abs_diff(output)) * BigInt::from(100u8)
        >= BigInt::from(fair.max(output)) * BigInt::from(99u8)
    {
        return None;
    }
    Some(Candidate {
        path: path.to_vec(),
        output,
        spot,
        hops,
        operations,
        oldest,
        block,
    })
}
fn better(a: &Candidate, b: &Candidate) -> bool {
    a.output > b.output || (a.output == b.output && a.path < b.path)
}
fn paths(s: &Snapshot, start: i32, goal: i32, depth: usize) -> Vec<Vec<route_paths::PathHop>> {
    skeletons::paths(s, start, goal, depth)
}
fn select(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    amount: u128,
    spread: u128,
    book_allowed: bool,
    maker_fills: u32,
) -> Option<Candidate> {
    let splits = if book_allowed {
        vec![0, amount / 2, amount]
    } else {
        vec![0]
    };
    paths
        .iter()
        .flat_map(|p| {
            splits
                .iter()
                .filter_map(move |b| score_with_book(s, p, amount, spread, *b, maker_fills))
        })
        .reduce(|a, b| if better(&b, &a) { b } else { a })
}

fn inverse_pool_path(s: &Snapshot, path: &[route_paths::PathHop], target: u128) -> Option<u128> {
    let mut wanted = target;
    for (pair, offer, ask) in path.iter().rev() {
        // Unsupported hook-aware inverse is explicit; never target gross under a net label.
        if s.hooks
            .get(pair)
            .is_some_and(|h| h.has_deductions(&s.addresses[ask]))
        {
            return None;
        }
        let m = s.mirrors.get(pair)?;
        let (rin, rout) = if s.addresses[offer] == m.asset_0_addr {
            (m.reserve_0, m.reserve_1)
        } else {
            (m.reserve_1, m.reserve_0)
        };
        if m.fee_bps >= 10_000 || wanted >= rout {
            return None;
        }
        // Find the smallest gross amount whose integer fee leaves the requested net.
        let mut lo = wanted;
        let mut hi = rout.checked_sub(1)?;
        if hi - super::orderbook_sim::swap_fee_amount(hi, m.fee_bps) < wanted {
            return None;
        }
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if mid - super::orderbook_sim::swap_fee_amount(mid, m.fee_bps) >= wanted {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        let numerator = BigInt::from(rin) * BigInt::from(lo);
        let denominator = BigInt::from(rout.checked_sub(lo)?);
        wanted = ((numerator + &denominator - BigInt::from(1u8)) / denominator)
            .to_string()
            .parse()
            .ok()?;
    }
    Some(wanted)
}
fn select_inverse(
    s: &Snapshot,
    paths: &[Vec<route_paths::PathHop>],
    target: u128,
    spread: u128,
) -> Option<(u128, Candidate)> {
    let mut best = None;
    for path in paths.iter().take(64) {
        let Some(required) = inverse_pool_path(s, path, target) else {
            continue;
        };
        if let Some(c) = score(s, path, required, spread) {
            if c.output >= target
                && best
                    .as_ref()
                    .is_none_or(|(amount, _): &(u128, Candidate)| required < *amount)
            {
                best = Some((required, c));
            }
        }
    }
    best
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
    let mut tokens = vec![q.token_in.clone()];
    let route_hops: Vec<_> = c
        .path
        .iter()
        .map(|(pair, offer, ask)| {
            tokens.push(s.addresses[ask].clone());
            json!({"pair":pair,"offer_token":s.addresses[offer],"ask_token":s.addresses[ask]})
        })
        .collect();
    let spread_text = format!("0.{spread:018}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    let floor = c.hops.last().unwrap()["min_return"].as_str().unwrap();
    let mut expires = c
        .oldest
        .saturating_add(s.max_age_ms as i64)
        .min(s.config_expiry_ms);
    for id in [c.path[0].1, c.path.last().unwrap().2] {
        if let Some(price) = s.references.prices.get(&id) {
            expires = expires.min(price.as_of.timestamp_millis() + 20_000);
        }
    }
    if !c.operations[0]["terra_swap"]["hybrid"].is_null() {
        let mirror = &s.mirrors[&c.path[0].0];
        let book = if s.addresses[&c.path[0].1] == mirror.asset_0_addr {
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
    json!({"token_in":q.token_in,"token_out":q.token_out,"hops":route_hops,"intermediate_tokens":tokens,
        "quote_kind":if c.operations[0]["terra_swap"]["hybrid"].is_null(){"indexer_pool_db"}else{"indexer_hybrid_db"},"router_operations":c.operations,"estimated_amount_out":c.output.to_string(),"estimated_amount_out_net":c.output.to_string(),
        "tax_kind":"none","buy_tax_bps":0,"sell_tax_bps":0,"router_hops_tax":false,"solver_version":"local_v1","paths_considered":count,
        "optimality_scope":"Best net output among bounded eligible local pool paths","lcd_hybrid_queries":0,"db_hybrid_queries":0,"fidelity_check":"skipped",
        "snapshot_age_ms":now_ms().saturating_sub(c.oldest),"snapshot_block_height":c.block,"spot_amount_out":c.spot.to_string(),"slippage_percent":impact,
        "required_amount_in":q.amount_out.as_ref().map(|_|input.to_string()),"target_amount_out":q.amount_out,"sizing_mode":q.amount_out.as_ref().map(|_|"local_pool_exact_output"),
        "execution_metadata":{"version":1,"generation":s.generation,"input_amount":input.to_string(),"max_spread":spread_text,"trader":q.trader,"sender":q.sender,
            "observed_at_ms":c.oldest,"expires_at_ms":expires,"policy":if c.operations[0]["terra_swap"]["hybrid"].is_null(){"pool_only"}else{"declared_hybrid_hop0"},"hops":c.hops,"minimum_receive":floor}})
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
    h.update(format!("{}|{relevant:?}", s.generation));
    format!("{:x}", h.finalize())
}
fn wrapper_generation(market: &str, w: &super::local_wrapper_quote::WrapperSnapshot) -> String {
    let mut mappings = w.mappings.iter().collect::<Vec<_>>();
    mappings.sort_by_key(|(k, _)| *k);
    let mut hash = Sha256::new();
    hash.update(format!(
        "{market}|{}|{}|{}|{}|{}|{:?}|{}|{mappings:?}",
        w.mapper,
        w.treasury,
        w.observed_at_ms,
        w.fee_wrap_bps,
        w.fee_unwrap_bps,
        w.tax_atomics,
        w.router_unwrap_supported
    ));
    format!("{:x}", hash.finalize())
}
fn publish_wrapper(namespace: String, w: super::local_wrapper_quote::WrapperSnapshot) {
    let mut g = store().lock().unwrap();
    if g.wrappers.len() < 32 || g.wrappers.contains_key(&namespace) {
        g.wrappers.insert(namespace, w);
    }
}
#[derive(Clone)]
struct NativeContext {
    request: route_solver::SolveRouteParams,
    wrapper: super::local_wrapper_quote::WrapperSnapshot,
    wrap: bool,
    unwrap: bool,
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
        let floor = route_net_floor
            .max(wide_floor(output, SCALE - spread, SCALE)?)
            .max(
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
        .min(n.wrapper.observed_at_ms + 20_000));
    body["execution_metadata"]["wrapper"] = json!({"input_token":n.request.token_in,"output_token":n.request.token_out,"input_amount":gross.to_string(),"route_input_amount":input.to_string(),"route_output_amount":c.output.to_string(),"minimum_receive_base":minimum_base.to_string(),"wrap_input":n.wrap,"unwrap_output":n.unwrap,"treasury":n.wrapper.treasury,"mapper":n.wrapper.mapper});
    Some(body)
}
struct JobGuard(String);
impl Drop for JobGuard {
    fn drop(&mut self) {
        if let Ok(mut g) = store().lock() {
            g.jobs.remove(&self.0);
        }
    }
}
/// Request handler. Deliberately cannot access LCD, SQL or recursively solved reference prices.
pub async fn solve(
    state: &AppState,
    q: &route_solver::SolveRouteParams,
) -> Result<Json<Value>, Error> {
    let namespace = state.lcd.chain_cache_namespace();
    solve_local(&namespace, q).await
}
fn topology(namespace: &str, q: &route_solver::SolveRouteParams) -> Result<Json<Value>, Error> {
    let g = store().lock().unwrap();
    let s = g.snapshots.get(namespace).ok_or_else(unavailable)?;
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
    let path = paths(s, start, goal, 4)
        .into_iter()
        .next()
        .ok_or_else(eligible_error)?;
    let hops=path.iter().map(|(pair,offer,ask)|json!({"pair":pair,"offer_token":s.addresses[offer],"ask_token":s.addresses[ask]})).collect::<Vec<_>>();
    let ops=path.iter().map(|(_,offer,ask)|json!({"terra_swap":{"offer_asset_info":{"token":{"contract_addr":s.addresses[offer]}},"ask_asset_info":{"token":{"contract_addr":s.addresses[ask]}},"hybrid":null}})).collect::<Vec<_>>();
    let mut tokens = vec![q.token_in.clone()];
    tokens.extend(path.iter().map(|(_, _, ask)| s.addresses[ask].clone()));
    Ok(Json(
        json!({"token_in":q.token_in,"token_out":q.token_out,"hops":hops,"intermediate_tokens":tokens,"router_operations":ops,"quote_kind":"indexer_route_only","estimated_amount_out":null,"solver_version":"local_v1"}),
    ))
}
async fn solve_local(
    namespace: &str,
    request: &route_solver::SolveRouteParams,
) -> Result<Json<Value>, Error> {
    if request.amount_in.is_none() && request.amount_out.is_none() {
        return topology(namespace, request);
    }
    let mut q = request.clone();
    let wrap = matches!(q.token_in.as_str(), "uluna" | "uusd");
    let unwrap = matches!(q.token_out.as_str(), "uluna" | "uusd");
    let native = if wrap || unwrap {
        let w = store()
            .lock()
            .unwrap()
            .wrappers
            .get(namespace)
            .cloned()
            .ok_or_else(unavailable)?;
        if wrap {
            let mapping = w.mappings.get(&q.token_in).ok_or_else(unavailable)?;
            if let Some(raw) = q.amount_in.as_ref() {
                let amount = raw
                    .parse::<u128>()
                    .map_err(|_| invalid("Invalid native offer"))?;
                let (_, credit) = w
                    .wrap_input(&q.token_in, amount, now_ms())
                    .ok_or_else(eligible_error)?;
                q.amount_in = Some(credit.to_string());
            }
            q.token_in = mapping.cw20.clone();
        }
        if unwrap {
            let mapping = w.mappings.get(&q.token_out).ok_or_else(unavailable)?;
            if let Some(raw) = q.amount_out.as_ref() {
                let target = raw
                    .parse::<u128>()
                    .map_err(|_| invalid("Invalid native target"))?;
                q.amount_out = Some(
                    w.inverse_unwrap(&q.token_out, target)
                        .ok_or_else(eligible_error)?
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
    let mut snapshot = store()
        .lock()
        .unwrap()
        .snapshots
        .get(namespace)
        .cloned()
        .ok_or_else(unavailable)?;
    let mut derived = (*snapshot).clone();
    derived.generation = admission_generation(&derived);
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
                "Trading is unavailable for this wallet".into(),
            ));
        }
    }
    let subject = request.trader.as_deref().or(request.sender.as_deref());
    let discount =
        if request.trader.is_some() && request.sender.is_some() && request.trader != request.sender
        {
            0
        } else {
            subject
                .and_then(|a| derived.actors.get(a))
                .map_or(0, |(bps, _)| *bps)
        };
    for (address, mirror) in derived.mirrors.iter_mut() {
        if !derived.discounted_pairs.contains(address) {
            continue;
        }
        mirror.fee_bps = sim::effective_fee_bps_from_discount_bps(mirror.fee_bps, discount);
    }
    snapshot = Arc::new(derived);
    if let Some(n) = native.as_ref() {
        let mut derived = (*snapshot).clone();
        derived.generation = wrapper_generation(&derived.generation, &n.wrapper);
        snapshot = Arc::new(derived);
    }
    if snapshot.config_expiry_ms <= now_ms() {
        return Err(unavailable());
    }
    let start = *snapshot
        .ids
        .get(q.token_in.trim())
        .ok_or_else(|| invalid("token_in has no eligible cached identity"))?;
    let goal = *snapshot
        .ids
        .get(q.token_out.trim())
        .ok_or_else(|| invalid("token_out has no eligible cached identity"))?;
    let ck = key(namespace, &snapshot, request, spread);
    {
        let mut g = store().lock().unwrap();
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
            return Ok(Json(body));
        }
    }
    let book_allowed =
        q.pool_only != Some(true) && q.hybrid_optimize != Some(false) && q.amount_out.is_none();
    let maker_fills = crate::hybrid_limits::clamp_max_maker_fills(q.max_maker_fills.unwrap_or(8));
    let mut initial = paths(&snapshot, start, goal, 2);
    // An eligible longer baseline is prepared locally when endpoints have no short corridor.
    if initial.is_empty()
        || q.amount_in
            .as_ref()
            .and_then(|a| a.parse().ok())
            .is_some_and(|amount| {
                select(
                    &snapshot,
                    &initial,
                    amount,
                    spread,
                    book_allowed,
                    maker_fills,
                )
                .is_none()
            })
    {
        initial = paths(&snapshot, start, goal, 4);
    }
    let mut considered = initial.len();
    let (amount, candidate) = if let Some(raw) = q.amount_in.as_ref() {
        let amount = raw
            .parse::<u128>()
            .ok()
            .filter(|a| *a > 0)
            .ok_or_else(|| invalid("amount_in must be a positive Uint128"))?;
        (
            amount,
            select(
                &snapshot,
                &initial,
                amount,
                spread,
                book_allowed,
                maker_fills,
            )
            .ok_or_else(eligible_error)?,
        )
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
        let mut best = select_inverse(&snapshot, &initial, target, spread);
        if best.is_none() && initial.iter().all(|p| p.len() <= 2) {
            let longer = paths(&snapshot, start, goal, 4)
                .into_iter()
                .filter(|p| p.len() > 2)
                .collect::<Vec<_>>();
            considered += longer.len().min(64);
            best = select_inverse(&snapshot, &longer, target, spread);
        }
        best.ok_or_else(eligible_error)?
    };
    let body = final_response(
        &snapshot,
        q,
        &candidate,
        amount,
        spread,
        considered,
        native.as_ref(),
    )
    .ok_or_else(eligible_error)?;
    {
        let mut g = store().lock().unwrap();
        if g.results.len() >= MAX_RESULTS {
            g.results.clear();
        }
        g.results.insert(ck.clone(), body.clone());
        if q.amount_in.is_some() && g.jobs.len() < MAX_JOBS && g.jobs.insert(ck.clone()) {
            g.started += 1;
            let q = q.clone();
            let s = snapshot.clone();
            let baseline = candidate.clone();
            let native = native.clone();
            tokio::task::spawn_blocking(move || {
                let _job = JobGuard(ck.clone());
                let all = paths(&s, start, goal, 4);
                let improved = select(&s, &all, amount, spread, book_allowed, maker_fills)
                    .filter(|c| c.output.saturating_sub(baseline.output) > baseline.output / 1000);
                let mut g = store().lock().unwrap();
                if let Some(c) = improved {
                    if g.results.contains_key(&ck) {
                        if let Some(body) =
                            final_response(&s, &q, &c, amount, spread, all.len(), native.as_ref())
                        {
                            g.results.insert(ck.clone(), body);
                        }
                    }
                }
                g.jobs.remove(&ck);
            });
        }
    }
    Ok(Json(body))
}

pub fn progress(
    state: &AppState,
    q: &route_solver::SolveRouteParams,
) -> Result<super::route_solve_progress::SolveProgress, Error> {
    let ns = state.lcd.chain_cache_namespace();
    let g = store().lock().unwrap();
    let mut s = (**g.snapshots.get(&ns).ok_or_else(unavailable)?).clone();
    s.generation = admission_generation(&s);
    if matches!(q.token_in.as_str(), "uluna" | "uusd")
        || matches!(q.token_out.as_str(), "uluna" | "uusd")
    {
        let w = g.wrappers.get(&ns).ok_or_else(unavailable)?;
        s.generation = wrapper_generation(&s.generation, w);
    }
    let ck = key(&ns, &s, q, spread_atomics(q.max_spread.as_deref())?);
    let searching = g.jobs.contains(&ck);
    let cached = g.results.contains_key(&ck);
    Ok(super::route_solve_progress::SolveProgress {
        stage: if searching {
            "evaluating"
        } else if cached {
            "cached"
        } else {
            "idle"
        }
        .into(),
        done: u32::from(!searching),
        total: 1,
        label: if searching {
            "Improving local route"
        } else {
            "Local route ready"
        }
        .into(),
        cache_hit: cached,
        updated_at_ms: now_ms().max(0) as u64,
    })
}

mod ingestion;
mod skeletons;
pub use ingestion::{
    cache_counters, ingest_once, refresh_reserves, refresh_wrapper, start_ingestion,
};

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
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
                    sim::HopMirror {
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
                    },
                );
            }
        }
        Snapshot {
            namespace: "unit1452".into(),
            generation: "fixture".into(),
            config_epoch: 0,
            ids: addresses.iter().map(|(id, a)| (a.clone(), *id)).collect(),
            addresses: addresses.clone(),
            pairs,
            mirrors,
            max_age_ms: 20_000,
            hooks: HashMap::new(),
            config_expiry_ms: now_ms() + 180_000,
            references: super::super::local_reference_quote::ReferencePrices::synthetic(
                addresses.keys().copied(),
            ),
            actors: HashMap::new(),
            discounted_pairs: HashSet::new(),
            policy_identity: "synthetic".into(),
        }
    }
    fn request(amount: u128) -> route_solver::SolveRouteParams {
        route_solver::SolveRouteParams {
            max_spread: Some("0.05".into()),
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
    async fn inverse_uses_eligible_longer_baseline_when_all_short_targets_fail_protection() {
        let mut s = fixture();
        for (name, m) in s.mirrors.iter_mut() {
            if name.starts_with("pair7") {
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
                sim::HopMirror {
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
                },
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
        s.mirrors.get_mut("pair7a").unwrap().snapshot_at =
            Utc::now() + chrono::Duration::seconds(1);
        assert!(score(&s, &p, 10u128.pow(18), SCALE / 20).is_none());
        s.mirrors.get_mut("pair7a").unwrap().snapshot_at =
            Utc::now() - chrono::Duration::seconds(21);
        assert!(score(&s, &p, 10u128.pow(18), SCALE / 20).is_none());
    }
    #[test]
    fn better_small_book_stays_eligible_and_emitted_plan_has_hop0_partition() {
        let mut s = fixture();
        let p = paths(&s, 1, 2, 2)[0].clone();
        let m = s.mirrors.get_mut(&p[0].0).unwrap();
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
    #[test]
    fn cached_hooks_deduct_before_positive_floors_and_inverse_fails_explicitly() {
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
        assert!(inverse_pool_path(&s, &p, 100).is_none());
    }
    #[tokio::test]
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
    fn delayed_reserve_publication_cannot_restore_previous_admission_epoch() {
        let namespace = "1452-policy-publication-regression".to_owned();
        let publish_fixture = |s: Snapshot, expected| {
            publish(
                namespace.clone(),
                s.pairs,
                s.addresses,
                s.mirrors,
                s.max_age_ms,
                s.hooks,
                s.config_expiry_ms,
                s.references,
                s.actors,
                s.discounted_pairs,
                s.policy_identity,
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
                router_unwrap_supported: false,
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
}
