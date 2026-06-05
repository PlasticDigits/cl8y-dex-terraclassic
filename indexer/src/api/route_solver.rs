//! Multihop route discovery and optional LCD simulation.
//!
//! - `GET /api/v1/route/solve`: **global best execution** when `amount_in` is set (top-K paths + joint hybrid, max **4 hops**; GitLab #209, #323).
//!   Use `pool_only=true` for legacy pool-only ops (max **4 hops**). Without `amount_in`, returns route discovery only.
//! - `GET /api/v1/route/solve/best`: **retail / Vyntrex best-execution alias** — same engine as default GET with `amount_in` (GitLab #189).
//! - `POST /api/v1/route/solve`: BFS discovery (max **4 hops**), optional `hybrid_by_hop` merged into ops.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use sqlx::PgPool;

use crate::api::hybrid_route_opt;
use crate::api::internal_err;
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};

pub use hybrid_route_opt::HybridHopJson;

const ROUTE_CACHE_TTL: Duration = Duration::from_secs(12);
const ROUTE_CACHE_MAX_ENTRIES: usize = 512;
/// Default GET hop cap (hybrid-aware routing per ADR 0001 / GitLab #191).
pub(crate) const GET_DEFAULT_MAX_HOPS: usize = 4;
/// Legacy pool-only GET escape hatch (`pool_only=true`).
const GET_POOL_ONLY_MAX_HOPS: usize = 4;
/// Coarse bucketing for hybrid GET cache keys (reduces LCD load).
const AMOUNT_CACHE_BUCKET: u128 = 1_000_000;

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct SolveRouteParams {
    /// CW20 contract address (must match indexed `assets.contract_address`). See [route-solver glossary](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/route-solver.md#glossary).
    pub token_in: String,
    /// CW20 contract address for the output asset (indexed `assets.contract_address`).
    pub token_out: String,
    /// Raw integer offer amount (optional). When set on GET (and `pool_only` is not true), runs **global_v1** best execution: top-5 paths, joint hybrid optimization, max **4 hops**. Required for `/route/solve/best`.
    pub amount_in: Option<String>,
    /// Deprecated alias: `hybrid_optimize=false` or `pool_only=true` for pool-only routing.
    #[serde(default)]
    pub hybrid_optimize: Option<bool>,
    /// Pool-only escape hatch: first BFS path, `hybrid: null` on every hop, max **4 hops** — skips the global optimizer. See `docs/route-solver.md`.
    #[serde(default)]
    pub pool_only: Option<bool>,
    /// Per-hop `max_maker_fills` for hybrid GET optimization (default **8**); caps limit-book matches per hop.
    #[serde(default)]
    pub max_maker_fills: Option<u32>,
    /// Optional beneficiary `terra1` wallet for CL8Y fee-tier discounted LCD quotes (GitLab #245). Omit for full-fee quotes. Cache keys include resolved `discount_tier`.
    #[serde(default)]
    pub trader: Option<String>,
    /// Optional CW20 sender `terra1` address when it differs from `trader` (trusted router execute path).
    #[serde(default)]
    pub sender: Option<String>,
}

/// JSON body for [`solve_route_post`]. Uses **first BFS path** (max **4 hops**); does **not** run `global_v1`. See `docs/route-solver.md#api-matrix-get-vs-post`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SolveRoutePostBody {
    pub token_in: String,
    pub token_out: String,
    /// Raw integer offer amount for optional router `simulate_swap_operations`.
    pub amount_in: Option<String>,
    /// Integrator override: one entry per hop after BFS. `null` = pool-only for that hop. Length must equal hop count when present.
    #[serde(default)]
    pub hybrid_by_hop: Option<Vec<Option<HybridHopJson>>>,
    /// Optional beneficiary `terra1` wallet for CL8Y fee-tier discounted LCD quotes (GitLab #245).
    #[serde(default)]
    pub trader: Option<String>,
    /// Optional CW20 sender `terra1` when it differs from `trader`.
    #[serde(default)]
    pub sender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteQuoteKind {
    /// Topology only — no `estimated_amount_out` (missing `ROUTER_ADDRESS` and/or `amount_in`, or router sim failed). Documented in `docs/route-solver.md#glossary`.
    IndexerRouteOnly,
    /// Pool-only router operations (`hybrid: null`); LCD `simulate_swap_operations` when configured.
    IndexerPoolLcd,
    /// At least one hop has a non-zero `book_input` after server hybrid optimization (`global_v1` GET).
    IndexerHybridLcd,
    /// Hybrid grid failed on at least one hop; degraded to pool-only `HybridSimulation` (`book_input: 0`). See `hybrid_notes`.
    IndexerHybridLcdDegraded,
    /// Pool-only ops; grid priced from Postgres mirror (`global_v2`).
    IndexerPoolDb,
    /// Hybrid legs from indexed mirror (`global_v2`).
    IndexerHybridDb,
    /// Mirror/grid degraded or fidelity drift rejected optimistic DB output.
    IndexerHybridDbDegraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FidelityCheck {
    Passed,
    Drift,
    #[default]
    Skipped,
}

impl FidelityCheck {
    pub fn as_str(self) -> &'static str {
        match self {
            FidelityCheck::Passed => "passed",
            FidelityCheck::Drift => "drift",
            FidelityCheck::Skipped => "skipped",
        }
    }
}

pub(crate) struct ResolvedRoute {
    pub token_in: String,
    pub token_out: String,
    pub hops: Vec<RouteHop>,
    pub ops: Vec<serde_json::Value>,
}

pub(crate) fn build_id_to_addr_map(
    all: &[assets::AssetRow],
) -> (HashMap<i32, String>, HashMap<String, i32>) {
    let mut id_to_addr = HashMap::new();
    let mut addr_to_id = HashMap::new();
    for a in all {
        if let Some(addr) = a.contract_address.as_ref().filter(|s| !s.is_empty()) {
            id_to_addr.insert(a.id, addr.clone());
            addr_to_id.insert(addr.clone(), a.id);
            addr_to_id.insert(addr.to_lowercase(), a.id);
        }
    }
    (id_to_addr, addr_to_id)
}

pub(crate) fn resolve_id(addr_to_id: &HashMap<String, i32>, token: &str) -> Option<i32> {
    let t = token.trim();
    if let Some(id) = addr_to_id.get(t) {
        return Some(*id);
    }
    addr_to_id.get(&t.to_lowercase()).copied()
}

fn asset_info_json(addr: &str) -> serde_json::Value {
    json!({ "token": { "contract_addr": addr } })
}

/// BFS by hop count, max `max_hops` edges (matches router `MAX_HOPS`).
fn find_path(
    start: i32,
    goal: i32,
    pair_rows: &[db_pairs::PairRow],
    max_hops: usize,
) -> Option<Vec<(String, i32, i32)>> {
    if start == goal {
        return Some(vec![]);
    }

    let mut adj: HashMap<i32, Vec<(i32, String)>> = HashMap::new();
    for p in pair_rows {
        let a0 = p.asset_0_id;
        let a1 = p.asset_1_id;
        adj.entry(a0)
            .or_default()
            .push((a1, p.contract_address.clone()));
        adj.entry(a1)
            .or_default()
            .push((a0, p.contract_address.clone()));
    }

    let mut q = VecDeque::new();
    let mut prev: HashMap<i32, (i32, String)> = HashMap::new();
    q.push_back(start);
    prev.insert(start, (start, String::new()));

    while let Some(u) = q.pop_front() {
        if u == goal {
            break;
        }
        let hops = hop_count(&prev, start, u);
        if hops >= max_hops {
            continue;
        }
        for (v, pair) in adj.get(&u).into_iter().flatten() {
            if prev.contains_key(v) {
                continue;
            }
            prev.insert(*v, (u, pair.clone()));
            q.push_back(*v);
        }
    }

    if !prev.contains_key(&goal) {
        return None;
    }

    let mut out = Vec::new();
    let mut cur = goal;
    while cur != start {
        let (p, pair_addr) = prev.get(&cur)?;
        let from = *p;
        out.push((pair_addr.clone(), from, cur));
        cur = from;
    }
    out.reverse();
    Some(out)
}

fn hop_count(prev: &HashMap<i32, (i32, String)>, start: i32, mut u: i32) -> usize {
    let mut n = 0;
    while u != start && n < 8 {
        if let Some((p, _)) = prev.get(&u) {
            u = *p;
            n += 1;
        } else {
            break;
        }
    }
    n
}

#[derive(Debug, Serialize, ToSchema, Clone)]
pub struct RouteHop {
    pub pair: String,
    pub offer_token: String,
    pub ask_token: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RouteSolveResponse {
    pub token_in: String,
    pub token_out: String,
    /// Ordered swap legs: pair contract plus offer/ask CW20 per hop.
    pub hops: Vec<RouteHop>,
    /// Full token path: `token_in`, then each hop's `ask_token` (ends at `token_out`).
    pub intermediate_tokens: Vec<String>,
    /// How the quote was produced — see `docs/route-solver.md#glossary` (`indexer_route_only`, `indexer_pool_lcd`, `indexer_hybrid_lcd`, `indexer_hybrid_lcd_degraded`).
    pub quote_kind: RouteQuoteKind,
    /// Advisory notes: search bounds, degradation, path/sim counts, LCD snapshot liability. Present on `global_v1` GET.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_notes: Option<String>,
    /// Router `ExecuteSwapOperations` operations (JSON). `terra_swap.hybrid` is `null` for pool-only hops.
    #[schema(value_type = Vec<Object>)]
    pub router_operations: Vec<serde_json::Value>,
    /// LCD router `simulate_swap_operations` output when `amount_in` and `ROUTER_ADDRESS` are set — **not** a guaranteed fill.
    pub estimated_amount_out: Option<String>,
    /// Solver generation label on global best-execution GET (shipped: `global_v1`). Absent on discovery-only GET and POST.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub solver_version: Option<String>,
    /// Simple paths evaluated (≤ `MAX_PATH_CANDIDATES` = 5). Present on `global_v1` GET only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths_considered: Option<u32>,
    /// Human-readable optimality bound — not global optimum over all paths. Matches `best_execution::OPTIMALITY_SCOPE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimality_scope: Option<String>,
    /// Approximate pair-level `HybridSimulation` LCD calls during optimization. Upper bound documented as `LCD_HYBRID_SIM_BUDGET`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lcd_hybrid_queries: Option<u32>,
    /// Postgres mirror hybrid grid evaluations (`global_v2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_hybrid_queries: Option<u32>,
    /// Router sim vs mirror grid drift guard (#319).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fidelity_check: Option<FidelityCheck>,
    /// Reserved: max chain lag vs mirror `block_height` (future).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror_max_block_lag: Option<i64>,
    /// True when path search was truncated by the concurrency cap (#324).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_truncated: Option<bool>,
}

pub(crate) fn build_intermediate_tokens(resolved: &ResolvedRoute) -> Vec<String> {
    let mut v = vec![resolved.token_in.clone()];
    for h in &resolved.hops {
        v.push(h.ask_token.clone());
    }
    v
}

pub(crate) fn build_hops_and_ops(
    hops_raw: &[(String, i32, i32)],
    id_to_addr: &HashMap<i32, String>,
) -> Result<(Vec<RouteHop>, Vec<serde_json::Value>), (StatusCode, String)> {
    let mut hops: Vec<RouteHop> = Vec::new();
    let mut ops: Vec<serde_json::Value> = Vec::new();

    for (pair_addr, from_id, to_id) in hops_raw {
        let offer_addr = id_to_addr.get(from_id).ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "missing asset address".to_string(),
            )
        })?;
        let ask_addr = id_to_addr.get(to_id).ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "missing asset address".to_string(),
            )
        })?;
        hops.push(RouteHop {
            pair: pair_addr.clone(),
            offer_token: offer_addr.clone(),
            ask_token: ask_addr.clone(),
        });
        ops.push(json!({
            "terra_swap": {
                "offer_asset_info": asset_info_json(offer_addr),
                "ask_asset_info": asset_info_json(ask_addr),
                "hybrid": null
            }
        }));
    }
    Ok((hops, ops))
}

pub(crate) fn apply_hybrid_by_hop(
    mut ops: Vec<serde_json::Value>,
    hybrid_by_hop: &[Option<HybridHopJson>],
) -> Result<Vec<serde_json::Value>, (StatusCode, String)> {
    if hybrid_by_hop.len() != ops.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "hybrid_by_hop length {} does not match hop count {}",
                hybrid_by_hop.len(),
                ops.len()
            ),
        ));
    }
    for (op, maybe_h) in ops.iter_mut().zip(hybrid_by_hop.iter()) {
        let Some(h) = maybe_h else {
            continue;
        };
        let terra = op
            .get_mut("terra_swap")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "invalid router operation shape".to_string(),
                )
            })?;
        terra.insert(
            "hybrid".to_string(),
            json!({
                "pool_input": h.pool_input,
                "book_input": h.book_input,
                "max_maker_fills": h.max_maker_fills,
                "book_start_hint": h.book_start_hint,
            }),
        );
    }
    Ok(ops)
}

async fn resolve_route_with_max_hops(
    pool: &PgPool,
    token_in: &str,
    token_out: &str,
    max_hops: usize,
) -> Result<ResolvedRoute, (StatusCode, String)> {
    let all_assets = assets::get_all_assets(pool).await.map_err(internal_err)?;
    let pair_rows = db_pairs::get_all_pairs(pool).await.map_err(internal_err)?;

    let (id_to_addr, addr_to_id) = build_id_to_addr_map(&all_assets);

    let start = resolve_id(&addr_to_id, token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".to_string(),
        )
    })?;
    let goal = resolve_id(&addr_to_id, token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".to_string(),
        )
    })?;

    let hops_raw = find_path(start, goal, &pair_rows, max_hops).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("no route within {} hops", max_hops),
        )
    })?;

    let (hops, ops) = build_hops_and_ops(&hops_raw, &id_to_addr)?;

    Ok(ResolvedRoute {
        token_in: token_in.trim().to_string(),
        token_out: token_out.trim().to_string(),
        hops,
        ops,
    })
}

async fn resolve_route(
    pool: &PgPool,
    token_in: &str,
    token_out: &str,
) -> Result<ResolvedRoute, (StatusCode, String)> {
    resolve_route_with_max_hops(pool, token_in, token_out, GET_POOL_ONLY_MAX_HOPS).await
}

fn parse_amount_in(raw: Option<&String>) -> Option<&str> {
    raw.map(|s| s.trim()).filter(|s| !s.is_empty())
}

fn get_pool_only_requested(q: &SolveRouteParams) -> bool {
    q.pool_only.unwrap_or(false) || q.hybrid_optimize == Some(false)
}

fn get_pool_only_max_hops(q: &SolveRouteParams) -> usize {
    if get_pool_only_requested(q) {
        GET_POOL_ONLY_MAX_HOPS
    } else {
        GET_DEFAULT_MAX_HOPS
    }
}

fn should_run_hybrid_get(q: &SolveRouteParams, amount_raw: Option<&str>) -> bool {
    !get_pool_only_requested(q) && amount_raw.is_some()
}

const TERRA_ADDR_MIN_LEN: usize = 39;
const TERRA_ADDR_MAX_LEN: usize = 128;

fn validate_optional_terra_address(label: &str, raw: &str) -> Result<String, (StatusCode, String)> {
    let addr = raw.trim();
    if addr.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{label} must not be empty when provided"),
        ));
    }
    if !addr.starts_with("terra1") {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{label} must be a terra1 bech32 address"),
        ));
    }
    if addr.len() < TERRA_ADDR_MIN_LEN || addr.len() > TERRA_ADDR_MAX_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{label} length out of range"),
        ));
    }
    if !addr.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("{label} contains invalid characters"),
        ));
    }
    Ok(addr.to_string())
}

pub(crate) fn parse_quote_trader(
    trader: Option<String>,
    sender: Option<String>,
) -> Result<hybrid_route_opt::QuoteTrader, (StatusCode, String)> {
    let trader = trader
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| validate_optional_terra_address("trader", s))
        .transpose()?;
    let sender = sender
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| validate_optional_terra_address("sender", s))
        .transpose()?;
    Ok(hybrid_route_opt::QuoteTrader { trader, sender })
}

pub(crate) async fn maybe_simulate(
    state: &AppState,
    amount_in: Option<&str>,
    ops: &[serde_json::Value],
    quote_trader: &hybrid_route_opt::QuoteTrader,
) -> Result<Option<String>, (StatusCode, String)> {
    let (Some(amt), Some(router)) = (amount_in, state.router_address.as_ref()) else {
        return Ok(None);
    };
    let Ok(n) = amt.parse::<u128>() else {
        return Ok(None);
    };
    let mut sim = json!({
        "simulate_swap_operations": {
            "offer_amount": n.to_string(),
            "operations": ops
        }
    });
    if let Some(trader) = quote_trader.trader.as_deref() {
        sim["simulate_swap_operations"]["trader"] = json!(trader);
    }
    if let Some(sender) = quote_trader.sender.as_deref() {
        sim["simulate_swap_operations"]["sender"] = json!(sender);
    }
    let sim: Result<serde_json::Value, _> = state.lcd.query_contract(router, &sim).await;
    match sim {
        Ok(v) => Ok(v
            .get("amount")
            .and_then(|a| a.as_str())
            .map(|s| s.to_string())),
        Err(e) => {
            let msg = e.to_string();
            tracing::warn!("router simulate_swap_operations failed: {}", msg);
            Err((
                StatusCode::BAD_REQUEST,
                "router simulation failed for the given route and hybrid parameters".to_string(),
            ))
        }
    }
}

pub(crate) fn quote_kind_after_sim(
    estimated: &Option<String>,
    base: RouteQuoteKind,
) -> RouteQuoteKind {
    if estimated.is_none()
        && matches!(
            base,
            RouteQuoteKind::IndexerPoolLcd
                | RouteQuoteKind::IndexerHybridLcd
                | RouteQuoteKind::IndexerHybridLcdDegraded
                | RouteQuoteKind::IndexerPoolDb
                | RouteQuoteKind::IndexerHybridDb
                | RouteQuoteKind::IndexerHybridDbDegraded
        )
    {
        return RouteQuoteKind::IndexerRouteOnly;
    }
    base
}

struct CacheEntry {
    at: Instant,
    body: serde_json::Value,
}

fn route_hybrid_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn amount_cache_key(amount: u128) -> u128 {
    if amount <= AMOUNT_CACHE_BUCKET {
        amount
    } else {
        (amount / AMOUNT_CACHE_BUCKET) * AMOUNT_CACHE_BUCKET
    }
}

/// Coarse bucket for hybrid GET cache keys — normal retail range 1–8 maps to default 8 (#324).
pub(crate) fn cache_key_maker_fills(max_maker_fills: u32) -> u32 {
    let m = max_maker_fills.max(1);
    if m <= 8 {
        8
    } else if m <= 16 {
        16
    } else {
        30
    }
}

/// GitLab #283: the on-chain discount is determined by the tier of the discount subject —
/// `trader` when set (the #245 off-chain forwarding), otherwise `sender` — and the router
/// simulate forwards both. The route cache must key on that resolved tier, or two senders on
/// different tiers (e.g. both with `trader` unset) collide and the wrong-discount quote is
/// served. Per Plastik's direction we key on the tier (not the raw address) so same-tier
/// callers still share the cache. Tier comes from synced `traders.tier_id` when
/// `registered` is true — no extra LCD call; absent/unknown/unregistered subject resolves to
/// [`FULL_FEE_TIER_SENTINEL`].
pub const FULL_FEE_TIER_SENTINEL: i16 = -1;

pub(crate) async fn resolve_discount_tier(
    state: &AppState,
    quote_trader: &hybrid_route_opt::QuoteTrader,
) -> i16 {
    let subject = quote_trader
        .trader
        .as_deref()
        .or(quote_trader.sender.as_deref());
    let Some(addr) = subject else {
        return FULL_FEE_TIER_SENTINEL;
    };
    match crate::db::queries::traders::get_trader(&state.pool, addr.trim()).await {
        Ok(Some(t)) if t.registered => t.tier_id,
        _ => FULL_FEE_TIER_SENTINEL,
    }
}

/// On-chain `GetDiscount` discount_bps for mirror grid pricing — matches pair `HybridSimulation` / router sim (#319).
pub(crate) async fn resolve_discount_bps(
    state: &AppState,
    quote_trader: &hybrid_route_opt::QuoteTrader,
) -> u16 {
    let (trader, sender) = match (
        quote_trader.trader.as_deref(),
        quote_trader.sender.as_deref(),
    ) {
        (None, None) => return 0,
        (Some(t), None) => (t.trim(), t.trim()),
        (None, Some(s)) => (s.trim(), s.trim()),
        (Some(t), Some(s)) => (t.trim(), s.trim()),
    };

    if let Some(fee_addr) = state
        .fee_discount_address
        .as_deref()
        .filter(|a| !a.is_empty())
    {
        let q = json!({
            "get_discount": {
                "trader": trader,
                "sender": sender,
            }
        });
        if let Ok(val) = state.lcd.query_contract::<serde_json::Value>(fee_addr, &q).await {
            return val
                .get("discount_bps")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16;
        }
    }

    crate::api::db_orderbook_sim::tier_discount_bps(
        resolve_discount_tier(state, quote_trader).await,
    )
}

fn hybrid_cache_key(
    solver_version: &str,
    token_in: &str,
    token_out: &str,
    amount_bucket: u128,
    max_maker_fills: u32,
    discount_bps: u16,
) -> String {
    let mmf_key = cache_key_maker_fills(max_maker_fills);
    format!(
        "{}|{}|{}|{}|{}|d{}",
        solver_version,
        token_in.trim().to_lowercase(),
        token_out.trim().to_lowercase(),
        amount_bucket,
        mmf_key,
        discount_bps
    )
}

fn cache_get(key: &str) -> Option<serde_json::Value> {
    let mut g = route_hybrid_cache().lock().ok()?;
    let e = g.get(key)?;
    if Instant::now().duration_since(e.at) > ROUTE_CACHE_TTL {
        g.remove(key);
        return None;
    }
    Some(e.body.clone())
}

fn cache_put(key: String, body: serde_json::Value) {
    if let Ok(mut g) = route_hybrid_cache().lock() {
        let now = Instant::now();
        g.retain(|_, v| now.duration_since(v.at) <= ROUTE_CACHE_TTL);
        if g.len() >= ROUTE_CACHE_MAX_ENTRIES {
            if let Some(oldest_k) = g.iter().min_by_key(|(_, v)| v.at).map(|(k, _)| k.clone()) {
                g.remove(&oldest_k);
            }
        }
        g.insert(key, CacheEntry { at: now, body });
    }
}

async fn execute_hybrid_route_solve(
    state: &AppState,
    token_in: &str,
    token_out: &str,
    amount_raw: &str,
    max_maker_fills: u32,
    quote_trader: &hybrid_route_opt::QuoteTrader,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Ok(amount_u) = amount_raw.parse::<u128>() else {
        return Err((
            StatusCode::BAD_REQUEST,
            "amount_in must be a non-negative integer".to_string(),
        ));
    };
    if amount_u == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "amount_in must be non-zero for hybrid best execution".to_string(),
        ));
    }

    let max_makers = max_maker_fills.max(1);
    let bucket = amount_cache_key(amount_u);
    let discount_bps = resolve_discount_bps(state, quote_trader).await;
    let solver_version = crate::api::best_execution::solver_version_for(state);
    let ck = hybrid_cache_key(
        solver_version,
        token_in,
        token_out,
        bucket,
        max_makers,
        discount_bps,
    );
    if let Some(cached) = cache_get(&ck) {
        return Ok(Json(cached));
    }

    let (body, _meta) = crate::api::best_execution::solve_global_best_execution(
        state,
        token_in,
        token_out,
        amount_u,
        amount_raw,
        max_makers,
        quote_trader,
    )
    .await?;

    let json_body = serde_json::to_value(&body).map_err(internal_err)?;
    cache_put(ck, json_body.clone());
    Ok(Json(json_body))
}

/// Retail / integrator best-execution route: server-chosen hybrid splits (GitLab #189).
#[utoipa::path(
    get,
    path = "/api/v1/route/solve/best",
    params(SolveRouteParams),
    responses(
        (status = 200, description = "Hybrid best-execution route with merged router operations", body = RouteSolveResponse),
        (status = 400, description = "Missing or invalid amount_in, or unknown token"),
        (status = 404, description = "No route within 4 hops"),
        (status = 502, description = "Hybrid optimization LCD failure (sanitized body)"),
    ),
    tag = "Routing"
)]
pub async fn solve_route_best(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let Some(amount_raw) = q
        .amount_in
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    else {
        return Err((
            StatusCode::BAD_REQUEST,
            "amount_in is required for GET /api/v1/route/solve/best".to_string(),
        ));
    };

    let quote_trader = parse_quote_trader(q.trader.clone(), q.sender.clone())?;
    execute_hybrid_route_solve(
        &state,
        &q.token_in,
        &q.token_out,
        amount_raw,
        q.max_maker_fills.unwrap_or(8),
        &quote_trader,
    )
    .await
}

/// Multihop route discovery (GET default **4 hops**, hybrid when `amount_in` set; `pool_only=true` → **4 hops** pool-only). Returns `router_operations`.
#[utoipa::path(
    get,
    path = "/api/v1/route/solve",
    params(SolveRouteParams),
    responses(
        (status = 200, description = "Route with hops and TerraSwap operations", body = RouteSolveResponse),
        (status = 400, description = "token_in or token_out not found in indexer assets"),
        (status = 404, description = "No route within hop limit"),
    ),
    tag = "Routing"
)]
pub async fn solve_route(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let amount_raw = parse_amount_in(q.amount_in.as_ref());
    let quote_trader = parse_quote_trader(q.trader.clone(), q.sender.clone())?;

    if q.hybrid_optimize == Some(true) && amount_raw.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "amount_in is required when hybrid_optimize=true".to_string(),
        ));
    }

    if should_run_hybrid_get(&q, amount_raw) {
        return execute_hybrid_route_solve(
            &state,
            &q.token_in,
            &q.token_out,
            amount_raw.expect("checked above"),
            q.max_maker_fills.unwrap_or(8),
            &quote_trader,
        )
        .await;
    }

    let max_hops = get_pool_only_max_hops(&q);
    let resolved =
        resolve_route_with_max_hops(&state.pool, &q.token_in, &q.token_out, max_hops).await?;
    let estimated = maybe_simulate(&state, amount_raw, &resolved.ops, &quote_trader).await?;

    let quote_kind = quote_kind_after_sim(
        &estimated,
        if estimated.is_some() {
            RouteQuoteKind::IndexerPoolLcd
        } else {
            RouteQuoteKind::IndexerRouteOnly
        },
    );

    let intermediate_tokens = build_intermediate_tokens(&resolved);
    let body = RouteSolveResponse {
        token_in: resolved.token_in,
        token_out: resolved.token_out,
        intermediate_tokens,
        quote_kind,
        hybrid_notes: None,
        hops: resolved.hops,
        router_operations: resolved.ops,
        estimated_amount_out: estimated,
        solver_version: None,
        paths_considered: None,
        optimality_scope: None,
        lcd_hybrid_queries: None,
        db_hybrid_queries: None,
        fidelity_check: None,
        mirror_max_block_lag: None,
        search_truncated: None,
    };

    Ok(Json(serde_json::to_value(body).map_err(internal_err)?))
}

/// Same discovery as GET (max 4 hops); optional `hybrid_by_hop` merges hybrid fields into `router_operations` before LCD simulation.
#[utoipa::path(
    post,
    path = "/api/v1/route/solve",
    request_body = SolveRoutePostBody,
    responses(
        (status = 200, description = "Route with hops and TerraSwap operations", body = RouteSolveResponse),
        (status = 400, description = "Bad request (unknown token, hybrid length mismatch, or router simulation error)"),
        (status = 404, description = "No route within 4 hops"),
    ),
    tag = "Routing"
)]
pub async fn solve_route_post(
    State(state): State<AppState>,
    Json(body): Json<SolveRoutePostBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let quote_trader = parse_quote_trader(body.trader.clone(), body.sender.clone())?;
    let resolved = resolve_route(&state.pool, &body.token_in, &body.token_out).await?;
    let intermediate_tokens = build_intermediate_tokens(&resolved);
    let ops = if let Some(ref hybrid) = body.hybrid_by_hop {
        apply_hybrid_by_hop(resolved.ops, hybrid)?
    } else {
        resolved.ops
    };

    let estimated = maybe_simulate(&state, body.amount_in.as_deref(), &ops, &quote_trader).await?;

    let any_hybrid = body
        .hybrid_by_hop
        .as_ref()
        .map(|v| v.iter().any(|x| x.is_some()))
        .unwrap_or(false);

    let base_kind = if any_hybrid {
        RouteQuoteKind::IndexerHybridLcd
    } else if estimated.is_some() {
        RouteQuoteKind::IndexerPoolLcd
    } else {
        RouteQuoteKind::IndexerRouteOnly
    };
    let quote_kind = quote_kind_after_sim(&estimated, base_kind);

    let out = RouteSolveResponse {
        token_in: resolved.token_in,
        token_out: resolved.token_out,
        hops: resolved.hops,
        intermediate_tokens,
        quote_kind,
        hybrid_notes: None,
        router_operations: ops,
        estimated_amount_out: estimated,
        solver_version: None,
        paths_considered: None,
        optimality_scope: None,
        lcd_hybrid_queries: None,
        db_hybrid_queries: None,
        fidelity_check: None,
        mirror_max_block_lag: None,
        search_truncated: None,
    };

    Ok(Json(serde_json::to_value(out).map_err(internal_err)?))
}

#[cfg(test)]
mod quote_trader_tests {
    use super::parse_quote_trader;
    use axum::http::StatusCode;

    const VALID: &str = "terra1discountwallet000000000000000000000";

    #[test]
    fn parse_quote_trader_empty_is_default() {
        let q = parse_quote_trader(None, None).expect("ok");
        assert!(q.trader.is_none());
        assert!(q.sender.is_none());
    }

    #[test]
    fn parse_quote_trader_accepts_terra1() {
        let q = parse_quote_trader(Some(VALID.into()), None).expect("ok");
        assert_eq!(q.trader.as_deref(), Some(VALID));
    }

    #[test]
    fn parse_quote_trader_rejects_non_terra() {
        let err = parse_quote_trader(Some("cosmos1bad".into()), None).unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}

#[cfg(test)]
mod hybrid_cache_key_tests {
    use super::{amount_cache_key, cache_key_maker_fills, hybrid_cache_key};

    const TIN: &str = "terra1tokenin000000000000000000000000000";
    const TOUT: &str = "terra1tokenout00000000000000000000000000";
    const SV: &str = "global_v4";

    #[test]
    fn hybrid_cache_key_same_tier_traders_share_key() {
        let key7 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 7, 5_000);
        let key8 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000);
        assert_eq!(key7, key8, "mmf 7 and 8 bucket to the same cache key");
        let other_wallet = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000);
        assert_eq!(key8, other_wallet, "trader address is not part of the key");
    }

    #[test]
    fn hybrid_cache_key_maker_fills_distinct_buckets() {
        let retail = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 0);
        let mid = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 12, 0);
        let high = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 25, 0);
        assert_ne!(retail, mid);
        assert_ne!(mid, high);
        assert_eq!(cache_key_maker_fills(7), 8);
        assert_eq!(cache_key_maker_fills(12), 16);
        assert_eq!(cache_key_maker_fills(25), 30);
    }

    // GitLab #283: the resolved discount bps is part of the key, so two callers on different
    // tiers can never collide — and two callers on the SAME tier still share the cache.
    #[test]
    fn hybrid_cache_key_distinguishes_discount_bps() {
        let tier0 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 10_000);
        let tier5 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000);
        let tier9 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 9_500);
        assert_ne!(
            tier0, tier5,
            "different discount bps must not share a cache key"
        );
        assert_ne!(tier5, tier9);
        assert_eq!(
            tier5,
            hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000),
            "same-discount callers should reuse the cache regardless of address"
        );
    }

    #[test]
    fn hybrid_cache_key_amount_bucket_regression() {
        let a = hybrid_cache_key(SV, TIN, TOUT, amount_cache_key(1_500_000), 8, 0);
        let b = hybrid_cache_key(SV, TIN, TOUT, amount_cache_key(1_900_000), 8, 0);
        assert_eq!(a, b, "micro-variation within one amount bucket shares key");
    }
}
