use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};
use crate::db::queries::oracle as db_oracle;
use crate::indexer::oracle::{
    OracleTicker, ORACLE_MAX_AGE_SECS, ORACLE_QUORUM, ORACLE_SOURCE_POLICY,
};
use crate::indexer::venus_vfdusd::{VenusVfdusdSnapshot, VENUS_SOURCE, VENUS_VFDUSD_VTOKEN};

/// Catalog metadata for `GET /api/v1/oracle/price` and `/history` (GitLab #515 / #580).
pub const ORACLE_CATALOG_METADATA: &str = concat!(
    "Indexer external USD reference prices (not on-chain pair TWAP). ",
    "GET /api/v1/oracle/price/{ticker} for the latest average + per-source snapshot; ",
    "GET /api/v1/oracle/history/{ticker} for average history. ",
    "Tickers: ustc = TerraClassic USTC/USD; lunc = TerraClassic LUNC/USD; ",
    "vfdusd path stores CEX FDUSD/USD (MEXC FDUSDUSDT, CoinGecko first-digital-usd); ",
    "it is not USD of Terra CW20 vFDUSD (Venus bridged). ",
    "GET /api/v1/oracle/price/vfdusd also includes additive venus { fdusd_per_vfdusd } ",
    "(Venus Core Pool eth_call of exchangeRateCurrent; not USD, not the UST1 window). ",
    "Path fdusd is 400 (no alias). ",
    "Sources are polled CEX/aggregator APIs (KuCoin, MEXC, CoinGecko). ",
    "KuCoin is skipped for vfdusd when unlisted. Advisory only — not used for on-chain settlement. ",
    "Quality fresh requires two independent sources agreeing within 10%, valid prices in [1e-12,1e6], ",
    "and age at most 300 seconds with no future timestamp. No quorum clears the current quote. ",
    "as_of is local HTTP poll observation time, not authenticated exchange market time. ",
    "Freshness does not prove market recency or fair value; stale HTTP-200 provider replay remains a trust risk. ",
    "Legacy averages without certified quality are unavailable; history certified=true means quorum at observation time, never current freshness."
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
    /// Whether this source participated in the currently certified aggregate.
    pub accepted: bool,
    pub source: String,
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct VenusVfdusdResponse {
    pub quality: String,
    pub as_of: Option<String>,
    pub max_age_seconds: i64,
    /// Human FDUSD redeemed for 1 human vFDUSD (`eth_call` of `exchangeRateCurrent`). Null when missing.
    pub fdusd_per_vfdusd: Option<String>,
    /// Allowlisted source label (`venus_bsc`). Never an RPC URL.
    pub source: String,
    pub fetched_at: Option<String>,
    /// Pinned Core Pool vToken.
    pub vtoken: String,
}

#[derive(Serialize, ToSchema)]
pub struct OraclePriceResponse {
    /// Path ticker (`ustc`, `lunc`, or `vfdusd`).
    pub ticker: String,
    /// CEX quote-asset identity. Path `vfdusd` → `FDUSD` (not Terra CW20 vFDUSD).
    pub quote_asset: String,
    /// Operator/integrator display pair (`USTC/USD`, `LUNC/USD`, `FDUSD/USD`).
    pub display_name: String,
    pub price_usd: Option<String>,
    /// fresh, degraded, stale, or unavailable. Financial callers require fresh.
    pub quality: String,
    /// Local completed HTTP observation time; does not attest exchange market recency.
    pub as_of: Option<String>,
    pub age_seconds: Option<i64>,
    pub max_age_seconds: i64,
    pub source_count: usize,
    pub quorum: usize,
    pub source_policy: String,
    pub accepted_sources: Vec<String>,
    pub sources: Vec<OracleSourcePrice>,
    /// Venus redeem snapshot. Present for `vfdusd` (null fields when uncached); `null` on USTC/LUNC.
    pub venus: Option<VenusVfdusdResponse>,
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
    let source_rows = db_oracle::get_latest_prices_by_source(&state.pool, ticker)
        .await
        .map_err(internal_err)?;

    let snapshot = db_oracle::get_oracle_state(&state.pool, ticker)
        .await
        .map_err(internal_err)?;
    let invalidated = state
        .oracle_prices
        .invalidates(ticker, snapshot.as_ref().and_then(|s| s.as_of))
        .await;
    let now = Utc::now();
    let fresh = !invalidated && snapshot.as_ref().is_some_and(|s| s.is_fresh_at(now));
    let quality = match snapshot.as_ref() {
        Some(_) if fresh => "fresh",
        Some(_) if invalidated => "unavailable",
        Some(s) if s.quality == "fresh" => "stale",
        Some(s) if s.quality == "degraded" => "degraded",
        _ => "unavailable",
    };
    let accepted_sources = snapshot
        .as_ref()
        .filter(|_| fresh)
        .map(|s| s.accepted_sources.clone())
        .unwrap_or_default();
    let as_of = snapshot.as_ref().and_then(|s| s.as_of);
    let current = snapshot
        .as_ref()
        .filter(|_| fresh)
        .and_then(|s| s.price_usd.clone());

    let sources: Vec<OracleSourcePrice> = source_rows
        .into_iter()
        .filter(|r| r.source != VENUS_SOURCE)
        .map(|r| OracleSourcePrice {
            accepted: accepted_sources.contains(&r.source)
                && as_of.is_some_and(|time| r.fetched_at == time),
            source: r.source,
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    let venus = if ticker == OracleTicker::Vfdusd {
        Some(venus_response(state.venus_vfdusd.read().await.clone()))
    } else {
        None
    };

    Ok(Json(OraclePriceResponse {
        ticker: ticker.as_str().to_string(),
        quote_asset: ticker.quote_asset().to_string(),
        display_name: ticker.display_name().to_string(),
        price_usd: current.map(|p| p.to_string()),
        quality: quality.to_string(),
        as_of: as_of.map(|time| time.to_rfc3339()),
        age_seconds: as_of.map(|time| (now - time).num_seconds()),
        max_age_seconds: ORACLE_MAX_AGE_SECS,
        source_count: accepted_sources.len(),
        quorum: ORACLE_QUORUM,
        source_policy: ORACLE_SOURCE_POLICY.to_string(),
        accepted_sources,
        sources,
        venus,
    }))
}

fn venus_response(snap: Option<VenusVfdusdSnapshot>) -> VenusVfdusdResponse {
    match snap {
        Some(s) => {
            let now = Utc::now();
            let fresh = s.source == VENUS_SOURCE
                && s.vtoken == VENUS_VFDUSD_VTOKEN
                && db_oracle::valid_decimal_price(&s.fdusd_per_vfdusd)
                && s.fetched_at <= now
                && now - s.fetched_at <= chrono::Duration::seconds(ORACLE_MAX_AGE_SECS);
            VenusVfdusdResponse {
                quality: if fresh { "fresh" } else { "stale" }.into(),
                as_of: Some(s.fetched_at.to_rfc3339()),
                max_age_seconds: ORACLE_MAX_AGE_SECS,
                fdusd_per_vfdusd: fresh.then(|| s.fdusd_per_vfdusd.to_string()),
                source: s.source.to_string(),
                fetched_at: Some(s.fetched_at.to_rfc3339()),
                vtoken: s.vtoken,
            }
        }
        None => VenusVfdusdResponse {
            quality: "unavailable".into(),
            as_of: None,
            max_age_seconds: ORACLE_MAX_AGE_SECS,
            fdusd_per_vfdusd: None,
            source: VENUS_SOURCE.to_string(),
            fetched_at: None,
            vtoken: VENUS_VFDUSD_VTOKEN.to_string(),
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/oracle/price/{ticker}/venus",
    params(
        ("ticker" = String, Path, description = "Must be vfdusd")
    ),
    responses(
        (status = 200, description = "Venus vFDUSD redeem snapshot (FDUSD per 1 vFDUSD)", body = VenusVfdusdResponse),
        (status = 400, description = "Unknown ticker or not vfdusd"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Oracle"
)]
pub async fn get_oracle_venus_vfdusd(
    State(state): State<AppState>,
    Path(ticker_raw): Path<String>,
) -> Result<Json<VenusVfdusdResponse>, (StatusCode, String)> {
    let ticker = parse_ticker_path(&ticker_raw)?;
    if ticker != OracleTicker::Vfdusd {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Venus snapshot is only available for ticker 'vfdusd', not '{}'",
                ticker.as_str()
            ),
        ));
    }
    Ok(Json(venus_response(
        state.venus_vfdusd.read().await.clone(),
    )))
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
    /// Quorum certification at observation time; does not assert freshness now.
    /// Legacy observations have no certificate and must not drive valuation.
    pub certified: bool,
    pub source_count: usize,
    pub source_policy: Option<String>,
    pub price_usd: String,
    pub fetched_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct OracleHistoryResponse {
    pub ticker: String,
    /// CEX quote-asset identity. Path `vfdusd` → `FDUSD` (not Terra CW20 vFDUSD).
    pub quote_asset: String,
    /// Operator/integrator display pair (`USTC/USD`, `LUNC/USD`, `FDUSD/USD`).
    pub display_name: String,
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
            certified: r.accepted_sources.len() >= ORACLE_QUORUM,
            source_count: r.accepted_sources.len(),
            source_policy: (r.accepted_sources.len() >= ORACLE_QUORUM)
                .then(|| ORACLE_SOURCE_POLICY.to_string()),
            price_usd: r.price_usd.to_string(),
            fetched_at: r.fetched_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(OracleHistoryResponse {
        ticker: ticker.as_str().to_string(),
        quote_asset: ticker.quote_asset().to_string(),
        display_name: ticker.display_name().to_string(),
        prices,
    }))
}
