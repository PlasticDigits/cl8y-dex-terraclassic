use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};

use super::overview::overview_volume_usd_field;
use super::pairs::{
    asset_map_decimals, limit_fill_response_from_row, limit_placement_response,
    parse_placement_lifecycle_filter, trade_response_from_swap_row, LimitCancellationResponse,
    LimitFillResponse, LimitPlacementResponse,
};
use super::{build_asset_map, internal_err, text_csv, AppState};
use crate::db::queries::{
    assets as db_assets, limit_order_fills, limit_order_lifecycle, pairs as db_pairs,
    positions as db_positions, swap_events, traders as db_traders,
};

pub const VALID_SORTS: &[&str] = &[
    "total_volume",
    "total_volume_usd",
    "volume_24h",
    "volume_7d",
    "volume_30d",
    "total_trades",
    "total_realized_pnl",
    "best_trade_pnl",
    "worst_trade_pnl",
    "total_fees_paid",
];

/// Sorts that can be computed from one pair’s `swap_events` / `trader_positions`
/// (GitLab #666). `best_trade_pnl` has no per-pair store — **400** when `pair=` is set.
pub const PAIR_SCOPED_SORTS: &[&str] = &[
    "total_volume",
    "total_volume_usd",
    "total_trades",
    "total_realized_pnl",
    "worst_trade_pnl",
];

// GitLab #280 / #666: short TTL cache for the unauthenticated /traders/leaderboard
// response. Keyed by `{sort}|{limit}|{pair_or_-}`. 404s are not cached.
const LEADERBOARD_CACHE_TTL: Duration = Duration::from_secs(60);
const LEADERBOARD_CACHE_MAX_ENTRIES: usize = 64;

struct LeaderboardCacheEntry {
    rows: Vec<TraderResponse>,
    at: Instant,
}

fn leaderboard_cache() -> &'static Mutex<HashMap<String, LeaderboardCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, LeaderboardCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn leaderboard_cache_get(key: &str) -> Option<Vec<TraderResponse>> {
    let guard = leaderboard_cache().lock().ok()?;
    let entry = guard.get(key)?;
    if Instant::now().duration_since(entry.at) > LEADERBOARD_CACHE_TTL {
        return None;
    }
    Some(entry.rows.clone())
}

fn leaderboard_cache_put(key: String, rows: Vec<TraderResponse>) {
    if let Ok(mut guard) = leaderboard_cache().lock() {
        let now = Instant::now();
        guard.retain(|_, v| now.duration_since(v.at) <= LEADERBOARD_CACHE_TTL);
        if guard.len() >= LEADERBOARD_CACHE_MAX_ENTRIES {
            guard.clear();
        }
        guard.insert(key, LeaderboardCacheEntry { rows, at: now });
    }
}

/// Drop the 60s `/traders/leaderboard` cache (tests).
pub fn reset_leaderboard_cache() {
    if let Ok(mut guard) = leaderboard_cache().lock() {
        guard.clear();
    }
}

#[derive(Serialize, ToSchema, Clone)]
pub struct TraderResponse {
    pub address: String,
    pub total_trades: i64,
    pub total_volume: String,
    /// P522-Q USD lifetime volume. JSON `null` when `total_trades > 0` and priced USD is 0 (#553).
    pub total_volume_usd: Option<String>,
    pub volume_24h: String,
    pub volume_7d: String,
    pub volume_30d: String,
    pub tier_id: Option<i16>,
    pub tier_name: Option<String>,
    pub registered: bool,
    pub first_trade_at: Option<String>,
    pub last_trade_at: Option<String>,
    pub total_realized_pnl: String,
    pub best_trade_pnl: Option<String>,
    pub worst_trade_pnl: Option<String>,
    pub total_fees_paid: String,
}

impl From<&db_traders::TraderRow> for TraderResponse {
    fn from(t: &db_traders::TraderRow) -> Self {
        Self {
            address: t.address.clone(),
            total_trades: t.total_trades,
            total_volume: t.total_volume.to_string(),
            total_volume_usd: overview_volume_usd_field(
                t.total_trades,
                t.total_volume_usd.as_ref().unwrap_or(&BigDecimal::from(0)),
            ),
            volume_24h: t.volume_24h.to_string(),
            volume_7d: t.volume_7d.to_string(),
            volume_30d: t.volume_30d.to_string(),
            tier_id: Some(t.tier_id),
            tier_name: Some(t.tier_name.clone()),
            registered: t.registered,
            first_trade_at: t.first_trade_at.map(|d| d.to_rfc3339()),
            last_trade_at: t.last_trade_at.map(|d| d.to_rfc3339()),
            total_realized_pnl: t.total_realized_pnl.to_string(),
            best_trade_pnl: t.best_trade_pnl.as_ref().map(|v| v.to_string()),
            worst_trade_pnl: t.worst_trade_pnl.as_ref().map(|v| v.to_string()),
            total_fees_paid: t.total_fees_paid.to_string(),
        }
    }
}

impl From<&db_traders::PairLeaderboardRow> for TraderResponse {
    fn from(r: &db_traders::PairLeaderboardRow) -> Self {
        Self {
            address: r.address.clone(),
            total_trades: r.total_trades,
            total_volume: r.total_volume.to_string(),
            total_volume_usd: overview_volume_usd_field(
                r.total_trades,
                r.total_volume_usd.as_ref().unwrap_or(&BigDecimal::from(0)),
            ),
            // Pair ranks are lifetime-on-this-pair; do not leak global rolling columns.
            volume_24h: "0".to_string(),
            volume_7d: "0".to_string(),
            volume_30d: "0".to_string(),
            tier_id: r.tier_id,
            tier_name: r.tier_name.clone(),
            registered: r.registered,
            first_trade_at: r.first_trade_at.map(|d| d.to_rfc3339()),
            last_trade_at: r.last_trade_at.map(|d| d.to_rfc3339()),
            total_realized_pnl: r.total_realized_pnl.to_string(),
            best_trade_pnl: None,
            worst_trade_pnl: None,
            total_fees_paid: "0".to_string(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PositionResponse {
    pub pair_address: String,
    pub asset_0_symbol: String,
    pub asset_1_symbol: String,
    /// Factory `asset_0` decimals — scale `total_cost_base` / `realized_pnl` / avg-entry (GitLab #551).
    /// `null` when the asset row is missing — dApp shows **—**, never assumes 6 (GitLab #560).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_0_decimals: Option<i16>,
    /// Factory `asset_1` (quote) decimals — scale `net_position_quote` (GitLab #551).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_1_decimals: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_0_denom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_1_denom: Option<String>,
    pub net_position_quote: String,
    pub avg_entry_price: String,
    pub total_cost_base: String,
    pub realized_pnl: String,
    pub trade_count: i32,
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}",
    params(
        ("addr" = String, Path, description = "Trader wallet address"),
    ),
    responses(
        (status = 200, description = "Trader profile", body = TraderResponse),
        (status = 404, description = "Trader not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_profile(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<TraderResponse>, (StatusCode, String)> {
    let trader = db_traders::get_trader(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Trader not found".to_string()))?;

    Ok(Json(TraderResponse::from(&trader)))
}

fn trader_history_format(fmt: Option<&str>) -> Result<bool, (StatusCode, String)> {
    match fmt.map(str::trim).filter(|s| !s.is_empty()) {
        None | Some("json") => Ok(false),
        Some("csv") => Ok(true),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid format '{other}'. Use json or csv"),
        )),
    }
}

fn trader_csv_response(filename: &str, body: String) -> Result<Response, (StatusCode, String)> {
    let ct = header::HeaderValue::from_static("text/csv; charset=utf-8");
    let cd = header::HeaderValue::try_from(format!("attachment; filename=\"{filename}\""))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        })?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CONTENT_DISPOSITION, cd)
        .body(Body::from(body))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            )
        })
}

fn trader_csv_slug(addr: &str) -> String {
    addr.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(24)
        .collect()
}

async fn resolve_pair_filter(
    pool: &PgPool,
    pair: Option<&str>,
) -> Result<Option<i32>, (StatusCode, String)> {
    let Some(raw) = pair.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let row = db_pairs::get_pair_by_address(pool, raw)
        .await
        .map_err(internal_err)?;
    let p = row.ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;
    Ok(Some(p.id))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct TraderTradesQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return trades with id < before
    pub before: Option<i64>,
    /// `json` (default) or `csv` (`Content-Type: text/csv`, attachment).
    pub format: Option<String>,
    /// When set, only swaps on this pair contract address are returned (**404** if unknown pair).
    pub pair: Option<String>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct TraderHistoryQuery {
    pub limit: Option<i64>,
    pub before: Option<i64>,
    pub format: Option<String>,
    /// When set, restrict rows to this pair contract (**404** if unknown).
    pub pair: Option<String>,
}

#[derive(Deserialize, IntoParams, utoipa::ToSchema)]
pub struct TraderLimitPlacementsQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return rows with id < before
    pub before: Option<i64>,
    /// Filter by lifecycle: omit for **`active` + `parked_expired`**; `active`, `parked_expired`, `refunded`, or `all`.
    pub status: Option<String>,
    /// When set, restrict rows to this pair contract (**404** if unknown).
    pub pair: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/trades",
    params(
        ("addr" = String, Path, description = "Trader wallet address"),
        TraderTradesQuery,
    ),
    responses(
        (status = 200, description = "Trader swap history (JSON default, or CSV when format=csv)", body = Vec<super::pairs::TradeResponse>),
        (status = 400, description = "Invalid format= or bad query"),
        (status = 404, description = "Unknown pair address (pair= filter)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_trades(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TraderTradesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let use_csv = trader_history_format(q.format.as_deref())?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;

    let trades = swap_events::get_trades_for_trader(&state.pool, &addr, pair_id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, String> = all_pairs
        .into_iter()
        .map(|p| (p.id, p.contract_address))
        .collect();

    let result: Vec<super::pairs::TradeResponse> = trades
        .iter()
        .map(|t| {
            let pair_addr = pair_map.get(&t.pair_id).cloned().unwrap_or_default();
            trade_response_from_swap_row(&pair_addr, t, &asset_map)
        })
        .collect();

    if use_csv {
        let slug = trader_csv_slug(&addr);
        let csv = text_csv::trader_swaps_csv(&result);
        let resp = trader_csv_response(&format!("trader-swaps-{slug}.csv"), csv)?;
        Ok(resp.into_response())
    } else {
        Ok(Json(result).into_response())
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/limit-fills",
    params(
        ("addr" = String, Path, description = "Trader wallet address (maker on fills)"),
        TraderHistoryQuery,
    ),
    responses(
        (status = 200, description = "Limit fills where this address is the indexed maker", body = Vec<LimitFillResponse>),
        (status = 400, description = "Invalid format="),
        (status = 404, description = "Unknown pair address (pair= filter)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_limit_fills(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TraderHistoryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let use_csv = trader_history_format(q.format.as_deref())?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;
    let rows =
        limit_order_fills::list_fills_for_maker(&state.pool, &addr, pair_id, limit, q.before)
            .await
            .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;
    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_meta: HashMap<i32, (String, Option<i16>, Option<i16>)> = all_pairs
        .into_iter()
        .map(|p| {
            (
                p.id,
                (
                    p.contract_address,
                    asset_map_decimals(&asset_map, p.asset_0_id),
                    asset_map_decimals(&asset_map, p.asset_1_id),
                ),
            )
        })
        .collect();

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| {
            let (pair_addr, token0_decimals, token1_decimals) =
                pair_meta.get(&r.pair_id).cloned().unwrap_or_default();
            limit_fill_response_from_row(&pair_addr, r, token0_decimals, token1_decimals)
        })
        .collect();

    if use_csv {
        let slug = trader_csv_slug(&addr);
        let csv = text_csv::trader_limit_fills_csv(&result);
        let resp = trader_csv_response(&format!("trader-limit-fills-{slug}.csv"), csv)?;
        Ok(resp.into_response())
    } else {
        Ok(Json(result).into_response())
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/limit-cancellations",
    params(
        ("addr" = String, Path, description = "Trader wallet address (indexed cancel owner)"),
        TraderHistoryQuery,
    ),
    responses(
        (status = 200, description = "Limit cancellations attributed to this owner in the indexer", body = Vec<LimitCancellationResponse>),
        (status = 400, description = "Invalid format="),
        (status = 404, description = "Unknown pair address (pair= filter)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_limit_cancellations(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TraderHistoryQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let use_csv = trader_history_format(q.format.as_deref())?;
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;
    let rows = limit_order_lifecycle::list_cancellations_for_owner(
        &state.pool,
        &addr,
        pair_id,
        limit,
        q.before,
    )
    .await
    .map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, String> = all_pairs
        .into_iter()
        .map(|p| (p.id, p.contract_address))
        .collect();

    let result: Vec<LimitCancellationResponse> = rows
        .iter()
        .map(|r| LimitCancellationResponse {
            id: r.id,
            pair_address: pair_map.get(&r.pair_id).cloned().unwrap_or_default(),
            block_height: r.block_height,
            block_timestamp: r.block_timestamp.to_rfc3339(),
            tx_hash: r.tx_hash.clone(),
            order_id: r.order_id,
            owner: r.owner.clone(),
        })
        .collect();

    if use_csv {
        let slug = trader_csv_slug(&addr);
        let csv = text_csv::trader_limit_cancellations_csv(&result);
        let resp = trader_csv_response(&format!("trader-limit-cancellations-{slug}.csv"), csv)?;
        Ok(resp.into_response())
    } else {
        Ok(Json(result).into_response())
    }
}

#[derive(Deserialize, IntoParams)]
pub struct LeaderboardQuery {
    /// Sort column: total_volume (raw), total_volume_usd (P522-Q), volume_24h, …
    pub sort: Option<String>,
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// When set, ranks wallets on this pair **contract address** only (GitLab #666).
    /// Unknown pair → **404**. `sort=best_trade_pnl` with `pair=` → **400**.
    pub pair: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/limit-placements",
    params(
        ("addr" = String, Path, description = "Trader wallet address (indexed placement owner)"),
        TraderLimitPlacementsQuery,
    ),
    responses(
        (status = 200, description = "Open limit placements for this wallet across pairs (excludes indexed cancels; GitLab #217)", body = Vec<LimitPlacementResponse>),
        (status = 400, description = "Invalid status= or bad query"),
        (status = 404, description = "Unknown pair address (pair= filter)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_limit_placements(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TraderLimitPlacementsQuery>,
) -> Result<Json<Vec<LimitPlacementResponse>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let lifecycle = parse_placement_lifecycle_filter(q.status.as_deref())?;
    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;
    let rows = limit_order_lifecycle::list_placements_for_owner(
        &state.pool,
        &addr,
        pair_id,
        limit,
        q.before,
        lifecycle,
    )
    .await
    .map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, String> = all_pairs
        .into_iter()
        .map(|p| (p.id, p.contract_address))
        .collect();

    let result: Vec<LimitPlacementResponse> = rows
        .iter()
        .map(|r| {
            let pair_addr = pair_map.get(&r.pair_id).cloned().unwrap_or_default();
            limit_placement_response(&pair_addr, r)
        })
        .collect();

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/leaderboard",
    params(LeaderboardQuery),
    responses(
        (status = 200, description = "Trader leaderboard (DEX-wide, or pair-scoped when pair= is set)", body = Vec<TraderResponse>),
        (status = 400, description = "Invalid sort column, or pair= with a sort that is not pair-scoped"),
        (status = 404, description = "Unknown pair address (pair= filter)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn leaderboard(
    State(state): State<AppState>,
    Query(q): Query<LeaderboardQuery>,
) -> Result<Json<Vec<TraderResponse>>, (StatusCode, String)> {
    if let Some(ref s) = q.sort {
        if !VALID_SORTS.contains(&s.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Invalid sort '{}'. Valid: {}", s, VALID_SORTS.join(", ")),
            ));
        }
    }

    let sort_by = q.sort.unwrap_or_else(|| "total_volume".to_string());
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let pair_filter = q.pair.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(pair_addr) = pair_filter {
        if !PAIR_SCOPED_SORTS.contains(&sort_by.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid sort '{sort_by}' for pair leaderboard. Valid: {}",
                    PAIR_SCOPED_SORTS.join(", ")
                ),
            ));
        }
        // Resolve by contract address (not internal pair_id). 404 is not cached as [].
        let pair_row = db_pairs::get_pair_by_address(&state.pool, pair_addr)
            .await
            .map_err(internal_err)?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

        let cache_key = format!("{sort_by}|{limit}|{}", pair_row.contract_address);
        if let Some(cached) = leaderboard_cache_get(&cache_key) {
            return Ok(Json(cached));
        }

        let rows =
            db_traders::get_leaderboard_for_pair(&state.pool, pair_row.id, &sort_by, limit)
                .await
                .map_err(internal_err)?;
        let resp: Vec<TraderResponse> = rows.iter().map(TraderResponse::from).collect();
        leaderboard_cache_put(cache_key, resp.clone());
        return Ok(Json(resp));
    }

    let cache_key = format!("{sort_by}|{limit}|-");
    if let Some(cached) = leaderboard_cache_get(&cache_key) {
        return Ok(Json(cached));
    }

    let rows = db_traders::get_leaderboard(&state.pool, &sort_by, limit)
        .await
        .map_err(internal_err)?;
    let resp: Vec<TraderResponse> = rows.iter().map(TraderResponse::from).collect();
    leaderboard_cache_put(cache_key, resp.clone());

    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/positions",
    params(
        ("addr" = String, Path, description = "Trader wallet address"),
    ),
    responses(
        (status = 200, description = "Trader's open positions", body = Vec<PositionResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_positions(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<Vec<PositionResponse>>, (StatusCode, String)> {
    let positions = db_positions::get_positions_for_trader(&state.pool, &addr)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, &db_pairs::PairRow> = all_pairs.iter().map(|p| (p.id, p)).collect();

    let result: Vec<PositionResponse> = positions
        .iter()
        .filter_map(|pos| {
            let pair = pair_map.get(&pos.pair_id)?;
            Some(map_trader_position(
                &pair.contract_address,
                asset_map.get(&pair.asset_0_id),
                asset_map.get(&pair.asset_1_id),
                pos,
            ))
        })
        .collect();

    Ok(Json(result))
}

struct AssetLeg {
    symbol: String,
    decimals: i16,
    denom: Option<String>,
}

impl From<&db_assets::AssetRow> for AssetLeg {
    fn from(a: &db_assets::AssetRow) -> Self {
        Self {
            symbol: a.symbol.clone(),
            decimals: a.decimals,
            denom: a.denom.clone(),
        }
    }
}

fn missing_asset_symbol() -> String {
    "—".to_string()
}

/// Keep the position row when a pair exists even if an asset row is missing (GitLab #560).
/// UI scales with decimals or shows **—**; do not drop the trade history.
fn map_trader_position(
    pair_address: &str,
    a0: Option<&db_assets::AssetRow>,
    a1: Option<&db_assets::AssetRow>,
    pos: &db_positions::PositionRow,
) -> PositionResponse {
    let a0_leg = a0.map(AssetLeg::from);
    let a1_leg = a1.map(AssetLeg::from);
    map_trader_position_legs(
        pair_address,
        a0_leg.as_ref(),
        a1_leg.as_ref(),
        &pos.net_position_quote.to_string(),
        &pos.avg_entry_price.to_string(),
        &pos.total_cost_base.to_string(),
        &pos.realized_pnl.to_string(),
        pos.trade_count,
    )
}

#[allow(clippy::too_many_arguments)]
fn map_trader_position_legs(
    pair_address: &str,
    a0: Option<&AssetLeg>,
    a1: Option<&AssetLeg>,
    net_position_quote: &str,
    avg_entry_price: &str,
    total_cost_base: &str,
    realized_pnl: &str,
    trade_count: i32,
) -> PositionResponse {
    let (s0, d0, n0) = match a0 {
        Some(a) => (
            if a.symbol.is_empty() {
                missing_asset_symbol()
            } else {
                a.symbol.clone()
            },
            Some(a.decimals),
            a.denom.clone(),
        ),
        None => (missing_asset_symbol(), None, None),
    };
    let (s1, d1, n1) = match a1 {
        Some(a) => (
            if a.symbol.is_empty() {
                missing_asset_symbol()
            } else {
                a.symbol.clone()
            },
            Some(a.decimals),
            a.denom.clone(),
        ),
        None => (missing_asset_symbol(), None, None),
    };
    PositionResponse {
        pair_address: pair_address.to_string(),
        asset_0_symbol: s0,
        asset_1_symbol: s1,
        asset_0_decimals: d0,
        asset_1_decimals: d1,
        asset_0_denom: n0,
        asset_1_denom: n1,
        net_position_quote: net_position_quote.to_string(),
        avg_entry_price: avg_entry_price.to_string(),
        total_cost_base: total_cost_base.to_string(),
        realized_pnl: realized_pnl.to_string(),
        trade_count,
    }
}

#[cfg(test)]
mod position_map_tests {
    use super::*;

    #[test]
    fn missing_asset_row_keeps_position_with_null_decimals() {
        let row = map_trader_position_legs(
            "terra1pair",
            None,
            Some(&AssetLeg {
                symbol: "cUSTC".into(),
                decimals: 6,
                denom: None,
            }),
            "100",
            "0.5",
            "50",
            "0",
            1,
        );
        assert_eq!(row.pair_address, "terra1pair");
        assert_eq!(row.asset_0_symbol, "—");
        assert_eq!(row.asset_1_symbol, "cUSTC");
        assert_eq!(row.asset_0_decimals, None);
        assert_eq!(row.asset_1_decimals, Some(6));
        assert_eq!(row.net_position_quote, "100");
        let json = serde_json::to_value(&row).expect("json");
        assert!(json.get("asset_0_decimals").is_none());
        assert_eq!(json["asset_1_decimals"], 6);
    }

    #[test]
    fn both_assets_present_keep_decimals() {
        let row = map_trader_position_legs(
            "terra1pair",
            Some(&AssetLeg {
                symbol: "UST1".into(),
                decimals: 6,
                denom: None,
            }),
            Some(&AssetLeg {
                symbol: "USTR".into(),
                decimals: 18,
                denom: None,
            }),
            "1",
            "1",
            "1",
            "1",
            2,
        );
        assert_eq!(row.asset_0_decimals, Some(6));
        assert_eq!(row.asset_1_decimals, Some(18));
        assert_eq!(row.trade_count, 2);
    }
}
