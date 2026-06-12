//! Global best-execution route solver: top-K path enumeration + joint hybrid optimization (GitLab #209).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::api::db_orderbook_sim::{self, MirrorLoadMeta};
use crate::api::hybrid_route_opt::{
    self, HopDescriptor, HybridSimError, HybridSimSource, OptimizationMeta,
};
use crate::api::route_paths;
use crate::api::route_solver::{
    apply_hybrid_by_hop, build_hops_and_ops, build_intermediate_tokens, quote_kind_after_sim,
    FidelityCheck, RouteHop, RouteQuoteKind, RouteSolveResponse, GET_DEFAULT_MAX_HOPS,
};
use crate::api::AppState;
use crate::db::queries::{assets, pairs as db_pairs};
use axum::http::StatusCode;
use sqlx::PgPool;

/// LCD-grid solver generation (legacy).
pub const SOLVER_VERSION_LCD: &str = "global_v3";

/// Postgres-mirror hybrid grid (#319 Phase 1c; cache-key bump #324).
pub const SOLVER_VERSION_DB: &str = "global_v4";

/// Max simple paths evaluated per request (hop-count order).
pub const MAX_PATH_CANDIDATES: usize = 5;

/// Bounded concurrent path-candidate evaluations per request (GitLab #324).
pub const SOLVE_CONCURRENCY: usize = MAX_PATH_CANDIDATES;

/// Documented optimality scope for clients.
pub const OPTIMALITY_SCOPE: &str =
    "optimal within top-5 simple paths by hop count and per-hop hybrid split grid (17 book fractions), with 2-pass coordinate refinement across hops";

/// Upper bound on pair-level hybrid simulations per request (worst-case estimate for docs/tests).
/// Post-#319 each hop is priced from the DB orderbook mirror, not live LCD; this constant
/// documents the theoretical sim budget if every candidate path used the full grid + coordinate
/// passes on every hop: `5 × 4 × 85 = 1700`.
pub const LCD_HYBRID_SIM_BUDGET: usize =
    MAX_PATH_CANDIDATES * GET_DEFAULT_MAX_HOPS * (17 + 2 * 2 * 17);

#[cfg(test)]
mod budget_tests {
    use super::{LCD_HYBRID_SIM_BUDGET, MAX_PATH_CANDIDATES};
    use crate::api::route_solver::GET_DEFAULT_MAX_HOPS;

    #[test]
    fn lcd_budget_is_documented_constant() {
        assert_eq!(
            LCD_HYBRID_SIM_BUDGET,
            MAX_PATH_CANDIDATES * GET_DEFAULT_MAX_HOPS * (17 + 2 * 2 * 17)
        );
        assert_eq!(LCD_HYBRID_SIM_BUDGET, 5 * 4 * 85);
    }
}

#[derive(Debug, Clone, Default)]
pub struct BestExecutionMeta {
    pub paths_considered: u32,
    pub lcd_hybrid_queries: u32,
    pub db_hybrid_queries: u32,
    pub degraded: bool,
    pub any_book_leg: bool,
    pub mirror_stale: bool,
    pub mirror_missing: bool,
    pub mirror_max_block_lag: Option<i64>,
    pub max_snapshot_age_ms: u64,
    pub fidelity_check: FidelityCheck,
    pub db_optimized_amount_out: Option<u128>,
    /// True when fewer path candidates were evaluated than enumerated (concurrency cap).
    pub search_truncated: bool,
}

pub fn solver_version_for(state: &AppState) -> &'static str {
    if state.route_solver_db_hybrid {
        SOLVER_VERSION_DB
    } else {
        SOLVER_VERSION_LCD
    }
}

pub fn hybrid_notes_for_global(meta: &BestExecutionMeta, solver_version: &str) -> String {
    let pricing = if solver_version == SOLVER_VERSION_DB {
        "hybrid grids priced from indexed Postgres mirror (pair_reserves + resting_limit_orders)"
    } else {
        "pair-level hybrid simulations via LCD"
    };
    let truncation = if meta.search_truncated {
        " Search was truncated by the concurrency cap — result is not guaranteed optimal over all enumerated paths."
    } else {
        ""
    };
    format!(
        "Global best-execution solver ({solver_version}): {OPTIMALITY_SCOPE}. \
         Evaluated {} path(s); {} db-hybrid + {} lcd-hybrid grid evals. \
         {pricing}. \
         Final output validated via router simulate_swap_operations when configured (fidelity_check={}). \
         Execution on-chain may differ from mirror/LCD snapshots.{truncation}",
        meta.paths_considered,
        meta.db_hybrid_queries,
        meta.lcd_hybrid_queries,
        meta.fidelity_check.as_str(),
    )
}

#[derive(Clone)]
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

fn hybrid_sim_gateway_err(e: HybridSimError) -> (StatusCode, String) {
    match e {
        HybridSimError::Lcd(le) => crate::api::lcd_gateway_err(le),
        HybridSimError::Db(de) => {
            tracing::warn!(detail = %de, "db hybrid sim error");
            (
                StatusCode::BAD_GATEWAY,
                "Route mirror simulation failed".to_string(),
            )
        }
        HybridSimError::PathUnusable => (
            StatusCode::NOT_FOUND,
            format!("no route within {} hops", GET_DEFAULT_MAX_HOPS),
        ),
    }
}

fn estimate_lcd_calls(hop_count: usize) -> u32 {
    let per_hop = 17u32 + 2 * 17;
    (hop_count as u32).saturating_mul(per_hop)
}

fn quote_kind_for(
    meta: &OptimizationMeta,
    estimated: &Option<String>,
    db_mode: bool,
) -> RouteQuoteKind {
    let mirror_degraded = meta.mirror_stale || meta.mirror_missing;
    let kind = if meta.degraded || mirror_degraded {
        if db_mode {
            RouteQuoteKind::IndexerHybridDbDegraded
        } else {
            RouteQuoteKind::IndexerHybridLcdDegraded
        }
    } else if meta.any_book_leg {
        if db_mode {
            RouteQuoteKind::IndexerHybridDb
        } else {
            RouteQuoteKind::IndexerHybridLcd
        }
    } else if db_mode {
        RouteQuoteKind::IndexerPoolDb
    } else {
        RouteQuoteKind::IndexerPoolLcd
    };
    quote_kind_after_sim(estimated, kind)
}

fn apply_fidelity_guard(
    meta: &mut BestExecutionMeta,
    db_out: u128,
    estimated: &Option<String>,
    drift_bps: u32,
) -> Option<String> {
    meta.db_optimized_amount_out = Some(db_out);
    let Some(sim_s) = estimated.as_ref() else {
        meta.fidelity_check = FidelityCheck::Skipped;
        return estimated.clone();
    };
    let Ok(sim_u) = sim_s.parse::<u128>() else {
        meta.fidelity_check = FidelityCheck::Skipped;
        return estimated.clone();
    };
    if sim_u == 0 {
        if db_out == 0 {
            meta.fidelity_check = FidelityCheck::Passed;
        } else {
            meta.fidelity_check = FidelityCheck::Drift;
            meta.degraded = true;
        }
        return estimated.clone();
    }
    if db_out > sim_u {
        let excess = db_out - sim_u;
        let drift = excess.saturating_mul(10_000) / sim_u;
        if drift > drift_bps as u128 {
            tracing::warn!(
                fidelity_drift = true,
                db_out,
                sim_u,
                drift_bps,
                "poisoned-mirror guard: DB output exceeds router sim"
            );
            meta.fidelity_check = FidelityCheck::Drift;
            meta.degraded = true;
            return Some(sim_u.to_string());
        }
    }
    meta.fidelity_check = FidelityCheck::Passed;
    estimated.clone()
}

#[derive(Clone)]
struct CandidateEval {
    index: usize,
    body: RouteSolveResponse,
    out_u: u128,
    grid_out: u128,
    lcd_delta: u32,
    db_delta: u32,
    degraded: bool,
    any_book_leg: bool,
    mirror_stale: bool,
    mirror_missing: bool,
    max_snapshot_age_ms: u64,
}

/// Merge per-candidate results: max `out_u` with first-seen (lowest index) tie-break; cumulative
/// query counts through the winner index match the serial loop (#324).
fn merge_candidate_evaluations(
    evals: &[CandidateEval],
    paths_enumerated: usize,
    search_truncated: bool,
) -> Option<(RouteSolveResponse, u128, u128, BestExecutionMeta)> {
    if evals.is_empty() {
        return None;
    }
    let mut sorted: Vec<_> = evals.to_vec();
    sorted.sort_by_key(|e| e.index);

    let mut winner: Option<&CandidateEval> = None;
    for ev in &sorted {
        let replace = winner.map(|w| ev.out_u > w.out_u).unwrap_or(true);
        if replace {
            winner = Some(ev);
        }
    }
    let winner = winner?;

    let mut lcd_total = 0u32;
    let mut db_total = 0u32;
    for ev in &sorted {
        lcd_total = lcd_total.saturating_add(ev.lcd_delta);
        db_total = db_total.saturating_add(ev.db_delta);
        if ev.index == winner.index {
            break;
        }
    }

    let meta = BestExecutionMeta {
        paths_considered: paths_enumerated as u32,
        lcd_hybrid_queries: lcd_total,
        db_hybrid_queries: db_total,
        degraded: winner.degraded,
        any_book_leg: winner.any_book_leg,
        mirror_stale: winner.mirror_stale,
        mirror_missing: winner.mirror_missing,
        mirror_max_block_lag: None,
        max_snapshot_age_ms: winner.max_snapshot_age_ms,
        fidelity_check: FidelityCheck::Skipped,
        db_optimized_amount_out: None,
        search_truncated,
    };

    Some((
        winner.body.clone(),
        winner.out_u,
        winner.grid_out,
        meta,
    ))
}

async fn evaluate_candidate(
    state: Arc<AppState>,
    index: usize,
    cand: PathCandidate,
    token_in: String,
    token_out: String,
    amount_in: u128,
    amount_raw: String,
    max_maker_fills: u32,
    quote_trader: hybrid_route_opt::QuoteTrader,
    db_mode: bool,
    discount_bps: u16,
    mirrors: Arc<HashMap<String, db_orderbook_sim::HopMirror>>,
    solver_version: &'static str,
) -> Result<Option<CandidateEval>, (StatusCode, String)> {
    let hops_desc: Vec<HopDescriptor> = cand
        .hops
        .iter()
        .map(|h| HopDescriptor {
            pair: h.pair.clone(),
            offer_token: h.offer_token.clone(),
            ask_token: h.ask_token.clone(),
        })
        .collect();

    let mut mirror_meta = MirrorLoadMeta::default();

    let source = if db_mode {
        HybridSimSource::Db {
            lcd_fallback: &state.lcd,
            mirrors: &mirrors,
            discount_bps,
        }
    } else {
        HybridSimSource::Lcd(&state.lcd)
    };

    let mm = if db_mode {
        Some(&mut mirror_meta)
    } else {
        None
    };

    let (hybrid_plan, opt_meta, grid_out) =
        match hybrid_route_opt::optimize_multihop_hybrid_joint(
            &source,
            mm,
            &hops_desc,
            amount_in,
            max_maker_fills,
            &quote_trader,
        )
        .await
        {
            Ok(v) => v,
            Err(HybridSimError::PathUnusable) => {
                tracing::debug!(
                    path_index = index,
                    hops = hops_desc.len(),
                    "skipping path candidate: unusable pool liquidity on hop"
                );
                return Ok(None);
            }
            Err(e) => return Err(hybrid_sim_gateway_err(e)),
        };

    let lcd_delta = if db_mode {
        mirror_meta.lcd_fallback_queries
    } else {
        estimate_lcd_calls(hops_desc.len())
    };
    let db_delta = if db_mode {
        mirror_meta.db_hybrid_queries
    } else {
        0
    };

    let hops = cand.hops.clone();
    let ops = apply_hybrid_by_hop(cand.ops, &hybrid_plan)?;
    let estimated =
        crate::api::route_solver::maybe_simulate(&state, Some(&amount_raw), &ops, &quote_trader)
            .await?;

    let out_u = estimated
        .as_ref()
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);

    let grid_out = if db_mode { grid_out } else { out_u };

    let quote_kind = quote_kind_for(&opt_meta, &estimated, db_mode);
    let resolved_route = crate::api::route_solver::ResolvedRoute {
        token_in: token_in.clone(),
        token_out: token_out.clone(),
        hops: hops.clone(),
        ops: ops.clone(),
    };
    let intermediate_tokens = build_intermediate_tokens(&resolved_route);
    let body = RouteSolveResponse {
        token_in,
        token_out,
        hops,
        intermediate_tokens,
        quote_kind,
        hybrid_notes: None,
        router_operations: ops,
        estimated_amount_out: estimated,
        solver_version: Some(solver_version.to_string()),
        paths_considered: None,
        optimality_scope: None,
        lcd_hybrid_queries: None,
        db_hybrid_queries: None,
        fidelity_check: None,
        mirror_max_block_lag: None,
        search_truncated: None,
        spot_amount_out: None,
        slippage_percent: None,
        token_in_price_quote: None,
        token_out_price_quote: None,
    };

    Ok(Some(CandidateEval {
        index,
        body,
        out_u,
        grid_out,
        lcd_delta,
        db_delta,
        degraded: opt_meta.degraded,
        any_book_leg: opt_meta.any_book_leg,
        mirror_stale: mirror_meta.mirror_stale_hops > 0,
        mirror_missing: mirror_meta.mirror_missing_hops > 0,
        max_snapshot_age_ms: mirror_meta.max_snapshot_age_ms,
    }))
}

/// Fan out path-candidate evaluation under `concurrency_cap` (#324).
/// Skips candidates that fail simulation or yield zero DB-hybrid output (#369).
async fn run_concurrent_candidate_evaluations(
    state: &AppState,
    candidates: &[PathCandidate],
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    amount_raw: &str,
    max_maker_fills: u32,
    quote_trader: &hybrid_route_opt::QuoteTrader,
    db_mode: bool,
    discount_bps: u16,
    mirrors: Arc<HashMap<String, db_orderbook_sim::HopMirror>>,
    solver_version: &'static str,
    concurrency_cap: usize,
) -> Result<(Vec<CandidateEval>, bool), (StatusCode, String)> {
    let cap = concurrency_cap.max(1);
    let search_truncated = candidates.len() > cap;
    let eval_count = candidates.len().min(cap);

    let state = Arc::new(state.clone());
    let token_in = token_in.trim().to_string();
    let token_out = token_out.trim().to_string();
    let amount_raw = amount_raw.to_string();
    let quote_trader = quote_trader.clone();

    let mut join_set = tokio::task::JoinSet::new();
    let mut next_idx = 0usize;

    while next_idx < eval_count.min(cap) {
        let cand = candidates[next_idx].clone();
        let idx = next_idx;
        let st = Arc::clone(&state);
        let mirrors = Arc::clone(&mirrors);
        let ti = token_in.clone();
        let to = token_out.clone();
        let ar = amount_raw.clone();
        let qt = quote_trader.clone();
        let sv = solver_version;
        join_set.spawn(async move {
            evaluate_candidate(
                st,
                idx,
                cand,
                ti,
                to,
                amount_in,
                ar,
                max_maker_fills,
                qt,
                db_mode,
                discount_bps,
                mirrors,
                sv,
            )
            .await
        });
        next_idx += 1;
    }

    let mut evals = Vec::with_capacity(eval_count);
    let mut gateway_err: Option<(StatusCode, String)> = None;
    let mut gateway_err_count = 0u32;
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(Ok(Some(eval))) => {
                let viable = !db_mode || eval.grid_out > 0;
                if viable {
                    evals.push(eval);
                } else {
                    tracing::debug!(
                        index = eval.index,
                        "skip route candidate: zero DB hybrid output"
                    );
                }
                if next_idx < eval_count {
                    let cand = candidates[next_idx].clone();
                    let idx = next_idx;
                    let st = Arc::clone(&state);
                    let mirrors = Arc::clone(&mirrors);
                    let ti = token_in.clone();
                    let to = token_out.clone();
                    let ar = amount_raw.clone();
                    let qt = quote_trader.clone();
                    let sv = solver_version;
                    join_set.spawn(async move {
                        evaluate_candidate(
                            st,
                            idx,
                            cand,
                            ti,
                            to,
                            amount_in,
                            ar,
                            max_maker_fills,
                            qt,
                            db_mode,
                            discount_bps,
                            mirrors,
                            sv,
                        )
                        .await
                    });
                    next_idx += 1;
                }
            }
            Ok(Ok(None)) => {
                if next_idx < eval_count {
                    let cand = candidates[next_idx].clone();
                    let idx = next_idx;
                    let st = Arc::clone(&state);
                    let mirrors = Arc::clone(&mirrors);
                    let ti = token_in.clone();
                    let to = token_out.clone();
                    let ar = amount_raw.clone();
                    let qt = quote_trader.clone();
                    let sv = solver_version;
                    join_set.spawn(async move {
                        evaluate_candidate(
                            st,
                            idx,
                            cand,
                            ti,
                            to,
                            amount_in,
                            ar,
                            max_maker_fills,
                            qt,
                            db_mode,
                            discount_bps,
                            mirrors,
                            sv,
                        )
                        .await
                    });
                    next_idx += 1;
                }
            }
            Ok(Err(e)) => {
                tracing::debug!(
                    status = %e.0,
                    detail = %e.1,
                    "skip route candidate: evaluation failed"
                );
                gateway_err_count = gateway_err_count.saturating_add(1);
                if gateway_err.is_none() {
                    gateway_err = Some(e);
                }
                if next_idx < eval_count {
                    let cand = candidates[next_idx].clone();
                    let idx = next_idx;
                    let st = Arc::clone(&state);
                    let mirrors = Arc::clone(&mirrors);
                    let ti = token_in.clone();
                    let to = token_out.clone();
                    let ar = amount_raw.clone();
                    let qt = quote_trader.clone();
                    let sv = solver_version;
                    join_set.spawn(async move {
                        evaluate_candidate(
                            st,
                            idx,
                            cand,
                            ti,
                            to,
                            amount_in,
                            ar,
                            max_maker_fills,
                            qt,
                            db_mode,
                            discount_bps,
                            mirrors,
                            sv,
                        )
                        .await
                    });
                    next_idx += 1;
                }
            }
            Err(e) => {
                join_set.abort_all();
                return Err(crate::api::internal_err(e));
            }
        }
    }

    if evals.is_empty()
        && gateway_err_count > 0
        && gateway_err_count == eval_count as u32
    {
        return Err(gateway_err.expect("gateway_err_count > 0"));
    }

    Ok((evals, search_truncated))
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
    solve_global_best_execution_inner(
        state,
        token_in,
        token_out,
        amount_in,
        amount_raw,
        max_maker_fills,
        quote_trader,
        true,
    )
    .await
}

pub(crate) async fn solve_global_best_execution_inner(
    state: &AppState,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    amount_raw: &str,
    max_maker_fills: u32,
    quote_trader: &hybrid_route_opt::QuoteTrader,
    enrich_slippage: bool,
) -> Result<(RouteSolveResponse, BestExecutionMeta), (StatusCode, String)> {
    let solver_version = solver_version_for(state);
    let db_mode = state.route_solver_db_hybrid;
    let candidates =
        enumerate_path_candidates(&state.pool, token_in, token_out, GET_DEFAULT_MAX_HOPS).await?;

    let discount_bps = crate::api::route_solver::resolve_discount_bps(state, quote_trader).await;

    let mut mirrors: HashMap<String, db_orderbook_sim::HopMirror> = HashMap::new();
    if db_mode {
        let all_assets = assets::get_all_assets(&state.pool)
            .await
            .map_err(crate::api::internal_err)?;
        let (id_to_addr, _) = crate::api::route_solver::build_id_to_addr_map(&all_assets);
        let mut pair_addrs: HashSet<String> = HashSet::new();
        for c in &candidates {
            for h in &c.hops {
                pair_addrs.insert(h.pair.clone());
            }
        }
        let addrs: Vec<String> = pair_addrs.into_iter().collect();
        mirrors = db_orderbook_sim::preload_mirrors_for_pairs(
            &state.pool,
            &addrs,
            &id_to_addr,
            state.book_snapshot_max_staleness_ms,
        )
        .await
        .map_err(crate::api::internal_err)?;
    }

    let mirrors = Arc::new(mirrors);
    let (evals, search_truncated) = run_concurrent_candidate_evaluations(
        state,
        &candidates,
        token_in,
        token_out,
        amount_in,
        amount_raw,
        max_maker_fills,
        quote_trader,
        db_mode,
        discount_bps,
        mirrors,
        solver_version,
        SOLVE_CONCURRENCY,
    )
    .await?;

    let (mut body, _router_out, grid_out, mut meta) = merge_candidate_evaluations(
        &evals,
        candidates.len(),
        search_truncated,
    )
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("no viable route within {} hops", GET_DEFAULT_MAX_HOPS),
        )
    })?;

    if db_mode {
        let final_est = apply_fidelity_guard(
            &mut meta,
            grid_out,
            &body.estimated_amount_out,
            state.route_fidelity_drift_bps,
        );
        body.estimated_amount_out = final_est;
        if meta.fidelity_check == FidelityCheck::Drift {
            body.quote_kind = quote_kind_for(
                &OptimizationMeta {
                    degraded: meta.degraded,
                    any_book_leg: meta.any_book_leg,
                    mirror_stale: meta.mirror_stale,
                    mirror_missing: meta.mirror_missing,
                },
                &body.estimated_amount_out,
                true,
            );
        } else {
            body.quote_kind = quote_kind_for(
                &OptimizationMeta {
                    degraded: meta.degraded,
                    any_book_leg: meta.any_book_leg,
                    mirror_stale: meta.mirror_stale,
                    mirror_missing: meta.mirror_missing,
                },
                &body.estimated_amount_out,
                true,
            );
        }
    }

    body.hybrid_notes = Some(hybrid_notes_for_global(&meta, solver_version));
    body.solver_version = Some(solver_version.to_string());
    body.paths_considered = Some(meta.paths_considered);
    body.optimality_scope = Some(OPTIMALITY_SCOPE.to_string());
    body.lcd_hybrid_queries = Some(meta.lcd_hybrid_queries);
    body.db_hybrid_queries = Some(meta.db_hybrid_queries);
    body.fidelity_check = Some(meta.fidelity_check);
    body.mirror_max_block_lag = meta.mirror_max_block_lag;
    body.search_truncated = Some(meta.search_truncated);

    tracing::info!(
        solver_version,
        paths_considered = meta.paths_considered,
        db_queries = meta.db_hybrid_queries,
        lcd_queries = meta.lcd_hybrid_queries,
        degraded = meta.degraded,
        mirror_stale = meta.mirror_stale,
        fidelity_check = meta.fidelity_check.as_str(),
        search_truncated = meta.search_truncated,
        "route best execution"
    );

    if enrich_slippage {
        crate::api::route_slippage::enrich_route_slippage(
            state,
            &mut body,
            amount_raw,
            quote_trader,
            max_maker_fills,
            None,
        )
        .await;
    }

    Ok((body, meta))
}

#[cfg(test)]
mod concurrent_solve_tests {
    use super::{merge_candidate_evaluations, CandidateEval, RouteHop, SOLVE_CONCURRENCY};
    use crate::api::route_solver::{RouteQuoteKind, RouteSolveResponse};
    use std::time::{Duration, Instant};

    fn stub_eval(index: usize, out_u: u128, lcd_delta: u32, db_delta: u32) -> CandidateEval {
        CandidateEval {
            index,
            body: RouteSolveResponse {
                token_in: "in".into(),
                token_out: "out".into(),
                hops: vec![RouteHop {
                    pair: format!("pair{index}"),
                    offer_token: "in".into(),
                    ask_token: "out".into(),
                }],
                intermediate_tokens: vec!["in".into(), "out".into()],
                quote_kind: RouteQuoteKind::IndexerHybridLcd,
                hybrid_notes: None,
                router_operations: vec![],
                estimated_amount_out: Some(out_u.to_string()),
                solver_version: None,
                paths_considered: None,
                optimality_scope: None,
                lcd_hybrid_queries: None,
                db_hybrid_queries: None,
                fidelity_check: None,
                mirror_max_block_lag: None,
                search_truncated: None,
                spot_amount_out: None,
                slippage_percent: None,
                token_in_price_quote: None,
                token_out_price_quote: None,
            },
            out_u,
            grid_out: out_u,
            lcd_delta,
            db_delta,
            degraded: index % 2 == 1,
            any_book_leg: index > 0,
            mirror_stale: false,
            mirror_missing: false,
            max_snapshot_age_ms: 0,
        }
    }

    #[test]
    fn merge_picks_max_output_with_first_seen_tie_break() {
        let evals = vec![
            stub_eval(0, 100, 10, 1),
            stub_eval(1, 200, 20, 2),
            stub_eval(2, 200, 30, 3),
        ];
        let (_, out, _, meta) = merge_candidate_evaluations(&evals, 3, false).unwrap();
        assert_eq!(out, 200);
        assert_eq!(meta.lcd_hybrid_queries, 30, "cumulative through winner index 1");
        assert_eq!(meta.db_hybrid_queries, 3);
        assert!(meta.degraded, "winner index 1 has degraded=true");
    }

    #[test]
    fn merge_equal_output_keeps_first_seen_candidate() {
        let evals = vec![stub_eval(0, 500, 5, 0), stub_eval(1, 500, 7, 0)];
        let (body, _, _, meta) = merge_candidate_evaluations(&evals, 2, false).unwrap();
        assert_eq!(body.hops[0].pair, "pair0");
        assert_eq!(meta.lcd_hybrid_queries, 5);
    }

    #[test]
    fn merge_winner_at_end_uses_cumulative_queries() {
        let evals = vec![
            stub_eval(0, 100, 10, 0),
            stub_eval(1, 150, 15, 0),
            stub_eval(2, 300, 25, 0),
        ];
        let (_, _, _, meta) = merge_candidate_evaluations(&evals, 3, false).unwrap();
        assert_eq!(meta.lcd_hybrid_queries, 50);
    }

    #[test]
    fn merge_sets_search_truncated_flag() {
        let evals = vec![stub_eval(0, 100, 10, 0)];
        let (_, _, _, meta) = merge_candidate_evaluations(&evals, 5, true).unwrap();
        assert!(meta.search_truncated);
        assert_eq!(meta.paths_considered, 5);
    }

    #[test]
    fn hybrid_notes_warn_when_search_truncated() {
        use super::hybrid_notes_for_global;
        let meta = super::BestExecutionMeta {
            paths_considered: 5,
            search_truncated: true,
            ..Default::default()
        };
        let notes = hybrid_notes_for_global(&meta, "global_v4");
        assert!(
            notes.contains("truncated"),
            "hybrid_notes must warn when search_truncated: {notes}"
        );
    }

    #[tokio::test]
    async fn concurrent_fanout_latency_tracks_max_not_sum() {
        let delay = Duration::from_millis(80);
        let n = 5usize;
        let cap = SOLVE_CONCURRENCY;
        let start = Instant::now();
        let mut join_set = tokio::task::JoinSet::new();
        let mut next = 0usize;
        while next < n.min(cap) {
            let idx = next;
            join_set.spawn(async move {
                tokio::time::sleep(delay).await;
                idx
            });
            next += 1;
        }
        while let Some(res) = join_set.join_next().await {
            let _ = res.expect("task join");
            if next < n {
                let idx = next;
                join_set.spawn(async move {
                    tokio::time::sleep(delay).await;
                    idx
                });
                next += 1;
            }
        }
        let elapsed = start.elapsed();
        let serial_floor = delay * n as u32;
        assert!(
            elapsed < serial_floor,
            "expected ~{delay:?} concurrent wall time, got {elapsed:?} (serial would be ~{serial_floor:?})"
        );
        assert!(
            elapsed >= delay,
            "expected at least one full delay, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn concurrent_eval_skips_failed_candidates_and_continues() {
        let mut join_set = tokio::task::JoinSet::new();
        join_set.spawn(async {
            Err::<u32, _>((
                axum::http::StatusCode::BAD_REQUEST,
                "router simulation failed".into(),
            ))
        });
        join_set.spawn(async {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok::<_, (axum::http::StatusCode, String)>(2)
        });
        let mut evals = Vec::new();
        while let Some(joined) = join_set.join_next().await {
            match joined.unwrap() {
                Ok(v) => evals.push(v),
                Err(_) => {}
            }
        }
        assert_eq!(evals, vec![2], "failed candidate skipped; later candidate kept");
    }
}
