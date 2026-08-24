//! Global best-execution route solver: top-K path enumeration + joint hybrid optimization (GitLab #209).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use crate::api::AppState;
use crate::api::db_orderbook_sim::{self, MirrorLoadMeta};
use crate::api::hybrid_route_opt::{
    self, HopDescriptor, HybridSimError, HybridSimSource, OptimizationMeta,
};
use crate::api::route_graph::{self, RouteGraphSnapshot};
use crate::api::route_paths;
use crate::api::route_solve_progress;
use crate::api::route_solver::{
    FidelityCheck, GET_DEFAULT_MAX_HOPS, RouteHop, RouteQuoteKind, RouteSolveResponse,
    apply_hybrid_by_hop, build_hops_and_ops, build_intermediate_tokens, quote_kind_after_sim,
};
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
pub const OPTIMALITY_SCOPE: &str = "optimal within top-5 simple paths by hop count and per-hop hybrid split grid (17 book fractions), with 2-pass coordinate refinement across hops";

/// Additive #615 note: rank is catalog net; hop sims unchanged.
pub const TAX_RANK_NOTE: &str = "Ranking is net of catalog buy/sell policy for this snapshot (GitLab #615); hop LCD/DB sims unchanged";

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
         {TAX_RANK_NOTE}. \
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
    snapshot: &RouteGraphSnapshot,
    token_in: &str,
    token_out: &str,
    max_hops: usize,
) -> Result<Vec<PathCandidate>, (StatusCode, String)> {
    let (id_to_addr, addr_to_id) = (&snapshot.id_to_addr, &snapshot.addr_to_id);

    let start = crate::api::route_solver::resolve_id(addr_to_id, token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".to_string(),
        )
    })?;
    let goal = crate::api::route_solver::resolve_id(addr_to_id, token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".to_string(),
        )
    })?;

    let pair_rows = Arc::clone(&snapshot.pairs);
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
        let (hops, ops) = build_hops_and_ops(&hops_raw, id_to_addr)?;
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
    /// Catalog net used for winner compare (#615). Equal to `out_u` when no buy tax.
    net_u: u128,
    grid_out: u128,
    lcd_delta: u32,
    db_delta: u32,
    degraded: bool,
    any_book_leg: bool,
    mirror_stale: bool,
    mirror_missing: bool,
    max_snapshot_age_ms: u64,
}

/// Merge per-candidate results: max `net_u` (catalog net, #615) with first-seen (lowest index)
/// tie-break; cumulative query counts through the winner index match the serial loop (#324).
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
        let replace = winner.map(|w| ev.net_u > w.net_u).unwrap_or(true);
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

    Some((winner.body.clone(), winner.out_u, winner.grid_out, meta))
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

    let (hybrid_plan, opt_meta, grid_out) = match hybrid_route_opt::optimize_multihop_hybrid_joint(
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
        Err(HybridSimError::Db(db_orderbook_sim::DbSimError::InsufficientLiquidity)) => {
            tracing::debug!(
                path_index = index,
                hops = hops_desc.len(),
                "skip path candidate: zero-reserve pool leg"
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
        estimated_amount_out_net: None,
        tax_kind: None,
        buy_tax_bps: None,
        sell_tax_bps: None,
        tax_notes: None,
        router_hops_tax: None,
    };

    Ok(Some(CandidateEval {
        index,
        body,
        out_u,
        net_u: out_u,
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

async fn preload_mirrors_with_progress(
    pool: &PgPool,
    addrs: &[String],
    id_to_addr: &HashMap<i32, String>,
    max_staleness_ms: u64,
    progress_key: Option<&str>,
) -> Result<HashMap<String, db_orderbook_sim::HopMirror>, (StatusCode, String)> {
    let total = addrs.len() as u32;
    if let Some(pk) = progress_key {
        route_solve_progress::progress_update(
            pk,
            route_solve_progress::STAGE_LOADING_MIRRORS,
            0,
            total,
            format!("Loading mirrors 0 of {total} pairs…"),
        );
    }

    let mut mirrors = HashMap::new();
    if progress_key.is_some() {
        use crate::api::db_orderbook_sim::{
            DbSimError, HopMirror, MirrorFreshness, load_hop_mirror,
        };
        use crate::db::queries::pairs;
        use chrono::Utc;

        let now_secs = Utc::now().timestamp().max(0) as u64;
        for (i, addr) in addrs.iter().enumerate() {
            let done = (i + 1) as u32;
            if let Some(pk) = progress_key {
                route_solve_progress::progress_update(
                    pk,
                    route_solve_progress::STAGE_LOADING_MIRRORS,
                    done,
                    total,
                    format!("Loading mirrors {done} of {total} pairs…"),
                );
            }
            let Some(pair_row) = pairs::get_pair_by_address(pool, addr)
                .await
                .map_err(crate::api::internal_err)?
            else {
                continue;
            };
            match load_hop_mirror(
                pool,
                addr,
                id_to_addr,
                &pair_row,
                max_staleness_ms,
                now_secs,
            )
            .await
            {
                Ok(m) => {
                    mirrors.insert(addr.clone(), m);
                }
                Err(DbSimError::MissingMirror) => {
                    mirrors.insert(
                        addr.clone(),
                        HopMirror {
                            pair_id: pair_row.id,
                            asset_0_addr: id_to_addr
                                .get(&pair_row.asset_0_id)
                                .cloned()
                                .unwrap_or_default(),
                            asset_1_addr: id_to_addr
                                .get(&pair_row.asset_1_id)
                                .cloned()
                                .unwrap_or_default(),
                            reserve_0: 0,
                            reserve_1: 0,
                            fee_bps: pair_row.fee_bps.unwrap_or(30) as u16,
                            block_height: None,
                            snapshot_at: Utc::now(),
                            freshness: MirrorFreshness::MissingReserves,
                            bids: vec![],
                            asks: vec![],
                        },
                    );
                }
                Err(e) => {
                    tracing::warn!(pair = %addr, "preload mirror failed: {}", e);
                }
            }
        }
    } else {
        mirrors =
            db_orderbook_sim::preload_mirrors_for_pairs(pool, addrs, id_to_addr, max_staleness_ms)
                .await
                .map_err(crate::api::internal_err)?;
    }

    Ok(mirrors)
}

/// Fan out path-candidate evaluation under `concurrency_cap` (#324).
/// Skips candidates with unusable/zero-reserve pool legs or zero DB-hybrid output (#369).
/// Fail-fast only when every evaluated candidate hits a fatal gateway error.
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
    progress_key: Option<&str>,
) -> Result<(Vec<CandidateEval>, bool), (StatusCode, String)> {
    let cap = concurrency_cap.max(1);
    let search_truncated = candidates.len() > cap;
    let eval_count = candidates.len().min(cap);

    if let Some(pk) = progress_key {
        route_solve_progress::progress_update(
            pk,
            route_solve_progress::STAGE_EVALUATING,
            0,
            eval_count as u32,
            format!("Searching 0 of {eval_count} paths…"),
        );
    }

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
    let mut completed = 0u32;
    let mut gateway_err: Option<(StatusCode, String)> = None;
    let mut gateway_err_count = 0u32;
    while let Some(joined) = join_set.join_next().await {
        completed = completed.saturating_add(1);
        if let Some(pk) = progress_key {
            route_solve_progress::progress_update(
                pk,
                route_solve_progress::STAGE_EVALUATING,
                completed,
                eval_count as u32,
                format!("Searching {completed} of {eval_count} paths…"),
            );
        }
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

    if evals.is_empty() && gateway_err_count > 0 && gateway_err_count == eval_count as u32 {
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
    progress_key: Option<&str>,
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
        progress_key,
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
    progress_key: Option<&str>,
) -> Result<(RouteSolveResponse, BestExecutionMeta), (StatusCode, String)> {
    let total_start = Instant::now();
    let solver_version = solver_version_for(state);
    let db_mode = state.route_solver_db_hybrid;

    let graph_start = Instant::now();
    if let Some(pk) = progress_key {
        route_solve_progress::progress_update(
            pk,
            route_solve_progress::STAGE_GRAPH_LOAD,
            0,
            0,
            "Loading token graph…",
        );
    }
    let snapshot = route_graph::get_route_graph_snapshot(&state.pool).await?;
    let graph_ms = graph_start.elapsed().as_millis();

    let enum_start = Instant::now();
    if let Some(pk) = progress_key {
        route_solve_progress::progress_update(
            pk,
            route_solve_progress::STAGE_ENUMERATING,
            0,
            0,
            "Enumerating paths…",
        );
    }
    let candidates =
        enumerate_path_candidates(&snapshot, token_in, token_out, GET_DEFAULT_MAX_HOPS).await?;
    let enum_ms = enum_start.elapsed().as_millis();

    let hop_extras: Vec<String> = candidates
        .iter()
        .flat_map(|c| crate::api::community_tax_rank::extra_addrs_from_hops(&c.hops))
        .collect();
    let tax_snap = crate::api::community_tax_rank::load_tax_rank_snapshot(
        state,
        token_in,
        token_out,
        &hop_extras,
        quote_trader.trader.as_deref(),
    )
    .await;
    let candidates: Vec<PathCandidate> = candidates
        .into_iter()
        .filter(|c| {
            !crate::api::community_tax_rank::path_sells_middle_tax_hop(&c.hops, token_in, &tax_snap)
        })
        .collect();
    if candidates.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("no viable route within {} hops", GET_DEFAULT_MAX_HOPS),
        ));
    }

    let discount_bps = crate::api::route_solver::resolve_discount_bps(state, quote_trader).await;

    let mirror_start = Instant::now();
    let mut mirrors: HashMap<String, db_orderbook_sim::HopMirror> = HashMap::new();
    if db_mode {
        let id_to_addr = &snapshot.id_to_addr;
        let mut pair_addrs: HashSet<String> = HashSet::new();
        for c in &candidates {
            for h in &c.hops {
                pair_addrs.insert(h.pair.clone());
            }
        }
        let addrs: Vec<String> = pair_addrs.into_iter().collect();
        mirrors = preload_mirrors_with_progress(
            &state.pool,
            &addrs,
            id_to_addr,
            state.book_snapshot_max_staleness_ms,
            progress_key,
        )
        .await?;
    }
    let mirror_ms = mirror_start.elapsed().as_millis();

    let mirrors = Arc::new(mirrors);
    let candidate_start = Instant::now();
    let (mut evals, search_truncated) = run_concurrent_candidate_evaluations(
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
        progress_key,
    )
    .await?;
    let candidate_ms = candidate_start.elapsed().as_millis();

    for ev in &mut evals {
        let scored = crate::api::community_tax_rank::score_path(
            ev.out_u,
            &ev.body.hops,
            token_in,
            &tax_snap,
        );
        ev.net_u = scored.net_out;
        apply_tax_rank_fields(&mut ev.body, &scored);
    }

    let (mut body, _router_out, grid_out, mut meta) =
        merge_candidate_evaluations(&evals, candidates.len(), search_truncated).ok_or_else(
            || {
                (
                    StatusCode::NOT_FOUND,
                    format!("no viable route within {} hops", GET_DEFAULT_MAX_HOPS),
                )
            },
        )?;

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
    body.optimality_scope = Some(format!("{OPTIMALITY_SCOPE}. {TAX_RANK_NOTE}"));
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
        graph_ms,
        enum_ms,
        mirror_ms,
        candidate_ms,
        total_ms = total_start.elapsed().as_millis(),
        "route best execution"
    );

    if enrich_slippage {
        if let Some(pk) = progress_key {
            route_solve_progress::progress_update(
                pk,
                route_solve_progress::STAGE_ENRICHING,
                0,
                0,
                "Computing slippage…",
            );
        }
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

fn apply_tax_rank_fields(
    body: &mut RouteSolveResponse,
    scored: &crate::api::community_tax_rank::TaxRankResult,
) {
    body.estimated_amount_out_net = Some(scored.net_out.to_string());
    body.tax_kind = Some(scored.tax_kind.to_string());
    body.buy_tax_bps = Some(scored.buy_tax_bps);
    body.sell_tax_bps = Some(scored.sell_tax_bps);
    body.tax_notes = Some(scored.tax_notes.clone());
    body.router_hops_tax = Some(scored.router_hops_tax);
}

#[cfg(test)]
mod concurrent_solve_tests {
    use super::{CandidateEval, RouteHop, SOLVE_CONCURRENCY, merge_candidate_evaluations};
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
                estimated_amount_out_net: None,
                tax_kind: None,
                buy_tax_bps: None,
                sell_tax_bps: None,
                tax_notes: None,
                router_hops_tax: None,
            },
            out_u,
            net_u: out_u,
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
        assert_eq!(
            meta.lcd_hybrid_queries, 30,
            "cumulative through winner index 1"
        );
        assert_eq!(meta.db_hybrid_queries, 3);
        assert!(meta.degraded, "winner index 1 has degraded=true");
    }

    #[test]
    fn merge_picks_max_net_not_raw() {
        let mut low_net = stub_eval(0, 200, 1, 0);
        low_net.net_u = 100;
        let mut high_net = stub_eval(1, 150, 1, 0);
        high_net.net_u = 140;
        let (body, raw, _, _) =
            merge_candidate_evaluations(&[low_net, high_net], 2, false).unwrap();
        assert_eq!(body.hops[0].pair, "pair1");
        assert_eq!(raw, 150);
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
        assert_eq!(
            evals,
            vec![2],
            "failed candidate skipped; later candidate kept"
        );
    }
}
