//! Post-event AMM `RESERVES` for GeckoTerminal `/gt/events` (GitLab #684).
//!
//! Reserves on an event are factory `asset_0` / `asset_1` **after** that event.
//! Only the **pool** leg mutates them. Hybrid book notional, parked dust, wallet
//! balances, wrap-mapper inventory, CW20 donations, and sweep/`Skim` are out.
//!
//! Pair wasm `reserve_0` / `reserve_1` attrs are optional gold-standard; ingest
//! reconstruction is the launch path. Do **not** JOIN `pair_reserves` on GET.

use std::collections::HashMap;

use bigdecimal::{BigDecimal, Zero};
use sqlx::{FromRow, PgPool};

/// Hybrid columns are present when any of pool/book/consumed was indexed.
pub fn hybrid_legs_present(
    pool_return: Option<&BigDecimal>,
    book_return: Option<&BigDecimal>,
    book_consumed: Option<&BigDecimal>,
) -> bool {
    pool_return.is_some() || book_return.is_some() || book_consumed.is_some()
}

/// Book-only: non-zero book return and zero (or absent) pool return.
pub fn is_book_only(pool_return: Option<&BigDecimal>, book_return: Option<&BigDecimal>) -> bool {
    let pool_zero = pool_return.map(BigDecimal::is_zero).unwrap_or(true);
    let book_pos = book_return
        .map(|v| *v > BigDecimal::zero())
        .unwrap_or(false);
    pool_zero && book_pos
}

/// Offer amount that actually hits the AMM.
///
/// Hybrid: `offer - limit_book_offer_consumed`. Book-only → `0`.
/// Legacy (no hybrid columns): whole `offer` (pool-only assumption).
pub fn pool_input(
    offer_amount: &BigDecimal,
    pool_return: Option<&BigDecimal>,
    book_return: Option<&BigDecimal>,
    book_consumed: Option<&BigDecimal>,
) -> BigDecimal {
    if is_book_only(pool_return, book_return) {
        return BigDecimal::zero();
    }
    if !hybrid_legs_present(pool_return, book_return, book_consumed) {
        return offer_amount.clone();
    }
    let consumed = book_consumed.cloned().unwrap_or_else(BigDecimal::zero);
    let rest = offer_amount - consumed;
    if rest < BigDecimal::zero() {
        BigDecimal::zero()
    } else {
        rest
    }
}

/// Amount removed from the ask-side AMM reserve (`pool_return + pool commission`).
///
/// Commission is transferred to treasury and is part of `gross_output` in pair
/// wasm (`RESERVES` subtracts gross, not net). Do **not** also subtract
/// `return_amount` (pool+book). Book-only → `0`. Legacy: `return + commission`.
pub fn gross_output(
    return_amount: &BigDecimal,
    pool_return: Option<&BigDecimal>,
    book_return: Option<&BigDecimal>,
    book_consumed: Option<&BigDecimal>,
    commission: Option<&BigDecimal>,
) -> BigDecimal {
    if is_book_only(pool_return, book_return) {
        return BigDecimal::zero();
    }
    let fee = commission.cloned().unwrap_or_else(BigDecimal::zero);
    if !hybrid_legs_present(pool_return, book_return, book_consumed) {
        return return_amount + fee;
    }
    pool_return.cloned().unwrap_or_else(BigDecimal::zero) + fee
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveDelta {
    pub d0: BigDecimal,
    pub d1: BigDecimal,
}

pub fn swap_pool_delta(
    offer_is_asset_0: bool,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
    pool_return: Option<&BigDecimal>,
    book_return: Option<&BigDecimal>,
    book_consumed: Option<&BigDecimal>,
    commission: Option<&BigDecimal>,
) -> ReserveDelta {
    let pool_in = pool_input(offer_amount, pool_return, book_return, book_consumed);
    let gross = gross_output(
        return_amount,
        pool_return,
        book_return,
        book_consumed,
        commission,
    );
    if offer_is_asset_0 {
        ReserveDelta {
            d0: pool_in,
            d1: -gross,
        }
    } else {
        ReserveDelta {
            d0: -gross,
            d1: pool_in,
        }
    }
}

pub fn liquidity_delta(is_add: bool, amount0: &BigDecimal, amount1: &BigDecimal) -> ReserveDelta {
    if is_add {
        ReserveDelta {
            d0: amount0.clone(),
            d1: amount1.clone(),
        }
    } else {
        ReserveDelta {
            d0: -amount0.clone(),
            d1: -amount1.clone(),
        }
    }
}

pub fn apply_delta(
    r0: &BigDecimal,
    r1: &BigDecimal,
    delta: &ReserveDelta,
) -> Option<(BigDecimal, BigDecimal)> {
    let n0 = r0 + &delta.d0;
    let n1 = r1 + &delta.d1;
    if n0 < BigDecimal::zero() || n1 < BigDecimal::zero() {
        return None;
    }
    Some((n0, n1))
}

pub fn invert_delta(
    post0: &BigDecimal,
    post1: &BigDecimal,
    delta: &ReserveDelta,
) -> Option<(BigDecimal, BigDecimal)> {
    apply_delta(
        post0,
        post1,
        &ReserveDelta {
            d0: -delta.d0.clone(),
            d1: -delta.d1.clone(),
        },
    )
}

#[derive(FromRow)]
struct ReservePair {
    reserve_0: BigDecimal,
    reserve_1: BigDecimal,
}

/// Latest persisted post-event reserves for a pair (parser order: swaps then join/exit).
pub async fn last_persisted_reserves(
    pool: &PgPool,
    pair_id: i32,
) -> Result<Option<(BigDecimal, BigDecimal)>, sqlx::Error> {
    sqlx::query_as::<_, ReservePair>(
        r#"
        SELECT reserve_0, reserve_1 FROM (
            SELECT block_height, tx_hash, 0 AS phase, swap_index AS seq, id,
                   reserve_0, reserve_1
            FROM swap_events
            WHERE pair_id = $1 AND reserve_0 IS NOT NULL AND reserve_1 IS NOT NULL
            UNION ALL
            SELECT block_height, tx_hash, 1 AS phase, 0 AS seq, id,
                   reserve_0, reserve_1
            FROM liquidity_events
            WHERE pair_id = $1 AND reserve_0 IS NOT NULL AND reserve_1 IS NOT NULL
        ) e
        ORDER BY block_height DESC, tx_hash DESC, phase DESC, seq DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(pair_id)
    .fetch_optional(pool)
    .await
    .map(|row| row.map(|r| (r.reserve_0, r.reserve_1)))
}

pub async fn resolve_pre_reserves(
    pool: &PgPool,
    cache: &HashMap<i32, (BigDecimal, BigDecimal)>,
    pair_id: i32,
) -> Result<Option<(BigDecimal, BigDecimal)>, sqlx::Error> {
    if let Some(v) = cache.get(&pair_id) {
        return Ok(Some(v.clone()));
    }
    last_persisted_reserves(pool, pair_id).await
}

#[derive(Debug, Clone, Default)]
pub struct BackfillStats {
    pub pairs: usize,
    pub swaps_filled: u64,
    pub liqs_filled: u64,
    pub skipped_ambiguous: u64,
}

#[derive(FromRow)]
struct SwapBackfillRow {
    id: i64,
    pair_id: i32,
    asset_0_id: i32,
    block_height: i64,
    tx_hash: String,
    swap_index: i32,
    offer_asset_id: i32,
    offer_amount: BigDecimal,
    return_amount: BigDecimal,
    pool_return_amount: Option<BigDecimal>,
    book_return_amount: Option<BigDecimal>,
    limit_book_offer_consumed: Option<BigDecimal>,
    commission_amount: Option<BigDecimal>,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
}

#[derive(FromRow)]
struct LiqBackfillRow {
    id: i64,
    pair_id: i32,
    block_height: i64,
    tx_hash: String,
    event_type: String,
    asset_0_amount: Option<BigDecimal>,
    asset_1_amount: Option<BigDecimal>,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
}

#[derive(FromRow)]
struct PairSeedRow {
    pair_id: i32,
    reserve_0: BigDecimal,
    reserve_1: BigDecimal,
}

enum TapeEvent {
    Swap(SwapBackfillRow),
    Liq(LiqBackfillRow),
}

impl TapeEvent {
    fn sort_key(&self) -> (i64, &str, u8, i32, i64) {
        match self {
            TapeEvent::Swap(s) => (s.block_height, s.tx_hash.as_str(), 0, s.swap_index, s.id),
            TapeEvent::Liq(l) => (l.block_height, l.tx_hash.as_str(), 1, 0, l.id),
        }
    }
    fn already_filled(&self) -> bool {
        match self {
            TapeEvent::Swap(s) => s.reserve_0.is_some() && s.reserve_1.is_some(),
            TapeEvent::Liq(l) => l.reserve_0.is_some() && l.reserve_1.is_some(),
        }
    }
    fn stored_post(&self) -> Option<(BigDecimal, BigDecimal)> {
        match self {
            TapeEvent::Swap(s) => Some((s.reserve_0.clone()?, s.reserve_1.clone()?)),
            TapeEvent::Liq(l) => Some((l.reserve_0.clone()?, l.reserve_1.clone()?)),
        }
    }
    fn delta(&self) -> ReserveDelta {
        match self {
            TapeEvent::Swap(s) => swap_pool_delta(
                s.offer_asset_id == s.asset_0_id,
                &s.offer_amount,
                &s.return_amount,
                s.pool_return_amount.as_ref(),
                s.book_return_amount.as_ref(),
                s.limit_book_offer_consumed.as_ref(),
                s.commission_amount.as_ref(),
            ),
            TapeEvent::Liq(l) => liquidity_delta(
                l.event_type == "add",
                l.asset_0_amount.as_ref().unwrap_or(&BigDecimal::zero()),
                l.asset_1_amount.as_ref().unwrap_or(&BigDecimal::zero()),
            ),
        }
    }
}

/// Reverse-apply from current `pair_reserves` onto NULL event columns.
/// Never copies the tip snapshot onto every row without inverting deltas.
pub async fn backfill_all(pool: &PgPool) -> Result<BackfillStats, sqlx::Error> {
    let seeds: Vec<PairSeedRow> =
        sqlx::query_as("SELECT pair_id, reserve_0, reserve_1 FROM pair_reserves")
            .fetch_all(pool)
            .await?;
    let seed_map: HashMap<i32, (BigDecimal, BigDecimal)> = seeds
        .into_iter()
        .map(|r| (r.pair_id, (r.reserve_0, r.reserve_1)))
        .collect();

    let swaps: Vec<SwapBackfillRow> = sqlx::query_as(
        r#"
        SELECT se.id, se.pair_id, p.asset_0_id, se.block_height, se.tx_hash, se.swap_index,
               se.offer_asset_id, se.offer_amount, se.return_amount,
               se.pool_return_amount, se.book_return_amount, se.limit_book_offer_consumed,
               se.commission_amount, se.reserve_0, se.reserve_1
        FROM swap_events se
        JOIN pairs p ON p.id = se.pair_id
        "#,
    )
    .fetch_all(pool)
    .await?;
    let liqs: Vec<LiqBackfillRow> = sqlx::query_as(
        r#"
        SELECT id, pair_id, block_height, tx_hash, event_type,
               asset_0_amount, asset_1_amount, reserve_0, reserve_1
        FROM liquidity_events
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut by_pair: HashMap<i32, Vec<TapeEvent>> = HashMap::new();
    for s in swaps {
        by_pair
            .entry(s.pair_id)
            .or_default()
            .push(TapeEvent::Swap(s));
    }
    for l in liqs {
        by_pair
            .entry(l.pair_id)
            .or_default()
            .push(TapeEvent::Liq(l));
    }

    let mut stats = BackfillStats::default();
    for (pair_id, mut events) in by_pair {
        let Some(mut running) = seed_map.get(&pair_id).cloned() else {
            continue;
        };
        stats.pairs += 1;
        events.sort_by(|a, b| b.sort_key().cmp(&a.sort_key()));
        let mut skip_rest = false;
        for ev in events {
            if skip_rest {
                stats.skipped_ambiguous += 1;
                continue;
            }
            if ev.already_filled() {
                if let Some(stored) = ev.stored_post() {
                    running = stored;
                }
                let Some(pre) = invert_delta(&running.0, &running.1, &ev.delta()) else {
                    skip_rest = true;
                    stats.skipped_ambiguous += 1;
                    continue;
                };
                running = pre;
                continue;
            }
            let post = running.clone();
            match &ev {
                TapeEvent::Swap(s) => {
                    sqlx::query(
                        "UPDATE swap_events SET reserve_0 = $1, reserve_1 = $2 WHERE id = $3
                         AND reserve_0 IS NULL AND reserve_1 IS NULL",
                    )
                    .bind(&post.0)
                    .bind(&post.1)
                    .bind(s.id)
                    .execute(pool)
                    .await?;
                    stats.swaps_filled += 1;
                }
                TapeEvent::Liq(l) => {
                    sqlx::query(
                        "UPDATE liquidity_events SET reserve_0 = $1, reserve_1 = $2 WHERE id = $3
                         AND reserve_0 IS NULL AND reserve_1 IS NULL",
                    )
                    .bind(&post.0)
                    .bind(&post.1)
                    .bind(l.id)
                    .execute(pool)
                    .await?;
                    stats.liqs_filled += 1;
                }
            }
            let Some(pre) = invert_delta(&post.0, &post.1, &ev.delta()) else {
                skip_rest = true;
                stats.skipped_ambiguous += 1;
                continue;
            };
            running = pre;
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn bd(s: &str) -> BigDecimal {
        BigDecimal::from_str(s).unwrap()
    }

    #[test]
    fn t3_hybrid_only_pool_leg_moves() {
        let d = swap_pool_delta(
            true,
            &bd("100"),
            &bd("90"),
            Some(&bd("30")),
            Some(&bd("60")),
            Some(&bd("60")),
            Some(&bd("1")),
        );
        assert_eq!(d.d0, bd("40"));
        assert_eq!(d.d1, bd("-31"));
    }

    #[test]
    fn t4_book_only_zero_delta() {
        let d = swap_pool_delta(
            true,
            &bd("100"),
            &bd("55"),
            Some(&bd("0")),
            Some(&bd("55")),
            Some(&bd("100")),
            Some(&bd("0")),
        );
        assert_eq!(d.d0, bd("0"));
        assert_eq!(d.d1, bd("0"));
    }

    #[test]
    fn naive_full_notional_is_not_used() {
        let d = swap_pool_delta(
            true,
            &bd("100"),
            &bd("90"),
            Some(&bd("30")),
            Some(&bd("60")),
            Some(&bd("60")),
            Some(&bd("1")),
        );
        assert_ne!(d.d0, bd("100"), "A1: must not add full offer");
        assert_ne!(d.d1, bd("-90"), "A1: must not drain full return");
    }

    #[test]
    fn legacy_null_hybrid_is_pool_only() {
        let d = swap_pool_delta(true, &bd("10"), &bd("8"), None, None, None, Some(&bd("1")));
        assert_eq!(d.d0, bd("10"));
        assert_eq!(d.d1, bd("-9"));
    }

    #[test]
    fn offer_asset1_maps_to_factory_legs() {
        let d = swap_pool_delta(false, &bd("100"), &bd("50"), None, None, None, None);
        assert_eq!(d.d0, bd("-50"));
        assert_eq!(d.d1, bd("100"));
    }

    #[test]
    fn join_exit_and_invert_roundtrip() {
        let add = liquidity_delta(true, &bd("5"), &bd("7"));
        let (r0, r1) = apply_delta(&bd("100"), &bd("200"), &add).unwrap();
        assert_eq!((r0.clone(), r1.clone()), (bd("105"), bd("207")));
        let pre = invert_delta(&r0, &r1, &add).unwrap();
        assert_eq!(pre, (bd("100"), bd("200")));
        let rem = liquidity_delta(false, &bd("5"), &bd("7"));
        let post = apply_delta(&r0, &r1, &rem).unwrap();
        assert_eq!(post, (bd("100"), bd("200")));
    }

    #[test]
    fn apply_rejects_negative() {
        let d = swap_pool_delta(true, &bd("1"), &bd("50"), None, None, None, None);
        assert!(apply_delta(&bd("1"), &bd("10"), &d).is_none());
    }

    #[test]
    fn invert_swap_matches_forward() {
        let d = swap_pool_delta(
            true,
            &bd("40"),
            &bd("30"),
            Some(&bd("30")),
            Some(&bd("0")),
            Some(&bd("0")),
            Some(&bd("1")),
        );
        let post = apply_delta(&bd("1000"), &bd("2000"), &d).unwrap();
        let pre = invert_delta(&post.0, &post.1, &d).unwrap();
        assert_eq!(pre, (bd("1000"), bd("2000")));
    }
}
