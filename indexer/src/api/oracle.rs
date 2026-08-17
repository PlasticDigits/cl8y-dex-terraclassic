use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};
use crate::db::queries::oracle as db_oracle;
use crate::indexer::oracle::OracleTicker;

/// Catalog metadata for `GET /api/v1/oracle/price` and `/history` (GitLab #515).
pub const ORACLE_CATALOG_METADATA: &str = concat!(
    "Indexer external USD reference prices (not on-chain pair TWAP). ",
    "GET /api/v1/oracle/price/{ticker} for the latest average + per-source snapshot; ",
    "GET /api/v1/oracle/history/{ticker} for average history. ",
    "Tickers: ustc = TerraClassic USTC/USD; lunc = TerraClassic LUNC/USD; ",
    "vfdusd = wrapped FDUSD CEX/USD reference (polls FDUSD, not a $1 peg). ",
    "Sources are polled CEX/aggregator APIs (KuCoin, MEXC, CoinGecko). ",
    "KuCoin is skipped for vfdusd when unlisted. Advisory only — not used for on-chain settlement."
);

#[derive(Serialize, ToSchema)]
pub struct OracleTickerCatalogResponse {
    /// How to use the ticker-scoped price and history endpoints.
    pub metadata: String,
    /// Supported ticker path segments (lowercase).
    pub tickers: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct OracleSourcePrice {
    pub source: String,
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OraclePriceResponse {
    /// Path ticker (`ustc`, `lunc`, or `vfdusd`).
    pub ticker: String,
    pub price_usd: Option<String>,
    pub sources: Vec<OracleSourcePrice>,
}

fn catalog_response() -> OracleTickerCatalogResponse {
    OracleTickerCatalogResponse {
        metadata: ORACLE_CATALOG_METADATA.to_string(),
        tickers: OracleTicker::ALL
            .iter()
            .map(|t| t.as_str().to_string())
            .collect(),
    }
}

fn parse_ticker_path(raw: &str) -> Result<OracleTicker, (StatusCode, String)> {
    OracleTicker::parse(raw).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Unknown oracle ticker '{raw}'. Supported: {}",
                OracleTicker::ALL
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/price",
    responses(
        (status = 200, description = "Available oracle tickers and usage metadata", body = OracleTickerCatalogResponse),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_price_catalog() -> Json<OracleTickerCatalogResponse> {
    Json(catalog_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/price/{ticker}",
    params(
        ("ticker" = String, Path, description = "Oracle ticker: ustc, lunc, or vfdusd")
    ),
    responses(
        (status = 200, description = "Latest ticker/USD oracle price", body = OraclePriceResponse),
        (status = 400, description = "Unknown ticker"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_price(
    State(state): State<AppState>,
    Path(ticker_raw): Path<String>,
) -> Result<Json<OraclePriceResponse>, (StatusCode, String)> {
    let ticker = parse_ticker_path(&ticker_raw)?;
    let current = state.oracle_prices.for_ticker(ticker).read().await.clone();

    let source_rows = db_oracle::get_latest_prices_by_source(&state.pool, ticker)
        .await
        .map_err(internal_err)?;

    let sources: Vec<OracleSourcePrice> = source_rows
        .into_iter()
        .map(|r| OracleSourcePrice {
            source: r.source,
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OraclePriceResponse {
        ticker: ticker.as_str().to_string(),
        price_usd: current.map(|p| p.to_string()),
        sources,
    }))
}

#[derive(Deserialize, IntoParams)]
pub struct OracleHistoryQuery {
    /// Start time (RFC 3339)
    pub from: Option<String>,
    /// End time (RFC 3339)
    pub to: Option<String>,
    /// Max results (capped at 1000)
    pub limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct OracleHistoryEntry {
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OracleHistoryResponse {
    pub ticker: String,
    pub prices: Vec<OracleHistoryEntry>,
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/history",
    responses(
        (status = 200, description = "Available oracle tickers and usage metadata", body = OracleTickerCatalogResponse),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_history_catalog() -> Json<OracleTickerCatalogResponse> {
    Json(catalog_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/history/{ticker}",
    params(
        ("ticker" = String, Path, description = "Oracle ticker: ustc, lunc, or vfdusd"),
        OracleHistoryQuery
    ),
    responses(
        (status = 200, description = "Ticker/USD oracle price history", body = OracleHistoryResponse),
        (status = 400, description = "Unknown ticker"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_history(
    State(state): State<AppState>,
    Path(ticker_raw): Path<String>,
    Query(q): Query<OracleHistoryQuery>,
) -> Result<Json<OracleHistoryResponse>, (StatusCode, String)> {
    let ticker = parse_ticker_path(&ticker_raw)?;
    let now = Utc::now();
    let from = q
        .from
        .and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(|| now - chrono::Duration::hours(24));
    let to =
        q.to.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or(now);
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);

    let rows = db_oracle::get_price_history(&state.pool, ticker, from, to, limit)
        .await
        .map_err(internal_err)?;

    let prices: Vec<OracleHistoryEntry> = rows
        .into_iter()
        .map(|r| OracleHistoryEntry {
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OracleHistoryResponse {
        ticker: ticker.as_str().to_string(),
        prices,
    }))
}
