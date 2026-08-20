//! DEX hub USD API (GitLab #556 / #570). Separate from CEX `GET /api/v1/oracle/price`.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::hub_prices::{self, HubPriceRow};
use crate::indexer::hub_usd::{hub_wrap_asset_address, HubTicker, HubUsdConfig, HUB_TICKERS};

pub const HUB_PRICES_METADATA: &str = concat!(
    "DEX hub USD marks from the largest-liquidity factory pools (not CEX, not settlement, ",
    "not TWAP, not the UST1 window rate). ",
    "GET /api/v1/hub-prices for the snapshot; GET /api/v1/hub-prices/{ticker} for one ticker. ",
    "Tickers: custc (1:1 USTC CEX oracle), lunc (1:1 LUNC CEX oracle), ",
    "ust1 (deepest cUSTC/UST1 pool), ustr (deepest pair vs cUSTC or UST1). ",
    "asset_address is the configured wrap CW20 (not a source pair). ",
    "Identity is hub CW20 contract, not symbol. GET /oracle/price/lunc remains the CEX history feed."
);

#[derive(Serialize, ToSchema)]
pub struct HubPriceEntry {
    pub ticker: String,
    pub asset_id: Option<i32>,
    pub price_usd: Option<String>,
    pub source_pair: Option<String>,
    /// Configured hub wrap CW20 (cUSTC / cLUNC / UST1 / USTR). Never a native denom.
    pub asset_address: Option<String>,
    pub tvl_usd: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct HubPricesResponse {
    pub metadata: String,
    pub tickers: Vec<String>,
    pub prices: Vec<HubPriceEntry>,
}

fn parse_hub_ticker(raw: &str) -> Result<HubTicker, (StatusCode, String)> {
    HubTicker::parse(raw).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Unknown hub ticker '{raw}'. Supported: {}",
                HUB_TICKERS.join(", ")
            ),
        )
    })
}

fn entry_for(ticker: HubTicker, row: Option<&HubPriceRow>, cfg: &HubUsdConfig) -> HubPriceEntry {
    let asset_address = hub_wrap_asset_address(ticker, cfg);
    match row {
        Some(r) => HubPriceEntry {
            ticker: ticker.as_str().to_string(),
            asset_id: r.asset_id,
            price_usd: Some(r.price_usd.to_string()),
            source_pair: r.source_pair_address.clone(),
            asset_address,
            tvl_usd: r.tvl_usd.as_ref().map(|v| v.to_string()),
            updated_at: Some(r.updated_at.to_rfc3339()),
        },
        None => HubPriceEntry {
            ticker: ticker.as_str().to_string(),
            asset_id: None,
            price_usd: None,
            source_pair: None,
            asset_address,
            tvl_usd: None,
            updated_at: None,
        },
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/hub-prices",
    responses(
        (status = 200, description = "DEX hub USD snapshot", body = HubPricesResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "HubPrices"
)]
pub async fn get_hub_prices(
    State(state): State<AppState>,
) -> Result<Json<HubPricesResponse>, (StatusCode, String)> {
    let rows = hub_prices::get_all_hub_prices(&state.pool)
        .await
        .map_err(internal_err)?;
    let prices = [
        HubTicker::Custc,
        HubTicker::Lunc,
        HubTicker::Ust1,
        HubTicker::Ustr,
    ]
    .into_iter()
    .map(|t| {
        let row = rows.iter().find(|r| r.ticker == t.as_str());
        entry_for(t, row, &state.hub_usd)
    })
    .collect();
    Ok(Json(HubPricesResponse {
        metadata: HUB_PRICES_METADATA.to_string(),
        tickers: HUB_TICKERS.iter().map(|s| (*s).to_string()).collect(),
        prices,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/hub-prices/{ticker}",
    params(
        ("ticker" = String, Path, description = "Hub ticker: custc, lunc, ust1, or ustr")
    ),
    responses(
        (status = 200, description = "One DEX hub USD mark", body = HubPriceEntry),
        (status = 400, description = "Unknown ticker"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "HubPrices"
)]
pub async fn get_hub_price(
    State(state): State<AppState>,
    Path(ticker_raw): Path<String>,
) -> Result<Json<HubPriceEntry>, (StatusCode, String)> {
    let ticker = parse_hub_ticker(&ticker_raw)?;
    let row = hub_prices::get_hub_price(&state.pool, ticker)
        .await
        .map_err(internal_err)?;
    Ok(Json(entry_for(ticker, row.as_ref(), &state.hub_usd)))
}
