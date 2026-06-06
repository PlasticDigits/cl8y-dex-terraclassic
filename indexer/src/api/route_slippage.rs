//! Route-based slippage from best-route token reference prices (GitLab #293).
//!
//! Token spot prices are derived by solving **1 whole token → quote asset** via global best
//! execution. Swap slippage compares the quoted `estimated_amount_out` to the fair cross-rate
//! implied by those prices (symmetric deviation — flags both worse fills and suspiciously good
//! multi-hop arb quotes on lopsided LocalTerra topology).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::api::best_execution::solve_global_best_execution_inner;
use crate::api::hybrid_route_opt::QuoteTrader;
use crate::api::route_solver::{cache_key_maker_fills, resolve_discount_bps, RouteSolveResponse};
use crate::api::AppState;
use crate::db::queries::assets;

const PRICE_CACHE_TTL: Duration = Duration::from_secs(30);
const QUOTE_SYMBOLS: &[&str] = &["USTC-C", "USTC", "LUNC-C", "LUNC"];

#[derive(Clone, Copy)]
struct PriceCacheEntry {
    at: Instant,
    price_quote: f64,
}

static PRICE_CACHE: OnceLock<Mutex<HashMap<String, PriceCacheEntry>>> = OnceLock::new();

fn price_cache() -> &'static Mutex<HashMap<String, PriceCacheEntry>> {
    PRICE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn price_cache_get(key: &str) -> Option<f64> {
    let g = price_cache().lock().ok()?;
    let entry = g.get(key)?;
    if entry.at.elapsed() > PRICE_CACHE_TTL {
        return None;
    }
    Some(entry.price_quote)
}

fn price_cache_set(key: &str, price: f64) {
    let Ok(mut g) = price_cache().lock() else {
        return;
    };
    let now = Instant::now();
    g.retain(|_, v| now.duration_since(v.at) <= PRICE_CACHE_TTL);
    g.insert(
        key.to_string(),
        PriceCacheEntry {
            at: now,
            price_quote: price,
        },
    );
}

/// Symmetric slippage vs a reference output: `100 × (1 − min(actual, expected) / max(actual, expected))`.
pub fn symmetric_slippage_percent(actual_out: f64, expected_out: f64) -> f64 {
    if !actual_out.is_finite() || !expected_out.is_finite() || actual_out <= 0.0 || expected_out <= 0.0 {
        return 0.0;
    }
    let (lo, hi) = if actual_out < expected_out {
        (actual_out, expected_out)
    } else {
        (expected_out, actual_out)
    };
    (1.0 - lo / hi) * 100.0
}

/// Human-scale output at the cross-rate implied by quote-denominated token prices.
pub fn expected_output_human(
    amount_in_raw: u128,
    decimals_in: i16,
    price_in_quote: f64,
    price_out_quote: f64,
) -> f64 {
    if price_out_quote <= 0.0 || !price_in_quote.is_finite() || !price_out_quote.is_finite() {
        return 0.0;
    }
    let scale_in = 10_f64.powi(decimals_in as i32);
    let amount_in_human = amount_in_raw as f64 / scale_in;
    amount_in_human * price_in_quote / price_out_quote
}

fn human_to_raw(human: f64, decimals: i16) -> u128 {
    let scale = 10_f64.powi(decimals as i32);
    (human * scale).round().max(0.0) as u128
}

fn asset_decimals(all: &[assets::AssetRow], addr: &str) -> Option<i16> {
    let key = addr.trim();
    all.iter()
        .find(|a| {
            a.contract_address
                .as_ref()
                .is_some_and(|c| c.eq_ignore_ascii_case(key))
        })
        .map(|a| a.decimals)
}

/// Prefer wrapped USTC, then LUNC, for the quote leg of token valuation.
pub fn find_quote_token(all: &[assets::AssetRow]) -> Option<(String, i16)> {
    for sym in QUOTE_SYMBOLS {
        if let Some(a) = all.iter().find(|a| a.symbol.eq_ignore_ascii_case(sym)) {
            if let Some(addr) = a.contract_address.as_ref().filter(|s| !s.is_empty()) {
                return Some((addr.clone(), a.decimals));
            }
        }
    }
    for a in all {
        if let Some(d) = a.denom.as_ref() {
            if d == "uusd" || d == "uluna" {
                if let Some(addr) = a.contract_address.as_ref().filter(|s| !s.is_empty()) {
                    return Some((addr.clone(), a.decimals));
                }
            }
        }
    }
    None
}

async fn token_price_in_quote(
    state: &AppState,
    token_addr: &str,
    token_decimals: i16,
    quote_addr: &str,
    quote_decimals: i16,
    quote_trader: &QuoteTrader,
    max_maker_fills: u32,
) -> Option<f64> {
    if token_addr.eq_ignore_ascii_case(quote_addr) {
        return Some(1.0);
    }

    let discount_bps = resolve_discount_bps(state, quote_trader).await;
    let maker_key = cache_key_maker_fills(max_maker_fills);
    let cache_key = format!("{token_addr}->{quote_addr}|d{discount_bps}|m{maker_key}");
    if let Some(price) = price_cache_get(&cache_key) {
        return Some(price);
    }

    let one_unit = 10_u128.checked_pow(token_decimals as u32)?;
    if one_unit == 0 {
        return None;
    }
    let amount_raw = one_unit.to_string();
    let max_makers = max_maker_fills.max(1);

    let (body, _) = Box::pin(solve_global_best_execution_inner(
        state,
        token_addr,
        quote_addr,
        one_unit,
        &amount_raw,
        max_makers,
        quote_trader,
        false,
    ))
    .await
    .ok()?;

    let out_raw = body.estimated_amount_out?.parse::<u128>().ok()?;
    if out_raw == 0 {
        return None;
    }
    let quote_scale = 10_f64.powi(quote_decimals as i32);
    let price = out_raw as f64 / quote_scale;
    if price <= 0.0 || !price.is_finite() {
        return None;
    }
    price_cache_set(&cache_key, price);
    Some(price)
}

/// Attach `spot_amount_out`, `slippage_percent`, and per-token quote prices when computable.
///
/// When `estimated_for_amount_in` is set and differs from `amount_in_raw` (hybrid GET cache hit
/// within the same amount bucket), `estimated_amount_out` is scaled linearly for slippage only.
pub async fn enrich_route_slippage(
    state: &AppState,
    body: &mut RouteSolveResponse,
    amount_in_raw: &str,
    quote_trader: &QuoteTrader,
    max_maker_fills: u32,
    estimated_for_amount_in: Option<u128>,
) {
    let Ok(amount_u) = amount_in_raw.parse::<u128>() else {
        return;
    };
    if amount_u == 0 {
        return;
    }
    let Some(mut out_raw) = body
        .estimated_amount_out
        .as_ref()
        .and_then(|s| s.parse::<u128>().ok())
    else {
        return;
    };
    if out_raw == 0 {
        return;
    }
    if let Some(solve_amt) = estimated_for_amount_in {
        if solve_amt > 0 && solve_amt != amount_u {
            out_raw = out_raw.saturating_mul(amount_u) / solve_amt;
        }
    }

    let Ok(all_assets) = assets::get_all_assets(&state.pool).await else {
        return;
    };
    let Some((quote_addr, quote_decimals)) = find_quote_token(&all_assets) else {
        return;
    };

    let Some(dec_in) = asset_decimals(&all_assets, &body.token_in) else {
        return;
    };
    let Some(dec_out) = asset_decimals(&all_assets, &body.token_out) else {
        return;
    };

    let max_makers = max_maker_fills.max(1);
    let Some(price_in) = token_price_in_quote(
        state,
        &body.token_in,
        dec_in,
        &quote_addr,
        quote_decimals,
        quote_trader,
        max_makers,
    )
    .await
    else {
        return;
    };
    let Some(price_out) = token_price_in_quote(
        state,
        &body.token_out,
        dec_out,
        &quote_addr,
        quote_decimals,
        quote_trader,
        max_makers,
    )
    .await
    else {
        return;
    };

    let expected_human = expected_output_human(amount_u, dec_in, price_in, price_out);
    if expected_human <= 0.0 {
        return;
    }

    let scale_out = 10_f64.powi(dec_out as i32);
    let actual_human = out_raw as f64 / scale_out;
    let slip = symmetric_slippage_percent(actual_human, expected_human);
    let spot_out_raw = human_to_raw(expected_human, dec_out);

    body.spot_amount_out = Some(spot_out_raw.to_string());
    body.slippage_percent = Some(format!("{slip:.2}"));
    body.token_in_price_quote = Some(format!("{price_in:.18}"));
    body.token_out_price_quote = Some(format!("{price_out:.18}"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_slippage_worse_fill() {
        let slip = symmetric_slippage_percent(0.97, 1.0);
        assert!((slip - 3.0).abs() < 0.01);
    }

    #[test]
    fn symmetric_slippage_suspiciously_good() {
        // EMBER→CORAL style: spot ~0.96, quote ~36000
        let slip = symmetric_slippage_percent(36_000.0, 0.96);
        assert!(slip > 99.0);
    }

    #[test]
    fn expected_output_from_prices() {
        let out = expected_output_human(1_000_000, 6, 2.0, 1.0);
        assert!((out - 2.0).abs() < 1e-9);
    }
}
