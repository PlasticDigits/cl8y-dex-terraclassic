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
    "DEX hub USD marks from independently priced-leg depth (not CEX, not settlement, ",
    "not TWAP, not the UST1 window rate). Non-oracle-grade advisory values. ",
    "Fixed USTC-unit reference rejects >10% divergence; bootstrap/source switches need 60s observations. ",
    "Initial manipulation and <=10% shifts remain possible; no financial automation may trust spots. ",
    "GET /api/v1/hub-prices for the snapshot; GET /api/v1/hub-prices/{ticker} for one ticker. ",
    "Tickers: custc (1:1 USTC CEX oracle), lunc (1:1 LUNC CEX oracle), ",
    "ust1 (deepest cUSTC/UST1 pool), ustr (deepest pair vs cUSTC or UST1). ",
    "asset_address is the configured wrap CW20 (not a source pair). ",
    "Identity is hub CW20 contract, not symbol. GET /oracle/price/lunc remains the CEX history feed."
);

#[derive(Serialize, ToSchema, Clone)]
pub struct HubPriceEntry {
    pub ticker: String,
    pub asset_id: Option<i32>,
    pub price_usd: Option<String>,
    pub source_pair: Option<String>,
    /// Configured hub wrap CW20 (cUSTC / cLUNC / UST1 / USTR). Never a native denom.
    pub asset_address: Option<String>,
    pub tvl_usd: Option<String>,
    pub updated_at: Option<String>,
    pub as_of: Option<String>,
    pub max_age_seconds: i64,
    pub quality: String,
    pub oracle_grade: bool,
    pub policy: String,
    pub priced_leg_depth_usd: Option<String>,
    pub divergence_pct: Option<String>,
    pub reference_at: Option<String>,
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

pub(crate) fn entry_for(
    ticker: HubTicker,
    row: Option<&HubPriceRow>,
    cfg: &HubUsdConfig,
    reference: Option<&crate::indexer::hub_guard::HubReference>,
) -> HubPriceEntry {
    let asset_address = hub_wrap_asset_address(ticker, cfg);
    let max_age_seconds = if matches!(ticker, HubTicker::Custc | HubTicker::Lunc) {
        crate::indexer::oracle::ORACLE_MAX_AGE_SECS
    } else {
        hub_prices::live_hub_max_age_seconds()
    };
    let usable = row.filter(|r| hub_prices::row_usable(r, chrono::Utc::now()));
    let quality = usable.map(|r| r.quality.clone()).unwrap_or_else(|| {
        if row.is_some() {
            "stale".into()
        } else {
            reference
                .map(|r| {
                    if r.quality == "bounded_spot" {
                        "unavailable".into()
                    } else {
                        r.quality.clone()
                    }
                })
                .unwrap_or_else(|| "unavailable".into())
        }
    });
    match usable {
        Some(r) => HubPriceEntry {
            ticker: ticker.as_str().to_string(),
            asset_id: r.asset_id,
            price_usd: Some(r.price_usd.to_string()),
            source_pair: r.source_pair_address.clone(),
            asset_address,
            tvl_usd: r.tvl_usd.as_ref().map(|v| v.to_string()),
            updated_at: Some(r.updated_at.to_rfc3339()),
            as_of: r.as_of.map(|v| v.to_rfc3339()),
            max_age_seconds,
            quality,
            oracle_grade: false,
            policy: crate::indexer::hub_guard::HUB_POLICY.into(),
            priced_leg_depth_usd: r.priced_leg_depth_usd.as_ref().map(|v| v.to_string()),
            divergence_pct: reference.map(|r| r.divergence_pct.to_string()),
            reference_at: reference.map(|r| r.reference_at.to_rfc3339()),
        },
        None => HubPriceEntry {
            ticker: ticker.as_str().to_string(),
            asset_id: None,
            price_usd: None,
            source_pair: None,
            asset_address,
            tvl_usd: None,
            updated_at: None,
            as_of: row.and_then(|r| r.as_of).map(|v| v.to_rfc3339()),
            max_age_seconds,
            quality,
            oracle_grade: false,
            policy: crate::indexer::hub_guard::HUB_POLICY.into(),
            priced_leg_depth_usd: None,
            divergence_pct: reference.map(|r| r.divergence_pct.to_string()),
            reference_at: reference.map(|r| r.reference_at.to_rfc3339()),
        },
    }
}

pub(super) async fn qualify_process_prices(rows: &mut [HubPriceRow], state: &AppState) {
    for row in rows {
        let ticker = if row.ticker == "lunc" {
            crate::indexer::oracle::OracleTicker::Lunc
        } else {
            crate::indexer::oracle::OracleTicker::Ustc
        };
        if state.oracle_prices.invalidates(ticker, None).await {
            row.upstream_available = false;
        }
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
    let mut rows = hub_prices::get_all_hub_prices(&state.pool)
        .await
        .map_err(internal_err)?;
    qualify_process_prices(&mut rows, &state).await;
    let references = hub_prices::get_hub_references(&state.pool)
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
        entry_for(
            t,
            row,
            &state.hub_usd,
            references.iter().find(|r| r.ticker == t.as_str()),
        )
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
    let mut row = hub_prices::get_hub_price(&state.pool, ticker)
        .await
        .map_err(internal_err)?;
    if let Some(row) = &mut row {
        qualify_process_prices(std::slice::from_mut(row), &state).await;
    }
    let references = hub_prices::get_hub_references(&state.pool)
        .await
        .map_err(internal_err)?;
    Ok(Json(entry_for(
        ticker,
        row.as_ref(),
        &state.hub_usd,
        references.iter().find(|r| r.ticker == ticker.as_str()),
    )))
}
