use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{
    aggregator_snapshot::{self, AggregatorListQuery},
    build_asset_map, consolidated_stats, find_pair_by_ticker, internal_err, listing_spread,
    listing_timestamps,
    listing_virtual_pairs::{self, ProjectedSwap, SwapFilter, VirtualMarket},
    orderbook_sim, AppState,
};
use crate::db::queries::{pairs as db_pairs, swap_events};
use crate::indexer::listing_exclude::pair_is_listing_excluded;

#[derive(Serialize, Clone, ToSchema)]
pub struct CgPairResponse {
    pub ticker_id: String,
    pub base: String,
    pub target: String,
    pub pool_id: String,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct CgPairsQuery {
    /// Max pairs returned (default 500, max 1000)
    pub limit: Option<i64>,
    /// Offset for pagination (max 10_000)
    pub offset: Option<i64>,
}

const CG_PAIRS_LIMIT_DEFAULT: i64 = 500;
const CG_PAIRS_LIMIT_MAX: i64 = 1000;
const CG_PAIRS_OFFSET_MAX: i64 = 10_000;

#[utoipa::path(
    get,
    path = "/cg/pairs",
    params(CgPairsQuery),
    responses(
        (status = 200, description = "CoinGecko-format pair list", body = Vec<CgPairResponse>),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_pairs(
    State(state): State<AppState>,
    Query(q): Query<CgPairsQuery>,
) -> Result<Json<Vec<CgPairResponse>>, (StatusCode, String)> {
    let limit = q
        .limit
        .unwrap_or(CG_PAIRS_LIMIT_DEFAULT)
        .clamp(1, CG_PAIRS_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);
    if offset > CG_PAIRS_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                CG_PAIRS_OFFSET_MAX
            ),
        ));
    }

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut result = Vec::new();
    for p in &all_pairs {
        if let (Some(a0), Some(a1)) = (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            if pair_is_listing_excluded(
                a0.contract_address.as_deref(),
                a1.contract_address.as_deref(),
            ) {
                continue;
            }
            result.push(CgPairResponse {
                ticker_id: format!("{}_{}", a0.symbol, a1.symbol),
                base: a0.symbol.clone(),
                target: a1.symbol.clone(),
                pool_id: p.contract_address.clone(),
            });
        }
    }

    for market in listing_virtual_pairs::load_markets(&state)
        .await
        .map_err(internal_err)?
    {
        if market.cg_enabled {
            result.push(CgPairResponse {
                ticker_id: market.ticker_id(),
                base: market.base.symbol.clone(),
                target: market.target.symbol.clone(),
                pool_id: market.id.to_string(),
            });
        }
    }

    let start = offset.min(result.len() as i64) as usize;
    let end = (offset + limit).min(result.len() as i64) as usize;
    let page = if start >= result.len() {
        Vec::new()
    } else {
        result[start..end].to_vec()
    };

    Ok(Json(page))
}

#[derive(Serialize, ToSchema)]
pub struct CgTickerResponse {
    pub ticker_id: String,
    pub base_currency: String,
    pub target_currency: String,
    pub last_price: String,
    pub base_volume: String,
    pub target_volume: String,
    pub bid: String,
    pub ask: String,
    pub high: String,
    pub low: String,
    pub pool_id: String,
    /// AMM v2 pool TVL from `pair_liquidity_usd` / `protocol_pair_tvl` (GitLab #685).
    /// Unpriced → `"0"`. Not 24h volume. Llama must not ingest this field.
    pub liquidity_in_usd: String,
    /// CL8Y consolidated hybrid + pool-only attribution (GitLab #189); safe for aggregators to ignore.
    pub cl8y_extensions: consolidated_stats::Cl8yConsolidatedExtensions,
}

#[utoipa::path(
    get,
    path = "/cg/tickers",
    params(AggregatorListQuery),
    responses(
        (status = 200, description = "CoinGecko-format ticker data (top pairs by 24h volume; default limit 100)", body = Vec<CgTickerResponse>),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_tickers(
    State(state): State<AppState>,
    Query(q): Query<AggregatorListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let params = aggregator_snapshot::parse_aggregator_list_query(q)?;
    let cache_key = aggregator_snapshot::aggregator_cache_key("cg_tickers", params);
    if let Some(cached) = super::aggregator_cache_get(&cache_key) {
        return Ok(Json(cached));
    }

    let rows = aggregator_snapshot::load_aggregator_pairs(&state.pool, params, true)
        .await
        .map_err(internal_err)?;

    let mut result = Vec::with_capacity(rows.len());
    for row in &rows {
        let a0 = &row.asset_0;
        let a1 = &row.asset_1;
        let stats = &row.stats;
        let extensions = row.extensions.clone().expect("extensions loaded");

        let base_addr = a0
            .contract_address
            .as_deref()
            .or(a0.denom.as_deref())
            .unwrap_or("");
        let target_addr = a1
            .contract_address
            .as_deref()
            .or(a1.denom.as_deref())
            .unwrap_or("");

        let fee_bps = row.reserve_fee_bps.or(row.pair.fee_bps).unwrap_or(0);
        let (bid, ask) = listing_spread::listing_bid_ask(
            stats.close_price.as_ref(),
            row.reserve_0.as_ref(),
            row.reserve_1.as_ref(),
            a0.decimals,
            a1.decimals,
            fee_bps,
        );

        result.push(CgTickerResponse {
            ticker_id: format!("{}_{}", a0.symbol, a1.symbol),
            base_currency: base_addr.to_string(),
            target_currency: target_addr.to_string(),
            last_price: stats
                .close_price
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_else(|| "0".to_string()),
            base_volume: stats.volume_base.to_string(),
            target_volume: stats.volume_quote.to_string(),
            bid,
            ask,
            high: stats
                .high
                .as_ref()
                .map(|h| h.to_string())
                .unwrap_or_else(|| "0".to_string()),
            low: stats
                .low
                .as_ref()
                .map(|l| l.to_string())
                .unwrap_or_else(|| "0".to_string()),
            pool_id: row.pair.contract_address.clone(),
            liquidity_in_usd: listing_spread::listing_liquidity_in_usd(row.liquidity_usd.as_ref()),
            cl8y_extensions: extensions,
        });
    }

    for market in listing_virtual_pairs::load_markets(&state)
        .await
        .map_err(internal_err)?
    {
        if !market.cg_enabled {
            continue;
        }
        let stats = &market.stats;
        let (bid, ask) = listing_spread::listing_bid_ask(
            Some(&market.last_price),
            Some(&market.reserve_0),
            Some(&market.reserve_1),
            market.base.decimals,
            market.target.decimals,
            market.fee_bps.unwrap_or(0),
        );
        result.push(CgTickerResponse {
            ticker_id: market.ticker_id(),
            base_currency: market.base_id().to_string(),
            target_currency: market.target_id().to_string(),
            last_price: super::gt::format_dec(&market.last_price),
            base_volume: super::gt::format_dec(&stats.volume_base),
            target_volume: super::gt::format_dec(&stats.volume_quote),
            bid: plain_decimal(&bid),
            ask: plain_decimal(&ask),
            high: stats
                .high
                .as_ref()
                .map(super::gt::format_dec)
                .unwrap_or_else(|| "0".into()),
            low: stats
                .low
                .as_ref()
                .map(super::gt::format_dec)
                .unwrap_or_else(|| "0".into()),
            pool_id: market.id.to_string(),
            liquidity_in_usd: plain_decimal(&listing_spread::listing_liquidity_in_usd(
                market.liquidity_usd.as_ref(),
            )),
            cl8y_extensions: consolidated_stats::Cl8yConsolidatedExtensions {
                consolidated: true,
                hybrid_trade_count_24h: "0".into(),
                pool_only_trade_count_24h: stats.trade_count.to_string(),
                book_leg_volume_quote_24h: "0".into(),
                pool_leg_volume_quote_24h: super::gt::format_dec(&stats.volume_quote),
            },
        });
    }

    let value = serde_json::to_value(&result).map_err(internal_err)?;
    super::aggregator_cache_put(&cache_key, value.clone());
    Ok(Json(value))
}

#[derive(Deserialize, IntoParams)]
pub struct OrderbookQuery {
    /// Ticker ID in BASE_TARGET format
    pub ticker_id: String,
    /// Total levels across the book, split evenly per side (max 100, default 20; Openware/CMC)
    pub depth: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct CgOrderbookResponse {
    pub ticker_id: String,
    /// Unix timestamp in **milliseconds** (GitLab #222).
    pub timestamp: i64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[utoipa::path(
    get,
    path = "/cg/orderbook",
    params(OrderbookQuery),
    responses(
        (status = 200, description = "Hybrid-simulated orderbook (AMM curve + resting limits, #220)", body = CgOrderbookResponse),
        (status = 400, description = "Invalid ticker_id format"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_orderbook(
    State(state): State<AppState>,
    Query(q): Query<OrderbookQuery>,
) -> Result<Json<CgOrderbookResponse>, (StatusCode, String)> {
    let depth = orderbook_sim::cap_orderbook_depth(q.depth);
    let pair_addr = match resolve_cg_market(&state, &q.ticker_id).await? {
        CgMarket::Factory(address) => address,
        CgMarket::Virtual(market) => {
            let trades = listing_virtual_pairs::load_swaps(
                &state,
                std::slice::from_ref(&market),
                SwapFilter {
                    limit: Some(1),
                    ..Default::default()
                },
            )
            .await
            .map_err(internal_err)?;
            let (r0, r1) = trades
                .first()
                .map(|trade| {
                    (
                        trade.reserve_0.as_ref().unwrap_or(&market.reserve_0),
                        trade.reserve_1.as_ref().unwrap_or(&market.reserve_1),
                    )
                })
                .unwrap_or((&market.reserve_0, &market.reserve_1));
            let price = trades
                .first()
                .map(|trade| &trade.price)
                .unwrap_or(&market.last_price);
            let base_size = super::gt::decimalize(r0, market.base.decimals);
            let target_size = super::gt::decimalize(r1, market.target.decimals);
            let ts = listing_timestamps::ListingOrderbookTimestamps::now();
            let price_string = super::gt::format_dec(price);
            let bids = vec![[
                price_string.clone(),
                super::gt::format_dec(&(target_size / price)),
            ]];
            let asks = vec![[price_string, super::gt::format_dec(&base_size)]];
            // cap_orderbook_depth has already admitted at least one level per side.
            return Ok(Json(CgOrderbookResponse {
                ticker_id: q.ticker_id,
                timestamp: ts.cg_ms,
                bids,
                asks,
            }));
        }
    };

    // Factory books retain fresh chain attestation, including cache hits (#1392).
    if state.lcd.require_verified_chain().await.is_err() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "No endpoint attests the expected chain".into(),
        ));
    }
    let ob = orderbook_sim::simulate_orderbook_cached(
        &state.orderbook_cache,
        &state.pool,
        &state.lcd,
        &pair_addr,
        depth,
    )
    .await
    .map_err(internal_err)?;

    let ts = listing_timestamps::ListingOrderbookTimestamps::now();
    Ok(Json(CgOrderbookResponse {
        ticker_id: q.ticker_id,
        timestamp: ts.cg_ms,
        bids: ob.bids,
        asks: ob.asks,
    }))
}

#[derive(Deserialize, IntoParams)]
pub struct HistoricalTradesQuery {
    /// Ticker ID in BASE_TARGET format
    pub ticker_id: String,
    /// Filter by type: "buy" or "sell"
    #[serde(rename = "type")]
    #[param(rename = "type")]
    pub trade_type: Option<String>,
    /// Max results (capped at 500)
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct CgTradeEntry {
    pub trade_id: i64,
    pub price: String,
    pub base_volume: String,
    pub target_volume: String,
    pub trade_timestamp: i64,
    #[serde(rename = "type")]
    pub trade_type: String,
    /// Ask-side pool leg output when indexed on a hybrid swap (GitLab #189).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_leg_volume: Option<String>,
    /// Ask-side book leg net output when indexed on a hybrid swap (GitLab #189).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_leg_volume: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct CgHistoricalTradesResponse {
    pub buy: Vec<CgTradeEntry>,
    pub sell: Vec<CgTradeEntry>,
}

#[utoipa::path(
    get,
    path = "/cg/historical_trades",
    params(HistoricalTradesQuery),
    responses(
        (status = 200, description = "Historical trades grouped by buy/sell", body = CgHistoricalTradesResponse),
        (status = 400, description = "Invalid ticker_id format"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_historical_trades(
    State(state): State<AppState>,
    Query(q): Query<HistoricalTradesQuery>,
) -> Result<Json<CgHistoricalTradesResponse>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    validate_trade_type(q.trade_type.as_deref())?;
    let pair_addr = match resolve_cg_market(&state, &q.ticker_id).await? {
        CgMarket::Factory(address) => address,
        CgMarket::Virtual(market) => {
            let trades = listing_virtual_pairs::load_swaps(
                &state,
                std::slice::from_ref(&market),
                SwapFilter {
                    limit: Some(limit),
                    ..Default::default()
                },
            )
            .await
            .map_err(internal_err)?;
            let mut response = CgHistoricalTradesResponse {
                buy: Vec::new(),
                sell: Vec::new(),
            };
            for trade in &trades {
                let entry = projected_trade_entry(trade);
                let is_buy = !trade.offer_is_base;
                if accepts_trade_type(q.trade_type.as_deref(), is_buy) {
                    if is_buy {
                        response.buy.push(entry);
                    } else {
                        response.sell.push(entry);
                    }
                }
            }
            return Ok(Json(response));
        }
    };

    let pair = db_pairs::get_pair_by_address(&state.pool, &pair_addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let trades = swap_events::get_trades_for_pair(&state.pool, pair.id, limit, None)
        .await
        .map_err(internal_err)?;

    let mut buys = Vec::new();
    let mut sells = Vec::new();

    for t in &trades {
        let is_buy = t.offer_asset_id == pair.asset_1_id;
        let (pool_leg_volume, book_leg_volume) = consolidated_stats::hybrid_leg_volumes(t);
        let entry = trade_entry(
            t.id,
            &t.price,
            &t.offer_amount,
            &t.return_amount,
            t.block_timestamp.timestamp(),
            is_buy,
            (pool_leg_volume, book_leg_volume),
        );

        match q.trade_type.as_deref() {
            None | Some("") => {
                if is_buy {
                    buys.push(entry);
                } else {
                    sells.push(entry);
                }
            }
            Some("buy") => {
                if is_buy {
                    buys.push(entry);
                }
            }
            Some("sell") => {
                if !is_buy {
                    sells.push(entry);
                }
            }
            Some(other) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("Invalid type '{}'. Valid values: buy, sell", other),
                ));
            }
        }
    }

    Ok(Json(CgHistoricalTradesResponse {
        buy: buys,
        sell: sells,
    }))
}

// Factory lookup/cache remains real-only for CMC. CG resolves a fresh published
// projection separately so a removed peg or colliding factory ticker cannot be
// resurrected from the shared factory cache (#1437).
enum CgMarket {
    Factory(String),
    Virtual(VirtualMarket),
}

async fn resolve_cg_market(
    state: &AppState,
    ticker_id: &str,
) -> Result<CgMarket, (StatusCode, String)> {
    match find_pair_by_ticker(state, ticker_id).await {
        Ok(address) => Ok(CgMarket::Factory(address)),
        Err((StatusCode::NOT_FOUND, _)) => {
            let markets = listing_virtual_pairs::load_markets(state)
                .await
                .map_err(internal_err)?;
            if let Some(market) = markets
                .into_iter()
                .find(|market| market.ticker_id().eq_ignore_ascii_case(ticker_id))
            {
                if market.cg_enabled {
                    return Ok(CgMarket::Virtual(market));
                }
                // A newly indexed factory collision must win even when the
                // previous virtual lookup cached a miss in the factory map.
                *state.ticker_map_cache.inner.write().await = None;
                return find_pair_by_ticker(state, &market.ticker_id())
                    .await
                    .map(CgMarket::Factory);
            }
            Err((
                StatusCode::NOT_FOUND,
                format!("Pair not found for ticker: {ticker_id}"),
            ))
        }
        Err(error) => Err(error),
    }
}

fn plain_decimal(value: &str) -> String {
    // listing_spread returns exact decimal strings; use GT's fixed notation on
    // the listing projection to keep very small reserve prices readable.
    value
        .parse::<bigdecimal::BigDecimal>()
        .map(|v| super::gt::format_dec(&v))
        .unwrap_or_else(|_| value.to_string())
}

fn validate_trade_type(trade_type: Option<&str>) -> Result<(), (StatusCode, String)> {
    match trade_type {
        None | Some("" | "buy" | "sell") => Ok(()),
        Some(other) => Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid type '{other}'. Valid values: buy, sell"),
        )),
    }
}

fn accepts_trade_type(trade_type: Option<&str>, is_buy: bool) -> bool {
    match trade_type {
        Some("buy") => is_buy,
        Some("sell") => !is_buy,
        _ => true,
    }
}

fn trade_entry(
    id: i64,
    price: &bigdecimal::BigDecimal,
    offer: &bigdecimal::BigDecimal,
    returned: &bigdecimal::BigDecimal,
    timestamp: i64,
    is_buy: bool,
    legs: (Option<String>, Option<String>),
) -> CgTradeEntry {
    CgTradeEntry {
        trade_id: id,
        price: super::gt::format_dec(price),
        base_volume: super::gt::format_dec(if is_buy { returned } else { offer }),
        target_volume: super::gt::format_dec(if is_buy { offer } else { returned }),
        trade_timestamp: timestamp,
        trade_type: if is_buy { "buy".into() } else { "sell".into() },
        pool_leg_volume: legs.0,
        book_leg_volume: legs.1,
    }
}

fn projected_trade_entry(trade: &ProjectedSwap) -> CgTradeEntry {
    trade_entry(
        trade.id,
        &trade.price,
        &trade.offer_amount,
        &trade.return_amount,
        trade.block_timestamp.timestamp(),
        !trade.offer_is_base,
        (None, None),
    )
}
