use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{build_asset_map, internal_err, AppState};
use crate::db::queries::assets::AssetRow;
use crate::db::queries::{candles, pairs as db_pairs, swap_events};

pub const VALID_INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

#[derive(Serialize, ToSchema)]
pub struct AssetBrief {
    pub symbol: String,
    pub contract_addr: Option<String>,
    pub denom: Option<String>,
    pub decimals: i16,
}

impl From<&AssetRow> for AssetBrief {
    fn from(a: &AssetRow) -> Self {
        Self {
            symbol: a.symbol.clone(),
            contract_addr: a.contract_address.clone(),
            denom: a.denom.clone(),
            decimals: a.decimals,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct PairResponse {
    pub pair_address: String,
    pub asset_0: AssetBrief,
    pub asset_1: AssetBrief,
    pub lp_token: Option<String>,
    pub fee_bps: Option<i16>,
    pub is_active: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs",
    responses(
        (status = 200, description = "List of all trading pairs", body = Vec<PairResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn list_pairs(
    State(state): State<AppState>,
) -> Result<Json<Vec<PairResponse>>, (StatusCode, String)> {
    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    let mut result = Vec::with_capacity(all_pairs.len());
    for p in &all_pairs {
        if let (Some(a0), Some(a1)) =
            (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id))
        {
            result.push(PairResponse {
                pair_address: p.contract_address.clone(),
                asset_0: AssetBrief::from(a0),
                asset_1: AssetBrief::from(a1),
                lp_token: p.lp_token.clone(),
                fee_bps: p.fee_bps,
                is_active: true,
            });
        }
    }

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
    ),
    responses(
        (status = 200, description = "Pair details", body = PairResponse),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<PairResponse>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    let a0 = asset_map
        .get(&pair.asset_0_id)
        .ok_or_else(|| internal_err("Asset 0 not found"))?;
    let a1 = asset_map
        .get(&pair.asset_1_id)
        .ok_or_else(|| internal_err("Asset 1 not found"))?;

    Ok(Json(PairResponse {
        pair_address: pair.contract_address,
        asset_0: AssetBrief::from(a0),
        asset_1: AssetBrief::from(a1),
        lp_token: pair.lp_token,
        fee_bps: pair.fee_bps,
        is_active: true,
    }))
}

#[derive(Deserialize, IntoParams)]
pub struct CandleQuery {
    /// Candle interval: 1m, 5m, 15m, 1h, 4h, 1d, 1w
    pub interval: Option<String>,
    /// Start time (RFC 3339)
    pub from: Option<String>,
    /// End time (RFC 3339)
    pub to: Option<String>,
    /// Max results (capped at 1000)
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CandleResponse {
    pub open_time: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume_base: String,
    pub volume_quote: String,
    pub trade_count: i32,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/candles",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        CandleQuery,
    ),
    responses(
        (status = 200, description = "OHLCV candle data", body = Vec<CandleResponse>),
        (status = 400, description = "Invalid interval"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_candles(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<CandleQuery>,
) -> Result<Json<Vec<CandleResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let interval = q.interval.unwrap_or_else(|| "1h".to_string());
    if !VALID_INTERVALS.contains(&interval.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid interval '{}'. Valid: {}", interval, VALID_INTERVALS.join(", ")),
        ));
    }

    let now = Utc::now();
    let from = q
        .from
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
        .unwrap_or_else(|| now - chrono::Duration::days(7));
    let to = q
        .to
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
        .unwrap_or(now);
    let limit = q.limit.unwrap_or(200).min(1000);

    let rows = candles::get_candles(&state.pool, pair.id, &interval, from, to, limit)
        .await
        .map_err(internal_err)?;

    let result: Vec<CandleResponse> = rows
        .iter()
        .map(|c| CandleResponse {
            open_time: c.open_time.to_rfc3339(),
            open: c.open.to_string(),
            high: c.high.to_string(),
            low: c.low.to_string(),
            close: c.close.to_string(),
            volume_base: c.volume_base.to_string(),
            volume_quote: c.volume_quote.to_string(),
            trade_count: c.trade_count,
        })
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams)]
pub struct TradesQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return trades with id < before
    pub before: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct TradeResponse {
    pub id: i64,
    pub pair_address: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub sender: String,
    pub offer_asset: String,
    pub ask_asset: String,
    pub offer_amount: String,
    pub return_amount: String,
    pub price: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/trades",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        TradesQuery,
    ),
    responses(
        (status = 200, description = "Recent trades for pair", body = Vec<TradeResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_trades(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<TradesQuery>,
) -> Result<Json<Vec<TradeResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let trades = swap_events::get_trades_for_pair(&state.pool, pair.id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    let result: Vec<TradeResponse> = trades
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
            TradeResponse {
                id: t.id,
                pair_address: addr.clone(),
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

#[derive(Serialize, ToSchema)]
pub struct PairStatsResponse {
    pub volume_base: String,
    pub volume_quote: String,
    pub trade_count: i64,
    pub high: Option<String>,
    pub low: Option<String>,
    pub open_price: Option<String>,
    pub close_price: Option<String>,
    pub price_change_pct: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/stats",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
    ),
    responses(
        (status = 200, description = "24h statistics for pair", body = PairStatsResponse),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_stats(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<PairStatsResponse>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let stats = swap_events::get_24h_stats_for_pair(&state.pool, pair.id)
        .await
        .map_err(internal_err)?;

    Ok(Json(PairStatsResponse {
        volume_base: stats.volume_base.to_string(),
        volume_quote: stats.volume_quote.to_string(),
        trade_count: stats.trade_count,
        high: stats.high.map(|v| v.to_string()),
        low: stats.low.map(|v| v.to_string()),
        open_price: stats.open_price.map(|v| v.to_string()),
        close_price: stats.close_price.map(|v| v.to_string()),
        price_change_pct: stats.price_change_pct,
    }))
}
