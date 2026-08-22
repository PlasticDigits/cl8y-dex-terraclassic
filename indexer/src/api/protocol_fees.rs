//! `GET /api/v1/protocol/fees` — O(1) rollup breakdown (GitLab #586).
//!
//! `window` is allowlisted (`24h` \| `7d` \| `30d`). Unknown / injection → **400**.
//! Whole-response cache 60s. Does not scan `protocol_fee_events` / `swap_events` / fills.
//! Standard governor only (not LCD-heavy). No CSV.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::protocol_fees as fee_q;
use crate::indexer::protocol_fees::FeeSource;

const FEES_CACHE_TTL: Duration = Duration::from_secs(60);

fn fees_cache() -> &'static Mutex<std::collections::HashMap<String, (ProtocolFeesResponse, Instant)>>
{
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, (ProtocolFeesResponse, Instant)>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[derive(Debug, Deserialize)]
pub struct ProtocolFeesQuery {
    pub window: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolFeeSourceRow {
    pub source: String,
    pub amount_usd: Option<String>,
    pub share_pct: Option<String>,
    pub event_count: i64,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolFeeTokenRow {
    pub asset_id: Option<i32>,
    pub symbol: String,
    pub contract_or_denom: Option<String>,
    pub amount_human: Option<String>,
    pub amount_usd: Option<String>,
    pub share_pct: Option<String>,
    pub is_other: bool,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolFeesResponse {
    pub window: String,
    pub wrap_mapper_configured: bool,
    pub by_source: Vec<ProtocolFeeSourceRow>,
    pub by_token: Vec<ProtocolFeeTokenRow>,
}

/// Drop the 60s `/protocol/fees` response cache (tests).
pub fn reset_protocol_fees_cache() {
    if let Ok(mut guard) = fees_cache().lock() {
        guard.clear();
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/protocol/fees",
    params(
        ("window" = Option<String>, Query, description = "Trailing window: 24h, 7d, or 30d")
    ),
    responses(
        (status = 200, description = "Protocol fee breakdown", body = ProtocolFeesResponse),
        (status = 400, description = "Invalid window"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_protocol_fees(
    State(state): State<AppState>,
    Query(q): Query<ProtocolFeesQuery>,
) -> Result<Json<ProtocolFeesResponse>, (StatusCode, String)> {
    let window = match fee_q::parse_fee_window(q.window.as_deref()) {
        Ok(w) => w,
        Err(()) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Invalid window, expected 24h, 7d, or 30d".to_string(),
            ));
        }
    };

    if let Ok(guard) = fees_cache().lock() {
        if let Some((resp, at)) = guard.get(window) {
            if Instant::now().duration_since(*at) <= FEES_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let rollup = fee_q::get_fee_rollup(&state.pool)
        .await
        .map_err(internal_err)?;
    let sources = fee_q::get_fees_by_source(&state.pool, window)
        .await
        .map_err(internal_err)?;
    let tokens = fee_q::get_fees_by_token(&state.pool, window)
        .await
        .map_err(internal_err)?;

    let by_source = sources
        .into_iter()
        .filter_map(|r| {
            let source = FeeSource::parse(&r.source)?;
            Some(ProtocolFeeSourceRow {
                source: source.as_str().to_string(),
                amount_usd: match (r.event_count, r.amount_usd) {
                    (n, _) if n <= 0 => Some("0".to_string()),
                    (_, Some(u)) if u > bigdecimal::BigDecimal::from(0) => Some(u.to_string()),
                    _ => None,
                },
                share_pct: r.share_pct.map(|p| p.to_string()),
                event_count: r.event_count,
            })
        })
        .collect();

    let by_token = tokens
        .into_iter()
        .map(|r| {
            let symbol = if r.is_other {
                "other".to_string()
            } else {
                r.symbol.unwrap_or_else(|| "—".to_string())
            };
            let contract_or_denom = if r.is_other {
                None
            } else {
                r.contract_address.or(r.denom)
            };
            ProtocolFeeTokenRow {
                asset_id: r.asset_id,
                symbol,
                contract_or_denom,
                amount_human: r.amount_human.map(|h| h.to_string()),
                amount_usd: r.amount_usd.map(|u| u.to_string()),
                share_pct: r.share_pct.map(|p| p.to_string()),
                is_other: r.is_other,
            }
        })
        .collect();

    let resp = ProtocolFeesResponse {
        window: window.to_string(),
        wrap_mapper_configured: rollup.wrap_mapper_configured,
        by_source,
        by_token,
    };

    if let Ok(mut guard) = fees_cache().lock() {
        let now = Instant::now();
        guard.retain(|_, (_, at)| now.duration_since(*at) <= FEES_CACHE_TTL);
        guard.insert(window.to_string(), (resp.clone(), now));
    }

    Ok(Json(resp))
}
