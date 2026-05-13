use std::collections::HashMap;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};

use super::pairs::{trade_response_from_swap_row, LimitCancellationResponse, LimitFillResponse};
use super::{build_asset_map, internal_err, text_csv, AppState};
use crate::db::queries::{
    limit_order_fills, limit_order_lifecycle, pairs as db_pairs, positions as db_positions,
    swap_events, traders as db_traders,
};

pub const VALID_SORTS: &[&str] = &[
    "total_volume",
    "volume_24h",
    "volume_7d",
    "volume_30d",
    "total_trades",
    "total_realized_pnl",
    "best_trade_pnl",
    "worst_trade_pnl",
    "total_fees_paid",
];

#[derive(Serialize, ToSchema)]
pub struct TraderResponse {
    pub address: String,
    pub total_trades: i64,
    pub total_volume: String,
    pub volume_24h: String,
    pub volume_7d: String,
    pub volume_30d: String,
    pub tier_id: Option<i16>,
    pub tier_name: Option<String>,
    pub registered: bool,
    pub first_trade_at: Option<String>,
    pub last_trade_at: Option<String>,
    pub total_realized_pnl: String,
    pub best_trade_pnl: String,
    pub worst_trade_pnl: String,
    pub total_fees_paid: String,
}

impl From<&db_traders::TraderRow> for TraderResponse {
    fn from(t: &db_traders::TraderRow) -> Self {
        Self {
            address: t.address.clone(),
            total_trades: t.total_trades,
            total_volume: t.total_volume.to_string(),
            volume_24h: t.volume_24h.to_string(),
            volume_7d: t.volume_7d.to_string(),
            volume_30d: t.volume_30d.to_string(),
            tier_id: Some(t.tier_id),
            tier_name: Some(t.tier_name.clone()),
            registered: t.registered,
            first_trade_at: t.first_trade_at.map(|d| d.to_rfc3339()),
            last_trade_at: t.last_trade_at.map(|d| d.to_rfc3339()),
            total_realized_pnl: t.total_realized_pnl.to_string(),
            best_trade_pnl: t.best_trade_pnl.to_string(),
            worst_trade_pnl: t.worst_trade_pnl.to_string(),
            total_fees_paid: t.total_fees_paid.to_string(),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PositionResponse {
    pub pair_address: String,
    pub asset_0_symbol: String,
    pub asset_1_symbol: String,
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
    let limit = q.limit.unwrap_or(50).min(200);

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
    let limit = q.limit.unwrap_or(50).min(200);
    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;
    let rows = limit_order_fills::list_fills_for_maker(&state.pool, &addr, pair_id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, String> = all_pairs
        .into_iter()
        .map(|p| (p.id, p.contract_address))
        .collect();

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| LimitFillResponse {
            id: r.id,
            pair_address: pair_map.get(&r.pair_id).cloned().unwrap_or_default(),
            swap_event_id: r.swap_event_id,
            block_height: r.block_height,
            block_timestamp: r.block_timestamp.to_rfc3339(),
            tx_hash: r.tx_hash.clone(),
            order_id: r.order_id,
            side: r.side.clone(),
            maker: r.maker.clone(),
            price: r.price.to_string(),
            token0_amount: r.token0_amount.to_string(),
            token1_amount: r.token1_amount.to_string(),
            commission_amount: r.commission_amount.to_string(),
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
    let limit = q.limit.unwrap_or(50).min(200);
    let pair_id = resolve_pair_filter(&state.pool, q.pair.as_deref()).await?;
    let rows =
        limit_order_lifecycle::list_cancellations_for_owner(&state.pool, &addr, pair_id, limit, q.before)
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
        let resp =
            trader_csv_response(&format!("trader-limit-cancellations-{slug}.csv"), csv)?;
        Ok(resp.into_response())
    } else {
        Ok(Json(result).into_response())
    }
}

#[derive(Deserialize, IntoParams)]
pub struct LeaderboardQuery {
    /// Sort column: total_volume, volume_24h, volume_7d, volume_30d, total_trades
    pub sort: Option<String>,
    /// Max results (capped at 200)
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/leaderboard",
    params(LeaderboardQuery),
    responses(
        (status = 200, description = "Trader leaderboard", body = Vec<TraderResponse>),
        (status = 400, description = "Invalid sort column"),
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
    let limit = q.limit.unwrap_or(50).min(200);

    let rows = db_traders::get_leaderboard(&state.pool, &sort_by, limit)
        .await
        .map_err(internal_err)?;

    Ok(Json(rows.iter().map(TraderResponse::from).collect()))
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
            let a0_sym = asset_map
                .get(&pair.asset_0_id)
                .map(|a| a.symbol.clone())
                .unwrap_or_default();
            let a1_sym = asset_map
                .get(&pair.asset_1_id)
                .map(|a| a.symbol.clone())
                .unwrap_or_default();
            Some(PositionResponse {
                pair_address: pair.contract_address.clone(),
                asset_0_symbol: a0_sym,
                asset_1_symbol: a1_sym,
                net_position_quote: pos.net_position_quote.to_string(),
                avg_entry_price: pos.avg_entry_price.to_string(),
                total_cost_base: pos.total_cost_base.to_string(),
                realized_pnl: pos.realized_pnl.to_string(),
                trade_count: pos.trade_count,
            })
        })
        .collect();

    Ok(Json(result))
}
