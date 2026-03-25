//! Multihop route discovery and optional LCD simulation.
//!
//! Returns TerraSwap-style router operations with `hybrid: null` (100% pool). For Pattern C
//! limit-order splits, supply `hybrid` per hop off-chain to match `max_maker_fills`.

use std::collections::{HashMap, VecDeque};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::api::internal_err;
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};

#[derive(Debug, Deserialize)]
pub struct SolveRouteParams {
    /// CW20 contract address (must match indexed `assets.contract_address`).
    pub token_in: String,
    pub token_out: String,
    /// Raw integer amount in offer token (optional; triggers router simulation when set).
    pub amount_in: Option<String>,
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

#[derive(serde::Serialize)]
struct RouteHop {
    pair: String,
    offer_token: String,
    ask_token: String,
}

#[derive(serde::Serialize)]
struct RouteSolveResponse {
    token_in: String,
    token_out: String,
    hops: Vec<RouteHop>,
    /// Router `ExecuteSwapOperations` operations (JSON), `hybrid: null` = pool-only.
    router_operations: Vec<serde_json::Value>,
    /// From `SimulateSwapOperations` when `amount_in` and `ROUTER_ADDRESS` are set.
    estimated_amount_out: Option<String>,
}

/// GET `/api/v1/route/solve?token_in=...&token_out=...&amount_in=...`
pub async fn solve_route(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let all_assets = assets::get_all_assets(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_rows = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;

    let (id_to_addr, addr_to_id) = build_id_to_addr_map(&all_assets);

    let start = resolve_id(&addr_to_id, &q.token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".to_string(),
        )
    })?;
    let goal = resolve_id(&addr_to_id, &q.token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".to_string(),
        )
    })?;

    let hops_raw = find_path(start, goal, &pair_rows, 4)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "no route within 4 hops".to_string()))?;

    let mut hops: Vec<RouteHop> = Vec::new();
    let mut ops: Vec<serde_json::Value> = Vec::new();

    for (pair_addr, from_id, to_id) in &hops_raw {
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

    let estimated = if let (Some(ref amt), Some(ref router)) = (&q.amount_in, &state.router_address)
    {
        if let Ok(n) = amt.parse::<u128>() {
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
            sim.ok().and_then(|v| {
                v.get("amount")
                    .and_then(|a| a.as_str())
                    .map(|s| s.to_string())
            })
        } else {
            None
        }
    } else {
        None
    };

    let body = RouteSolveResponse {
        token_in: q.token_in.trim().to_string(),
        token_out: q.token_out.trim().to_string(),
        hops,
        router_operations: ops,
        estimated_amount_out: estimated,
    };

    Ok(Json(serde_json::to_value(body).map_err(internal_err)?))
}
