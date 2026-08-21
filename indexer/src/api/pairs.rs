use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use super::{
    build_asset_map, consolidated_stats, internal_err, lcd_gateway_err, limit_book_lcd, AppState,
};
use crate::db::queries::assets::AssetRow;
use crate::db::queries::{
    candles, limit_order_fills, limit_order_lifecycle, liquidity, pairs as db_pairs, swap_events,
};

pub use limit_book_lcd::LimitBookOrderItem;

pub const VALID_INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];

/// Max levels returned by `limit-book-shallow` (each level is one LCD `limit_order` query).
const LIMIT_BOOK_SHALLOW_DEPTH_MAX: i64 = 20;

fn limit_book_lcd_err(e: limit_book_lcd::LimitBookLcdError) -> (StatusCode, String) {
    match e {
        limit_book_lcd::LimitBookLcdError::Lcd(le) => lcd_gateway_err(le),
        limit_book_lcd::LimitBookLcdError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
    }
}

fn parse_book_side(raw: &str) -> Result<&'static str, (StatusCode, String)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bid" => Ok("bid"),
        "ask" => Ok("ask"),
        _ => Err((
            StatusCode::BAD_REQUEST,
            "Invalid side: use bid or ask".to_string(),
        )),
    }
}

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
    /// F6 code-id freeze: live CW20 `code_id` ≠ pin or not factory-whitelisted (GitLab #585).
    /// Pair remains listed (quotes/charts ok); `route/solve` excludes it as an executable hop.
    #[serde(default)]
    pub code_id_frozen: bool,
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
pub const PAIR_LIST_LIMIT_MAX: i64 = 100;
pub const PAIR_LIST_OFFSET_MAX: i64 = 10_000;
const PAIR_LIST_Q_MAX_LEN: usize = 128;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListPairsQuery {
    /// Page size (default 50, max 100)
    pub limit: Option<i64>,
    /// Offset for pagination (max [`PAIR_LIST_OFFSET_MAX`])
    pub offset: Option<i64>,
    /// Search pair address, token symbols, contract addresses, or denoms (substring, case-insensitive)
    pub q: Option<String>,
    /// Filter to pairs that include this token (exact CW20 contract or native denom)
    pub asset: Option<String>,
    /// Sort: `id`, `fee`, `created`, `symbol`, `volume_24h`, `relevance` (default `relevance` when `q` is set, else `id`)
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
        Some("relevance") => Ok(db_pairs::PairListSort::Relevance),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid sort '{}'. Use id, fee, created, symbol, volume_24h, or relevance",
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
        None => Ok(matches!(
            sort,
            db_pairs::PairListSort::Volume24h | db_pairs::PairListSort::Relevance
        )),
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
    if offset > PAIR_LIST_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                PAIR_LIST_OFFSET_MAX
            ),
        ));
    }

    let q_trimmed = q.q.as_ref().map(|s| {
        let t = s.trim();
        t.chars().take(PAIR_LIST_Q_MAX_LEN).collect::<String>()
    });
    let q_ref = q_trimmed.as_deref().filter(|s| !s.is_empty());

    let asset_trimmed = q.asset.as_ref().map(|s| s.trim().to_string());
    let asset_ref = asset_trimmed.as_deref().filter(|s| !s.is_empty());

    let sort = match q.sort.as_deref().map(str::trim).filter(|x| !x.is_empty()) {
        Some(raw) => parse_pair_list_sort(Some(raw))?,
        None if q_ref.is_some() => db_pairs::PairListSort::Relevance,
        None => db_pairs::PairListSort::Id,
    };
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
            code_id_frozen: crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(
                &p.contract_address,
            ),
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
        pair_address: pair.contract_address.clone(),
        asset_0: AssetBrief::from(a0),
        asset_1: AssetBrief::from(a1),
        lp_token: pair.lp_token,
        fee_bps: pair.fee_bps,
        is_active: true,
        code_id_frozen: crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(
            &pair.contract_address,
        ),
        volume_quote_24h: None,
    }))
}

/// When `from` / `to` are omitted, candles are filtered to this many days before `to`.
/// 90 days avoids empty charts for LocalTerra / QA data whose block timestamps sit outside a short window.
const DEFAULT_CANDLE_LOOKBACK_DAYS: i64 = 90;

#[derive(Deserialize, IntoParams)]
pub struct CandleQuery {
    /// Candle interval: 1m, 5m, 15m, 1h, 4h, 1d, 1w
    pub interval: Option<String>,
    /// Start time (RFC 3339). Omitted: `to` minus [`DEFAULT_CANDLE_LOOKBACK_DAYS`] (see `get_pair_candles`).
    pub from: Option<String>,
    /// End time (RFC 3339). Omitted: current UTC.
    pub to: Option<String>,
    /// Max results (capped at 1000)
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CandleResponse {
    pub open_time: String,
    /// Factory USD of 1 human `asset_0` (`price_usd` only — never human quote-per-base).
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    /// Human quote-per-base OHLC for per-bar `invertUsd` (GitLab #543). Absent on pre-upgrade rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_human: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_human: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_human: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_human: Option<String>,
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
        .unwrap_or_else(|| now - chrono::Duration::days(DEFAULT_CANDLE_LOOKBACK_DAYS));
    let to =
        q.to.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or(now);
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);

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
            open_human: c.open_human.as_ref().map(|v| v.to_string()),
            high_human: c.high_human.as_ref().map(|v| v.to_string()),
            low_human: c.low_human.as_ref().map(|v| v.to_string()),
            close_human: c.close_human.as_ref().map(|v| v.to_string()),
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

/// Plain decimal string (no `1e+19`). Tape/CSV amounts are parsed with JS `BigInt` (#557).
pub(crate) fn bd_plain_string(v: &bigdecimal::BigDecimal) -> String {
    v.normalized().to_plain_string()
}

pub(crate) fn opt_bd_string(v: &Option<bigdecimal::BigDecimal>) -> Option<String> {
    v.as_ref().map(bd_plain_string)
}

/// Max CosmWasm/SDK decimal places we will publish on tape JSON (GitLab #557).
const TAPE_DECIMALS_MAX: i16 = 38;

/// Decimals from indexed `assets.decimals` only — never from wasm events.
/// Out of range (`< 0` or `> 38`) is omitted so the UI can show `—` (T557-8 / A1 spoof).
pub fn api_asset_decimals(decimals: i16) -> Option<i16> {
    if (0..=TAPE_DECIMALS_MAX).contains(&decimals) {
        Some(decimals)
    } else {
        None
    }
}

pub fn asset_map_decimals(asset_map: &HashMap<i32, AssetRow>, asset_id: i32) -> Option<i16> {
    asset_map
        .get(&asset_id)
        .and_then(|a| api_asset_decimals(a.decimals))
}

fn pair_token_decimals(
    pair: &db_pairs::PairRow,
    asset_map: &HashMap<i32, AssetRow>,
) -> (Option<i16>, Option<i16>) {
    (
        asset_map_decimals(asset_map, pair.asset_0_id),
        asset_map_decimals(asset_map, pair.asset_1_id),
    )
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
    /// Offer-asset decimals from indexed `assets` (GitLab #557). Omitted when missing or out of range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_decimals: Option<i16>,
    /// Ask-asset decimals from indexed `assets` (GitLab #557). Omitted when missing or out of range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_decimals: Option<i16>,
    pub price: String,
    /// USD of 1 human unit of pair base (`asset_0`). GitLab #522.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<String>,
    /// Pattern C / hybrid: pool leg output (when present on-chain).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_book_offer_consumed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_fee_bps: Option<i16>,
    /// Swap commission attributed on-chain when indexed (CSV / reconciliation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spread_amount: Option<String>,
    /// Ask-side pool leg net (CG/CMC `pool_leg_volume` alias when hybrid attrs indexed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_leg_volume: Option<String>,
    /// Ask-side book leg net (CG/CMC `book_leg_volume` alias).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_leg_volume: Option<String>,
}

/// Map an indexed swap row to [`TradeResponse`] (pair + asset symbols).
pub fn trade_response_from_swap_row(
    pair_address: &str,
    t: &swap_events::SwapEventRow,
    asset_map: &HashMap<i32, AssetRow>,
) -> TradeResponse {
    let offer_sym = asset_map
        .get(&t.offer_asset_id)
        .map(|a| a.symbol.clone())
        .unwrap_or_default();
    let ask_sym = asset_map
        .get(&t.ask_asset_id)
        .map(|a| a.symbol.clone())
        .unwrap_or_default();
    let (pool_leg_volume, book_leg_volume) = consolidated_stats::hybrid_leg_volumes(t);
    TradeResponse {
        id: t.id,
        pair_address: pair_address.to_string(),
        block_height: t.block_height,
        block_timestamp: t.block_timestamp.to_rfc3339(),
        tx_hash: t.tx_hash.clone(),
        sender: t.sender.clone(),
        offer_asset: offer_sym,
        ask_asset: ask_sym,
        offer_amount: bd_plain_string(&t.offer_amount),
        return_amount: bd_plain_string(&t.return_amount),
        offer_decimals: asset_map_decimals(asset_map, t.offer_asset_id),
        ask_decimals: asset_map_decimals(asset_map, t.ask_asset_id),
        price: bd_plain_string(&t.price),
        price_usd: opt_bd_string(&t.price_usd),
        pool_return_amount: opt_bd_string(&t.pool_return_amount),
        book_return_amount: opt_bd_string(&t.book_return_amount),
        limit_book_offer_consumed: opt_bd_string(&t.limit_book_offer_consumed),
        effective_fee_bps: t.effective_fee_bps,
        commission_amount: opt_bd_string(&t.commission_amount),
        spread_amount: opt_bd_string(&t.spread_amount),
        pool_leg_volume,
        book_leg_volume,
    }
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
    /// Pair `asset_0` decimals from indexed `assets` (GitLab #557).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token0_decimals: Option<i16>,
    /// Pair `asset_1` decimals from indexed `assets` (GitLab #557).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token1_decimals: Option<i16>,
    pub commission_amount: String,
}

/// Map an indexed limit fill to [`LimitFillResponse`] (pair address + pair-leg decimals).
pub fn limit_fill_response_from_row(
    pair_address: &str,
    r: &limit_order_fills::LimitOrderFillRow,
    token0_decimals: Option<i16>,
    token1_decimals: Option<i16>,
) -> LimitFillResponse {
    LimitFillResponse {
        id: r.id,
        pair_address: pair_address.to_string(),
        swap_event_id: r.swap_event_id,
        block_height: r.block_height,
        block_timestamp: r.block_timestamp.to_rfc3339(),
        tx_hash: r.tx_hash.clone(),
        order_id: r.order_id,
        side: r.side.clone(),
        maker: r.maker.clone(),
        price: bd_plain_string(&r.price),
        token0_amount: bd_plain_string(&r.token0_amount),
        token1_amount: bd_plain_string(&r.token1_amount),
        token0_decimals,
        token1_decimals,
        commission_amount: bd_plain_string(&r.commission_amount),
    }
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

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let trades = swap_events::get_trades_for_pair(&state.pool, pair.id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let result: Vec<TradeResponse> = trades
        .iter()
        .map(|t| trade_response_from_swap_row(&addr, t, &asset_map))
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

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
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

pub(crate) fn parse_placement_lifecycle_filter(
    raw: Option<&str>,
) -> Result<limit_order_lifecycle::PlacementLifecycleFilter, (StatusCode, String)> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(limit_order_lifecycle::PlacementLifecycleFilter::DefaultOpen),
        Some("active") => Ok(limit_order_lifecycle::PlacementLifecycleFilter::ActiveOnly),
        Some("parked_expired") => {
            Ok(limit_order_lifecycle::PlacementLifecycleFilter::ParkedExpiredOnly)
        }
        Some("refunded") => Ok(limit_order_lifecycle::PlacementLifecycleFilter::RefundedOnly),
        Some("all") => Ok(limit_order_lifecycle::PlacementLifecycleFilter::All),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid status '{other}'. Use active, parked_expired, refunded, or all"),
        )),
    }
}

#[derive(Serialize, ToSchema)]
pub struct LimitPlacementResponse {
    pub id: i64,
    pub pair_address: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    pub order_id: i64,
    /// `active` — on the book or not yet expired; `parked_expired` — expired during a match walk, claimable; `refunded` — maker claimed escrow.
    pub lifecycle_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// Escrow remaining when parked (from `limit_order_expired_parked`); preserved after refund for UX.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_escrow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parked_block_height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parked_block_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parked_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refunded_block_height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refunded_block_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refunded_tx_hash: Option<String>,
}

#[derive(Deserialize, IntoParams, utoipa::ToSchema)]
pub struct LimitPlacementsQuery {
    /// Max results (capped at 200)
    pub limit: Option<i64>,
    /// Cursor: return rows with id < before
    pub before: Option<i64>,
    /// Filter by lifecycle: omit for **`active` + `parked_expired`** (excludes terminal refunds); `active`, `parked_expired`, `refunded`, or `all`.
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-placements",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitPlacementsQuery,
    ),
    responses(
        (status = 200, description = "Indexed limit placements with lifecycle status (GitLab #142); excludes cancelled `(pair, order_id)` rows when matched in limit_order_cancellations", body = Vec<LimitPlacementResponse>),
        (status = 400, description = "Invalid query parameters"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_placements(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitPlacementsQuery>,
) -> Result<Json<Vec<LimitPlacementResponse>>, (StatusCode, String)> {
    let pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let lifecycle = parse_placement_lifecycle_filter(q.status.as_deref())?;
    let rows = limit_order_lifecycle::list_placements_for_pair(
        &state.pool,
        pair.id,
        limit,
        q.before,
        lifecycle,
    )
    .await
    .map_err(internal_err)?;

    let result: Vec<LimitPlacementResponse> = rows
        .iter()
        .map(|r| limit_placement_response(&addr, r))
        .collect();

    Ok(Json(result))
}

pub(crate) fn limit_placement_response(
    pair_address: &str,
    r: &limit_order_lifecycle::PlacementRow,
) -> LimitPlacementResponse {
    LimitPlacementResponse {
        id: r.id,
        pair_address: pair_address.to_string(),
        block_height: r.block_height,
        block_timestamp: r.block_timestamp.to_rfc3339(),
        tx_hash: r.tx_hash.clone(),
        order_id: r.order_id,
        lifecycle_status: r.lifecycle_status.clone(),
        owner: r.owner.clone(),
        side: r.side.clone(),
        price: r.price.as_ref().map(|p| p.normalized().to_string()),
        expires_at: r.expires_at,
        remaining_escrow: r
            .remaining_escrow
            .as_ref()
            .map(|x| x.normalized().to_string()),
        parked_block_height: r.parked_block_height,
        parked_block_timestamp: r.parked_block_timestamp.map(|t| t.to_rfc3339()),
        parked_tx_hash: r.parked_tx_hash.clone(),
        refunded_block_height: r.refunded_block_height,
        refunded_block_timestamp: r.refunded_block_timestamp.map(|t| t.to_rfc3339()),
        refunded_tx_hash: r.refunded_tx_hash.clone(),
    }
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

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
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

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = limit_order_fills::list_fills_for_pair(&state.pool, pair.id, limit, q.before)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;
    let (token0_decimals, token1_decimals) = pair_token_decimals(&pair, &asset_map);

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| limit_fill_response_from_row(&addr, r, token0_decimals, token1_decimals))
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

    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = limit_order_fills::list_fills_for_order(&state.pool, pair.id, order_id, limit)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;
    let (token0_decimals, token1_decimals) = pair_token_decimals(&pair, &asset_map);

    let result: Vec<LimitFillResponse> = rows
        .iter()
        .map(|r| limit_fill_response_from_row(&addr, r, token0_decimals, token1_decimals))
        .collect();

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct OrderBookHeadQuery {
    /// Book side: `bid` or `ask`
    pub side: String,
}

#[derive(Serialize, ToSchema)]
pub struct OrderBookHeadResponse {
    /// Best order id on this side, or `null` if the book is empty.
    pub head_order_id: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/order-book-head",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        OrderBookHeadQuery,
    ),
    responses(
        (status = 200, description = "On-chain book head via LCD proxy", body = OrderBookHeadResponse),
        (status = 400, description = "Invalid side"),
        (status = 404, description = "Pair not found"),
        (status = 502, description = "Upstream LCD failure (sanitized body)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_order_book_head(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<OrderBookHeadQuery>,
) -> Result<Json<OrderBookHeadResponse>, (StatusCode, String)> {
    let _pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let side = parse_book_side(&q.side)?;
    let head: Option<u64> = state
        .lcd
        .query_contract(&addr, &json!({ "order_book_head": { "side": side } }))
        .await
        .map_err(lcd_gateway_err)?;

    Ok(Json(OrderBookHeadResponse {
        head_order_id: head,
    }))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct LimitBookShallowQuery {
    /// Book side: `bid` or `ask`
    pub side: String,
    /// Max orders to walk from the head (default 10, max 20)
    pub depth: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct LimitBookShallowResponse {
    pub side: String,
    pub orders: Vec<LimitBookOrderItem>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-book-shallow",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitBookShallowQuery,
    ),
    responses(
        (status = 200, description = "Shallow on-chain book via LCD proxy", body = LimitBookShallowResponse),
        (status = 400, description = "Invalid side or depth"),
        (status = 404, description = "Pair not found"),
        (status = 502, description = "Upstream LCD failure (sanitized body)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_book_shallow(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitBookShallowQuery>,
) -> Result<Json<LimitBookShallowResponse>, (StatusCode, String)> {
    let _pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let side_label = parse_book_side(&q.side)?;
    let depth = q.depth.unwrap_or(10).clamp(1, LIMIT_BOOK_SHALLOW_DEPTH_MAX);

    let (orders, _, _) =
        limit_book_lcd::fetch_limit_book_page(&state.lcd, &addr, side_label, depth, None)
            .await
            .map_err(limit_book_lcd_err)?;

    Ok(Json(LimitBookShallowResponse {
        side: side_label.to_string(),
        orders,
    }))
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct LimitBookPagedQuery {
    /// Book side: `bid` or `ask`
    pub side: String,
    /// Max orders in this page (default 50, max 100)
    pub limit: Option<i64>,
    /// Keyset cursor: `order_id` from `next_after_order_id` of the previous page
    pub after_order_id: Option<u64>,
    /// Inclusive band high bound (bids: highest price in window; asks: lowest price in window)
    pub price_from: Option<String>,
    /// Inclusive band low bound (bids: lowest price; asks: highest price in window)
    pub price_to: Option<String>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct LimitBookInsertHintsQuery {
    /// Book side: `bid` or `ask`
    pub side: String,
    /// Comma-separated positive decimal prices (max 100)
    pub prices: String,
}

#[derive(Serialize, ToSchema)]
pub struct LimitBookInsertHintsResponse {
    pub side: String,
    pub hints: Vec<limit_book_lcd::LimitBookInsertHintItem>,
    pub budget_exhausted: bool,
}

#[derive(Serialize, ToSchema)]
pub struct LimitBookPagedResponse {
    pub side: String,
    pub orders: Vec<LimitBookOrderItem>,
    pub has_more: bool,
    pub next_after_order_id: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-book",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitBookPagedQuery,
    ),
    responses(
        (status = 200, description = "Paginated on-chain book via LCD proxy", body = LimitBookPagedResponse),
        (status = 400, description = "Invalid side, limit, or cursor"),
        (status = 404, description = "Pair not found"),
        (status = 502, description = "Upstream LCD failure (sanitized body)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_book(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitBookPagedQuery>,
) -> Result<Json<LimitBookPagedResponse>, (StatusCode, String)> {
    let _pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let side_label = parse_book_side(&q.side)?;
    let page_limit = q
        .limit
        .unwrap_or(limit_book_lcd::LIMIT_BOOK_PAGE_DEFAULT)
        .clamp(1, limit_book_lcd::LIMIT_BOOK_PAGE_MAX);

    let (orders, has_more, next_after_order_id) = match (&q.price_from, &q.price_to) {
        (Some(from), Some(to)) => {
            limit_book_lcd::fetch_limit_book_price_window(
                &state.lcd,
                &addr,
                side_label,
                from,
                to,
                page_limit,
                q.after_order_id,
            )
            .await
        }
        (None, None) => {
            limit_book_lcd::fetch_limit_book_page(
                &state.lcd,
                &addr,
                side_label,
                page_limit,
                q.after_order_id,
            )
            .await
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "price_from and price_to must both be set for a price window".to_string(),
            ));
        }
    }
    .map_err(limit_book_lcd_err)?;

    Ok(Json(LimitBookPagedResponse {
        side: side_label.to_string(),
        orders,
        has_more,
        next_after_order_id,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/pairs/{addr}/limit-book/insert-hints",
    params(
        ("addr" = String, Path, description = "Pair contract address"),
        LimitBookInsertHintsQuery,
    ),
    responses(
        (status = 200, description = "Batch insert-hint resolution via LCD walk", body = LimitBookInsertHintsResponse),
        (status = 400, description = "Invalid side, prices, or oversized list"),
        (status = 404, description = "Pair not found"),
        (status = 502, description = "Upstream LCD failure (sanitized body)"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Pairs"
)]
pub async fn get_pair_limit_book_insert_hints(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<LimitBookInsertHintsQuery>,
) -> Result<Json<LimitBookInsertHintsResponse>, (StatusCode, String)> {
    let _pair = db_pairs::get_pair_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let side_label = parse_book_side(&q.side)?;
    let prices: Vec<String> = q
        .prices
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if prices.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "prices must contain at least one entry".to_string(),
        ));
    }

    let result = limit_book_lcd::resolve_limit_insert_hints(&state.lcd, &addr, side_label, &prices)
        .await
        .map_err(limit_book_lcd_err)?;

    Ok(Json(LimitBookInsertHintsResponse {
        side: side_label.to_string(),
        hints: result.hints,
        budget_exhausted: result.budget_exhausted,
    }))
}

#[derive(Serialize, ToSchema)]
pub struct PairStatsResponse {
    /// Raw oriented 24h base volume. Integrator / CG JSON — do not humanize here (GitLab #565).
    pub volume_base: String,
    /// Raw oriented 24h quote volume. Integrator / CG JSON — do not humanize here (GitLab #565).
    pub volume_quote: String,
    /// Human USD 24h notional (`SUM(swap_events.volume_usd)`, P522-Q). Retail Charts pair strip (#565).
    pub volume_usd: Option<String>,
    pub trade_count: i64,
    pub high: Option<String>,
    pub low: Option<String>,
    pub open_price: Option<String>,
    pub close_price: Option<String>,
    pub price_change_pct: Option<f64>,
    /// USD OHLC of 1 human base (GitLab #522). Absent when quote has no catalog USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_price_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_price_usd: Option<String>,
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
        high_usd: stats.high_usd.map(|v| v.to_string()),
        low_usd: stats.low_usd.map(|v| v.to_string()),
        open_price_usd: stats.open_price_usd.map(|v| v.to_string()),
        close_price_usd: stats.close_price_usd.map(|v| v.to_string()),
    }))
}

#[cfg(test)]
mod tape_decimals_tests {
    use super::{api_asset_decimals, asset_map_decimals, bd_plain_string};
    use crate::db::queries::assets::AssetRow;
    use bigdecimal::BigDecimal;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn sample_asset(id: i32, decimals: i16) -> AssetRow {
        AssetRow {
            id,
            contract_address: Some(format!("terra1asset{id}")),
            denom: None,
            is_cw20: true,
            name: "Tok".into(),
            symbol: "TOK".into(),
            decimals,
            logo_url: None,
            coingecko_id: None,
            cmc_id: None,
            first_seen_block: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn api_asset_decimals_accepts_0_through_38() {
        assert_eq!(api_asset_decimals(0), Some(0));
        assert_eq!(api_asset_decimals(6), Some(6));
        assert_eq!(api_asset_decimals(18), Some(18));
        assert_eq!(api_asset_decimals(38), Some(38));
    }

    #[test]
    fn api_asset_decimals_omits_out_of_range() {
        assert_eq!(api_asset_decimals(-1), None);
        assert_eq!(api_asset_decimals(39), None);
        assert_eq!(api_asset_decimals(99), None);
    }

    #[test]
    fn bd_plain_string_emits_18_dec_raw_without_scientific_notation() {
        let v = BigDecimal::from_str("10000000000000000000").expect("parse");
        let s = bd_plain_string(&v);
        assert_eq!(s, "10000000000000000000");
        assert!(!s.contains('e') && !s.contains('E'));
    }

    #[test]
    fn asset_map_decimals_reads_indexed_assets_row() {
        let mut map = HashMap::new();
        map.insert(1, sample_asset(1, 18));
        map.insert(2, sample_asset(2, 99));
        assert_eq!(asset_map_decimals(&map, 1), Some(18));
        assert_eq!(asset_map_decimals(&map, 2), None);
        assert_eq!(asset_map_decimals(&map, 3), None);
    }
}
