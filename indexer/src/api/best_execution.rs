//! Global best-execution route solver: top-K path enumeration + joint hybrid optimization (GitLab #209).

use std::collections::{HashMap, HashSet};

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
pub const SOLVER_VERSION_LCD: &str = "global_v1";

/// Postgres-mirror hybrid grid (#319 Phase 1c).
pub const SOLVER_VERSION_DB: &str = "global_v2";

/// Max simple paths evaluated per request (hop-count order).
pub const MAX_PATH_CANDIDATES: usize = 5;

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
    format!(
        "Global best-execution solver ({solver_version}): {OPTIMALITY_SCOPE}. \
         Evaluated {} path(s); {} db-hybrid + {} lcd-hybrid grid evals. \
         {pricing}. \
         Final output validated via router simulate_swap_operations when configured (fidelity_check={}). \
         Execution on-chain may differ from mirror/LCD snapshots.",
        meta.paths_considered,
        meta.db_hybrid_queries,
        meta.lcd_hybrid_queries,
        meta.fidelity_check.as_str(),
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
    let solver_version = solver_version_for(state);
    let db_mode = state.route_solver_db_hybrid;
    let candidates =
        enumerate_path_candidates(&state.pool, token_in, token_out, GET_DEFAULT_MAX_HOPS).await?;

    let discount_tier = crate::api::route_solver::resolve_discount_tier(state, quote_trader).await;

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

    let mut best: Option<(RouteSolveResponse, u128, u128, BestExecutionMeta)> = None;
    let mut lcd_queries: u32 = 0;
    let mut db_queries: u32 = 0;

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

        let queries_before_lcd = lcd_queries;
        let queries_before_db = db_queries;
        let mut mirror_meta = MirrorLoadMeta::default();

        let source = if db_mode {
            HybridSimSource::Db {
                lcd_fallback: &state.lcd,
                mirrors: &mirrors,
                discount_tier,
            }
        } else {
            HybridSimSource::Lcd(&state.lcd)
        };

        let mm = if db_mode {
            Some(&mut mirror_meta)
        } else {
            None
        };

        let (hybrid_plan, opt_meta, grid_out) = hybrid_route_opt::optimize_multihop_hybrid_joint(
            &source,
            mm,
            &hops_desc,
            amount_in,
            max_maker_fills,
            quote_trader,
        )
        .await
        .map_err(hybrid_sim_gateway_err)?;

        if db_mode {
            db_queries = queries_before_db.saturating_add(mirror_meta.db_hybrid_queries);
            lcd_queries = queries_before_lcd.saturating_add(mirror_meta.lcd_fallback_queries);
        } else {
            lcd_queries = queries_before_lcd.saturating_add(estimate_lcd_calls(hops_desc.len()));
        }

        let token_in_t = token_in.trim().to_string();
        let token_out_t = token_out.trim().to_string();
        let hops = cand.hops.clone();
        let ops = apply_hybrid_by_hop(cand.ops.clone(), &hybrid_plan)?;
        let estimated =
            crate::api::route_solver::maybe_simulate(state, Some(amount_raw), &ops, quote_trader)
                .await?;

        let out_u = estimated
            .as_ref()
            .and_then(|s| s.parse::<u128>().ok())
            .unwrap_or(0);

        let grid_out = if db_mode { grid_out } else { out_u };

        let mut path_meta = BestExecutionMeta {
            paths_considered: candidates.len() as u32,
            lcd_hybrid_queries: lcd_queries,
            db_hybrid_queries: db_queries,
            degraded: opt_meta.degraded,
            any_book_leg: opt_meta.any_book_leg,
            mirror_stale: mirror_meta.mirror_stale_hops > 0,
            mirror_missing: mirror_meta.mirror_missing_hops > 0,
            mirror_max_block_lag: None,
            max_snapshot_age_ms: mirror_meta.max_snapshot_age_ms,
            fidelity_check: FidelityCheck::Skipped,
            db_optimized_amount_out: None,
        };

        let quote_kind = quote_kind_for(&opt_meta, &estimated, db_mode);
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
            solver_version: Some(solver_version.to_string()),
            paths_considered: None,
            optimality_scope: None,
            lcd_hybrid_queries: None,
            db_hybrid_queries: None,
            fidelity_check: None,
            mirror_max_block_lag: None,
        };

        let replace = best
            .as_ref()
            .map(|(_, prev_out, _, _)| out_u > *prev_out)
            .unwrap_or(true);
        if replace {
            best = Some((body, out_u, grid_out, path_meta));
        }
    }

    let (mut body, _router_out, grid_out, mut meta) = best.expect("candidates non-empty");

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

    tracing::info!(
        solver_version,
        paths_considered = meta.paths_considered,
        db_queries = meta.db_hybrid_queries,
        lcd_queries = meta.lcd_hybrid_queries,
        degraded = meta.degraded,
        mirror_stale = meta.mirror_stale,
        fidelity_check = meta.fidelity_check.as_str(),
        "route best execution"
    );

    Ok((body, meta))
}
