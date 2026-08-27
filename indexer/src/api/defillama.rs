//! `GET /api/v1/defillama/daily` — UTC calendar-day volume/fees (GitLab #631 / #687).
//!
//! Single `timestamp` (unix 00:00 UTC). Invalid / unaligned / future → **400**.
//! Day not rolled → **404**. Idle closed day → `"0"`. Volume: any unpriced swap → `null`.
//! Headline fees: EFee-6 partial priced SUM (one unpriced source does not null wrap).
//! Headline `null` only when fee activity exists and priced SUM is empty. Per-source
//! `fees.*` stay fail-closed. 60s whole-response cache. Reads `defillama_daily_*` only.
//! No range dump (`from`/`to` ignored; timestamp required). GET only.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{AppState, internal_err};
use crate::db::queries::defillama as daily_q;
use crate::indexer::defillama::{
    COLUMBUS5_FACTORY, DAILY_ASSET_TICKERS, UST1_ADDRESS, UST1_DECIMALS, USTR_ADDRESS,
    USTR_DECIMALS, daily_headline_usd, daily_usd_field_fail_closed, naive_utc_day,
    parse_utc_day_timestamp,
};
use crate::indexer::protocol_fees::FeeSource;

const DAILY_CACHE_TTL: Duration = Duration::from_secs(60);

fn daily_cache() -> &'static Mutex<HashMap<i64, (DefillamaDailyResponse, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<i64, (DefillamaDailyResponse, Instant)>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Drop the 60s daily response cache (tests).
pub fn reset_defillama_cache() {
    if let Ok(mut guard) = daily_cache().lock() {
        guard.clear();
    }
}

#[derive(Debug, Deserialize)]
pub struct DailyQuery {
    pub timestamp: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DefillamaFeeBreakdown {
    pub swap_amm: Option<String>,
    pub book_take: Option<String>,
    pub limit_place: Option<String>,
    pub wrap: Option<String>,
    pub unwrap: Option<String>,
    pub ust1_mint: Option<String>,
    pub ust1_redeem: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DefillamaMethodology {
    pub volume: String,
    pub fees: String,
    pub tvl: String,
    pub factory: String,
    pub ust1: String,
    pub ustr: String,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DefillamaAssetRow {
    pub ticker: String,
    pub symbol: String,
    pub address: String,
    pub decimals: u8,
    pub category: String,
    pub product: String,
    pub peg_type: Option<String>,
    pub peg_mechanism: Option<String>,
    pub volume_usd: Option<String>,
    pub trade_count: i64,
    pub fees_usd: Option<String>,
    pub price_usd: Option<String>,
    pub circulating: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DefillamaDailyAssets {
    pub ust1: DefillamaAssetRow,
    pub ustr: DefillamaAssetRow,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct DefillamaDailyResponse {
    pub date: String,
    pub timestamp: i64,
    pub volume_usd: Option<String>,
    pub trade_count: i64,
    pub fees: DefillamaFeeBreakdown,
    /// Treasury-bound DEX + labeled wrap/window sources. SSR is always `"0"`.
    pub daily_fees_usd: Option<String>,
    pub daily_revenue_usd: Option<String>,
    pub daily_protocol_revenue_usd: Option<String>,
    pub daily_supply_side_revenue_usd: String,
    pub assets: DefillamaDailyAssets,
    pub methodology: DefillamaMethodology,
}

fn methodology() -> DefillamaMethodology {
    DefillamaMethodology {
        volume: "UTC-day SUM(swap_events.volume_usd) once per taker swap (L10). \
                 Excludes gem pairs (#562), wrap/unwrap, UST1 window, and limit_order_fills."
            .to_string(),
        fees: "PFee/L7 treasury: swap_amm + book_take + limit_place + labeled wrap/window. \
               Headline is priced SUM (EFee-6 / #687); one unpriced source does not null wrap. \
               spread_amount and community-tax extra-debit are not fees. SSR is 0."
            .to_string(),
        tvl: "On-chain factory Pairs + pair Pool {} raw balances only. \
              Do not use this indexer USD, overview.total_liquidity_usd, or CG liquidity_in_usd."
            .to_string(),
        factory: COLUMBUS5_FACTORY.to_string(),
        ust1: "UST1 unstablecoin. Llama Stablecoins: peggedUSD, crypto-backed via vFDUSD window. \
               assets.ust1 volume/fees are UTC-day DEX + window mint/redeem. Price is hub, not $1."
            .to_string(),
        ustr: "USTR reserve token (not a USD stablecoin). assets.ustr is DEX volume, pair fees, \
               and hub price. Do not treat 2.5× USTC as a peg."
            .to_string(),
    }
}

fn empty_asset_row(ticker: &str) -> DefillamaAssetRow {
    match ticker {
        "ustr" => DefillamaAssetRow {
            ticker: "ustr".to_string(),
            symbol: "USTR".to_string(),
            address: USTR_ADDRESS.to_string(),
            decimals: USTR_DECIMALS,
            category: "reserve".to_string(),
            product: "ustr".to_string(),
            peg_type: None,
            peg_mechanism: None,
            volume_usd: Some("0".to_string()),
            trade_count: 0,
            fees_usd: Some("0".to_string()),
            price_usd: None,
            circulating: None,
        },
        _ => DefillamaAssetRow {
            ticker: "ust1".to_string(),
            symbol: "UST1".to_string(),
            address: UST1_ADDRESS.to_string(),
            decimals: UST1_DECIMALS,
            category: "stablecoins".to_string(),
            product: "unstablecoin".to_string(),
            peg_type: Some("peggedUSD".to_string()),
            peg_mechanism: Some("crypto-backed".to_string()),
            volume_usd: Some("0".to_string()),
            trade_count: 0,
            fees_usd: Some("0".to_string()),
            price_usd: None,
            circulating: None,
        },
    }
}

fn map_asset_row(ticker: &str, row: Option<&daily_q::DailyAssetRow>) -> DefillamaAssetRow {
    let mut out = empty_asset_row(ticker);
    let Some(row) = row else {
        return out;
    };
    out.volume_usd =
        daily_usd_field_fail_closed(row.trade_count, row.unpriced_trade_count, &row.volume_usd);
    out.trade_count = row.trade_count;
    out.fees_usd =
        daily_usd_field_fail_closed(row.fee_event_count, row.fee_unpriced_count, &row.fees_usd);
    out.price_usd = row
        .price_usd
        .as_ref()
        .filter(|p| *p > &BigDecimal::from(0))
        .map(|p| p.to_string());
    out.circulating = row
        .circulating_raw
        .as_ref()
        .filter(|c| *c > &BigDecimal::from(0))
        .map(|c| c.to_string());
    out
}

fn daily_assets_from_rows(rows: &[daily_q::DailyAssetRow]) -> DefillamaDailyAssets {
    debug_assert!(DAILY_ASSET_TICKERS.contains(&"ust1") && DAILY_ASSET_TICKERS.contains(&"ustr"));
    DefillamaDailyAssets {
        ust1: map_asset_row("ust1", rows.iter().find(|r| r.ticker == "ust1")),
        ustr: map_asset_row("ustr", rows.iter().find(|r| r.ticker == "ustr")),
    }
}

fn fee_field(rows: &[daily_q::DailyFeeRow], source: FeeSource) -> Option<String> {
    let row = rows.iter().find(|r| r.source == source.as_str());
    match row {
        Some(r) => daily_usd_field_fail_closed(r.event_count, r.unpriced_count, &r.amount_usd),
        None => Some("0".to_string()),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/defillama/daily",
    params(
        ("timestamp" = i64, Query, description = "Unix seconds of 00:00 UTC for one calendar day")
    ),
    responses(
        (status = 200, description = "UTC-day volume and treasury fees", body = DefillamaDailyResponse),
        (status = 400, description = "Missing / invalid / unaligned / future timestamp"),
        (status = 404, description = "Day not rolled yet"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "DeFiLlama"
)]
pub async fn get_defillama_daily(
    State(state): State<AppState>,
    Query(q): Query<DailyQuery>,
) -> Result<Json<DefillamaDailyResponse>, (StatusCode, String)> {
    let raw = match q.timestamp.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "timestamp is required (unix 00:00 UTC)".to_string(),
            ));
        }
    };
    let ts = match parse_utc_day_timestamp(raw) {
        Ok(t) => t,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, e.as_http_message().to_string()));
        }
    };

    if let Ok(guard) = daily_cache().lock() {
        if let Some((resp, at)) = guard.get(&ts) {
            if Instant::now().duration_since(*at) <= DAILY_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let utc_day = naive_utc_day(ts).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid timestamp: expected unix seconds of 00:00 UTC".to_string(),
        )
    })?;

    let stat = daily_q::get_daily_stat(&state.pool, utc_day)
        .await
        .map_err(internal_err)?;
    let Some(stat) = stat else {
        return Err((StatusCode::NOT_FOUND, "UTC day not rolled yet".to_string()));
    };

    let fee_rows = daily_q::get_daily_fees(&state.pool, utc_day)
        .await
        .map_err(internal_err)?;
    let asset_rows = daily_q::get_daily_assets(&state.pool, utc_day)
        .await
        .map_err(internal_err)?;

    let volume_usd = daily_usd_field_fail_closed(
        stat.trade_count,
        stat.unpriced_trade_count,
        &stat.volume_usd,
    );

    let mut fee_events = 0i64;
    let mut fee_usd = BigDecimal::from(0);
    for r in &fee_rows {
        fee_events += r.event_count;
        fee_usd += &r.amount_usd;
    }
    let daily_fees = daily_headline_usd(fee_events, &fee_usd);

    let resp = DefillamaDailyResponse {
        date: utc_day.to_string(),
        timestamp: ts,
        volume_usd,
        trade_count: stat.trade_count,
        fees: DefillamaFeeBreakdown {
            swap_amm: fee_field(&fee_rows, FeeSource::SwapAmm),
            book_take: fee_field(&fee_rows, FeeSource::BookTake),
            limit_place: fee_field(&fee_rows, FeeSource::LimitPlace),
            wrap: fee_field(&fee_rows, FeeSource::Wrap),
            unwrap: fee_field(&fee_rows, FeeSource::Unwrap),
            ust1_mint: fee_field(&fee_rows, FeeSource::Ust1Mint),
            ust1_redeem: fee_field(&fee_rows, FeeSource::Ust1Redeem),
        },
        daily_fees_usd: daily_fees.clone(),
        daily_revenue_usd: daily_fees.clone(),
        daily_protocol_revenue_usd: daily_fees,
        daily_supply_side_revenue_usd: "0".to_string(),
        assets: daily_assets_from_rows(&asset_rows),
        methodology: methodology(),
    };

    if let Ok(mut guard) = daily_cache().lock() {
        let now = Instant::now();
        guard.retain(|_, (_, at)| now.duration_since(*at) <= DAILY_CACHE_TTL);
        guard.insert(ts, (resp.clone(), now));
    }

    Ok(Json(resp))
}
