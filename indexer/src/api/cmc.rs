use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::ToPrimitive;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{
    build_asset_map, consolidated_stats, find_pair_by_ticker, internal_err, listing_timestamps,
    orderbook_sim, AppState,
};
use crate::db::queries::{assets, pairs as db_pairs, swap_events};

// ---------- /cmc/summary ----------

#[derive(Serialize, ToSchema)]
pub struct CmcSummaryEntry {
    pub trading_pairs: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub last_price: String,
    pub lowest_ask: String,
    pub highest_bid: String,
    pub base_volume: String,
    pub quote_volume: String,
    pub price_change_percent_24h: String,
    pub highest_price_24h: String,
    pub lowest_price_24h: String,
    pub cl8y_extensions: consolidated_stats::Cl8yConsolidatedExtensions,
}

#[utoipa::path(
    get,
    path = "/cmc/summary",
    responses(
        (status = 200, description = "CoinMarketCap summary for all pairs", body = Vec<CmcSummaryEntry>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinMarketCap"
)]
pub async fn cmc_summary(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(cached) = super::aggregator_cache_get("cmc_summary") {
        return Ok(Json(cached));
    }
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
        let extensions = consolidated_stats::fetch_consolidated_extensions(&state.pool, p.id)
            .await
            .map_err(internal_err)?;

        let last_price_f = stats
            .close_price
            .as_ref()
            .and_then(|p| p.to_f64())
            .unwrap_or(0.0);

        result.push(CmcSummaryEntry {
            trading_pairs: format!("{}_{}", a0.symbol, a1.symbol),
            base_currency: a0.symbol.clone(),
            quote_currency: a1.symbol.clone(),
            last_price: stats
                .close_price
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_else(|| "0".to_string()),
            lowest_ask: format!("{:.18}", last_price_f * 1.001),
            highest_bid: format!("{:.18}", last_price_f * 0.999),
            base_volume: stats.volume_base.to_string(),
            quote_volume: stats.volume_quote.to_string(),
            price_change_percent_24h: stats
                .price_change_pct
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "0.00".to_string()),
            highest_price_24h: stats
                .high
                .map(|h| h.to_string())
                .unwrap_or_else(|| "0".to_string()),
            lowest_price_24h: stats
                .low
                .map(|l| l.to_string())
                .unwrap_or_else(|| "0".to_string()),
            cl8y_extensions: extensions,
        });
    }

    let value = serde_json::to_value(&result).map_err(internal_err)?;
    super::aggregator_cache_put("cmc_summary", value.clone());
    Ok(Json(value))
}

// ---------- /cmc/assets ----------

#[derive(Serialize, ToSchema)]
pub struct CmcAssetEntry {
    pub name: String,
    pub unified_cryptoasset_id: Option<i32>,
    pub can_withdraw: bool,
    pub can_deposit: bool,
    pub min_withdraw: String,
}

#[utoipa::path(
    get,
    path = "/cmc/assets",
    responses(
        (status = 200, description = "Asset list in CoinMarketCap format", body = HashMap<String, CmcAssetEntry>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinMarketCap"
)]
pub async fn cmc_assets(
    State(state): State<AppState>,
) -> Result<Json<HashMap<String, CmcAssetEntry>>, (StatusCode, String)> {
    let all = assets::get_all_assets(&state.pool)
        .await
        .map_err(internal_err)?;

    let mut map = HashMap::new();
    for a in &all {
        map.insert(
            a.symbol.clone(),
            CmcAssetEntry {
                name: a.name.clone(),
                unified_cryptoasset_id: a.cmc_id,
                can_withdraw: true,
                can_deposit: true,
                min_withdraw: "0".to_string(),
            },
        );
    }

    Ok(Json(map))
}

// ---------- /cmc/ticker ----------

#[derive(Serialize, ToSchema)]
pub struct CmcTickerEntry {
    pub base_id: String,
    pub quote_id: String,
    pub last_price: String,
    pub base_volume: String,
    pub quote_volume: String,
    #[serde(rename = "isFrozen")]
    pub is_frozen: String,
}

#[utoipa::path(
    get,
    path = "/cmc/ticker",
    responses(
        (status = 200, description = "Ticker data in CoinMarketCap format", body = HashMap<String, CmcTickerEntry>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinMarketCap"
)]
pub async fn cmc_ticker(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(cached) = super::aggregator_cache_get("cmc_ticker") {
        return Ok(Json(cached));
    }
    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut map = HashMap::new();
    for p in &all_pairs {
        let (a0, a1) = match (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            (Some(a0), Some(a1)) => (a0, a1),
            _ => continue,
        };

        let stats = swap_events::get_24h_stats_for_pair(&state.pool, p.id)
            .await
            .map_err(internal_err)?;

        let base_id = a0
            .contract_address
            .as_deref()
            .or(a0.denom.as_deref())
            .unwrap_or("")
            .to_string();
        let quote_id = a1
            .contract_address
            .as_deref()
            .or(a1.denom.as_deref())
            .unwrap_or("")
            .to_string();

        let key = format!("{}_{}", a0.symbol, a1.symbol);
        map.insert(
            key,
            CmcTickerEntry {
                base_id,
                quote_id,
                last_price: stats
                    .close_price
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                base_volume: stats.volume_base.to_string(),
                quote_volume: stats.volume_quote.to_string(),
                is_frozen: "0".to_string(),
            },
        );
    }

    let value = serde_json::to_value(&map).map_err(internal_err)?;
    super::aggregator_cache_put("cmc_ticker", value.clone());
    Ok(Json(value))
}

// ---------- /cmc/orderbook/:market_pair ----------

#[derive(Deserialize, IntoParams)]
pub struct CmcOrderbookQuery {
    /// Total levels across the book, split evenly per side (max 100, default 20; Openware/CMC)
    pub depth: Option<usize>,
}

#[derive(Serialize, ToSchema)]
pub struct CmcOrderbookResponse {
    /// Unix timestamp in **seconds** (GitLab #222; same unit as `/cmc/trades`).
    pub timestamp: i64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[utoipa::path(
    get,
    path = "/cmc/orderbook/{market_pair}",
    params(
        ("market_pair" = String, Path, description = "Market pair in BASE_TARGET format"),
        CmcOrderbookQuery,
    ),
    responses(
        (status = 200, description = "Hybrid-simulated orderbook (Openware array wrapper; #223)", body = Vec<CmcOrderbookResponse>),
        (status = 400, description = "Invalid market pair format"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinMarketCap"
)]
pub async fn cmc_orderbook(
    State(state): State<AppState>,
    Path(market_pair): Path<String>,
    Query(q): Query<CmcOrderbookQuery>,
) -> Result<Json<Vec<CmcOrderbookResponse>>, (StatusCode, String)> {
    let depth = orderbook_sim::cap_orderbook_depth(q.depth);
    let pair_addr = find_pair_by_ticker(&state, &market_pair).await?;

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
    Ok(Json(vec![CmcOrderbookResponse {
        timestamp: ts.cmc_s,
        bids: ob.bids,
        asks: ob.asks,
    }]))
}

// ---------- /cmc/trades/:market_pair ----------

#[derive(Serialize, ToSchema)]
pub struct CmcTradeEntry {
    pub trade_id: i64,
    pub price: String,
    pub base_volume: String,
    pub quote_volume: String,
    pub timestamp: i64,
    #[serde(rename = "type")]
    pub trade_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_leg_volume: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_leg_volume: Option<String>,
}

#[utoipa::path(
    get,
    path = "/cmc/trades/{market_pair}",
    params(
        ("market_pair" = String, Path, description = "Market pair in BASE_TARGET format"),
    ),
    responses(
        (status = 200, description = "Recent trades for market pair", body = Vec<CmcTradeEntry>),
        (status = 400, description = "Invalid market pair format"),
        (status = 404, description = "Pair not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CoinMarketCap"
)]
pub async fn cmc_trades(
    State(state): State<AppState>,
    Path(market_pair): Path<String>,
) -> Result<Json<Vec<CmcTradeEntry>>, (StatusCode, String)> {
    let pair_addr = find_pair_by_ticker(&state, &market_pair).await?;

    let pair = db_pairs::get_pair_by_address(&state.pool, &pair_addr)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Pair not found".to_string()))?;

    let trades = swap_events::get_trades_for_pair(&state.pool, pair.id, 200, None)
        .await
        .map_err(internal_err)?;

    let result: Vec<CmcTradeEntry> = trades
        .iter()
        .map(|t| {
            let is_buy = t.offer_asset_id == pair.asset_1_id;
            let (pool_leg_volume, book_leg_volume) = consolidated_stats::hybrid_leg_volumes(t);
            CmcTradeEntry {
                trade_id: t.id,
                price: t.price.to_string(),
                base_volume: if is_buy {
                    t.return_amount.to_string()
                } else {
                    t.offer_amount.to_string()
                },
                quote_volume: if is_buy {
                    t.offer_amount.to_string()
                } else {
                    t.return_amount.to_string()
                },
                timestamp: t.block_timestamp.timestamp(),
                trade_type: if is_buy {
                    "buy".to_string()
                } else {
                    "sell".to_string()
                },
                pool_leg_volume,
                book_leg_volume,
            }
        })
        .collect();

    Ok(Json(result))
}
