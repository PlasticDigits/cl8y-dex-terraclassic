//! Multihop route discovery and optional LCD simulation.
//!
//! - `GET /api/v1/route/solve`: returns TerraSwap-style router operations with `hybrid: null` (pool-only).
//! - `POST /api/v1/route/solve`: same discovery, optional `hybrid_by_hop` merged into ops for hybrid quotes
//!   when `ROUTER_ADDRESS` and `amount_in` trigger LCD `simulate_swap_operations`.

use std::collections::{HashMap, VecDeque};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use sqlx::PgPool;

use crate::api::internal_err;
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};

#[derive(Debug, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct SolveRouteParams {
    /// CW20 contract address (must match indexed `assets.contract_address`).
    pub token_in: String,
    pub token_out: String,
    /// Raw integer amount in offer token (optional; triggers router simulation when `ROUTER_ADDRESS` is set).
    pub amount_in: Option<String>,
}

/// Hybrid parameters for one hop (matches on-chain `HybridSwapParams`; amounts as decimal strings).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct HybridHopJson {
    pub pool_input: String,
    pub book_input: String,
    pub max_maker_fills: u32,
    #[serde(default)]
    pub book_start_hint: Option<u64>,
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
    /// Router `ExecuteSwapOperations` operations (JSON). `terra_swap.hybrid` is `null` unless merged from POST `hybrid_by_hop`.
    #[schema(value_type = Vec<Object>)]
    pub router_operations: Vec<serde_json::Value>,
    /// From `SimulateSwapOperations` when `amount_in` and `ROUTER_ADDRESS` are set.
    pub estimated_amount_out: Option<String>,
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

async fn resolve_route(
    pool: &PgPool,
    token_in: &str,
    token_out: &str,
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

    let hops_raw = find_path(start, goal, &pair_rows, 4)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no route within 4 hops".to_string()))?;

    let (hops, ops) = build_hops_and_ops(&hops_raw, &id_to_addr)?;

    Ok(ResolvedRoute {
        token_in: token_in.trim().to_string(),
        token_out: token_out.trim().to_string(),
        hops,
        ops,
    })
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

/// Multihop route discovery (BFS, max 4 hops). Returns pool-only `router_operations`.
#[utoipa::path(
    get,
    path = "/api/v1/route/solve",
    params(SolveRouteParams),
    responses(
        (status = 200, description = "Route with hops and TerraSwap operations", body = RouteSolveResponse),
        (status = 400, description = "token_in or token_out not found in indexer assets"),
        (status = 404, description = "No route within 4 hops"),
    ),
    tag = "Routing"
)]
pub async fn solve_route(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let resolved = resolve_route(&state.pool, &q.token_in, &q.token_out).await?;
    let estimated = maybe_simulate(&state, q.amount_in.as_deref(), &resolved.ops).await?;

    let body = RouteSolveResponse {
        token_in: resolved.token_in,
        token_out: resolved.token_out,
        hops: resolved.hops,
        router_operations: resolved.ops,
        estimated_amount_out: estimated,
    };

    Ok(Json(serde_json::to_value(body).map_err(internal_err)?))
}

/// Same discovery as GET; optional `hybrid_by_hop` merges hybrid fields into `router_operations` before LCD simulation.
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
    let ops = if let Some(ref hybrid) = body.hybrid_by_hop {
        apply_hybrid_by_hop(resolved.ops, hybrid)?
    } else {
        resolved.ops
    };

    let estimated = maybe_simulate(&state, body.amount_in.as_deref(), &ops).await?;

    let out = RouteSolveResponse {
        token_in: resolved.token_in,
        token_out: resolved.token_out,
        hops: resolved.hops,
        router_operations: ops,
        estimated_amount_out: estimated,
    };

    Ok(Json(serde_json::to_value(out).map_err(internal_err)?))
}
