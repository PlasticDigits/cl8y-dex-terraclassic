//! Global best-execution route solver: top-K path enumeration + joint hybrid optimization (GitLab #209).

use crate::api::hybrid_route_opt::{self, HopDescriptor, OptimizationMeta};
use axum::http::StatusCode;
use crate::api::route_paths;
use crate::api::route_solver::{
    apply_hybrid_by_hop, build_hops_and_ops, build_intermediate_tokens, quote_kind_after_sim,
    RouteHop, RouteQuoteKind, RouteSolveResponse, GET_DEFAULT_MAX_HOPS,
};
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};
use sqlx::PgPool;

/// Solver generation label exposed on API responses.
pub const SOLVER_VERSION: &str = "global_v1";

/// Max simple paths evaluated per request (hop-count order).
pub const MAX_PATH_CANDIDATES: usize = 5;

/// Documented optimality scope for clients.
pub const OPTIMALITY_SCOPE: &str =
    "optimal within top-5 simple paths by hop count and per-hop hybrid split grid (17 book fractions), with 2-pass coordinate refinement across hops";

/// Upper bound on pair-level `HybridSimulation` LCD calls per request (worst-case estimate for docs/tests).
/// Documented upper bound: top-K paths × hops × (grid + coordinate passes).
pub const LCD_HYBRID_SIM_BUDGET: usize =
    MAX_PATH_CANDIDATES * GET_DEFAULT_MAX_HOPS * (17 + 2 * 2 * 17);

#[cfg(test)]
mod budget_tests {
    use super::LCD_HYBRID_SIM_BUDGET;

    #[test]
    fn lcd_budget_is_documented_constant() {
        assert!(LCD_HYBRID_SIM_BUDGET > 0);
    }
}

#[derive(Debug, Clone, Default)]
pub struct BestExecutionMeta {
    pub paths_considered: u32,
    pub lcd_hybrid_queries: u32,
    pub degraded: bool,
    pub any_book_leg: bool,
}

pub fn hybrid_notes_for_global(meta: &BestExecutionMeta) -> String {
    format!(
        "Global best-execution solver ({SOLVER_VERSION}): {OPTIMALITY_SCOPE}. \
         Evaluated {} path(s); {} pair-level hybrid simulations. \
         Final output validated via router simulate_swap_operations when configured. \
         Execution on-chain may differ from this LCD snapshot.",
        meta.paths_considered, meta.lcd_hybrid_queries
    )
}

struct PathCandidate {
    hops: Vec<RouteHop>,
    ops: Vec<serde_json::Value>,
}

async fn enumerate_path_candidates(
    pool: &PgPool,
    token_in: &str,
    token_out: &str,
    max_hops: usize,
) -> Result<Vec<PathCandidate>, (StatusCode, String)> {
    let all_assets = assets::get_all_assets(pool)
        .await
        .map_err(crate::api::internal_err)?;
    let pair_rows = db_pairs::get_all_pairs(pool)
        .await
        .map_err(crate::api::internal_err)?;

    let (id_to_addr, addr_to_id) = crate::api::route_solver::build_id_to_addr_map(&all_assets);

    let start = crate::api::route_solver::resolve_id(&addr_to_id, token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".to_string(),
        )
    })?;
    let goal = crate::api::route_solver::resolve_id(&addr_to_id, token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".to_string(),
        )
    })?;

    // The enumeration is CPU-bound and synchronous; run it on the blocking pool so a large
    // legitimate pair graph never stalls the async executor (#286). The reachability gate inside
    // find_paths_top_k bounds the work to O(V+E), so there is no truncation — a route that exists
    // is always discovered.
    let paths_raw = tokio::task::spawn_blocking(move || {
        route_paths::find_paths_top_k(start, goal, &pair_rows, max_hops, MAX_PATH_CANDIDATES)
    })
    .await
    .map_err(crate::api::internal_err)?;
    if paths_raw.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("no route within {} hops", max_hops),
        ));
    }

    let mut out = Vec::with_capacity(paths_raw.len());
    for hops_raw in paths_raw {
        let (hops, ops) = build_hops_and_ops(&hops_raw, &id_to_addr)?;
        out.push(PathCandidate { hops, ops });
    }
    Ok(out)
}

/// Pick the path + hybrid plan maximizing router `estimated_amount_out`.
pub async fn solve_global_best_execution(
    state: &AppState,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    amount_raw: &str,
    max_maker_fills: u32,
    quote_trader: &hybrid_route_opt::QuoteTrader,
) -> Result<(RouteSolveResponse, BestExecutionMeta), (StatusCode, String)> {
    let candidates =
        enumerate_path_candidates(&state.pool, token_in, token_out, GET_DEFAULT_MAX_HOPS).await?;

    let mut best: Option<(RouteSolveResponse, u128, BestExecutionMeta)> = None;
    let mut lcd_queries: u32 = 0;

    for cand in &candidates {
        let hops_desc: Vec<HopDescriptor> = cand
            .hops
            .iter()
            .map(|h| HopDescriptor {
                pair: h.pair.clone(),
                offer_token: h.offer_token.clone(),
                ask_token: h.ask_token.clone(),
            })
            .collect();

        let queries_before = lcd_queries;
        let (hybrid_plan, opt_meta) = hybrid_route_opt::optimize_multihop_hybrid_joint(
            &state.lcd,
            &hops_desc,
            amount_in,
            max_maker_fills,
            quote_trader,
        )
        .await
        .map_err(crate::api::lcd_gateway_err)?;
        // Approximate LCD calls: grid per hop + coordinate passes (bounded by design).
        lcd_queries = queries_before.saturating_add(estimate_lcd_calls(hops_desc.len()));

        let token_in_t = token_in.trim().to_string();
        let token_out_t = token_out.trim().to_string();
        let hops = cand.hops.clone();
        let ops = apply_hybrid_by_hop(cand.ops.clone(), &hybrid_plan)?;
        let estimated =
            crate::api::route_solver::maybe_simulate(state, Some(amount_raw), &ops, quote_trader).await?;

        let out_u = estimated
            .as_ref()
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);

        let quote_kind = quote_kind_for(&opt_meta, &estimated);
        let resolved_route = crate::api::route_solver::ResolvedRoute {
            token_in: token_in_t.clone(),
            token_out: token_out_t.clone(),
            hops: hops.clone(),
            ops: ops.clone(),
        };
        let intermediate_tokens = build_intermediate_tokens(&resolved_route);
        let body = RouteSolveResponse {
            token_in: token_in_t,
            token_out: token_out_t,
            hops,
            intermediate_tokens,
            quote_kind,
            hybrid_notes: None,
            router_operations: ops,
            estimated_amount_out: estimated.clone(),
            solver_version: Some(SOLVER_VERSION.to_string()),
            paths_considered: None,
            optimality_scope: None,
            lcd_hybrid_queries: None,
        };

        let replace = best
            .as_ref()
            .map(|(_, prev_out, _)| out_u > *prev_out)
            .unwrap_or(true);
        if replace {
            let meta = BestExecutionMeta {
                paths_considered: candidates.len() as u32,
                lcd_hybrid_queries: lcd_queries,
                degraded: opt_meta.degraded,
                any_book_leg: opt_meta.any_book_leg,
            };
            best = Some((body, out_u, meta));
        }
    }

    let (mut body, _, meta) = best.expect("candidates non-empty");
    body.hybrid_notes = Some(hybrid_notes_for_global(&meta));
    body.solver_version = Some(SOLVER_VERSION.to_string());
    body.paths_considered = Some(meta.paths_considered);
    body.optimality_scope = Some(OPTIMALITY_SCOPE.to_string());
    body.lcd_hybrid_queries = Some(meta.lcd_hybrid_queries);

    tracing::info!(
        solver_version = SOLVER_VERSION,
        paths_considered = meta.paths_considered,
        lcd_queries = meta.lcd_hybrid_queries,
        degraded = meta.degraded,
        "route best execution"
    );

    Ok((body, meta))
}

fn estimate_lcd_calls(hop_count: usize) -> u32 {
    // 17 grid + 2 coordinate passes × 17 per hop (upper bound).
    let per_hop = 17u32 + 2 * 17;
    (hop_count as u32).saturating_mul(per_hop)
}

fn quote_kind_for(meta: &OptimizationMeta, estimated: &Option<String>) -> RouteQuoteKind {
    let kind = if meta.degraded {
        RouteQuoteKind::IndexerHybridLcdDegraded
    } else if meta.any_book_leg {
        RouteQuoteKind::IndexerHybridLcd
    } else {
        RouteQuoteKind::IndexerPoolLcd
    };
    quote_kind_after_sim(estimated, kind)
}
