//! Per-hop hybrid split search using pair `HybridSimulation` (pool-only fallback uses zero book leg).

use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

use crate::lcd::LcdClient;

/// Hybrid parameters for one hop (matches on-chain `HybridSwapParams`; amounts as decimal strings).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct HybridHopJson {
    pub pool_input: String,
    pub book_input: String,
    pub max_maker_fills: u32,
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
    /// A hop used pool-only because `HybridSimulation` failed for all split candidates.
    pub degraded: bool,
    /// At least one hop has a non-zero book leg in the chosen params.
    pub any_book_leg: bool,
}

#[derive(Deserialize)]
struct HybridSimResp {
    return_amount: String,
}

const GRID_POINTS: u32 = 17;

fn asset_info_token(contract: &str) -> serde_json::Value {
    json!({ "token": { "contract_addr": contract } })
}

async fn query_pool_only_hybrid(
    lcd: &LcdClient,
    pair: &str,
    offer_token: &str,
    offer_amount: u128,
) -> Result<u128, crate::lcd::LcdError> {
    query_hybrid_sim(lcd, pair, offer_token, offer_amount, offer_amount, 0, 1).await
}

async fn query_hybrid_sim(
    lcd: &LcdClient,
    pair: &str,
    offer_token: &str,
    offer_amount: u128,
    pool_input: u128,
    book_input: u128,
    max_maker_fills: u32,
) -> Result<u128, crate::lcd::LcdError> {
    let q = json!({
        "hybrid_simulation": {
            "offer_asset": {
                "info": asset_info_token(offer_token),
                "amount": offer_amount.to_string(),
            },
            "hybrid": {
                "pool_input": pool_input.to_string(),
                "book_input": book_input.to_string(),
                "max_maker_fills": max_maker_fills,
                "book_start_hint": serde_json::Value::Null,
            }
        }
    });
    let r: HybridSimResp = lcd.query_contract(pair, &q).await?;
    r.return_amount
        .parse::<u128>()
        .map_err(|e| crate::lcd::LcdError::Deserialize(format!("return_amount: {}", e)))
}

/// Grid search over `book_input`; picks the split maximizing `return_amount`.
/// On persistent LCD failure, falls back to pool-only hybrid for this hop (`degraded`).
async fn optimize_one_hop(
    lcd: &LcdClient,
    hop: &HopDescriptor,
    offer_amount: u128,
    max_maker_fills: u32,
    meta: &mut OptimizationMeta,
) -> Result<(Option<HybridHopJson>, u128), crate::lcd::LcdError> {
    if offer_amount == 0 {
        return Ok((None, 0));
    }

    let max_maker_fills = max_maker_fills.max(1);
    let mut best_book = 0u128;
    let mut best_out = 0u128;
    let mut any_candidate_ok = false;

    for i in 0..GRID_POINTS {
        let book = if GRID_POINTS <= 1 {
            0
        } else {
            offer_amount.saturating_mul(i as u128) / (GRID_POINTS - 1) as u128
        };
        let pool = offer_amount.saturating_sub(book);
        match query_hybrid_sim(
            lcd,
            &hop.pair,
            &hop.offer_token,
            offer_amount,
            pool,
            book,
            max_maker_fills,
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
            Err(e) => {
                tracing::debug!(
                    pair = %hop.pair,
                    book,
                    pool,
                    "hybrid_simulation candidate failed: {}",
                    e
                );
            }
        }
    }

    if !any_candidate_ok {
        meta.degraded = true;
        // HybridSimulation grid failed — pool-only hybrid (`book_input: 0`) for this hop.
        let out = query_pool_only_hybrid(lcd, &hop.pair, &hop.offer_token, offer_amount).await?;
        return Ok((None, out));
    }

    if best_book > 0 {
        meta.any_book_leg = true;
        let pool_input = offer_amount.saturating_sub(best_book);
        let h = HybridHopJson {
            pool_input: pool_input.to_string(),
            book_input: best_book.to_string(),
            max_maker_fills,
            book_start_hint: None,
        };
        return Ok((Some(h), best_out));
    }

    // Prefer explicit pool-only hybrid (book=0) vs null: both are valid; use null for fewer bytes.
    let out = query_hybrid_sim(
        lcd,
        &hop.pair,
        &hop.offer_token,
        offer_amount,
        offer_amount,
        0,
        1,
    )
    .await?;
    Ok((None, out))
}

/// Sequential per-hop optimization: output of hop *i* is the offer amount for hop *i+1*.
#[allow(dead_code)]
pub async fn optimize_multihop_hybrid(
    lcd: &LcdClient,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
) -> Result<(Vec<Option<HybridHopJson>>, OptimizationMeta), crate::lcd::LcdError> {
    let mut meta = OptimizationMeta::default();
    let plan = optimize_multihop_hybrid_with_plan(lcd, hops, amount_in, max_maker_fills, &mut meta).await?;
    Ok((plan, meta))
}

/// Coordinate-descent refinement on top of the sequential baseline (GitLab #209).
pub async fn optimize_multihop_hybrid_joint(
    lcd: &LcdClient,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
) -> Result<(Vec<Option<HybridHopJson>>, OptimizationMeta), crate::lcd::LcdError> {
    let mut meta = OptimizationMeta::default();
    let mut plan =
        optimize_multihop_hybrid_with_plan(lcd, hops, amount_in, max_maker_fills, &mut meta).await?;

    const COORDINATE_PASSES: u32 = 2;
    for _ in 0..COORDINATE_PASSES {
        for hop_idx in 0..hops.len() {
            let offer = propagate_offer_through_plan(lcd, hops, &plan, amount_in, hop_idx, max_maker_fills)
                .await?;
            let (hybrid, _) =
                optimize_one_hop(lcd, &hops[hop_idx], offer, max_maker_fills, &mut meta).await?;
            plan[hop_idx] = hybrid;
        }
    }

    Ok((plan, meta))
}

async fn optimize_multihop_hybrid_with_plan(
    lcd: &LcdClient,
    hops: &[HopDescriptor],
    amount_in: u128,
    max_maker_fills: u32,
    meta: &mut OptimizationMeta,
) -> Result<Vec<Option<HybridHopJson>>, crate::lcd::LcdError> {
    let mut out_vec = Vec::with_capacity(hops.len());
    let mut running = amount_in;

    for hop in hops {
        let (hybrid, next_in) = optimize_one_hop(lcd, hop, running, max_maker_fills, meta).await?;
        out_vec.push(hybrid);
        running = next_in;
    }

    Ok(out_vec)
}

async fn propagate_offer_through_plan(
    lcd: &LcdClient,
    hops: &[HopDescriptor],
    plan: &[Option<HybridHopJson>],
    amount_in: u128,
    target_hop: usize,
    max_maker_fills: u32,
) -> Result<u128, crate::lcd::LcdError> {
    let mut running = amount_in;
    for (idx, hop) in hops.iter().enumerate().take(target_hop) {
        let (pool, book) = plan
            .get(idx)
            .and_then(|h| h.as_ref())
            .map(|h| {
                (
                    h.pool_input.parse::<u128>().unwrap_or(0),
                    h.book_input.parse::<u128>().unwrap_or(0),
                )
            })
            .unwrap_or((running, 0));
        let mut offer = pool.saturating_add(book);
        if offer == 0 {
            offer = running;
        }
        running = query_hybrid_sim(
            lcd,
            &hop.pair,
            &hop.offer_token,
            offer,
            pool,
            book,
            max_maker_fills.max(1),
        )
        .await?;
    }
    Ok(running)
}
