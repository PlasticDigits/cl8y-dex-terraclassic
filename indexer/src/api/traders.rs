use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{build_asset_map, internal_err, AppState};
use crate::db::queries::{pairs as db_pairs, positions as db_positions, swap_events, traders as db_traders};

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

#[derive(Deserialize, IntoParams)]
pub struct TraderTradesQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return trades with id < before
    pub before: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/traders/{addr}/trades",
    params(
        ("addr" = String, Path, description = "Trader wallet address"),
        TraderTradesQuery,
    ),
    responses(
        (status = 200, description = "Trader's trade history", body = Vec<super::pairs::TradeResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Traders"
)]
pub async fn get_trader_trades(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TraderTradesQuery>,
) -> Result<Json<Vec<super::pairs::TradeResponse>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(50).min(200);
    let trades = swap_events::get_trades_for_trader(&state.pool, &addr, limit, q.before)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

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
            let offer_sym = asset_map
                .get(&t.offer_asset_id)
                .map(|a| a.symbol.clone())
                .unwrap_or_default();
            let ask_sym = asset_map
                .get(&t.ask_asset_id)
                .map(|a| a.symbol.clone())
                .unwrap_or_default();
            let pair_addr = pair_map.get(&t.pair_id).cloned().unwrap_or_default();
            super::pairs::TradeResponse {
                id: t.id,
                pair_address: pair_addr,
                block_height: t.block_height,
                block_timestamp: t.block_timestamp.to_rfc3339(),
                tx_hash: t.tx_hash.clone(),
                sender: t.sender.clone(),
                offer_asset: offer_sym,
                ask_asset: ask_sym,
                offer_amount: t.offer_amount.to_string(),
                return_amount: t.return_amount.to_string(),
                price: t.price.to_string(),
            }
        })
        .collect();

    Ok(Json(result))
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
                format!(
                    "Invalid sort '{}'. Valid: {}",
                    s,
                    VALID_SORTS.join(", ")
                ),
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

    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let pair_map: HashMap<i32, &db_pairs::PairRow> = all_pairs.iter().map(|p| (p.id, p)).collect();

    let result: Vec<PositionResponse> = positions
        .iter()
        .filter_map(|pos| {
            let pair = pair_map.get(&pos.pair_id)?;
            let a0_sym = asset_map.get(&pair.asset_0_id).map(|a| a.symbol.clone()).unwrap_or_default();
            let a1_sym = asset_map.get(&pair.asset_1_id).map(|a| a.symbol.clone()).unwrap_or_default();
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
