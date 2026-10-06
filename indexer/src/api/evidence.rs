//! Version 2 public-chain-linked UTC-day evidence (#1397). No actor identifiers.

use axum::Json;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use utoipa::{IntoParams, ToSchema};

use super::AppState;
use super::pairs::bd_plain_string;
use crate::db::queries::evidence_daily::{EvidenceRawRow, EvidenceSortKey, EvidenceSurfaceSet};

const CURSOR_PREFIX: &str = "e2.";
const MAX_CURSOR_BYTES: usize = 2048;
pub const MAX_CONCURRENT_EXPORTS: usize = 2;
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const PRIVACY_POLICY: &str = "public-chain-linked-no-actor-identifiers";
const DEFAULT_LIMIT: i64 = 500;
const MAX_LIMIT: i64 = 1000;

fn bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

/// Admission is shared by every evidence request on this router, independent of
/// client/proxy headers. No queue: exports cannot accumulate pool waiters.
pub async fn admit_export(
    State(admission): State<Arc<Semaphore>>,
    request: Request,
    next: Next,
) -> Response {
    let Ok(permit) = admission.try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "1")],
            "Evidence export busy",
        )
            .into_response();
    };
    // A disconnected caller must not free admission while PostgreSQL is still
    // running its query. The bounded worker owns the permit until SQL settles;
    // dropping the HTTP future detaches the worker, without opening a new slot.
    tokio::spawn(async move {
        let _permit = permit;
        match tokio::time::timeout(REQUEST_TIMEOUT, next.run(request)).await {
            Ok(response) => response,
            Err(_) => unavailable(),
        }
    })
    .await
    .unwrap_or_else(|_| unavailable())
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Evidence export unavailable",
    )
        .into_response()
}

fn export_error(_: sqlx::Error) -> (StatusCode, String) {
    // Database errors can contain data or SQL; never echo/log them for exports.
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Evidence export unavailable".to_string(),
    )
}

#[derive(Debug, IntoParams, ToSchema)]
pub struct EvidenceDailyQuery {
    pub day: Option<String>,
    /// Collected from every `surface` key. A derived `Vec` rejects `surface=swap`
    /// (string, not a sequence) and a custom field deserializer rejects
    /// `surface=swap&surface=lp` (`duplicate field`).
    pub surface: Vec<String>,
    pub limit: Option<String>,
    pub cursor: Option<String>,
    pub format: Option<String>,
}

impl<'de> Deserialize<'de> for EvidenceDailyQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // serde_urlencoded yields one pair per key, including repeats.
        let pairs = Vec::<(String, String)>::deserialize(deserializer)?;
        let mut day = None;
        let mut surface = Vec::new();
        let mut limit = None;
        let mut cursor = None;
        let mut format = None;
        for (key, value) in pairs {
            match key.as_str() {
                "day" => day = Some(value),
                "surface" => surface.push(value),
                "limit" => limit = Some(value),
                "cursor" => cursor = Some(value),
                "format" => format = Some(value),
                _ => {}
            }
        }
        Ok(Self {
            day,
            surface,
            limit,
            cursor,
            format,
        })
    }
}

#[derive(Serialize, ToSchema, Clone)]
pub struct EvidenceDailyEvent {
    pub surface: String,
    pub kind: String,
    pub block_height: i64,
    pub block_timestamp: String,
    pub tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pair_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book_return_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offer_decimals: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_decimals: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token0_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token1_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commission_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_event_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_0_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_1_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lp_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_raw: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_denom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_contract: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_usd: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct EvidenceDailyResponse {
    pub schema_version: u8,
    /// Transaction joins can recover public-chain actors; this is not anonymity.
    pub privacy_policy: String,
    pub day: String,
    pub timezone: String,
    pub complete: bool,
    pub surfaces: Vec<String>,
    pub events: Vec<EvidenceDailyEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

fn parse_limit(raw: Option<&String>) -> i64 {
    let n = match raw {
        Some(s) => s.parse::<i64>().unwrap_or(DEFAULT_LIMIT),
        None => DEFAULT_LIMIT,
    };
    n.clamp(1, MAX_LIMIT)
}

fn parse_day(raw: &str) -> Result<NaiveDate, ()> {
    if raw.len() != 10 || raw.eq_ignore_ascii_case("today") {
        return Err(());
    }
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| ())
}

fn parse_surfaces(tokens: &[String]) -> Result<EvidenceSurfaceSet, ()> {
    if tokens.is_empty() {
        return Ok(EvidenceSurfaceSet::all());
    }
    let mut swap = false;
    let mut wrap = false;
    let mut limit = false;
    let mut lp = false;
    for t in tokens {
        for part in t.split(',') {
            let p = part.trim().to_ascii_lowercase();
            if p.is_empty() {
                return Err(());
            }
            match p.as_str() {
                "swap" => swap = true,
                "wrap" => wrap = true,
                "limit" => limit = true,
                "lp" => lp = true,
                _ => return Err(()),
            }
        }
    }
    Ok(EvidenceSurfaceSet {
        swap,
        wrap,
        limit,
        lp,
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceCursor {
    v: u8,
    day: String,
    surfaces: Vec<String>,
    block_height: i64,
    tx_hash: String,
    surface: String,
    kind: String,
    ordinal: i64,
    row_id: i64,
}

fn encode_cursor(key: &EvidenceSortKey, day: NaiveDate, surfaces: EvidenceSurfaceSet) -> String {
    let payload = EvidenceCursor {
        v: 2,
        day: day.to_string(),
        surfaces: surfaces_wire(surfaces),
        block_height: key.block_height,
        tx_hash: key.tx_hash.clone(),
        surface: key.surface.clone(),
        kind: key.kind.clone(),
        ordinal: key.ordinal,
        row_id: key.row_id,
    };
    let bytes = serde_json::to_vec(&payload).expect("cursor serialization");
    format!(
        "{}{}",
        CURSOR_PREFIX,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn decode_cursor(
    raw: &str,
    day: NaiveDate,
    surfaces: EvidenceSurfaceSet,
) -> Result<EvidenceSortKey, ()> {
    if raw.len() > MAX_CURSOR_BYTES {
        return Err(());
    }
    let rest = raw.strip_prefix(CURSOR_PREFIX).ok_or(())?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(rest)
        .map_err(|_| ())?;
    let c: EvidenceCursor = serde_json::from_slice(&bytes).map_err(|_| ())?;
    let valid_kind = match c.surface.as_str() {
        "swap" => c.kind == "swap",
        "lp" => matches!(c.kind.as_str(), "add" | "remove"),
        "limit" => matches!(c.kind.as_str(), "place" | "cancel" | "fill"),
        "wrap" => matches!(c.kind.as_str(), "wrap" | "unwrap"),
        _ => false,
    };
    if c.v != 2
        || c.day != day.to_string()
        || c.surfaces != surfaces_wire(surfaces)
        || !c.surfaces.contains(&c.surface)
        || !valid_kind
        || c.block_height < 0
        || c.ordinal < 0
        || c.row_id <= 0
        || c.tx_hash.is_empty()
        || c.tx_hash.len() > 128
        || !c
            .tx_hash
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(());
    }
    Ok(EvidenceSortKey {
        block_height: c.block_height,
        tx_hash: c.tx_hash,
        surface: c.surface,
        kind: c.kind,
        ordinal: c.ordinal,
        row_id: c.row_id,
    })
}

fn map_row(row: EvidenceRawRow) -> EvidenceDailyEvent {
    let fee_usd = row.fee_usd.as_ref().map(|v| bd_plain_string(v));
    EvidenceDailyEvent {
        surface: row.surface,
        kind: row.kind,
        block_height: row.block_height,
        block_timestamp: row.block_timestamp.to_rfc3339(),
        tx_hash: row.tx_hash,
        pair_address: row.pair_address,
        offer_amount: row.offer_amount.as_ref().map(bd_plain_string),
        return_amount: row.return_amount.as_ref().map(bd_plain_string),
        pool_return_amount: row.pool_return_amount.as_ref().map(bd_plain_string),
        book_return_amount: row.book_return_amount.as_ref().map(bd_plain_string),
        offer_decimals: row.offer_decimals,
        ask_decimals: row.ask_decimals,
        order_id: row.order_id,
        side: row.side,
        price: row.price.as_ref().map(bd_plain_string),
        token0_amount: row.token0_amount.as_ref().map(bd_plain_string),
        token1_amount: row.token1_amount.as_ref().map(bd_plain_string),
        commission_amount: row.commission_amount.as_ref().map(bd_plain_string),
        swap_event_id: row.swap_event_id,
        asset_0_amount: row.asset_0_amount.as_ref().map(bd_plain_string),
        asset_1_amount: row.asset_1_amount.as_ref().map(bd_plain_string),
        lp_amount: row.lp_amount.as_ref().map(bd_plain_string),
        amount_raw: row.amount_raw.as_ref().map(bd_plain_string),
        decimals: row.fee_decimals,
        token_denom: row.token_denom,
        token_contract: row.token_contract,
        fee_usd,
    }
}

fn surfaces_wire(set: EvidenceSurfaceSet) -> Vec<String> {
    let mut out = Vec::new();
    if set.swap {
        out.push("swap".to_string());
    }
    if set.wrap {
        out.push("wrap".to_string());
    }
    if set.limit {
        out.push("limit".to_string());
    }
    if set.lp {
        out.push("lp".to_string());
    }
    out
}

#[utoipa::path(
    get,
    path = "/api/v1/evidence/daily",
    params(EvidenceDailyQuery),
    responses(
        (status = 200, description = "Schema v2 public-chain-linked evidence; no actor identifiers or anonymity guarantee", body = EvidenceDailyResponse),
        (status = 400, description = "Invalid day, surface, cursor, or format"),
        (status = 429, description = "Export admission full"),
        (status = 503, description = "Export database or request budget exceeded"),
    ),
    tag = "Evidence"
)]
pub async fn get_evidence_daily(
    State(state): State<AppState>,
    Query(params): Query<EvidenceDailyQuery>,
) -> Result<Json<EvidenceDailyResponse>, (StatusCode, String)> {
    if params
        .format
        .as_deref()
        .is_some_and(|f| !f.eq_ignore_ascii_case("json"))
    {
        return Err(bad_request("only format=json is supported on this route"));
    }

    let day_raw = params
        .day
        .as_deref()
        .ok_or_else(|| bad_request("day is required"))?;
    let day = parse_day(day_raw).map_err(|_| bad_request("invalid day"))?;
    let today = Utc::now().date_naive();
    if day > today {
        return Err(bad_request("day cannot be in the future"));
    }

    let surfaces = parse_surfaces(&params.surface).map_err(|_| bad_request("invalid surface"))?;

    let limit = parse_limit(params.limit.as_ref());
    let cursor_key = match params.cursor.as_deref() {
        Some(c) if c.is_empty() => None,
        Some(c) => {
            Some(decode_cursor(c, day, surfaces).map_err(|_| bad_request("invalid cursor"))?)
        }
        None => None,
    };

    let day_start = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).expect("midnight"));
    let day_end = day_start
        .checked_add_signed(chrono::Duration::days(1))
        .ok_or_else(|| bad_request("invalid day"))?;

    let fetch_limit = limit + 1;
    let rows = crate::db::queries::evidence_daily::list_evidence_rows(
        &state.pool,
        day_start,
        day_end,
        surfaces,
        cursor_key.as_ref(),
        fetch_limit,
    )
    .await
    .map_err(export_error)?;

    let has_more = rows.len() as i64 > limit;
    let page_rows: Vec<EvidenceRawRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        let last = page_rows.last().expect("has_more implies row");
        Some(encode_cursor(
            &EvidenceSortKey {
                block_height: last.block_height,
                tx_hash: last.tx_hash.clone(),
                surface: last.surface.clone(),
                kind: last.kind.clone(),
                ordinal: last.ordinal,
                row_id: last.row_id,
            },
            day,
            surfaces,
        ))
    } else {
        None
    };
    let events: Vec<EvidenceDailyEvent> = page_rows.into_iter().map(map_row).collect();

    Ok(Json(EvidenceDailyResponse {
        schema_version: 2,
        privacy_policy: PRIVACY_POLICY.to_string(),
        day: day.to_string(),
        timezone: "UTC".to_string(),
        complete: day < today,
        surfaces: surfaces_wire(surfaces),
        events,
        next_cursor,
        has_more,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        let key = EvidenceSortKey {
            block_height: 99,
            tx_hash: "ABC".to_string(),
            surface: "swap".to_string(),
            kind: "swap".to_string(),
            ordinal: 2,
            row_id: 7,
        };
        let day = parse_day("2026-06-10").unwrap();
        let surfaces = EvidenceSurfaceSet::all();
        let c = encode_cursor(&key, day, surfaces);
        let back = decode_cursor(&c, day, surfaces).expect("decode");
        assert_eq!(back.block_height, 99);
        assert_eq!(back.tx_hash, "ABC");
        assert_eq!(back.row_id, 7);
        assert!(decode_cursor("e1.e30", day, surfaces).is_err());
    }

    #[test]
    fn limit_clamp_matches_list_routes() {
        assert_eq!(parse_limit(None), DEFAULT_LIMIT);
        assert_eq!(parse_limit(Some(&"0".to_string())), 1);
        assert_eq!(parse_limit(Some(&"-1".to_string())), 1);
        assert_eq!(parse_limit(Some(&"10000".to_string())), MAX_LIMIT);
        assert_eq!(parse_limit(Some(&"3".to_string())), 3);
        assert_eq!(parse_limit(Some(&"1e999".to_string())), DEFAULT_LIMIT);
    }
    /// No database is involved: paused time isolates the HTTP deadline from
    /// PostgreSQL's shorter statement/lock budgets and avoids a three-second sleep.
    #[tokio::test(start_paused = true)]
    async fn evidence_daily_middleware_deadline_and_cancelled_caller() {
        use super::{REQUEST_TIMEOUT, admit_export};
        use axum::{
            Router,
            body::{Body, to_bytes},
            http::Request,
            middleware,
            routing::get,
        };
        use std::sync::Arc;
        use tokio::sync::{Semaphore, mpsc};
        use tower::ServiceExt;

        let admission = Arc::new(Semaphore::new(2));
        let (entered, mut entries) = mpsc::unbounded_channel();
        let app = Router::new()
            .route(
                "/blocked",
                get(move || {
                    let entered = entered.clone();
                    async move {
                        entered.send(()).unwrap();
                        std::future::pending::<StatusCode>().await
                    }
                }),
            )
            .route("/quick", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn_with_state(
                admission.clone(),
                admit_export,
            ));
        let request = |path: &str| Request::builder().uri(path).body(Body::empty()).unwrap();
        let cancelled = tokio::spawn(app.clone().oneshot(request("/blocked")));
        let survivor = tokio::spawn(app.clone().oneshot(request("/blocked")));
        entries.recv().await.unwrap();
        entries.recv().await.unwrap();
        assert_eq!(admission.available_permits(), 0);
        cancelled.abort();
        assert!(cancelled.await.unwrap_err().is_cancelled());
        let busy = app.clone().oneshot(request("/quick")).await.unwrap();
        assert_eq!(busy.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(admission.available_permits(), 0);

        tokio::time::advance(REQUEST_TIMEOUT).await;
        let timeout = survivor.await.unwrap().unwrap();
        assert_eq!(timeout.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(timeout.into_body(), 1024).await.unwrap();
        assert_eq!(body.as_ref(), b"Evidence export unavailable");
        tokio::task::yield_now().await;
        assert_eq!(admission.available_permits(), 2);
        let recovered = app.oneshot(request("/quick")).await.unwrap();
        assert_eq!(recovered.status(), StatusCode::OK);
    }
}
