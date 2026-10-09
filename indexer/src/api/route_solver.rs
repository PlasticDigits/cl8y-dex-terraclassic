//! Cached local executable quotes and topology discovery (#1452).
//!
//! GET/POST/best/progress use initialized immutable observations maintained by
//! scheduled ingestion. No request performs LCD attestation, simulation or
//! configuration/reference discovery. Historical network-solver helpers below
//! compile only for regression comparisons; they cannot be request fallbacks.

use std::collections::HashMap;
#[cfg(test)]
use std::collections::VecDeque;
use std::net::SocketAddr;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
#[cfg(test)]
use std::time::Instant;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

#[cfg(test)]
use sqlx::PgPool;

use crate::api::hybrid_route_opt;
#[cfg(test)]
use crate::api::internal_err;
#[cfg(test)]
use crate::api::route_graph;
#[cfg(test)]
use crate::api::route_solve_progress;
use crate::api::AppState;
use crate::db::queries::assets;
#[cfg(test)]
use crate::db::queries::pairs as db_pairs;
#[cfg(test)]
use crate::hybrid_limits::{clamp_max_maker_fills, MAX_MAKER_FILLS_HARD_CAP};

pub use hybrid_route_opt::HybridHopJson;

const ROUTE_CACHE_TTL: Duration = Duration::from_secs(12);
/// TTL for `(trader, sender)` → `discount_bps` so progress polls do not LCD every tick (#694).
pub const DISCOUNT_BPS_CACHE_TTL: Duration = ROUTE_CACHE_TTL;
#[cfg(test)]
const ROUTE_CACHE_TTL_DISTANT: Duration = Duration::from_secs(90);
#[cfg(test)]
const ROUTE_CACHE_MAX_ENTRIES: usize = 512;
#[cfg(test)]
const DISCOUNT_BPS_CACHE_MAX_ENTRIES: usize = 512;
/// Default GET hop cap (hybrid-aware routing per ADR 0001 / GitLab #191).
pub(crate) const GET_DEFAULT_MAX_HOPS: usize = 4;
/// Legacy pool-only GET escape hatch (`pool_only=true`).
#[cfg(test)]
const GET_POOL_ONLY_MAX_HOPS: usize = 4;
/// Coarse bucketing for hybrid GET cache keys (reduces LCD load).
#[cfg(test)]
const AMOUNT_CACHE_BUCKET: u128 = 1_000_000;

#[derive(Debug, Clone, Deserialize, Serialize, IntoParams, ToSchema)]
pub struct SolveRouteParams {
    /// Caller L9 execution constraint; local quote admission and signed floors share this value.
    #[serde(default)]
    pub max_spread: Option<String>,
    /// CW20 contract address (must match indexed `assets.contract_address`). See [route-solver glossary](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/blob/main/docs/route-solver.md#glossary).
    pub token_in: String,
    /// CW20 contract address for the output asset (indexed `assets.contract_address`).
    pub token_out: String,
    /// Exact raw integer offer amount; local executable ranking within four hops. Required for `/best` unless `amount_out` is provided. Omit both amounts for cached topology discovery.
    pub amount_in: Option<String>,
    /// Exact-output sizing: positive raw NET CW20 target. Mutually exclusive with amount_in.
    /// Uses bounded locally cached pool-only inverse candidates; execution remains offer-in. Unsupported inverse semantics fail explicitly.
    #[serde(default)]
    pub amount_out: Option<String>,
    /// Deprecated alias: `hybrid_optimize=false` or `pool_only=true` for pool-only routing.
    #[serde(default)]
    pub hybrid_optimize: Option<bool>,
    /// Disable book splits; compare bounded locally eligible pool paths within four hops.
    #[serde(default)]
    pub pool_only: Option<bool>,
    /// Per-hop `max_maker_fills` for hybrid GET optimization (default **8**); caps limit-book matches per hop.
    #[serde(default)]
    pub max_maker_fills: Option<u32>,
    /// Optional beneficiary. Cached known fee context participates in ranking and identity; unknown context uses a conservative full-fee estimate. No request-time tier lookup.
    #[serde(default)]
    pub trader: Option<String>,
    /// Optional CW20 sender `terra1` address when it differs from `trader` (trusted router execute path).
    #[serde(default)]
    pub sender: Option<String>,
    /// Legacy progress hint. Local quotes never query GetDiscount during a request.
    /// Ignored by `/route/solve` quote math. Range `0..=10000`.
    #[serde(default)]
    pub discount_bps: Option<u16>,
}

/// Local quote POST; explicit unsupported splits fail rather than using remote simulation.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SolveRoutePostBody {
    pub token_in: String,
    pub token_out: String,
    /// Exact raw integer offer amount for locally cached arithmetic.
    pub amount_in: Option<String>,
    /// Exact-output is supported on GET only; providing this on POST is rejected.
    #[serde(default)]
    pub amount_out: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FidelityCheck {
    Passed,
    Drift,
    #[default]
    Skipped,
}

impl FidelityCheck {
    #[cfg(test)]
    pub fn as_str(self) -> &'static str {
        match self {
            FidelityCheck::Passed => "passed",
            FidelityCheck::Drift => "drift",
            FidelityCheck::Skipped => "skipped",
        }
    }
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn resolve_id(addr_to_id: &HashMap<String, i32>, token: &str) -> Option<i32> {
    let t = token.trim();
    if let Some(id) = addr_to_id.get(t) {
        return Some(*id);
    }
    addr_to_id.get(&t.to_lowercase()).copied()
}

#[cfg(test)]
fn asset_info_json(addr: &str) -> serde_json::Value {
    json!({ "token": { "contract_addr": addr } })
}

/// BFS by hop count, max `max_hops` edges (matches router `MAX_HOPS`).
#[cfg(test)]
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
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&p.contract_address) {
            continue;
        }
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

#[cfg(test)]
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

/// Locally checked observation and exact operations reviewed by the client.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalExecutionMetadata {
    pub version: u8,
    pub generation: String,
    pub input_amount: String,
    pub max_spread: String,
    pub trader: Option<String>,
    pub sender: Option<String>,
    pub observed_at_ms: i64,
    pub expires_at_ms: i64,
    pub policy: String,
    pub hops: Vec<LocalExecutionHop>,
    pub minimum_receive: String,
    pub wrapper: Option<LocalExecutionWrapper>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalExecutionHop {
    pub pair: String,
    pub offer_token: String,
    pub ask_token: String,
    pub offer_amount: String,
    /// Delivered post-hook ask output; not the L9 pool return basis.
    pub return_amount: String,
    pub pool_return_amount: String,
    pub book_return_net: String,
    pub pool_input_actual: String,
    pub book_offer_consumed: String,
    pub spread_amount: String,
    pub commission_amount: String,
    pub min_return: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocalExecutionWrapper {
    pub input_token: String,
    pub output_token: String,
    pub input_amount: String,
    pub route_input_amount: String,
    pub route_output_amount: String,
    /// Router minimum basis after mapper fee and before native BankMsg burn tax.
    pub minimum_receive_base: String,
    pub wrap_input: bool,
    pub unwrap_output: bool,
    pub treasury: String,
    pub mapper: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct RouteHop {
    pub pair: String,
    pub offer_token: String,
    pub ask_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RouteSolveResponse {
    /// Generation-pinned local execution evidence, positive floors and wrapper bindings.
    /// Absent on topology-only responses and historical fixtures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_metadata: Option<LocalExecutionMetadata>,
    /// Exact-output required raw CW20 offer; absent on offer-in/discovery responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_amount_in: Option<String>,
    /// Requested raw NET CW20 output; execution remains offer-in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_amount_out: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizing_mode: Option<String>,
    /// Oldest selected reserve snapshot age at validation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_age_ms: Option<u64>,
    /// Lowest indexed reserve block among selected hops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_block_height: Option<i64>,
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
    /// Final executable NET output. For route_local_v1 this equals estimated_amount_out:
    /// all per-hop tax has already been applied by the router simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_amount_out_net: Option<String>,
    /// `route_local_v1` means the authorized offer is gross and simulated output is already net.
    /// Legacy compatibility labels: buy / sell_in / exempt / honest_hops / none / skipped_middle_sell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_kind: Option<String>,
    /// Effective buy bps used for `estimated_amount_out_net` (0 when exempt / Honest hops).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buy_tax_bps: Option<u16>,
    /// Effective token_in sell bps; route_local_v1 simulation includes its gross/economic split.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sell_tax_bps: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_notes: Option<String>,
    /// True when a token on this route uses authenticated router-hop tax; v1 charges route funds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router_hops_tax: Option<bool>,
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
    /// Fair output at best-route token cross-rate (raw integer, output token decimals). GitLab #293.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spot_amount_out: Option<String>,
    /// Symmetric deviation vs `spot_amount_out` (percent string, e.g. `"99.97"`). GitLab #293.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_percent: Option<String>,
    /// Input token price in the quote asset (best-route valuation). GitLab #293.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_in_price_quote: Option<String>,
    /// Output token price in the quote asset (best-route valuation). GitLab #293.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_out_price_quote: Option<String>,
}

#[cfg(test)]
pub(crate) fn build_intermediate_tokens(resolved: &ResolvedRoute) -> Vec<String> {
    let mut v = vec![resolved.token_in.clone()];
    for h in &resolved.hops {
        v.push(h.ask_token.clone());
    }
    v
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
async fn resolve_route_with_max_hops(
    pool: &PgPool,
    namespace: &str,
    token_in: &str,
    token_out: &str,
    max_hops: usize,
) -> Result<ResolvedRoute, (StatusCode, String)> {
    let snapshot = route_graph::get_route_graph_snapshot(pool, namespace).await?;
    let (id_to_addr, addr_to_id) = (&snapshot.id_to_addr, &snapshot.addr_to_id);

    let start = resolve_id(addr_to_id, token_in).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_in not found in indexer assets".to_string(),
        )
    })?;
    let goal = resolve_id(addr_to_id, token_out).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "token_out not found in indexer assets".to_string(),
        )
    })?;

    let hops_raw = find_path(start, goal, &snapshot.pairs, max_hops).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("no route within {} hops", max_hops),
        )
    })?;

    let (hops, ops) = build_hops_and_ops(&hops_raw, id_to_addr)?;

    Ok(ResolvedRoute {
        token_in: token_in.trim().to_string(),
        token_out: token_out.trim().to_string(),
        hops,
        ops,
    })
}

#[cfg(test)]
async fn resolve_route(
    pool: &PgPool,
    namespace: &str,
    token_in: &str,
    token_out: &str,
) -> Result<ResolvedRoute, (StatusCode, String)> {
    resolve_route_with_max_hops(pool, namespace, token_in, token_out, GET_POOL_ONLY_MAX_HOPS).await
}

#[cfg(test)]
fn parse_amount_in(raw: Option<&String>) -> Option<&str> {
    raw.map(|s| s.trim()).filter(|s| !s.is_empty())
}

#[cfg(test)]
fn get_pool_only_requested(q: &SolveRouteParams) -> bool {
    q.pool_only.unwrap_or(false) || q.hybrid_optimize == Some(false)
}

#[cfg(test)]
fn get_pool_only_max_hops(q: &SolveRouteParams) -> usize {
    if get_pool_only_requested(q) {
        GET_POOL_ONLY_MAX_HOPS
    } else {
        GET_DEFAULT_MAX_HOPS
    }
}

#[cfg(test)]
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

#[cfg(test)]
pub(crate) async fn validate_route_tax(
    state: &AppState,
    hops: &[RouteHop],
    token_in: &str,
    token_out: &str,
    trader: &hybrid_route_opt::QuoteTrader,
) -> Result<crate::api::community_tax_rank::TaxRankSnapshot, (StatusCode, String)> {
    let taxes = crate::api::community_tax_rank::load_tax_rank_snapshot(
        state,
        token_in,
        token_out,
        &crate::api::community_tax_rank::extra_addrs_from_hops(hops),
        trader.trader.as_deref().or(trader.sender.as_deref()),
    )
    .await;
    if let Some(asset) = crate::api::community_tax_rank::repeated_route_tax_asset(hops, &taxes) {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("repeated route-tax-capable asset in swap path: {asset}"),
        ));
    }
    if crate::api::community_tax_rank::path_sells_middle_tax_hop(hops, token_in, &taxes) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
            "Trading tax capability unavailable or unsupported legacy route; refresh or choose a supported route".into()));
    }
    Ok(taxes)
}
#[cfg(test)]
fn apply_route_tax_fields(
    body: &mut RouteSolveResponse,
    taxes: &crate::api::community_tax_rank::TaxRankSnapshot,
) {
    let Some(out) = body
        .estimated_amount_out
        .as_deref()
        .and_then(|n| n.parse::<u128>().ok())
    else {
        return;
    };
    let score = crate::api::community_tax_rank::score_path(out, &body.hops, &body.token_in, taxes);
    body.estimated_amount_out_net = Some(score.net_out.to_string());
    body.tax_kind = Some(score.tax_kind.into());
    body.tax_notes = Some(score.tax_notes);
    body.router_hops_tax = Some(score.router_hops_tax);
    body.buy_tax_bps = Some(score.buy_tax_bps);
    body.sell_tax_bps = Some(score.sell_tax_bps);
}

#[cfg(test)]
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
            tracing::warn!(error = %e, "router simulate_swap_operations failed");
            Err(router_preview_error(&e))
        }
    }
}

/// Preserve deterministic on-chain rejection as an observable validation failure.
/// Never expose LCD URLs, raw contract errors or trader addresses in the API response.
#[cfg(test)]
fn router_preview_error(error: &crate::lcd::LcdError) -> (StatusCode, String) {
    if matches!(error, crate::lcd::LcdError::QueryGasLimitExceeded(_)) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "router_query_gas_limit_exceeded: this route exceeds the provider's smart-query gas limit; choose another route or retry after the provider or contract changes".into(),
        );
    }
    if matches!(error, crate::lcd::LcdError::ContractQueryRejected(_)) {
        return (StatusCode::UNPROCESSABLE_ENTITY,
            "router_preview_rejected: on-chain route or tax/launch guard validation rejected the preview; refresh or choose another route".into());
    }
    (
        StatusCode::BAD_REQUEST,
        "router simulation failed for the given route and hybrid parameters".into(),
    )
}

#[cfg(test)]
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

#[cfg(test)]
struct CacheEntry {
    at: Instant,
    body: serde_json::Value,
    amount_in: u128,
    ttl: Duration,
}

#[cfg(test)]
fn route_hybrid_cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
pub(crate) fn amount_cache_key(amount: u128) -> u128 {
    if amount <= AMOUNT_CACHE_BUCKET {
        amount
    } else {
        (amount / AMOUNT_CACHE_BUCKET) * AMOUNT_CACHE_BUCKET
    }
}

/// Coarse bucket for hybrid GET cache keys — normal retail range 1–8 maps to default 8 (#324).
#[cfg(test)]
pub(crate) fn cache_key_maker_fills(max_maker_fills: u32) -> u32 {
    let m = clamp_max_maker_fills(max_maker_fills);
    if m <= 8 {
        8
    } else if m <= 16 {
        16
    } else {
        MAX_MAKER_FILLS_HARD_CAP
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
#[cfg(test)]
pub const FULL_FEE_TIER_SENTINEL: i16 = -1;

#[cfg(test)]
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

#[cfg(test)]
struct DiscountBpsCacheEntry {
    at: Instant,
    bps: u16,
}

#[cfg(test)]
fn discount_bps_cache() -> &'static Mutex<HashMap<String, DiscountBpsCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, DiscountBpsCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
fn discount_bps_cache_key(trader: &str, sender: &str) -> String {
    format!(
        "{}|{}",
        trader.trim().to_ascii_lowercase(),
        sender.trim().to_ascii_lowercase()
    )
}

#[cfg(test)]
fn discount_bps_cache_get(key: &str) -> Option<u16> {
    let Ok(guard) = discount_bps_cache().lock() else {
        return None;
    };
    let entry = guard.get(key)?;
    if entry.at.elapsed() <= DISCOUNT_BPS_CACHE_TTL {
        Some(entry.bps)
    } else {
        None
    }
}

#[cfg(test)]
fn discount_bps_cache_put(key: &str, bps: u16) {
    let Ok(mut guard) = discount_bps_cache().lock() else {
        return;
    };
    if guard.len() >= DISCOUNT_BPS_CACHE_MAX_ENTRIES {
        let now = Instant::now();
        guard.retain(|_, e| now.duration_since(e.at) <= DISCOUNT_BPS_CACHE_TTL);
        if guard.len() >= DISCOUNT_BPS_CACHE_MAX_ENTRIES {
            if let Some(oldest) = guard
                .iter()
                .min_by_key(|(_, e)| e.at)
                .map(|(k, _)| k.clone())
            {
                guard.remove(&oldest);
            }
        }
    }
    guard.insert(
        key.to_string(),
        DiscountBpsCacheEntry {
            at: Instant::now(),
            bps,
        },
    );
}

/// Clear the `(trader, sender)` discount cache (integration tests).
#[allow(dead_code)]
#[cfg(test)]
pub fn reset_discount_bps_cache() {
    if let Ok(mut guard) = discount_bps_cache().lock() {
        guard.clear();
    }
}

/// Validate a client-supplied progress `discount_bps` (0..=10000).
#[cfg(test)]
pub(crate) fn parse_progress_discount_bps(
    raw: Option<u16>,
) -> Result<Option<u16>, (StatusCode, String)> {
    let Some(bps) = raw else {
        return Ok(None);
    };
    if bps > 10_000 {
        return Err((
            StatusCode::BAD_REQUEST,
            "discount_bps must be between 0 and 10000".to_string(),
        ));
    }
    Ok(Some(bps))
}

/// On-chain `GetDiscount` discount_bps for mirror grid pricing — matches pair `HybridSimulation` / router sim (#319).
/// Cached per `(trader, sender)` for [`DISCOUNT_BPS_CACHE_TTL`] so 1 Hz progress polls do not LCD every tick (#694).
#[cfg(test)]
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

    if state.lcd.require_verified_chain().await.is_err() {
        return 0;
    }
    let cache_key = format!(
        "{}|{}",
        state.lcd.chain_cache_namespace(),
        discount_bps_cache_key(trader, sender)
    );
    if let Some(bps) = discount_bps_cache_get(&cache_key) {
        return bps;
    }

    let bps = resolve_discount_bps_uncached(state, quote_trader, trader, sender).await;
    if state.lcd.require_verified_chain().await.is_ok() {
        discount_bps_cache_put(&cache_key, bps);
    }
    bps
}

#[cfg(test)]
async fn resolve_discount_bps_uncached(
    state: &AppState,
    quote_trader: &hybrid_route_opt::QuoteTrader,
    trader: &str,
    sender: &str,
) -> u16 {
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
        if let Ok(val) = state
            .lcd
            .query_contract::<serde_json::Value>(fee_addr, &q)
            .await
        {
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

#[cfg(test)]
pub(crate) fn hybrid_cache_key(
    solver_version: &str,
    token_in: &str,
    token_out: &str,
    amount_bucket: u128,
    max_maker_fills: u32,
    discount_bps: u16,
    tax_identity: &str,
) -> String {
    let mmf_key = cache_key_maker_fills(max_maker_fills);
    format!(
        "{}|{}|{}|{}|{}|d{}|t{}",
        solver_version,
        token_in.trim().to_lowercase(),
        token_out.trim().to_lowercase(),
        amount_bucket,
        mmf_key,
        discount_bps,
        tax_identity
    )
}

#[cfg(test)]
fn cache_ttl_for_hops(hop_count: usize) -> Duration {
    if hop_count >= 2 {
        ROUTE_CACHE_TTL_DISTANT
    } else {
        ROUTE_CACHE_TTL
    }
}

#[cfg(test)]
fn cache_get(key: &str) -> Option<(serde_json::Value, u128)> {
    let mut g = route_hybrid_cache().lock().ok()?;
    let e = g.get(key)?;
    if Instant::now().duration_since(e.at) > e.ttl {
        g.remove(key);
        return None;
    }
    Some((e.body.clone(), e.amount_in))
}

#[cfg(test)]
fn cache_put(key: String, body: serde_json::Value, amount_in: u128, ttl: Duration) {
    if let Ok(mut g) = route_hybrid_cache().lock() {
        let now = Instant::now();
        g.retain(|_, v| now.duration_since(v.at) <= v.ttl);
        if g.len() >= ROUTE_CACHE_MAX_ENTRIES {
            if let Some(oldest_k) = g.iter().min_by_key(|(_, v)| v.at).map(|(k, _)| k.clone()) {
                g.remove(&oldest_k);
            }
        }
        g.insert(
            key,
            CacheEntry {
                at: now,
                body,
                amount_in,
                ttl,
            },
        );
    }
}

#[cfg(test)]
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

    state.lcd.require_verified_chain().await.map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "No endpoint attests the expected chain".to_string(),
        )
    })?;
    let max_makers = clamp_max_maker_fills(max_maker_fills);
    let bucket = amount_cache_key(amount_u);
    let discount_bps = resolve_discount_bps(state, quote_trader).await;
    let solver_version = crate::api::best_execution::solver_version_for(state);
    let cache_assets =
        crate::api::best_execution::candidate_tax_addresses(state, token_in, token_out).await?;
    let cache_taxes = crate::api::community_tax_rank::load_tax_rank_snapshot(
        state,
        token_in,
        token_out,
        &cache_assets,
        quote_trader
            .trader
            .as_deref()
            .or(quote_trader.sender.as_deref()),
    )
    .await;
    let tax_identity = format!(
        "{}|{}",
        state.lcd.chain_cache_namespace(),
        cache_taxes.cache_identity()
    );
    let bucket = if cache_taxes
        .by_addr
        .values()
        .any(|t| t.route_funding_version == Some(1))
    {
        amount_u
    } else {
        bucket
    };
    let ck = hybrid_cache_key(
        solver_version,
        token_in,
        token_out,
        bucket,
        max_makers,
        discount_bps,
        &tax_identity,
    );
    route_solve_progress::progress_begin(&ck);

    if let Some((cached, cached_amount_in)) = cache_get(&ck).filter(|(_, amount)| {
        *amount == amount_u
            || !cache_taxes
                .by_addr
                .values()
                .any(|t| t.route_funding_version == Some(1))
    }) {
        let mut body: RouteSolveResponse =
            serde_json::from_value(cached).map_err(crate::api::internal_err)?;
        route_solve_progress::progress_update(
            &ck,
            route_solve_progress::STAGE_ENRICHING,
            0,
            0,
            "Computing slippage…",
        );
        crate::api::route_slippage::enrich_route_slippage(
            state,
            &mut body,
            amount_raw,
            quote_trader,
            max_makers,
            Some(cached_amount_in),
        )
        .await;
        route_solve_progress::progress_complete(&ck, true);
        return Ok(Json(
            serde_json::to_value(body).map_err(crate::api::internal_err)?,
        ));
    }

    let solve_result = crate::api::best_execution::solve_global_best_execution(
        state,
        token_in,
        token_out,
        amount_u,
        amount_raw,
        max_makers,
        quote_trader,
        Some(&ck),
    )
    .await;

    let (body, _meta) = match solve_result {
        Ok(v) => v,
        Err(e) => {
            route_solve_progress::progress_fail(&ck, &e.1);
            return Err(e);
        }
    };

    let ttl = cache_ttl_for_hops(body.hops.len());
    let ttl = if body.tax_kind.as_deref() == Some("route_local_v1") {
        ttl.min(crate::api::trading_tax_discovery::DISCOVERY_TTL)
    } else {
        ttl
    };
    let json_body = serde_json::to_value(&body).map_err(internal_err)?;
    cache_put(ck.clone(), json_body.clone(), amount_u, ttl);
    route_solve_progress::progress_complete(&ck, false);
    Ok(Json(json_body))
}

#[cfg(test)]
async fn exact_output_get(
    state: &AppState,
    q: &SolveRouteParams,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let target = crate::api::route_exact_output::validate_request(q)?;
    let trader = parse_quote_trader(q.trader.clone(), q.sender.clone())?;
    crate::api::route_exact_output::solve(
        state,
        q,
        target,
        &trader,
        crate::api::route_exact_output_admission::client_key(peer),
    )
    .await
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
        (status = 422, description = "Unsupported trading tax capability, repeated capable asset, on-chain preview/guard rejection, or infeasible receive target"),
        (status = 429, description = "Exact-output concurrency limit"),
        (status = 503, description = "Fresh router validation unavailable or rejected"),
        (status = 504, description = "Exact-output sizing exceeded time budget"),
    ),
    tag = "Routing"
)]
pub async fn solve_route_best(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if q.amount_out.is_some() {
        static ADMISSION: std::sync::OnceLock<
            super::route_exact_output_admission::AdmissionController,
        > = std::sync::OnceLock::new();
        return ADMISSION
            .get_or_init(Default::default)
            .run(
                super::route_exact_output_admission::client_key(peer),
                Duration::from_secs(1),
                super::local_quote::solve(&state, &q),
            )
            .await;
    }
    super::local_quote::solve(&state, &q).await
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
        (status = 422, description = "Unsupported trading tax capability, repeated capable asset, on-chain preview/guard rejection, or infeasible receive target"),
        (status = 429, description = "Exact-output concurrency limit"),
        (status = 503, description = "Fresh router validation unavailable or rejected"),
        (status = 504, description = "Exact-output sizing exceeded time budget"),
    ),
    tag = "Routing"
)]
pub async fn solve_route(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if q.amount_out.is_some() {
        static ADMISSION: std::sync::OnceLock<
            super::route_exact_output_admission::AdmissionController,
        > = std::sync::OnceLock::new();
        return ADMISSION
            .get_or_init(Default::default)
            .run(
                super::route_exact_output_admission::client_key(peer),
                Duration::from_secs(1),
                super::local_quote::solve(&state, &q),
            )
            .await;
    }
    super::local_quote::solve(&state, &q).await
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
        (status = 422, description = "Unsupported trading tax capability, repeated capable asset, or on-chain preview/guard rejection"),
    ),
    tag = "Routing"
)]
pub async fn solve_route_post(
    State(state): State<AppState>,
    Json(body): Json<SolveRoutePostBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if body
        .hybrid_by_hop
        .as_ref()
        .is_some_and(|h| h.iter().any(Option::is_some))
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Declared hybrid execution is unavailable in local quote snapshots".into(),
        ));
    }
    let q = SolveRouteParams {
        max_spread: None,
        token_in: body.token_in,
        token_out: body.token_out,
        amount_in: body.amount_in,
        amount_out: body.amount_out,
        hybrid_optimize: None,
        pool_only: Some(true),
        max_maker_fills: None,
        trader: body.trader,
        sender: body.sender,
        discount_bps: None,
    };
    super::local_quote::solve(&state, &q).await
}

#[cfg(test)]
mod quote_trader_tests {
    use super::{parse_quote_trader, router_preview_error};
    use axum::http::StatusCode;

    const VALID: &str = "terra1discountwallet000000000000000000000";

    #[test]
    fn router_preview_gas_limit_is_explicit_and_sanitized() {
        let error = crate::lcd::LcdError::QueryGasLimitExceeded(
            "out of gas: gasWanted: 3000000, gasUsed: 3043317 terra1private http://private-lcd"
                .into(),
        );
        let (status, message) = router_preview_error(&error);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(message.starts_with("router_query_gas_limit_exceeded:"));
        assert!(!message.contains("terra1private"));
        assert!(!message.contains("http://"));
        assert!(!message.contains("3043317"));
        assert!(!message.contains("guard"));
    }

    #[test]
    fn router_preview_rejection_is_explicit_and_sanitized() {
        let rejected = crate::lcd::LcdError::ContractQueryRejected(
            "query wasm contract failed: cooldown terra1private http://private-lcd".into(),
        );
        let (status, message) = router_preview_error(&rejected);
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(message.contains("router_preview_rejected"));
        assert!(message.contains("guard"));
        assert!(!message.contains("terra1private"));
        assert!(!message.contains("http://"));
        let (status, message) =
            router_preview_error(&crate::lcd::LcdError::AllEndpointsFailed("LCD down".into()));
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(!message.contains("guard"));
    }

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
mod discount_bps_cache_tests {
    use super::{
        discount_bps_cache_get, discount_bps_cache_key, discount_bps_cache_put,
        parse_progress_discount_bps, reset_discount_bps_cache,
    };
    use axum::http::StatusCode;

    #[test]
    fn parse_progress_discount_bps_accepts_none_and_cap() {
        assert_eq!(parse_progress_discount_bps(None).unwrap(), None);
        assert_eq!(parse_progress_discount_bps(Some(0)).unwrap(), Some(0));
        assert_eq!(
            parse_progress_discount_bps(Some(10_000)).unwrap(),
            Some(10_000)
        );
        let err = parse_progress_discount_bps(Some(10_001)).unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn discount_bps_cache_hit_within_ttl() {
        reset_discount_bps_cache();
        let key = discount_bps_cache_key(
            "terra1discountwallet000000000000000000000",
            "terra1discountwallet000000000000000000000",
        );
        discount_bps_cache_put(&key, 5_000);
        assert_eq!(discount_bps_cache_get(&key), Some(5_000));
        reset_discount_bps_cache();
        assert_eq!(discount_bps_cache_get(&key), None);
    }
}

#[cfg(test)]
mod hybrid_cache_key_tests {
    use super::{amount_cache_key, cache_key_maker_fills, hybrid_cache_key};
    use crate::hybrid_limits::MAX_MAKER_FILLS_HARD_CAP;

    const TIN: &str = "terra1tokenin000000000000000000000000000";
    const TOUT: &str = "terra1tokenout00000000000000000000000000";
    const SV: &str = "global_v4";

    #[test]
    fn hybrid_cache_key_same_tier_traders_share_key() {
        let key7 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 7, 5_000, "none");
        let key8 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000, "none");
        assert_eq!(key7, key8, "mmf 7 and 8 bucket to the same cache key");
        let other_wallet = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000, "none");
        assert_eq!(key8, other_wallet, "trader address is not part of the key");
    }

    #[test]
    fn hybrid_cache_key_maker_fills_distinct_buckets() {
        let retail = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 0, "none");
        let mid = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 12, 0, "none");
        let high = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 25, 0, "none");
        assert_ne!(retail, mid);
        assert_ne!(mid, high);
        assert_eq!(cache_key_maker_fills(7), 8);
        assert_eq!(cache_key_maker_fills(12), 16);
        assert_eq!(cache_key_maker_fills(25), MAX_MAKER_FILLS_HARD_CAP);
        assert_eq!(cache_key_maker_fills(100), MAX_MAKER_FILLS_HARD_CAP);
        assert_eq!(cache_key_maker_fills(4294967295), MAX_MAKER_FILLS_HARD_CAP);
    }

    // GitLab #283: the resolved discount bps is part of the key, so two callers on different
    // tiers can never collide — and two callers on the SAME tier still share the cache.
    #[test]
    fn hybrid_cache_key_distinguishes_discount_bps() {
        let tier0 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 10_000, "none");
        let tier5 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000, "none");
        let tier9 = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 9_500, "none");
        assert_ne!(
            tier0, tier5,
            "different discount bps must not share a cache key"
        );
        assert_ne!(tier5, tier9);
        assert_eq!(
            tier5,
            hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 5_000, "none"),
            "same-discount callers should reuse the cache regardless of address"
        );
    }

    #[test]
    fn hybrid_cache_key_amount_bucket_regression() {
        let a = hybrid_cache_key(SV, TIN, TOUT, amount_cache_key(1_500_000), 8, 0, "none");
        let b = hybrid_cache_key(SV, TIN, TOUT, amount_cache_key(1_900_000), 8, 0, "none");
        assert_eq!(a, b, "micro-variation within one amount bucket shares key");
    }

    #[test]
    fn amount_cache_key_18dec_sizes_do_not_alias() {
        let one_ustr = 10u128.pow(18);
        let ten_k_ustr = 10_000 * 10u128.pow(18);
        let one_ust1 = 1_000_000u128;
        assert_eq!(amount_cache_key(one_ustr), one_ustr);
        assert_ne!(
            amount_cache_key(one_ustr),
            amount_cache_key(ten_k_ustr),
            "1 USTR and 10000 USTR must not share a raw bucket"
        );
        assert_ne!(
            amount_cache_key(one_ustr),
            amount_cache_key(one_ust1),
            "1 USTR (1e18) must not bucket as 1 UST1 (1e6)"
        );
        let ustr_key = hybrid_cache_key(SV, TIN, TOUT, amount_cache_key(one_ustr), 8, 0, "none");
        let ust1_pair = hybrid_cache_key(
            SV,
            "terra1ust1in00000000000000000000000000000",
            TOUT,
            amount_cache_key(one_ust1),
            8,
            0,
            "none",
        );
        assert_ne!(ustr_key, ust1_pair, "cache key includes token_in");
    }

    #[test]
    fn hybrid_cache_key_isolates_tax_identity() {
        let ordinary = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 0, "none");
        let taxed = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 0, "b0s0/b100s0/rh0/e0");
        let exempt = hybrid_cache_key(SV, TIN, TOUT, 1_000_000, 8, 0, "b0s0/b100s0/rh0/e1");
        assert_ne!(ordinary, taxed);
        assert_ne!(taxed, exempt);
    }
}

#[cfg(test)]
mod cache_ttl_tests {
    use super::{cache_ttl_for_hops, ROUTE_CACHE_TTL, ROUTE_CACHE_TTL_DISTANT};
    use std::time::Duration;

    #[test]
    fn cache_ttl_one_hop_uses_short_ttl() {
        assert_eq!(cache_ttl_for_hops(1), ROUTE_CACHE_TTL);
        assert_eq!(cache_ttl_for_hops(0), ROUTE_CACHE_TTL);
    }

    #[test]
    fn cache_ttl_multi_hop_uses_distant_ttl() {
        assert_eq!(cache_ttl_for_hops(2), ROUTE_CACHE_TTL_DISTANT);
        assert_eq!(cache_ttl_for_hops(4), ROUTE_CACHE_TTL_DISTANT);
        assert_eq!(ROUTE_CACHE_TTL_DISTANT, Duration::from_secs(90));
    }
}

/// Compatibility reset: the historical discount cache is absent from production.
#[cfg(not(test))]
pub fn reset_discount_bps_cache() {}
