use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::BigDecimal;
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

use super::{internal_err, AppState};
use crate::db::queries::{assets, pairs, volume};

// GitLab #281 / #333: /overview reads materialized global_stats_24h on cache miss;
// cache the whole response for 1 minute so a request burst can't hammer the DB.
const OVERVIEW_CACHE_TTL: Duration = Duration::from_secs(60);

fn overview_cache() -> &'static Mutex<Option<(OverviewResponse, Instant)>> {
    static CACHE: OnceLock<Mutex<Option<(OverviewResponse, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[derive(Serialize, ToSchema, Clone)]
pub struct OverviewResponse {
    pub total_volume_24h: String,
    /// Human USD 24h volume from catalog-priced swaps. JSON `null` when there is
    /// 24h trade activity but no priced USD (unpriced / oracle down) — not `"0"`.
    /// `"0"` only when `total_trades_24h` is 0 (GitLab #548 **C3**).
    pub total_volume_24h_usd: Option<String>,
    pub total_trades_24h: i64,
    pub pair_count: i64,
    pub token_count: i64,
    pub ustc_price_usd: Option<String>,
    /// SUM(volume_usd) 7d rollup (P522-Q catalog; GitLab #550).
    pub total_volume_7d_usd: String,
    /// SUM(volume_usd) 30d rollup (P522-Q catalog; GitLab #550).
    pub total_volume_30d_usd: String,
    pub total_trades_7d: i64,
    pub total_trades_30d: i64,
    /// Indexer first-seen `assets.created_at` in last 30d (not on-chain genesis).
    pub tokens_added_30d: i64,
    /// Indexer first-seen `pairs.created_at` in last 30d (not on-chain genesis).
    pub pairs_added_30d: i64,
    /// Distinct pairs with ≥1 swap in last 24h (materialized). Dust swaps count.
    pub active_pairs_24h: i64,
    /// Distinct swap senders in last 24h (materialized).
    pub unique_traders_24h: i64,
}

/// Map rollup USD + trade count to the overview JSON contract (#548).
pub fn overview_volume_usd_field(trades: i64, usd: &BigDecimal) -> Option<String> {
    if trades <= 0 {
        return Some("0".to_string());
    }
    if usd <= &BigDecimal::from(0) {
        return None;
    }
    Some(usd.to_string())
}

#[utoipa::path(
    get,
    path = "/api/v1/overview",
    responses(
        (status = 200, description = "Global DEX statistics", body = OverviewResponse),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Overview"
)]
pub async fn get_overview(
    State(state): State<AppState>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    if let Ok(guard) = overview_cache().lock() {
        if let Some((resp, at)) = guard.as_ref() {
            if Instant::now().duration_since(*at) <= OVERVIEW_CACHE_TTL {
                return Ok(Json(resp.clone()));
            }
        }
    }

    let global = volume::get_global_stats(&state.pool)
        .await
        .map_err(internal_err)?;

    let cutoff_30d = Utc::now() - chrono::Duration::days(30);
    let token_count = assets::count_pair_leg_assets(&state.pool)
        .await
        .map_err(internal_err)?;
    let tokens_added_30d = assets::count_assets_created_since(&state.pool, cutoff_30d)
        .await
        .map_err(internal_err)?;
    let pairs_added_30d = pairs::count_pairs_created_since(&state.pool, cutoff_30d)
        .await
        .map_err(internal_err)?;

    let ustc_price = state.oracle_prices.ustc.read().await.clone();

    let resp = OverviewResponse {
        total_volume_24h: global.total_volume_24h.to_string(),
        total_volume_24h_usd: overview_volume_usd_field(
            global.total_trades_24h,
            &global.total_volume_24h_usd,
        ),
        total_trades_24h: global.total_trades_24h,
        pair_count: global.pair_count,
        token_count,
        ustc_price_usd: ustc_price.map(|p| p.to_string()),
        total_volume_7d_usd: global.total_volume_7d_usd.to_string(),
        total_volume_30d_usd: global.total_volume_30d_usd.to_string(),
        total_trades_7d: global.total_trades_7d,
        total_trades_30d: global.total_trades_30d,
        tokens_added_30d,
        pairs_added_30d,
        active_pairs_24h: global.active_pairs_24h,
        unique_traders_24h: global.unique_traders_24h,
    };

    if let Ok(mut guard) = overview_cache().lock() {
        *guard = Some((resp.clone(), Instant::now()));
    }

    Ok(Json(resp))
}

/// Drop the 60s `/overview` response cache (tests + operator cache bust).
pub fn reset_overview_cache() {
    if let Ok(mut guard) = overview_cache().lock() {
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::overview_volume_usd_field;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn idle_dex_sends_zero_usd_string() {
        assert_eq!(overview_volume_usd_field(0, &bd("0")), Some("0".to_string()));
        assert_eq!(overview_volume_usd_field(0, &bd("12.5")), Some("0".to_string()));
    }

    #[test]
    fn unpriced_activity_sends_json_null() {
        assert_eq!(overview_volume_usd_field(4, &bd("0")), None);
        assert_eq!(overview_volume_usd_field(1, &bd("-1")), None);
    }

    #[test]
    fn priced_activity_sends_decimal_string() {
        assert_eq!(
            overview_volume_usd_field(4, &bd("1234.56")),
            Some("1234.56".to_string())
        );
    }
}
