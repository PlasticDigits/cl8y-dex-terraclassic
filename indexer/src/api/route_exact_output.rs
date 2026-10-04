//! #1376 receive sizing. Only monotonic pool operations are inverted; book partitions are never
//! assumed monotonic. No executable response cache: every exact target gets fresh validation.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration;

use axum::{http::StatusCode, Json};
use bigdecimal::{num_bigint::BigUint, ToPrimitive};
use serde_json::Value;
use tokio::sync::Semaphore;

use super::{
    best_execution, community_tax_rank as tax, db_orderbook_sim as dbsim,
    hybrid_route_opt::QuoteTrader,
    route_exact_output_reference as reference, route_graph, route_paths,
    route_solver::{
        self, FidelityCheck, RouteHop, RouteQuoteKind, RouteSolveResponse, SolveRouteParams,
    },
    AppState,
};

type Error = (StatusCode, String);
pub const TIME_BUDGET: Duration = Duration::from_secs(20);
pub const VALIDATION_TIMEOUT: Duration = Duration::from_secs(3);
pub const MAX_CONCURRENT_REQUESTS: usize = 4;
pub const PATH_WORK_BUDGET: u64 = 100_000;
pub const MAX_GRAPH_PAIRS: usize = 8192;
pub const OPTIMALITY_SCOPE: &str = "minimum raw offer within the first 5 feasible fresh pool-only simple paths among 32 hop-count-ordered structural paths, at most 4 hops; indexed snapshot minimum, fresh router validation required; excludes hybrid books and option-2 taxed middle hops";

pub(crate) fn validate_request(q: &SolveRouteParams) -> Result<u128, Error> {
    if q.amount_in.is_some() || q.hybrid_optimize == Some(true) || q.pool_only == Some(false) {
        return Err((StatusCode::BAD_REQUEST,
            "amount_out requires pool-only sizing and cannot be combined with amount_in or hybrid optimization".into()));
    }
    let raw = q.amount_out.as_deref().unwrap_or("");
    if raw.is_empty() || raw.len() > 39 || !raw.bytes().all(|c| c.is_ascii_digit()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "amount_out must be a positive Uint128 raw integer".into(),
        ));
    }
    raw.parse::<u128>().ok().filter(|n| *n > 0).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "amount_out must be a positive Uint128 raw integer".into(),
        )
    })
}

/// Smallest gross such that gross-floor(gross*bps/10000) >= target, with wide products.
fn inverse_net(target: u128, bps: u16) -> Option<u128> {
    if target == 0 || bps >= 10_000 {
        return None;
    }
    ((BigUint::from(target - 1) * BigUint::from(10_000u32)
        / BigUint::from(10_000u32 - u32::from(bps)))
        + BigUint::from(1u32))
    .to_u128()
}

/// Invert the pair's exact integer constant product and output-side fee. No search or LCD loop.
fn inverse_pool(m: &dbsim::HopMirror, offer: &str, target: u128, discount: u16) -> Option<u128> {
    if m.freshness != dbsim::MirrorFreshness::Fresh || discount > 10_000 || m.fee_bps > 10_000 {
        return None;
    }
    let (rin, rout) = if offer.eq_ignore_ascii_case(&m.asset_0_addr) {
        (m.reserve_0, m.reserve_1)
    } else if offer.eq_ignore_ascii_case(&m.asset_1_addr) {
        (m.reserve_1, m.reserve_0)
    } else {
        return None;
    };
    let fee = dbsim::effective_fee_bps_from_discount_bps(m.fee_bps, discount);
    let gross = inverse_net(target, fee)?;
    if rin == 0 || gross >= rout {
        return None;
    }
    let den = BigUint::from(rout - gross);
    let new_in = ((BigUint::from(rin) * BigUint::from(rout) + &den - BigUint::from(1u32)) / den)
        .to_u128()?;
    let amount = new_in.checked_sub(rin)?;
    let out = dbsim::simulate_pool_only_from_mirror(m, offer, amount, discount).ok()?;
    if out < target {
        return None;
    }
    Some(amount)
}

fn size_path(
    hops: &[RouteHop],
    mirrors: &HashMap<String, dbsim::HopMirror>,
    target: u128,
    discount: u16,
    tax_snapshot: &tax::TaxRankSnapshot,
    token_in: &str,
) -> Option<(u128, u128)> {
    if hops.is_empty() {
        return None;
    }
    let tax_info = tax::score_path(target, hops, token_in, tax_snapshot);
    if !tax_info.eligible {
        return None;
    }
    let mut required = inverse_net(target, tax_info.buy_tax_bps)?;
    for h in hops.iter().rev() {
        required = inverse_pool(mirrors.get(&h.pair)?, &h.offer_token, required, discount)?;
    }
    let mut out = required;
    for h in hops {
        let m = mirrors.get(&h.pair)?;
        out = dbsim::simulate_pool_only_from_mirror(m, &h.offer_token, out, discount).ok()?;
        let rout = if h.offer_token.eq_ignore_ascii_case(&m.asset_0_addr) {
            m.reserve_1
        } else {
            m.reserve_0
        };
        if out == 0 || dbsim::hop_near_full_ask_drain(out, rout) {
            return None;
        }
    }
    if tax::score_path(out, hops, token_in, tax_snapshot).net_out < target {
        return None;
    }
    Some((required, out))
}

fn fidelity_passed(mirror: u128, live: u128, drift_bps: u32) -> bool {
    live > 0
        && BigUint::from(mirror.abs_diff(live)) * BigUint::from(10_000u32)
            <= BigUint::from(live) * BigUint::from(drift_bps)
}

pub(crate) async fn solve(
    state: &AppState,
    q: &SolveRouteParams,
    target: u128,
    trader: &QuoteTrader,
) -> Result<Json<Value>, Error> {
    static LIMIT: OnceLock<Semaphore> = OnceLock::new();
    let _permit = LIMIT
        .get_or_init(|| Semaphore::new(MAX_CONCURRENT_REQUESTS))
        .try_acquire()
        .map_err(|_| {
            (
                StatusCode::TOO_MANY_REQUESTS,
                "Exact-output sizing is busy; retry shortly".into(),
            )
        })?;
    tokio::time::timeout(TIME_BUDGET, solve_inner(state, q, target, trader))
        .await
        .map_err(|_| {
            (
                StatusCode::GATEWAY_TIMEOUT,
                "Exact-output sizing timed out; retry".into(),
            )
        })?
}

/// Router simulation is arithmetic only: enforce execution eligibility separately, fail closed.
async fn path_eligible(state: &AppState, hops: &[RouteHop], trader: &QuoteTrader) -> bool {
    let Some(factory) = state.factory_address.as_deref() else {
        return false;
    };
    for h in hops {
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&h.pair) {
            return false;
        }
        let paused = tokio::time::timeout(
            Duration::from_secs(1),
            state
                .lcd
                .query_contract::<Value>(&h.pair, &serde_json::json!({"is_paused":{}})),
        )
        .await;
        if !matches!(paused, Ok(Ok(ref v)) if v["paused"].as_bool() == Some(false)) {
            return false;
        }
    }
    let tokens: Vec<_> = hops
        .iter()
        .flat_map(|h| [&h.offer_token, &h.ask_token])
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let pairs: Vec<_> = hops.iter().map(|h| &h.pair).collect();
    let query = serde_json::json!({"blacklist_check": {"wallet": trader.trader.as_ref().or(trader.sender.as_ref()),
        "tokens":tokens,"pair":null,"pairs":pairs}});
    let blocked = tokio::time::timeout(
        Duration::from_secs(1),
        state.lcd.query_contract::<Value>(factory, &query),
    )
    .await;
    matches!(blocked, Ok(Ok(ref v)) if v["blocked"].as_bool() == Some(false))
}

async fn solve_inner(
    state: &AppState,
    q: &SolveRouteParams,
    target: u128,
    trader: &QuoteTrader,
) -> Result<Json<Value>, Error> {
    if state.router_address.is_none() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Exact-output sizing requires router forward validation".into(),
        ));
    }
    let snap = route_graph::get_route_graph_snapshot(&state.pool).await?;
    let start = route_solver::resolve_id(&snap.addr_to_id, &q.token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".into(),
        )
    })?;
    let goal = route_solver::resolve_id(&snap.addr_to_id, &q.token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".into(),
        )
    })?;
    if start == goal {
        return Err((
            StatusCode::BAD_REQUEST,
            "exact-output tokens must differ".into(),
        ));
    }
    if snap.pairs.len() > MAX_GRAPH_PAIRS {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Exact-output graph exceeds work budget".into(),
        ));
    }
    let rows = snap.pairs.clone();
    let (raw, work_truncated) = tokio::task::spawn_blocking(move || {
        route_paths::find_paths_top_k_with_work_budget(
            start,
            goal,
            &rows,
            route_solver::GET_DEFAULT_MAX_HOPS,
            best_execution::PATH_ENUM_POOL,
            PATH_WORK_BUDGET,
        )
    })
    .await
    .map_err(super::internal_err)?;
    if raw.is_empty() && work_truncated {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Exact-output discovery exhausted work budget".into(),
        ));
    }
    if raw.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            "No eligible route within 4 hops".into(),
        ));
    }
    let mut candidates: Vec<_> = raw
        .iter()
        .map(|r| route_solver::build_hops_and_ops(r, &snap.id_to_addr))
        .collect::<Result<_, _>>()?;
    candidates.retain(|(hops, _)| reference::supported_hops(&snap, hops));
    if candidates.is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "No route with supported token decimals".into(),
        ));
    }
    let extras: Vec<_> = candidates
        .iter()
        .flat_map(|(h, _)| tax::extra_addrs_from_hops(h))
        .collect();
    // Exact output must never silently drop the tax layer when the catalog is unavailable.
    let taxes = tax::try_load_tax_rank_snapshot(
        state,
        &q.token_in,
        &q.token_out,
        &extras,
        trader.trader.as_deref().or(trader.sender.as_deref()),
    )
    .await
    .map_err(super::internal_err)?;
    let discount = route_solver::resolve_discount_bps(state, trader).await;
    let pairs: Vec<_> = candidates
        .iter()
        .flat_map(|(h, _)| h.iter().map(|h| h.pair.clone()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut mirrors = dbsim::preload_pool_mirrors_for_pairs(
        &state.pool,
        &pairs,
        &snap.id_to_addr,
        state.book_snapshot_max_staleness_ms,
    )
    .await
    .map_err(super::internal_err)?;
    let mut sized = Vec::new();
    for (hops, ops) in candidates {
        if let Some((amount, out)) =
            size_path(&hops, &mirrors, target, discount, &taxes, &q.token_in)
        {
            sized.push((amount, out, hops, ops));
            if sized.len() == best_execution::MAX_PATH_CANDIDATES {
                break;
            }
        }
    }
    if sized.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
            "Receive target unavailable: no feasible route with fresh reserves and supported tax policy".into()));
    }
    let fair_reference = reference::load(
        state,
        &snap,
        &q.token_in,
        &q.token_out,
        discount,
        &taxes,
        &mut mirrors,
    )
    .await?;
    let mut extreme_rejected = false;
    let paths_considered = sized.len() as u32;
    sized.sort_by_key(|(amount, _, _, _)| *amount);
    for (amount, mirrored, hops, ops) in sized {
        if !path_eligible(state, &hops, trader).await {
            continue;
        }
        // Recheck age after loading every path; a slow load cannot revive a stale reserve quote.
        if hops.iter().any(|h| {
            dbsim::is_snapshot_stale(
                mirrors[&h.pair].snapshot_at,
                state.book_snapshot_max_staleness_ms,
            )
        }) {
            continue;
        }
        let live = tokio::time::timeout(
            VALIDATION_TIMEOUT,
            route_solver::maybe_simulate(state, Some(&amount.to_string()), &ops, trader),
        )
        .await;
        let Ok(Ok(Some(live))) = live else {
            continue;
        };
        let Some(out) = live.parse::<u128>().ok() else {
            continue;
        };
        let scored = tax::score_path(out, &hops, &q.token_in, &taxes);
        if scored.net_out < target
            || !fidelity_passed(mirrored, out, state.route_fidelity_drift_bps)
        {
            continue;
        }
        if !fair_reference.is_fresh(&mirrors, state.book_snapshot_max_staleness_ms) {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Fair-route reference expired during validation; retry".into(),
            ));
        }
        let Some(fair) = fair_reference.fields(amount, out) else {
            continue;
        };
        if fair.extreme {
            extreme_rejected = true;
            continue;
        }
        let age = hops
            .iter()
            .map(|h| dbsim::snapshot_age_ms(mirrors[&h.pair].snapshot_at))
            .max();
        if age.is_some_and(|age| age > state.book_snapshot_max_staleness_ms) {
            continue;
        }
        let block = hops
            .iter()
            .map(|h| mirrors[&h.pair].block_height)
            .collect::<Option<Vec<_>>>()
            .and_then(|b| b.into_iter().min());
        let intermediate_tokens = std::iter::once(hops[0].offer_token.clone())
            .chain(hops.iter().map(|h| h.ask_token.clone()))
            .collect();
        let response = RouteSolveResponse {
            required_amount_in: Some(amount.to_string()),
            target_amount_out: Some(target.to_string()),
            sizing_mode: Some("pool_only_exact_output".into()),
            snapshot_age_ms: age,
            snapshot_block_height: block,
            token_in: hops[0].offer_token.clone(),
            token_out: hops.last().unwrap().ask_token.clone(),
            intermediate_tokens,
            hops,
            quote_kind: RouteQuoteKind::IndexerPoolLcd,
            router_operations: ops,
            estimated_amount_out: Some(out.to_string()),
            estimated_amount_out_net: Some(scored.net_out.to_string()),
            tax_kind: Some(scored.tax_kind.into()),
            buy_tax_bps: Some(scored.buy_tax_bps),
            sell_tax_bps: Some(scored.sell_tax_bps),
            tax_notes: Some(scored.tax_notes),
            router_hops_tax: Some(scored.router_hops_tax),
            solver_version: Some("pool_exact_output_v1".into()),
            paths_considered: Some(paths_considered),
            optimality_scope: Some(OPTIMALITY_SCOPE.into()),
            hybrid_notes: Some("Pool-only sizing; execution remains offer-in. Snapshot minimum is not an atomic exact-output guarantee; mapper gross-up and sell trader debit are client responsibilities.".into()),
            lcd_hybrid_queries: Some(0),
            db_hybrid_queries: Some(paths_considered),
            fidelity_check: Some(FidelityCheck::Passed),
            mirror_max_block_lag: None,
            search_truncated: Some(work_truncated || raw.len() == best_execution::PATH_ENUM_POOL || paths_considered as usize == best_execution::MAX_PATH_CANDIDATES),
            spot_amount_out: Some(fair.spot),
            slippage_percent: Some(fair.slippage),
            token_in_price_quote: Some(fair.price_in),
            token_out_price_quote: Some(fair.price_out),
        };
        return Ok(Json(
            serde_json::to_value(response).map_err(super::internal_err)?,
        ));
    }
    if extreme_rejected {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Receive route rejected by >=99% symmetric fair-rate protection".into(),
        ));
    }
    Err((
        StatusCode::SERVICE_UNAVAILABLE,
        "Receive quote failed fresh router validation; reserves may have changed. Retry.".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mirror(r0: u128, r1: u128, fee: u16) -> dbsim::HopMirror {
        dbsim::HopMirror {
            pair_id: 1,
            asset_0_addr: "a".into(),
            asset_1_addr: "b".into(),
            reserve_0: r0,
            reserve_1: r1,
            fee_bps: fee,
            block_height: Some(100),
            snapshot_at: Utc::now(),
            freshness: dbsim::MirrorFreshness::Fresh,
            bids: vec![],
            asks: vec![],
        }
    }

    #[test]
    fn route_exact_output_real_6_18_fixture_is_minimal_across_old_call_boundary() {
        let m = mirror(297665983808, 2498115414904458263864, 180);
        for (target, expected) in [
            (10_000_000_000_000_000_000u128, 1218369992u128),
            (100_000_000_000_000_000_000, 12649685215),
        ] {
            let amount = inverse_pool(&m, "a", target, 0).unwrap();
            assert_eq!(amount, expected);
            assert!(dbsim::simulate_pool_only_from_mirror(&m, "a", amount, 0).unwrap() >= target);
            assert!(
                dbsim::simulate_pool_only_from_mirror(&m, "a", amount - 1, 0).unwrap() < target
            );
        }
    }

    #[test]
    fn route_exact_output_orientations_scales_and_discount_are_minimal() {
        for (r0, r1) in [
            (10u128.pow(12), 10u128.pow(24)),
            (10u128.pow(24), 10u128.pow(12)),
            (10u128.pow(24), 10u128.pow(24)),
        ] {
            let m = mirror(r0, r1, 180);
            for (offer, rout) in [("a", r1), ("b", r0)] {
                for discount in [0, 5000, 10000] {
                    let target = rout / 10;
                    let amount = inverse_pool(&m, offer, target, discount).unwrap();
                    assert!(
                        dbsim::simulate_pool_only_from_mirror(&m, offer, amount, discount).unwrap()
                            >= target
                    );
                    assert!(
                        dbsim::simulate_pool_only_from_mirror(&m, offer, amount - 1, discount)
                            .unwrap()
                            < target
                    );
                }
            }
        }
    }

    #[test]
    fn route_exact_output_wide_net_inverse_and_fee_are_exact() {
        for bps in [0, 1, 180, 9999] {
            for target in [1u128, 10000, 10u128.pow(30)] {
                let gross = inverse_net(target, bps).unwrap();
                assert!(tax::buy_split_net(gross, bps) >= target);
                assert!(tax::buy_split_net(gross - 1, bps) < target);
                assert_eq!(
                    gross - super::super::orderbook_sim::swap_fee_amount(gross, bps),
                    tax::buy_split_net(gross, bps)
                );
            }
        }
        assert!(inverse_net(u128::MAX, 9999).is_none());
        assert!(inverse_net(1, 10000).is_none());
        assert_eq!(tax::buy_split_net(u128::MAX, 10000), 0);
    }

    #[test]
    fn route_exact_output_infeasible_stale_unknown_overflow_rejected() {
        let mut m = mirror(1000, 1000, 30);
        assert!(inverse_pool(&m, "a", 1000, 0).is_none());
        assert!(inverse_pool(&m, "unknown", 100, 0).is_none());
        m.freshness = dbsim::MirrorFreshness::Stale;
        assert!(inverse_pool(&m, "a", 100, 0).is_none());
        assert!(inverse_pool(&mirror(u128::MAX, 1000, 30), "a", 100, 0).is_none());
    }

    #[test]
    fn route_exact_output_request_rejects_abuse_and_conflicting_modes() {
        for raw in [
            "",
            "0",
            "-1",
            "+1",
            " 1",
            "1e18",
            "NaN",
            "∞",
            "١",
            "<script>",
            "340282366920938463463374607431768211456",
        ] {
            let q: SolveRouteParams = serde_json::from_value(
                serde_json::json!({"token_in":"a","token_out":"b","amount_out":raw}),
            )
            .unwrap();
            assert!(validate_request(&q).is_err(), "{raw}");
        }
        for extra in [
            serde_json::json!({"amount_in":""}),
            serde_json::json!({"hybrid_optimize":true}),
            serde_json::json!({"pool_only":false}),
        ] {
            let mut v = serde_json::json!({"token_in":"a","token_out":"b","amount_out":"100"});
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            assert!(validate_request(&serde_json::from_value(v).unwrap()).is_err());
        }
        let q = serde_json::from_value(
            serde_json::json!({"token_in":"a","token_out":"b","amount_out":"100"}),
        )
        .unwrap();
        assert_eq!(validate_request(&q).unwrap(), 100);
    }

    #[test]
    fn route_exact_output_buy_net_multi_hop_and_middle_tax_policy() {
        let mut taxes = tax::TaxRankSnapshot::default();
        let taxed = tax::CataloguedTaxToken {
            contract_address: "b".into(),
            code_id: Some(11611),
            data_hash: None,
            buy_bps: 1000,
            sell_bps: 100,
            exemption_directory: false,
        };
        taxes.token_out = Some(taxed.clone());
        taxes.by_addr.insert("b".into(), taxed);
        let hops = vec![RouteHop {
            pair: "p".into(),
            offer_token: "a".into(),
            ask_token: "b".into(),
        }];
        let mirrors = HashMap::from([("p".into(), mirror(1000000, 1000000, 30))]);
        let (amount, out) = size_path(&hops, &mirrors, 10000, 0, &taxes, "a").unwrap();
        assert!(tax::score_path(out, &hops, "a", &taxes).net_out >= 10000);
        let prior =
            dbsim::simulate_pool_only_from_mirror(&mirrors["p"], "a", amount - 1, 0).unwrap();
        assert!(tax::score_path(prior, &hops, "a", &taxes).net_out < 10000);
        taxes.trader_exempt = true;
        assert!(size_path(&hops, &mirrors, 10000, 0, &taxes, "a").unwrap().0 < amount);
    }

    #[test]
    fn route_exact_output_fidelity_is_wide_and_symmetric() {
        assert!(fidelity_passed(u128::MAX, u128::MAX, 1));
        assert!(!fidelity_passed(100, 50, 100));
        assert!(!fidelity_passed(100, 200, 100));
        assert!(!fidelity_passed(1, 0, 100));
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn route_exact_output_one_to_four_hops_preserve_minimality_and_reject_middle_tax() {
        let mut mirrors = HashMap::new();
        let mut hops = Vec::new();
        for i in 0..4 {
            let offer = format!("t{i}");
            let ask = format!("t{}", i + 1);
            let pair = format!("p{i}");
            mirrors.insert(
                pair.clone(),
                dbsim::HopMirror {
                    pair_id: i,
                    asset_0_addr: offer.clone(),
                    asset_1_addr: ask.clone(),
                    reserve_0: 10u128.pow(12),
                    reserve_1: 10u128.pow(12),
                    fee_bps: 180,
                    block_height: Some(100),
                    snapshot_at: Utc::now(),
                    freshness: dbsim::MirrorFreshness::Fresh,
                    bids: vec![],
                    asks: vec![],
                },
            );
            hops.push(RouteHop {
                pair,
                offer_token: offer,
                ask_token: ask,
            });
            let (amount, out) = size_path(
                &hops,
                &mirrors,
                1000000,
                0,
                &tax::TaxRankSnapshot::default(),
                "t0",
            )
            .unwrap();
            assert!(out >= 1000000);
            let mut prior = amount - 1;
            for h in &hops {
                prior = dbsim::simulate_pool_only_from_mirror(
                    &mirrors[&h.pair],
                    &h.offer_token,
                    prior,
                    0,
                )
                .unwrap();
            }
            assert!(prior < 1000000);
        }
        let mut taxes = tax::TaxRankSnapshot::default();
        taxes.policy.option2_code_ids.insert(99);
        taxes.by_addr.insert(
            "t1".into(),
            tax::CataloguedTaxToken {
                contract_address: "t1".into(),
                code_id: Some(99),
                data_hash: None,
                buy_bps: 100,
                sell_bps: 200,
                exemption_directory: false,
            },
        );
        assert!(size_path(&hops, &mirrors, 1000000, 0, &taxes, "t0").is_none());
    }
}
