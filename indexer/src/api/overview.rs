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
use crate::db::queries::{assets, hub_prices, pairs, volume};
use crate::indexer::hub_usd::HubTicker;
use crate::indexer::protocol_fees::overview_fee_usd_field;

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
    /// DEX hub USD of 1 human cUSTC (wrap = USTC oracle). Null when unresolved (#556).
    pub custc_price_usd: Option<String>,
    /// Hub spot values are advisory and must not drive automated financial actions (#1391).
    pub hub_oracle_grade: bool,
    pub hub_price_policy: String,
    /// Each live USD mark includes source, original timestamp and confidence policy.
    pub hub_price_quality: Vec<super::hub_prices::HubPriceEntry>,
    /// DEX hub USD of 1 human UST1 from the deepest cUSTC/UST1 pool. Null when unresolved.
    pub ust1_price_usd: Option<String>,
    /// DEX hub USD of 1 human USTR from the deepest vs cUSTC or UST1. Null when unresolved.
    pub ustr_price_usd: Option<String>,
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
    /// Humanized USD TVL of factory reserves. Partial is a labeled lower bound; unknown is null.
    /// Not volume, not CG `liquidity_in_usd`, not book escrow.
    pub total_liquidity_usd: Option<String>,
    /// fresh / verified_empty / partial / stale / unavailable / unknown.
    pub liquidity_quality: String,
    pub liquidity_as_of: Option<String>,
    pub liquidity_coverage_identity: String,
    pub liquidity_generation: i64,
    pub liquidity_reasons: serde_json::Value,
    pub liquidity_max_age_seconds: u64,
    /// Signed percent vs ~24h snapshot. JSON `null` if no baseline / then=0 / overflow.
    pub liquidity_change_24h_pct: Option<String>,
    /// Signed percent vs ~30d snapshot. JSON `null` if no baseline / then=0 / overflow.
    pub liquidity_change_30d_pct: Option<String>,
    /// Pairs included in `total_liquidity_usd` this refresh (operator; not a Protocol headline).
    pub priced_pair_count: i64,
    pub unpriced_pair_count: i64,
    pub total_liquidity_usd_24h_ago: Option<String>,
    pub total_liquidity_usd_30d_ago: Option<String>,
    /// Trailing 24h treasury fees USD (GitLab #586). `"0"` idle; `null` activity+unpriced.
    pub total_fees_24h_usd: Option<String>,
    pub total_fees_7d_usd: Option<String>,
    pub total_fees_30d_usd: Option<String>,
    /// Flow Δ% vs prior equal window. `null` if no baseline / then≤0.
    pub fees_change_24h_pct: Option<String>,
    pub fees_change_7d_pct: Option<String>,
    pub fees_change_30d_pct: Option<String>,
    /// Flow Δ% vs prior equal trailing window (GitLab #652). Additive; `null` until 2×W fills.
    pub volume_change_24h_pct: Option<String>,
    pub volume_change_7d_pct: Option<String>,
    pub volume_change_30d_pct: Option<String>,
}

/// Qualify cached and newly built values without changing their observation time.
fn qualify_liquidity_age(resp: &mut OverviewResponse) {
    let now = Utc::now();
    let current = resp
        .liquidity_as_of
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .is_some_and(|at| {
            at <= now
                && now.signed_duration_since(at)
                    <= chrono::Duration::seconds(resp.liquidity_max_age_seconds as i64)
        });
    if matches!(
        resp.liquidity_quality.as_str(),
        "fresh" | "partial" | "verified_empty"
    ) && !current
    {
        resp.liquidity_quality = "stale".into();
    }
    if !matches!(resp.liquidity_quality.as_str(), "fresh" | "verified_empty") {
        resp.liquidity_change_24h_pct = None;
        resp.liquidity_change_30d_pct = None;
    }
    if !matches!(
        resp.liquidity_quality.as_str(),
        "fresh" | "verified_empty" | "partial"
    ) {
        resp.total_liquidity_usd = None;
    }
}

async fn qualify_live_prices(
    resp: &mut OverviewResponse,
    state: &AppState,
) -> Result<(), (StatusCode, String)> {
    resp.ustc_price_usd = crate::db::queries::oracle::get_latest_average_price(
        &state.pool,
        crate::indexer::oracle::OracleTicker::Ustc,
    )
    .await
    .map_err(internal_err)?
    .map(|v| v.to_string());
    let mut rows = hub_prices::get_all_hub_prices(&state.pool)
        .await
        .map_err(internal_err)?;
    if state
        .oracle_prices
        .invalidates(crate::indexer::oracle::OracleTicker::Ustc, None)
        .await
    {
        resp.ustc_price_usd = None;
    }
    super::hub_prices::qualify_process_prices(&mut rows, &state).await;
    let references = hub_prices::get_hub_references(&state.pool)
        .await
        .map_err(internal_err)?;
    resp.hub_price_quality = [
        HubTicker::Custc,
        HubTicker::Lunc,
        HubTicker::Ust1,
        HubTicker::Ustr,
    ]
    .into_iter()
    .map(|ticker| {
        super::hub_prices::entry_for(
            ticker,
            rows.iter().find(|r| r.ticker == ticker.as_str()),
            &state.hub_usd,
            references.iter().find(|r| r.ticker == ticker.as_str()),
        )
    })
    .collect();
    let price = |ticker: &str| {
        resp.hub_price_quality
            .iter()
            .find(|r| r.ticker == ticker)
            .and_then(|r| r.price_usd.clone())
    };
    resp.custc_price_usd = price("custc");
    resp.ust1_price_usd = price("ust1");
    resp.ustr_price_usd = price("ustr");
    Ok(())
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
    let cached = overview_cache().lock().ok().and_then(|guard| {
        guard
            .as_ref()
            .filter(|(_, at)| Instant::now().duration_since(*at) <= OVERVIEW_CACHE_TTL)
            .map(|(resp, _)| resp.clone())
    });
    if let Some(mut cached) = cached {
        qualify_liquidity_age(&mut cached);
        qualify_live_prices(&mut cached, &state).await?;
        return Ok(Json(cached));
    }

    let global = volume::get_global_stats(&state.pool)
        .await
        .map_err(internal_err)?;

    let liquidity_max_age_seconds = if global.liquidity_quality.status == "verified_empty" {
        crate::indexer::protocol_tvl::FACTORY_INVENTORY_MAX_AGE_SECONDS as u64
    } else {
        crate::indexer::protocol_tvl::max_staleness_from_env().as_secs()
    };
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
    let hub_rows = hub_prices::get_all_hub_prices(&state.pool)
        .await
        .map_err(internal_err)?;
    let hub_usd = |ticker: HubTicker| {
        hub_rows
            .iter()
            .find(|r| r.ticker == ticker.as_str() && hub_prices::row_usable(r, Utc::now()))
            .map(|r| r.price_usd.to_string())
    };

    let mut resp = OverviewResponse {
        total_volume_24h: global.total_volume_24h.to_string(),
        total_volume_24h_usd: overview_volume_usd_field(
            global.total_trades_24h,
            &global.total_volume_24h_usd,
        ),
        total_trades_24h: global.total_trades_24h,
        pair_count: global.pair_count,
        token_count,
        ustc_price_usd: ustc_price.map(|p| p.to_string()),
        custc_price_usd: hub_usd(HubTicker::Custc),
        hub_oracle_grade: false,
        hub_price_policy: crate::indexer::hub_guard::HUB_POLICY.into(),
        hub_price_quality: Vec::new(),
        ust1_price_usd: hub_usd(HubTicker::Ust1),
        ustr_price_usd: hub_usd(HubTicker::Ustr),
        total_volume_7d_usd: global.total_volume_7d_usd.to_string(),
        total_volume_30d_usd: global.total_volume_30d_usd.to_string(),
        total_trades_7d: global.total_trades_7d,
        total_trades_30d: global.total_trades_30d,
        tokens_added_30d,
        pairs_added_30d,
        active_pairs_24h: global.active_pairs_24h,
        unique_traders_24h: global.unique_traders_24h,
        total_liquidity_usd: global.total_liquidity_usd.map(|v| v.to_string()),
        liquidity_quality: global.liquidity_quality.status,
        liquidity_as_of: global.liquidity_quality.as_of.map(|v| v.to_rfc3339()),
        liquidity_coverage_identity: global.liquidity_quality.coverage_identity,
        liquidity_generation: global.liquidity_quality.generation,
        liquidity_reasons: global.liquidity_quality.reasons,
        liquidity_max_age_seconds,
        liquidity_change_24h_pct: global.liquidity_change_24h_pct.map(|p| p.to_string()),
        liquidity_change_30d_pct: global.liquidity_change_30d_pct.map(|p| p.to_string()),
        priced_pair_count: i64::from(global.priced_pair_count),
        unpriced_pair_count: i64::from(global.unpriced_pair_count),
        total_liquidity_usd_24h_ago: global.total_liquidity_usd_24h_ago.map(|v| v.to_string()),
        total_liquidity_usd_30d_ago: global.total_liquidity_usd_30d_ago.map(|v| v.to_string()),
        total_fees_24h_usd: overview_fee_usd_field(
            global.fee_event_count_24h,
            global.total_fees_24h_usd.as_ref(),
        ),
        total_fees_7d_usd: overview_fee_usd_field(
            global.fee_event_count_7d,
            global.total_fees_7d_usd.as_ref(),
        ),
        total_fees_30d_usd: overview_fee_usd_field(
            global.fee_event_count_30d,
            global.total_fees_30d_usd.as_ref(),
        ),
        fees_change_24h_pct: global.fees_change_24h_pct.map(|p| p.to_string()),
        fees_change_7d_pct: global.fees_change_7d_pct.map(|p| p.to_string()),
        fees_change_30d_pct: global.fees_change_30d_pct.map(|p| p.to_string()),
        volume_change_24h_pct: global.volume_change_24h_pct.map(|p| p.to_string()),
        volume_change_7d_pct: global.volume_change_7d_pct.map(|p| p.to_string()),
        volume_change_30d_pct: global.volume_change_30d_pct.map(|p| p.to_string()),
    };

    qualify_liquidity_age(&mut resp);
    qualify_live_prices(&mut resp, &state).await?;
    let newer = if let Ok(mut guard) = overview_cache().lock() {
        if let Some((current, _)) = guard
            .as_ref()
            .filter(|(current, _)| current.liquidity_generation > resp.liquidity_generation)
        {
            Some(current.clone())
        } else {
            *guard = Some((resp.clone(), Instant::now()));
            None
        }
    } else {
        None
    };
    if let Some(mut newer) = newer {
        qualify_liquidity_age(&mut newer);
        qualify_live_prices(&mut newer, &state).await?;
        return Ok(Json(newer));
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
        assert_eq!(
            overview_volume_usd_field(0, &bd("0")),
            Some("0".to_string())
        );
        assert_eq!(
            overview_volume_usd_field(0, &bd("12.5")),
            Some("0".to_string())
        );
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
