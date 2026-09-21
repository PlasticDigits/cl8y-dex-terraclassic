//! `GET /api/v1/protocol/top-pairs` — trailing 30d factory pair ranking (Forgejo #1263).
//!
//! Window is implicit 30d. `limit` omitted or `5` only; `window` omitted or `30d` only.
//! `from` / `to` / `sort` / `ticker` / other known-bad keys → **400**. Extra junk query
//! keys are ignored (60s cache is a single slot). Reads `pair_volume_30d` +
//! `pair_liquidity_usd` — never `swap_events` / `pair_reserves`. Gems use
//! `COLUMBUS5_GEM_ADDRESSES`. Ratio is server-side; TVL missing/`≤0`/overflow → JSON `null`.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::pairs::{bd_plain_string, AssetBrief};
use super::{internal_err, AppState};
use crate::db::queries::protocol_top_pairs as top_q;
use crate::indexer::defillama::gem_addresses_lowercased;

const CACHE_TTL: Duration = Duration::from_secs(60);
const TOP_PAIRS_LIMIT: usize = 5;

fn overflow_cap() -> BigDecimal {
    "100000000000000000000"
        .parse()
        .expect("10^20 is a valid BigDecimal")
}

fn top_pairs_cache() -> &'static Mutex<Option<(ProtocolTopPairsResponse, Instant)>> {
    static CACHE: OnceLock<Mutex<Option<(ProtocolTopPairsResponse, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Drop the 60s top-pairs response cache (tests).
pub fn reset_protocol_top_pairs_cache() {
    if let Ok(mut guard) = top_pairs_cache().lock() {
        *guard = None;
    }
}

#[derive(Debug, Deserialize)]
pub struct ProtocolTopPairsQuery {
    pub limit: Option<String>,
    pub window: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub sort: Option<String>,
    pub ticker: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolTopPairItem {
    pub pair_address: String,
    pub asset_0: AssetBrief,
    pub asset_1: AssetBrief,
    pub volume_usd_30d: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub liquidity_usd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_per_tvl: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
pub struct ProtocolTopPairsResponse {
    pub items: Vec<ProtocolTopPairItem>,
}

fn bad_request(msg: &'static str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

fn reject_disallowed(q: &ProtocolTopPairsQuery) -> Result<(), (StatusCode, String)> {
    if q.from.is_some() || q.to.is_some() {
        return Err(bad_request("from/to are not allowed"));
    }
    if q.sort.is_some() {
        return Err(bad_request("sort is not allowed"));
    }
    if q.ticker.is_some() {
        return Err(bad_request("ticker is not allowed"));
    }
    match q.window.as_deref() {
        None | Some("30d") => {}
        _ => return Err(bad_request("Invalid window, expected 30d")),
    }
    match q.limit.as_deref() {
        None | Some("5") => {}
        _ => return Err(bad_request("Invalid limit, expected 5")),
    }
    Ok(())
}

/// Trailing 30d USD ÷ current v2 LP USD. Missing / ≤0 / overflow TVL → `None` (never Inf).
pub fn compute_volume_per_tvl(volume: &BigDecimal, liquidity: Option<&BigDecimal>) -> Option<String> {
    let tvl = liquidity?;
    let zero = BigDecimal::from(0);
    let cap = overflow_cap();
    if tvl <= &zero || tvl >= &cap {
        return None;
    }
    let ratio = volume / tvl;
    if ratio <= zero || ratio >= cap {
        return None;
    }
    Some(bd_plain_string(&ratio))
}

fn row_to_item(row: top_q::ProtocolTopPairRow) -> ProtocolTopPairItem {
    let liquidity_usd = row.liquidity_usd.as_ref().and_then(|v| {
        if *v <= BigDecimal::from(0) || *v >= overflow_cap() {
            None
        } else {
            Some(bd_plain_string(v))
        }
    });
    let volume_per_tvl = compute_volume_per_tvl(&row.volume_usd_30d, row.liquidity_usd.as_ref());
    ProtocolTopPairItem {
        pair_address: row.pair_address,
        asset_0: AssetBrief {
            symbol: row.asset_0_symbol,
            contract_addr: row.asset_0_contract,
            denom: row.asset_0_denom,
            decimals: row.asset_0_decimals,
        },
        asset_1: AssetBrief {
            symbol: row.asset_1_symbol,
            contract_addr: row.asset_1_contract,
            denom: row.asset_1_denom,
            decimals: row.asset_1_decimals,
        },
        volume_usd_30d: bd_plain_string(&row.volume_usd_30d),
        liquidity_usd,
        volume_per_tvl,
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/protocol/top-pairs",
    params(
        ("limit" = Option<String>, Query, description = "Must be omitted or 5"),
        ("window" = Option<String>, Query, description = "Must be omitted or 30d")
    ),
    responses(
        (status = 200, description = "Top-5 factory pairs by trailing 30d USD volume", body = ProtocolTopPairsResponse),
        (status = 400, description = "Invalid limit / window / disallowed query"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_protocol_top_pairs(
    State(state): State<AppState>,
    Query(q): Query<ProtocolTopPairsQuery>,
) -> Result<Json<ProtocolTopPairsResponse>, (StatusCode, String)> {
    reject_disallowed(&q)?;

    if let Ok(guard) = top_pairs_cache().lock() {
        if let Some((resp, at)) = guard.as_ref() {
            if Instant::now().duration_since(*at) <= CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let gems = gem_addresses_lowercased();
    let rows = top_q::list_top_pairs_30d(&state.pool, &gems)
        .await
        .map_err(internal_err)?;
    let mut items: Vec<ProtocolTopPairItem> = rows.into_iter().map(row_to_item).collect();
    items.truncate(TOP_PAIRS_LIMIT);
    let resp = ProtocolTopPairsResponse { items };

    if let Ok(mut guard) = top_pairs_cache().lock() {
        *guard = Some((resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}

#[cfg(test)]
mod volume_per_tvl_tests {
    use super::*;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn ratio_is_volume_over_current_tvl() {
        let out = compute_volume_per_tvl(&bd("1234.5"), Some(&bd("500"))).expect("ratio");
        assert_eq!(out, "2.469");
    }

    #[test]
    fn missing_zero_negative_overflow_tvl_are_null() {
        assert!(compute_volume_per_tvl(&bd("10"), None).is_none());
        assert!(compute_volume_per_tvl(&bd("10"), Some(&bd("0"))).is_none());
        assert!(compute_volume_per_tvl(&bd("10"), Some(&bd("-1"))).is_none());
        assert!(compute_volume_per_tvl(&bd("10"), Some(&bd("1e20"))).is_none());
        assert!(compute_volume_per_tvl(&bd("10"), Some(&bd("1e21"))).is_none());
    }
}
