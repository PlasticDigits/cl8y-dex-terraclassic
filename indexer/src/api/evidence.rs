//! `GET /api/v1/evidence/daily` — redacted UTC-day protocol activity (GitLab #1205).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use base64::Engine;
use chrono::{NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use utoipa::{IntoParams, ToSchema};

use super::pairs::bd_plain_string;
use super::{internal_err, AppState};
use crate::db::queries::evidence_daily::{EvidenceRawRow, EvidenceSortKey, EvidenceSurfaceSet};

const CURSOR_PREFIX: &str = "e1.";
const DEFAULT_LIMIT: i64 = 500;
const MAX_LIMIT: i64 = 1000;

fn bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

/// SHA-256 of trimmed bech32 actor; wire form is first 32 hex chars (16 bytes).
pub fn actor_hash(actor: &str) -> Option<String> {
    let trimmed = actor.trim();
    if trimmed.is_empty() || !trimmed.starts_with("terra1") || trimmed.len() < 20 {
        return None;
    }
    let digest = Sha256::digest(trimmed.as_bytes());
    Some(hex::encode(&digest[..16]))
}

mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
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
    pub actor_hash: Option<String>,
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

fn flatten_surface_params(params: &EvidenceDailyQuery) -> Vec<String> {
    params.surface.clone()
}

fn encode_cursor(key: &EvidenceSortKey) -> String {
    let payload = serde_json::json!({
        "v": 1,
        "block_height": key.block_height,
        "tx_hash": key.tx_hash,
        "surface": key.surface,
        "kind": key.kind,
        "ordinal": key.ordinal,
        "row_id": key.row_id,
    });
    let b = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("{}{}", CURSOR_PREFIX, b)
}

fn decode_cursor(raw: &str) -> Result<EvidenceSortKey, ()> {
    let rest = raw.strip_prefix(CURSOR_PREFIX).ok_or(())?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(rest)
        .map_err(|_| ())?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if v.get("v").and_then(|x| x.as_i64()) != Some(1) {
        return Err(());
    }
    Ok(EvidenceSortKey {
        block_height: v.get("block_height").and_then(|x| x.as_i64()).ok_or(())?,
        tx_hash: v
            .get("tx_hash")
            .and_then(|x| x.as_str())
            .ok_or(())?
            .to_string(),
        surface: v
            .get("surface")
            .and_then(|x| x.as_str())
            .ok_or(())?
            .to_string(),
        kind: v
            .get("kind")
            .and_then(|x| x.as_str())
            .ok_or(())?
            .to_string(),
        ordinal: v.get("ordinal").and_then(|x| x.as_i64()).ok_or(())?,
        row_id: v.get("row_id").and_then(|x| x.as_i64()).ok_or(())?,
    })
}

fn map_row(row: EvidenceRawRow) -> EvidenceDailyEvent {
    let actor_hash = row.actor.as_deref().and_then(actor_hash);
    let fee_usd = row.fee_usd.as_ref().map(|v| bd_plain_string(v));
    EvidenceDailyEvent {
        surface: row.surface,
        kind: row.kind,
        block_height: row.block_height,
        block_timestamp: row.block_timestamp.to_rfc3339(),
        tx_hash: row.tx_hash,
        pair_address: row.pair_address,
        actor_hash,
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
        (status = 200, description = "Redacted UTC-day evidence export", body = EvidenceDailyResponse),
        (status = 400, description = "Invalid day, surface, cursor, or format"),
        (status = 500, description = "Internal server error"),
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
        .is_some_and(|f| f.eq_ignore_ascii_case("csv"))
    {
        return Err(bad_request("format=csv is not supported on this route"));
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

    let surfaces = parse_surfaces(&flatten_surface_params(&params))
        .map_err(|_| bad_request("invalid surface"))?;

    let limit = parse_limit(params.limit.as_ref());
    let cursor_key = match params.cursor.as_deref() {
        Some(c) if c.is_empty() => None,
        Some(c) => Some(decode_cursor(c).map_err(|_| bad_request("invalid cursor"))?),
        None => None,
    };

    let day_start = Utc.from_utc_datetime(&day.and_hms_opt(0, 0, 0).expect("midnight"));
    let day_end = day_start + chrono::Duration::days(1);

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
    .map_err(internal_err)?;

    let has_more = rows.len() as i64 > limit;
    let page_rows: Vec<EvidenceRawRow> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if has_more {
        let last = page_rows.last().expect("has_more implies row");
        Some(encode_cursor(&EvidenceSortKey {
            block_height: last.block_height,
            tx_hash: last.tx_hash.clone(),
            surface: last.surface.clone(),
            kind: last.kind.clone(),
            ordinal: last.ordinal,
            row_id: last.row_id,
        }))
    } else {
        None
    };
    let events: Vec<EvidenceDailyEvent> = page_rows.into_iter().map(map_row).collect();

    Ok(Json(EvidenceDailyResponse {
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
    fn actor_hash_stable_and_sized() {
        let a = "terra1abcdefghijklmnopqrstuvwxyz";
        let h1 = actor_hash(a).expect("hash");
        let h2 = actor_hash("  terra1abcdefghijklmnopqrstuvwxyz  ").expect("trim");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(actor_hash("terra1otheraddressxxxxxxxxxxxx"), actor_hash(a));
    }

    #[test]
    fn actor_hash_rejects_garbage() {
        assert!(actor_hash("").is_none());
        assert!(actor_hash("cosmos1foo").is_none());
        assert!(actor_hash("terra1").is_none());
    }

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
        let c = encode_cursor(&key);
        let back = decode_cursor(&c).expect("decode");
        assert_eq!(back.block_height, 99);
        assert_eq!(back.tx_hash, "ABC");
        assert_eq!(back.row_id, 7);
        assert!(decode_cursor("e1.e30").is_err());
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
}
