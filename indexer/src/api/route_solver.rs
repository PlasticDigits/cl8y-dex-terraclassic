//! Multihop route discovery and optional LCD simulation.
//!
//! - `GET /api/v1/route/solve`: pool-only `hybrid: null` by default; optional `hybrid_optimize=true`
//!   (requires `amount_in`) runs per-hop hybrid split search via pair `HybridSimulation` (max **3 hops**),
//!   merges params into `router_operations`, then LCD `simulate_swap_operations` when configured.
//! - `POST /api/v1/route/solve`: same discovery (max **4 hops**), optional `hybrid_by_hop` merged into ops.

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

use crate::api::hybrid_route_opt::{self, HopDescriptor};
use crate::api::internal_err;
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};

pub use hybrid_route_opt::HybridHopJson;

const ROUTE_CACHE_TTL: Duration = Duration::from_secs(12);
const ROUTE_CACHE_MAX_ENTRIES: usize = 512;
/// Coarse bucketing for hybrid GET cache keys (reduces LCD load).
const AMOUNT_CACHE_BUCKET: u128 = 1_000_000;

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct SolveRouteParams {
    /// CW20 contract address (must match indexed `assets.contract_address`).
    pub token_in: String,
    pub token_out: String,
    /// Raw integer amount in offer token (optional; triggers router simulation when `ROUTER_ADDRESS` is set).
    /// Required when `hybrid_optimize=true`.
    pub amount_in: Option<String>,
    /// When true, requires `amount_in`; runs hybrid split optimization (LCD), **max 3 hops**.
    #[serde(default)]
    pub hybrid_optimize: Option<bool>,
    /// Used when `hybrid_optimize` is true (default **8**).
    #[serde(default)]
    pub max_maker_fills: Option<u32>,
}

/// JSON body for [`solve_route_post`]. When `hybrid_by_hop` is omitted or `null`, all hops are pool-only (same as GET).
#[derive(Debug, Deserialize, ToSchema)]
pub struct SolveRoutePostBody {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Option<String>,
    /// One entry per hop after BFS. `null` = pool-only for that hop. Length must match hop count when present.
    #[serde(default)]
    pub hybrid_by_hop: Option<Vec<Option<HybridHopJson>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouteQuoteKind {
    /// No `estimated_amount_out` (missing `ROUTER_ADDRESS` and/or `amount_in`).
    IndexerRouteOnly,
    /// Pool-only router operations; LCD sim when configured.
    IndexerPoolLcd,
    /// At least one hop uses a limit-book leg after hybrid optimization.
    IndexerHybridLcd,
    /// Hybrid optimization fell back to pool-only `Simulation` on at least one hop (LCD).
    IndexerHybridLcdDegraded,
}

struct ResolvedRoute {
    token_in: String,
    token_out: String,
    hops: Vec<RouteHop>,
    ops: Vec<serde_json::Value>,
}

fn build_id_to_addr_map(all: &[assets::AssetRow]) -> (HashMap<i32, String>, HashMap<String, i32>) {
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

fn resolve_id(addr_to_id: &HashMap<String, i32>, token: &str) -> Option<i32> {
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

#[derive(Serialize, ToSchema, Clone)]
pub struct RouteHop {
    pub pair: String,
    pub offer_token: String,
    pub ask_token: String,
}

#[derive(Serialize, ToSchema)]
pub struct RouteSolveResponse {
    pub token_in: String,
    pub token_out: String,
    pub hops: Vec<RouteHop>,
    /// Full token path: `token_in`, then each hop's `ask_token` (ends at `token_out`).
    pub intermediate_tokens: Vec<String>,
    pub quote_kind: RouteQuoteKind,
    /// Model limits, degradation, or execution-risk notes for clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_notes: Option<String>,
    /// Router `ExecuteSwapOperations` operations (JSON). `terra_swap.hybrid` is `null` unless merged.
    #[schema(value_type = Vec<Object>)]
    pub router_operations: Vec<serde_json::Value>,
    /// From `SimulateSwapOperations` when `amount_in` and `ROUTER_ADDRESS` are set.
    pub estimated_amount_out: Option<String>,
}

fn build_intermediate_tokens(resolved: &ResolvedRoute) -> Vec<String> {
    let mut v = vec![resolved.token_in.clone()];
    for h in &resolved.hops {
        v.push(h.ask_token.clone());
    }
    v
}

fn build_hops_and_ops(
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

fn apply_hybrid_by_hop(
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
    resolve_route_with_max_hops(pool, token_in, token_out, 4).await
}

async fn maybe_simulate(
    state: &AppState,
    amount_in: Option<&str>,
    ops: &[serde_json::Value],
) -> Result<Option<String>, (StatusCode, String)> {
    let (Some(amt), Some(router)) = (amount_in, state.router_address.as_ref()) else {
        return Ok(None);
    };
    let Ok(n) = amt.parse::<u128>() else {
        return Ok(None);
    };
    let sim: Result<serde_json::Value, _> = state
        .lcd
        .query_contract(
            router,
            &json!({
                "simulate_swap_operations": {
                    "offer_amount": n.to_string(),
                    "operations": ops
                }
            }),
        )
        .await;
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

fn quote_kind_after_sim(
    estimated: &Option<String>,
    base: RouteQuoteKind,
) -> RouteQuoteKind {
    if estimated.is_none() && matches!(base, RouteQuoteKind::IndexerPoolLcd | RouteQuoteKind::IndexerHybridLcd | RouteQuoteKind::IndexerHybridLcdDegraded) {
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

fn hybrid_cache_key(
    token_in: &str,
    token_out: &str,
    amount_bucket: u128,
    max_maker_fills: u32,
) -> String {
    format!(
        "{}|{}|{}|{}",
        token_in.trim().to_lowercase(),
        token_out.trim().to_lowercase(),
        amount_bucket,
        max_maker_fills
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
            if let Some(oldest_k) = g
                .iter()
                .min_by_key(|(_, v)| v.at)
                .map(|(k, _)| k.clone())
            {
                g.remove(&oldest_k);
            }
        }
        g.insert(key, CacheEntry { at: now, body });
    }
}

/// Multihop route discovery (BFS, max 4 hops unless `hybrid_optimize`). Returns `router_operations`.
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
    let hybrid_opt = q.hybrid_optimize.unwrap_or(false);

    if hybrid_opt {
        let Some(amount_raw) = q.amount_in.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) else {
            return Err((
                StatusCode::BAD_REQUEST,
                "amount_in is required when hybrid_optimize=true".to_string(),
            ));
        };
        let Ok(amount_u) = amount_raw.parse::<u128>() else {
            return Err((StatusCode::BAD_REQUEST, "amount_in must be a non-negative integer".to_string()));
        };
        if amount_u == 0 {
            return Err((
                StatusCode::BAD_REQUEST,
                "amount_in must be non-zero when hybrid_optimize=true".to_string(),
            ));
        }

        let max_makers = q.max_maker_fills.unwrap_or(8).max(1);
        let bucket = amount_cache_key(amount_u);
        let ck = hybrid_cache_key(&q.token_in, &q.token_out, bucket, max_makers);
        if let Some(cached) = cache_get(&ck) {
            return Ok(Json(cached));
        }

        let resolved = resolve_route_with_max_hops(&state.pool, &q.token_in, &q.token_out, 3).await?;
        let hops_desc: Vec<HopDescriptor> = resolved
            .hops
            .iter()
            .map(|h| HopDescriptor {
                pair: h.pair.clone(),
                offer_token: h.offer_token.clone(),
                ask_token: h.ask_token.clone(),
            })
            .collect();

        let (hybrid_plan, meta) = hybrid_route_opt::optimize_multihop_hybrid(
            &state.lcd,
            &hops_desc,
            amount_u,
            max_makers,
        )
        .await
        .map_err(|e| {
            tracing::warn!("hybrid optimization LCD error: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                "hybrid optimization failed (LCD)".to_string(),
            )
        })?;

        let intermediate_tokens = build_intermediate_tokens(&resolved);
        let ops = apply_hybrid_by_hop(resolved.ops, &hybrid_plan)?;
        let estimated = maybe_simulate(&state, Some(amount_raw), &ops).await?;

        let mut quote_kind = if meta.degraded {
            RouteQuoteKind::IndexerHybridLcdDegraded
        } else if meta.any_book_leg {
            RouteQuoteKind::IndexerHybridLcd
        } else {
            RouteQuoteKind::IndexerPoolLcd
        };
        quote_kind = quote_kind_after_sim(&estimated, quote_kind);

        let hybrid_notes = Some(
            "Sequential per-hop hybrid optimizer (not globally optimal across hops). Quotes use the same LCD snapshot per call; execution may differ.".to_string(),
        );

        let body = RouteSolveResponse {
            token_in: resolved.token_in.clone(),
            token_out: resolved.token_out.clone(),
            hops: resolved.hops.clone(),
            intermediate_tokens,
            quote_kind,
            hybrid_notes,
            router_operations: ops,
            estimated_amount_out: estimated.clone(),
        };

        let json_body = serde_json::to_value(&body).map_err(internal_err)?;
        cache_put(ck, json_body.clone());
        return Ok(Json(json_body));
    }

    let resolved = resolve_route(&state.pool, &q.token_in, &q.token_out).await?;
    let estimated = maybe_simulate(&state, q.amount_in.as_deref(), &resolved.ops).await?;

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
    let resolved = resolve_route(&state.pool, &body.token_in, &body.token_out).await?;
    let intermediate_tokens = build_intermediate_tokens(&resolved);
    let ops = if let Some(ref hybrid) = body.hybrid_by_hop {
        apply_hybrid_by_hop(resolved.ops, hybrid)?
    } else {
        resolved.ops
    };

    let estimated = maybe_simulate(&state, body.amount_in.as_deref(), &ops).await?;

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
    };

    Ok(Json(serde_json::to_value(out).map_err(internal_err)?))
}
