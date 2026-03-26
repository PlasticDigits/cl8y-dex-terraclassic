use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{build_asset_map, internal_err, AppState};
use crate::db::queries::assets::AssetRow;
use crate::db::queries::{
    candles, limit_order_fills, limit_order_lifecycle, liquidity, pairs as db_pairs, swap_events,
};

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
    /// Sum of quote-side amounts in swaps over the last 24h (from indexer). Omitted when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_quote_24h: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct PairListResponse {
    pub items: Vec<PairResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

const PAIR_LIST_LIMIT_DEFAULT: i64 = 50;
const PAIR_LIST_LIMIT_MAX: i64 = 100;
const PAIR_LIST_Q_MAX_LEN: usize = 128;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListPairsQuery {
    /// Page size (default 50, max 100)
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
    /// Search pair address, token symbols, contract addresses, or denoms (substring, case-insensitive)
    pub q: Option<String>,
    /// Filter to pairs that include this token (exact CW20 contract or native denom)
    pub asset: Option<String>,
    /// Sort: `id`, `fee`, `created`, `symbol`, `volume_24h` (default `id`)
    pub sort: Option<String>,
    /// `asc` or `desc`. Default: `asc` for id/fee/created/symbol; `desc` for volume_24h
    pub order: Option<String>,
}

fn parse_pair_list_sort(s: Option<&str>) -> Result<db_pairs::PairListSort, (StatusCode, String)> {
    match s.map(str::trim).filter(|x| !x.is_empty()) {
        None | Some("id") => Ok(db_pairs::PairListSort::Id),
        Some("fee") => Ok(db_pairs::PairListSort::Fee),
        Some("created") => Ok(db_pairs::PairListSort::Created),
        Some("symbol") => Ok(db_pairs::PairListSort::Symbol),
        Some("volume_24h") => Ok(db_pairs::PairListSort::Volume24h),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid sort '{}'. Use id, fee, created, symbol, or volume_24h",
                other
            ),
        )),
    }
}

fn parse_pair_list_order(
    sort: db_pairs::PairListSort,
    order: Option<&str>,
) -> Result<bool, (StatusCode, String)> {
    match order.map(str::trim).filter(|x| !x.is_empty()) {
        None => Ok(matches!(sort, db_pairs::PairListSort::Volume24h)),
        Some(o) if o.eq_ignore_ascii_case("asc") => Ok(false),
        Some(o) if o.eq_ignore_ascii_case("desc") => Ok(true),
        Some(o) => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid order '{}'. Use asc or desc", o),
        )),
    }
}

fn volume_quote_to_string(v: &bigdecimal::BigDecimal) -> String {
    v.normalized().to_string()
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs",
    params(ListPairsQuery),
    responses(
        (status = 200, description = "Paginated trading pairs", body = PairListResponse),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn list_pairs(
    State(state): State<AppState>,
    Query(q): Query<ListPairsQuery>,
) -> Result<Json<PairListResponse>, (StatusCode, String)> {
    let limit = q
        .limit
        .unwrap_or(PAIR_LIST_LIMIT_DEFAULT)
        .clamp(1, PAIR_LIST_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);

    let q_trimmed = q.q.as_ref().map(|s| {
        let t = s.trim();
        t.chars().take(PAIR_LIST_Q_MAX_LEN).collect::<String>()
    });
    let q_ref = q_trimmed.as_deref().filter(|s| !s.is_empty());

    let asset_trimmed = q.asset.as_ref().map(|s| s.trim().to_string());
    let asset_ref = asset_trimmed.as_deref().filter(|s| !s.is_empty());

    let sort = parse_pair_list_sort(q.sort.as_deref())?;
    let sort_desc = parse_pair_list_order(sort, q.order.as_deref())?;

    let total = db_pairs::count_pairs_filtered(&state.pool, q_ref, asset_ref)
        .await
        .map_err(internal_err)?;

    let rows = db_pairs::list_pairs_filtered(
        &state.pool,
        db_pairs::PairListParams {
            q: q_ref,
            asset: asset_ref,
            sort,
            sort_desc,
            limit,
            offset,
        },
    )
    .await
    .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let p = &row.pair;
        let (Some(a0), Some(a1)) = (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id))
        else {
            continue;
        };
        let volume_quote_24h = row.volume_quote_24h.as_ref().map(volume_quote_to_string);
        items.push(PairResponse {
            pair_address: p.contract_address.clone(),
            asset_0: AssetBrief::from(a0),
            asset_1: AssetBrief::from(a1),
            lp_token: p.lp_token.clone(),
            fee_bps: p.fee_bps,
            is_active: true,
            volume_quote_24h,
        });
    }

    Ok(Json(PairListResponse {
        items,
        total,
        limit,
        offset,
    }))
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

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

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
        volume_quote_24h: None,
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
            format!(
                "Invalid interval '{}'. Valid: {}",
                interval,
                VALID_INTERVALS.join(", ")
            ),
        ));
    }

    let now = Utc::now();
    let from = q
        .from
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(|| now - chrono::Duration::days(7));
    let to =
        q.to.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
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

#[derive(Deserialize, IntoParams, utoipa::ToSchema)]
pub struct TradesQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return trades with id < before
    pub before: Option<i64>,
}

pub(crate) fn opt_bd_string(v: &Option<bigdecimal::BigDecimal>) -> Option<String> {
    v.as_ref().map(|b| b.normalized().to_string())
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
    /// Pattern C / hybrid: pool leg output (when present on-chain).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_book_offer_consumed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_fee_bps: Option<i16>,
}

/// Indexed per-maker fill from wasm `limit_order_fill` events.
#[derive(Serialize, ToSchema)]
pub struct LimitFillResponse {
    pub id: i64,
    pub pair_address: String,
    pub swap_event_id: Option<i64>,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub order_id: i64,
    pub side: String,
    pub maker: String,
    pub price: String,
    pub token0_amount: String,
    pub token1_amount: String,
    pub commission_amount: String,
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

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

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
                pool_return_amount: opt_bd_string(&t.pool_return_amount),
                book_return_amount: opt_bd_string(&t.book_return_amount),
                limit_book_offer_consumed: opt_bd_string(&t.limit_book_offer_consumed),
                effective_fee_bps: t.effective_fee_bps,
            }
        })
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams, utoipa::ToSchema)]
pub struct LiquidityEventsQuery {
    pub limit: Option<i64>,
    pub before: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct LiquidityEventResponse {
    pub id: i64,
    pub pair_address: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub provider: String,
    pub event_type: String,
    pub asset_0_amount: String,
    pub asset_1_amount: String,
    pub lp_amount: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/liquidity-events",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LiquidityEventsQuery,
    ),
    responses(
        (status = 200, description = "Add/remove liquidity events", body = Vec<LiquidityEventResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_liquidity_events(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LiquidityEventsQuery>,
) -> Result<Json<Vec<LiquidityEventResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let rows = liquidity::list_liquidity_for_pair(&state.pool, pair.id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let result: Vec<LiquidityEventResponse> = rows
        .iter()
        .map(|r| LiquidityEventResponse {
            id: r.id,
            pair_address: addr.clone(),
            block_height: r.block_height,
            block_timestamp: r.block_timestamp.to_rfc3339(),
            tx_hash: r.tx_hash.clone(),
            provider: r.provider.clone(),
            event_type: r.event_type.clone(),
            asset_0_amount: r.asset_0_amount.to_string(),
            asset_1_amount: r.asset_1_amount.to_string(),
            lp_amount: r.lp_amount.to_string(),
        })
        .collect();

    Ok(Json(result))
}

#[derive(Serialize, ToSchema)]
pub struct LimitPlacementResponse {
    pub id: i64,
    pub pair_address: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub order_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-placements",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitFillsQuery,
    ),
    responses(
        (status = 200, description = "Indexed place_limit_order events", body = Vec<LimitPlacementResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_placements(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitFillsQuery>,
) -> Result<Json<Vec<LimitPlacementResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let rows =
        limit_order_lifecycle::list_placements_for_pair(&state.pool, pair.id, limit, q.before)
            .await
            .map_err(internal_err)?;

    let result: Vec<LimitPlacementResponse> = rows
        .iter()
        .map(|r| LimitPlacementResponse {
            id: r.id,
            pair_address: addr.clone(),
            block_height: r.block_height,
            block_timestamp: r.block_timestamp.to_rfc3339(),
            tx_hash: r.tx_hash.clone(),
            order_id: r.order_id,
            owner: r.owner.clone(),
            side: r.side.clone(),
            price: r.price.as_ref().map(|p| p.normalized().to_string()),
            expires_at: r.expires_at,
        })
        .collect();

    Ok(Json(result))
}

#[derive(Serialize, ToSchema)]
pub struct LimitCancellationResponse {
    pub id: i64,
    pub pair_address: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub order_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-cancellations",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitFillsQuery,
    ),
    responses(
        (status = 200, description = "Indexed cancel_limit_order events", body = Vec<LimitCancellationResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_cancellations(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitFillsQuery>,
) -> Result<Json<Vec<LimitCancellationResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let rows =
        limit_order_lifecycle::list_cancellations_for_pair(&state.pool, pair.id, limit, q.before)
            .await
            .map_err(internal_err)?;

    let result: Vec<LimitCancellationResponse> = rows
        .iter()
        .map(|r| LimitCancellationResponse {
            id: r.id,
            pair_address: addr.clone(),
            block_height: r.block_height,
            block_timestamp: r.block_timestamp.to_rfc3339(),
            tx_hash: r.tx_hash.clone(),
            order_id: r.order_id,
            owner: r.owner.clone(),
        })
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams, utoipa::ToSchema)]
pub struct LimitFillsQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return rows with id < before
    pub before: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-fills",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitFillsQuery,
    ),
    responses(
        (status = 200, description = "Per-maker limit fills indexed from chain", body = Vec<LimitFillResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_fills(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitFillsQuery>,
) -> Result<Json<Vec<LimitFillResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let rows = limit_order_fills::list_fills_for_pair(&state.pool, pair.id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| LimitFillResponse {
            id: r.id,
            pair_address: addr.clone(),
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

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams)]
pub struct LimitFillsForOrderQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-orders/{order_id}/fills",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        ("order_id" = i64, Path, description = "On-chain limit order id"),
        LimitFillsForOrderQuery,
    ),
    responses(
        (status = 200, description = "Fills for a single resting order", body = Vec<LimitFillResponse>),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_order_limit_fills(
    State(state): State<AppState>,
    Path((addr, order_id)): Path<(String, i64)>,
    Query(q): Query<LimitFillsForOrderQuery>,
) -> Result<Json<Vec<LimitFillResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).min(200);
    let rows = limit_order_fills::list_fills_for_order(&state.pool, pair.id, order_id, limit)
        .await
        .map_err(internal_err)?;

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| LimitFillResponse {
            id: r.id,
            pair_address: addr.clone(),
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

    Ok(Json(result))
}

#[derive(Serialize, ToSchema)]
pub struct PairStatsResponse {
    pub volume_base: String,
    pub volume_quote: String,
    pub volume_usd: Option<String>,
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
        volume_usd: stats.volume_usd.map(|v| v.to_string()),
        trade_count: stats.trade_count,
        high: stats.high.map(|v| v.to_string()),
        low: stats.low.map(|v| v.to_string()),
        open_price: stats.open_price.map(|v| v.to_string()),
        close_price: stats.close_price.map(|v| v.to_string()),
        price_change_pct: stats.price_change_pct,
    }))
}
