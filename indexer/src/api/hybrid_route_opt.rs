//! Per-hop hybrid split search using pair `HybridSimulation` or Postgres mirror (Phase 1c).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::api::db_orderbook_sim::{self, HopMirror, MirrorFreshness, MirrorLoadMeta};
use crate::lcd::LcdClient;

/// Optional wallet forwarded to pair `HybridSimulation` / router sim for CL8Y fee-tier parity (GitLab #245).
#[derive(Clone, Debug, Default)]
pub struct QuoteTrader {
    pub trader: Option<String>,
    pub sender: Option<String>,
}

/// Hybrid parameters for one hop (matches on-chain `HybridSwapParams`; amounts as decimal strings). See `docs/route-solver.md#glossary`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct HybridHopJson {
    /// Offer amount routed to the constant-product pool leg (raw integer string).
    pub pool_input: String,
    /// Offer amount routed to the limit book leg (raw integer string); `pool_input + book_input` = hop offer.
    pub book_input: String,
    /// Maximum maker orders to match on the book leg for this hop.
    pub max_maker_fills: u32,
    /// Optional on-chain book walk start hint (advanced; usually omitted).
    #[serde(default)]
    pub book_start_hint: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct HopDescriptor {
    pub pair: String,
    pub offer_token: String,
    /// Output token for this hop (used by callers for path display / debugging).
    #[allow(dead_code)]
    pub ask_token: String,
}

#[derive(Clone, Debug, Default)]
pub struct OptimizationMeta {
    /// A hop used pool-only because grid / mirror sim failed for all split candidates.
    pub degraded: bool,
    /// At least one hop has a non-zero book leg in the chosen params.
    pub any_book_leg: bool,
    /// Mirror was stale on at least one hop (LCD fallback may have been used).
    pub mirror_stale: bool,
    /// Mirror reserves missing on at least one hop.
    pub mirror_missing: bool,
}

/// Hybrid grid pricing source: LCD `HybridSimulation` (legacy) or Postgres mirror (#319).
pub enum HybridSimSource<'a> {
    Lcd(&'a LcdClient),
    Db {
        lcd_fallback: &'a LcdClient,
        mirrors: &'a HashMap<String, HopMirror>,
        discount_bps: u16,
    },
}

#[derive(Debug)]
pub enum HybridSimError {
    Lcd(crate::lcd::LcdError),
    Db(db_orderbook_sim::DbSimError),
    /// Hop cannot be simulated (e.g. zero-reserve pool with no viable book leg).
    PathUnusable,
}

fn is_infra_lcd_error(e: &crate::lcd::LcdError) -> bool {
    matches!(
        e,
        crate::lcd::LcdError::AllEndpointsFailed(_) | crate::lcd::LcdError::Request(_)
    )
}

impl From<crate::lcd::LcdError> for HybridSimError {
    fn from(e: crate::lcd::LcdError) -> Self {
        HybridSimError::Lcd(e)
    }
}

impl From<db_orderbook_sim::DbSimError> for HybridSimError {
    fn from(e: db_orderbook_sim::DbSimError) -> Self {
        HybridSimError::Db(e)
    }
}

fn is_empty_pool_err(e: &HybridSimError) -> bool {
    matches!(
        e,
        HybridSimError::Db(db_orderbook_sim::DbSimError::InsufficientLiquidity)
    )
}

async fn pool_only_or_zero(
    source: &HybridSimSource<'_>,
    mirror_meta: Option<&mut MirrorLoadMeta>,
    hop: &HopDescriptor,
    offer_amount: u128,
    max_maker_fills: u32,
    quote_trader: &QuoteTrader,
) -> Result<u128, HybridSimError> {
    match query_pool_only_unified(
        source,
        mirror_meta,
        hop,
        offer_amount,
        max_maker_fills,
        quote_trader,
    )
    .await
    {
        Ok(v) => Ok(v),
        Err(e) if is_empty_pool_err(&e) => Ok(0),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
struct HybridSimResp {
    return_amount: String,
}

const GRID_POINTS: u32 = 17;

fn asset_info_token(contract: &str) -> serde_json::Value {
    json!({ "token": { "contract_addr": contract } })
}

fn hybrid_sim_query(
    offer_token: &str,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    quote_trader: &QuoteTrader,
) -> serde_json::Value {
    let hint_json = match book_start_hint {
        Some(h) => json!(h),
        None => serde_json::Value::Null,
    };
    let mut sim = json!({
        "offer_asset": {
            "info": asset_info_token(offer_token),
            "amount": offer_amount.to_string(),
        },
        "hybrid": {
            "pool_input": pool_input.to_string(),
            "book_input": book_input.to_string(),
            "max_maker_fills": max_maker_fills,
            "book_start_hint": hint_json,
        }
    });
    if let Some(trader) = quote_trader.trader.as_deref() {
        sim["trader"] = json!(trader);
    }
    if let Some(sender) = quote_trader.sender.as_deref() {
        sim["sender"] = json!(sender);
    }
    json!({ "hybrid_simulation": sim })
}

fn resolve_hop_book_start_hint(source: &HybridSimSource<'_>, hop: &HopDescriptor) -> Option<u64> {
    match source {
        HybridSimSource::Lcd(_) => None,
        HybridSimSource::Db { mirrors, .. } => mirrors
            .get(&hop.pair)
            .and_then(|m| db_orderbook_sim::first_live_book_start_hint(m, &hop.offer_token)),
    }
}

async fn query_hybrid_sim_lcd(
    lcd: &LcdClient,
    pair: &str,
    offer_token: &str,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    quote_trader: &QuoteTrader,
) -> Result<u128, crate::lcd::LcdError> {
    let hint = if book_input > 0 {
        book_start_hint
    } else {
        None
    };
    let q = hybrid_sim_query(
        offer_token,
        offer_amount,
        pool_input,
        book_input,
        max_maker_fills,
        hint,
        quote_trader,
    );
    let r: HybridSimResp = lcd.query_contract(pair, &q).await?;
    r.return_amount
        .parse::<u128>()
        .map_err(|e| crate::lcd::LcdError::Deserialize(format!("return_amount: {}", e)))
}

async fn query_hybrid_sim_unified(
    source: &HybridSimSource<'_>,
    mut mirror_meta: Option<&mut MirrorLoadMeta>,
    hop: &HopDescriptor,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
    book_start_hint: Option<u64>,
    quote_trader: &QuoteTrader,
) -> Result<u128, HybridSimError> {
    let hint = if book_input > 0 {
        book_start_hint
    } else {
        None
    };
    match source {
        HybridSimSource::Lcd(lcd) => query_hybrid_sim_lcd(
            lcd,
            &hop.pair,
            &hop.offer_token,
            offer_amount,
            pool_input,
            book_input,
            max_maker_fills,
            hint,
            quote_trader,
        )
        .await
        .map_err(HybridSimError::from),
        HybridSimSource::Db {
            lcd_fallback,
            mirrors,
            discount_bps,
        } => {
            let mirror_meta = mirror_meta
                .as_mut()
                .expect("db hybrid requires mirror_meta");
            let mirror = mirrors.get(&hop.pair);
            if let Some(m) = mirror {
                if db_orderbook_sim::pool_reserves_unusable(m.reserve_0, m.reserve_1)
                    && m.freshness != MirrorFreshness::MissingReserves
                {
                    mirror_meta.mirror_missing_hops =
                        mirror_meta.mirror_missing_hops.saturating_add(1);
                    return Ok(0);
                }
                if m.freshness == MirrorFreshness::Fresh {
                    mirror_meta.db_hybrid_queries = mirror_meta.db_hybrid_queries.saturating_add(1);
                    mirror_meta.max_snapshot_age_ms = mirror_meta
                        .max_snapshot_age_ms
                        .max(db_orderbook_sim::snapshot_age_ms(m.snapshot_at));
                    match db_orderbook_sim::simulate_hybrid_from_mirror(
                        m,
                        &hop.offer_token,
                        offer_amount,
                        pool_input,
                        book_input,
                        max_maker_fills,
                        *discount_bps,
                        hint,
                    ) {
                        Ok(v) => return Ok(v),
                        Err(db_orderbook_sim::DbSimError::InsufficientLiquidity) => {
                            mirror_meta.mirror_missing_hops =
                                mirror_meta.mirror_missing_hops.saturating_add(1);
                            tracing::debug!(
                                pair = %hop.pair,
                                "mirror zero pool reserves — LCD fallback for hop"
                            );
                        }
                        Err(e) => return Err(HybridSimError::from(e)),
                    }
                } else if m.freshness == MirrorFreshness::Stale {
                    mirror_meta.mirror_stale_hops = mirror_meta.mirror_stale_hops.saturating_add(1);
                    tracing::warn!(
                        pair = %hop.pair,
                        age_ms = db_orderbook_sim::snapshot_age_ms(m.snapshot_at),
                        "mirror stale — LCD fallback for hop"
                    );
                } else {
                    mirror_meta.mirror_missing_hops =
                        mirror_meta.mirror_missing_hops.saturating_add(1);
                    if m.freshness == MirrorFreshness::EmptyPool {
                        tracing::debug!(
                            pair = %hop.pair,
                            "mirror empty pool — LCD fallback for hop"
                        );
                    }
                }
            } else {
                mirror_meta.mirror_missing_hops = mirror_meta.mirror_missing_hops.saturating_add(1);
            }
            mirror_meta.lcd_fallback_queries = mirror_meta.lcd_fallback_queries.saturating_add(1);
            query_hybrid_sim_lcd(
                lcd_fallback,
                &hop.pair,
                &hop.offer_token,
                offer_amount,
                pool_input,
                book_input,
                max_maker_fills,
                None,
                quote_trader,
            )
            .await
            .map_err(HybridSimError::from)
        }
    }
}

async fn query_pool_only_unified(
    source: &HybridSimSource<'_>,
    mirror_meta: Option<&mut MirrorLoadMeta>,
    hop: &HopDescriptor,
    offer_amount: u128,
    max_maker_fills: u32,
    quote_trader: &QuoteTrader,
) -> Result<u128, HybridSimError> {
    query_hybrid_sim_unified(
        source,
        mirror_meta,
        hop,
        offer_amount,
        offer_amount,
        0,
        max_maker_fills.max(1),
        None,
        quote_trader,
    )
    .await
}

/// Grid search over `book_input`; picks the split maximizing `return_amount`.
async fn optimize_one_hop(
    source: &HybridSimSource<'_>,
    mut mirror_meta: Option<&mut MirrorLoadMeta>,
    hop: &HopDescriptor,
    offer_amount: u128,
    max_maker_fills: u32,
    meta: &mut OptimizationMeta,
    quote_trader: &QuoteTrader,
) -> Result<(Option<HybridHopJson>, u128), HybridSimError> {
    if offer_amount == 0 {
        return Ok((None, 0));
    }

    let max_maker_fills = max_maker_fills.max(1);
    let book_start_hint = resolve_hop_book_start_hint(source, hop);
    let mut best_book = 0u128;
    let mut best_out = 0u128;
    let mut any_candidate_ok = false;
    let mut saw_infra_lcd = false;
    let mut first_infra_lcd: Option<crate::lcd::LcdError> = None;

    for i in 0..GRID_POINTS {
        let book = if GRID_POINTS <= 1 {
            0
        } else {
            offer_amount.saturating_mul(i as u128) / (GRID_POINTS - 1) as u128
        };
        let pool = offer_amount.saturating_sub(book);
        match query_hybrid_sim_unified(
            source,
            mirror_meta.as_deref_mut(),
            hop,
            offer_amount,
            pool,
            book,
            max_maker_fills,
            book_start_hint,
            quote_trader,
        )
        .await
        {
            Ok(out) => {
                any_candidate_ok = true;
                if out > best_out || (out == best_out && book < best_book) {
                    best_out = out;
                    best_book = book;
                }
            }
            Err(HybridSimError::Lcd(e)) => {
                tracing::debug!(
                    pair = %hop.pair,
                    book,
                    pool,
                    "hybrid_simulation candidate failed: {}",
                    e
                );
                if is_infra_lcd_error(&e) {
                    saw_infra_lcd = true;
                    if first_infra_lcd.is_none() {
                        first_infra_lcd = Some(e);
                    }
                }
            }
            Err(HybridSimError::Db(e)) => {
                tracing::debug!(
                    pair = %hop.pair,
                    book,
                    pool,
                    "db hybrid candidate failed: {}",
                    e
                );
            }
            Err(HybridSimError::PathUnusable) => {}
        }
    }

    if !any_candidate_ok {
        if saw_infra_lcd {
            return Err(HybridSimError::Lcd(
                first_infra_lcd.expect("saw_infra_lcd implies error"),
            ));
        }
        meta.degraded = true;
        let out =
            pool_only_or_zero(source, mirror_meta, hop, offer_amount, 1, quote_trader).await?;
        return Ok((None, out));
    }

    if best_book > 0 {
        meta.any_book_leg = true;
        let pool_input = offer_amount.saturating_sub(best_book);
        let h = HybridHopJson {
            pool_input: pool_input.to_string(),
            book_input: best_book.to_string(),
            max_maker_fills,
            book_start_hint,
        };
        return Ok((Some(h), best_out));
    }

    let out = pool_only_or_zero(source, mirror_meta, hop, offer_amount, 1, quote_trader).await?;
    Ok((None, out))
}

/// Coordinate-descent refinement on top of the sequential baseline (GitLab #209).
pub async fn optimize_multihop_hybrid_joint(
    source: &HybridSimSource<'_>,
    mut mirror_meta: Option<&mut MirrorLoadMeta>,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
    quote_trader: &QuoteTrader,
) -> Result<(Vec<Option<HybridHopJson>>, OptimizationMeta, u128), HybridSimError> {
    let mut meta = OptimizationMeta::default();
    let mut plan = optimize_multihop_hybrid_with_plan(
        source,
        mirror_meta.as_deref_mut(),
        hops,
        amount_in,
        max_maker_fills,
        &mut meta,
        quote_trader,
    )
    .await?;

    const COORDINATE_PASSES: u32 = 2;
    for _ in 0..COORDINATE_PASSES {
        for hop_idx in 0..hops.len() {
            let offer = propagate_offer_through_plan(
                source,
                mirror_meta.as_deref_mut(),
                hops,
                &plan,
                amount_in,
                hop_idx,
                max_maker_fills,
                quote_trader,
            )
            .await?;
            let (hybrid, _) = optimize_one_hop(
                source,
                mirror_meta.as_deref_mut(),
                &hops[hop_idx],
                offer,
                max_maker_fills,
                &mut meta,
                quote_trader,
            )
            .await?;
            plan[hop_idx] = hybrid;
        }
    }

    let final_out = propagate_offer_through_plan(
        source,
        mirror_meta,
        hops,
        &plan,
        amount_in,
        hops.len(),
        max_maker_fills,
        quote_trader,
    )
    .await?;

    Ok((plan, meta, final_out))
}

/// Forward-simulate a fixed plan through all hops (output of final hop).
/// Legacy LCD-only entry (pool-only tests / POST overrides).
pub async fn optimize_multihop_hybrid_joint_lcd(
    lcd: &LcdClient,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
    quote_trader: &QuoteTrader,
) -> Result<(Vec<Option<HybridHopJson>>, OptimizationMeta), crate::lcd::LcdError> {
    let source = HybridSimSource::Lcd(lcd);
    optimize_multihop_hybrid_joint(
        &source,
        None,
        hops,
        amount_in,
        max_maker_fills,
        quote_trader,
    )
    .await
    .map(|(plan, meta, _)| (plan, meta))
    .map_err(|e| match e {
        HybridSimError::Lcd(le) => le,
        HybridSimError::Db(_) => crate::lcd::LcdError::Deserialize("unexpected db sim".into()),
        HybridSimError::PathUnusable => {
            crate::lcd::LcdError::Deserialize("unexpected path unusable".into())
        }
    })
}

async fn optimize_multihop_hybrid_with_plan(
    source: &HybridSimSource<'_>,
    mut mirror_meta: Option<&mut MirrorLoadMeta>,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
    meta: &mut OptimizationMeta,
    quote_trader: &QuoteTrader,
) -> Result<Vec<Option<HybridHopJson>>, HybridSimError> {
    let mut out_vec = Vec::with_capacity(hops.len());
    let mut running = amount_in;

    for hop in hops {
        let (hybrid, next_in) = optimize_one_hop(
            source,
            mirror_meta.as_deref_mut(),
            hop,
            running,
            max_maker_fills,
            meta,
            quote_trader,
        )
        .await?;
        out_vec.push(hybrid);
        running = next_in;
    }

    Ok(out_vec)
}

async fn propagate_offer_through_plan(
    source: &HybridSimSource<'_>,
    mut mirror_meta: Option<&mut MirrorLoadMeta>,
    hops: &[HopDescriptor],
    plan: &[Option<HybridHopJson>],
    amount_in: u128,
    target_hop: usize,
    max_maker_fills: u32,
    quote_trader: &QuoteTrader,
) -> Result<u128, HybridSimError> {
    let mut running = amount_in;
    for (idx, hop) in hops.iter().enumerate().take(target_hop) {
        let (pool, book, hint) = plan
            .get(idx)
            .and_then(|h| h.as_ref())
            .map(|h| {
                (
                    h.pool_input.parse::<u128>().unwrap_or(0),
                    h.book_input.parse::<u128>().unwrap_or(0),
                    h.book_start_hint,
                )
            })
            .unwrap_or((running, 0, None));
        let mut offer = pool.saturating_add(book);
        if offer == 0 {
            offer = running;
        }
        running = match query_hybrid_sim_unified(
            source,
            mirror_meta.as_deref_mut(),
            hop,
            offer,
            pool,
            book,
            max_maker_fills.max(1),
            hint,
            quote_trader,
        )
        .await
        {
            Ok(v) => v,
            // Degraded plans may reference book legs that no longer simulate (#190).
            Err(HybridSimError::Lcd(_))
            | Err(HybridSimError::Db(_))
            | Err(HybridSimError::PathUnusable) => {
                pool_only_or_zero(
                    source,
                    mirror_meta.as_deref_mut(),
                    hop,
                    offer,
                    1,
                    quote_trader,
                )
                .await?
            }
        };
    }
    Ok(running)
}
