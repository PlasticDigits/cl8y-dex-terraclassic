use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::ToPrimitive;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{build_asset_map, find_pair_by_ticker, internal_err, orderbook_sim, AppState};
use crate::db::queries::{pairs as db_pairs, swap_events};

#[derive(Serialize, ToSchema)]
pub struct CgPairResponse {
    pub ticker_id: String,
    pub base: String,
    pub target: String,
    pub pool_id: String,
}

#[utoipa::path(
    get,
    path = "/cg/pairs",
    responses(
        (status = 200, description = "CoinGecko-format pair list", body = Vec<CgPairResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_pairs(
    State(state): State<AppState>,
) -> Result<Json<Vec<CgPairResponse>>, (StatusCode, String)> {
    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut result = Vec::new();
    for p in &all_pairs {
        if let (Some(a0), Some(a1)) = (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            result.push(CgPairResponse {
                ticker_id: format!("{}_{}", a0.symbol, a1.symbol),
                base: a0.symbol.clone(),
                target: a1.symbol.clone(),
                pool_id: p.contract_address.clone(),
            });
        }
    }

    Ok(Json(result))
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
    pub liquidity_in_usd: String,
}

#[utoipa::path(
    get,
    path = "/cg/tickers",
    responses(
        (status = 200, description = "CoinGecko-format ticker data", body = Vec<CgTickerResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinGecko"
)]
pub async fn cg_tickers(
    State(state): State<AppState>,
) -> Result<Json<Vec<CgTickerResponse>>, (StatusCode, String)> {
    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut result = Vec::new();
    for p in &all_pairs {
        let (a0, a1) = match (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            (Some(a0), Some(a1)) => (a0, a1),
            _ => continue,
        };

        let stats = swap_events::get_24h_stats_for_pair(&state.pool, p.id)
            .await
            .map_err(internal_err)?;

        let last_price_f = stats
            .close_price
            .as_ref()
            .and_then(|p| p.to_f64())
            .unwrap_or(0.0);

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

        let liquidity_usd = stats
            .volume_usd
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0".to_string());

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
            bid: format!("{:.18}", last_price_f * 0.999),
            ask: format!("{:.18}", last_price_f * 1.001),
            high: stats
                .high
                .map(|h| h.to_string())
                .unwrap_or_else(|| "0".to_string()),
            low: stats
                .low
                .map(|l| l.to_string())
                .unwrap_or_else(|| "0".to_string()),
            pool_id: p.contract_address.clone(),
            liquidity_in_usd: liquidity_usd,
        });
    }

    Ok(Json(result))
}

#[derive(Deserialize, IntoParams)]
pub struct OrderbookQuery {
    /// Ticker ID in BASE_TARGET format
    pub ticker_id: String,
    /// Number of levels (capped at 100, default 20)
    pub depth: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct CgOrderbookResponse {
    pub ticker_id: String,
    pub timestamp: String,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[utoipa::path(
    get,
    path = "/cg/orderbook",
    params(OrderbookQuery),
    responses(
        (status = 200, description = "Simulated AMM orderbook", body = CgOrderbookResponse),
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
    let depth = q.depth.unwrap_or(20).min(100);
    let pair_addr = find_pair_by_ticker(&state, &q.ticker_id).await?;

    let ob = orderbook_sim::simulate_orderbook_cached(
        &state.orderbook_cache,
        &state.pool,
        &state.lcd,
        &pair_addr,
        depth,
    )
    .await
    .map_err(internal_err)?;

    Ok(Json(CgOrderbookResponse {
        ticker_id: q.ticker_id,
        timestamp: Utc::now().to_rfc3339(),
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
    let limit = q.limit.unwrap_or(100).min(500);
    let pair_addr = find_pair_by_ticker(&state, &q.ticker_id).await?;

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
        let entry = CgTradeEntry {
            trade_id: t.id,
            price: t.price.to_string(),
            base_volume: if is_buy {
                t.return_amount.to_string()
            } else {
                t.offer_amount.to_string()
            },
            target_volume: if is_buy {
                t.offer_amount.to_string()
            } else {
                t.return_amount.to_string()
            },
            trade_timestamp: t.block_timestamp.timestamp(),
            trade_type: if is_buy {
                "buy".to_string()
            } else {
                "sell".to_string()
            },
        };

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
