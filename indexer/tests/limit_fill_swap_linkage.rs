//! GitLab #316 / #331 — limit_order_fill -> swap_events linkage by per-pair swap ordinal.
//!
//! The old `swap_id_for_tx_pair` used `ORDER BY id ASC LIMIT 1`, so every fill in a tx linked to
//! the FIRST swap on the pair. When a tx had multiple swaps on the same pair (router revisit /
//! batch), fills produced by the second swap were mis-attributed to the first. The fix keys the
//! lookup on `(tx_hash, pair_id, swap_index)` — the deterministic parser walk ordinal the fill
//! carries — so each fill resolves to the exact swap that produced it.

mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::{limit_order_fills, swap_events};
use std::str::FromStr;

#[tokio::test]
async fn fill_links_to_its_own_swap_not_the_first_on_the_pair() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    // Unique tx_hash so the assertions are isolated from anything else in the shared test DB.
    let tx_hash = "TX316_MULTI_SWAP_SAME_PAIR";
    let amt = BigDecimal::from_str("100").unwrap();
    let price = BigDecimal::from_str("1").unwrap();

    // Two swaps on the SAME pair within one tx, as the parser would persist them: swap_index 0
    // then 1 (unique on (tx_hash, pair_id, swap_index) per GitLab #287).
    let id0 = swap_events::insert_swap(
        &pool, seed.pair_id, 0, 1, Utc::now(), tx_hash, "terra1taker", None,
        seed.asset_0_id, seed.asset_1_id, &amt, &amt, None, None, None, &price, None, None, None,
        None,
    )
    .await
    .unwrap()
    .expect("first swap inserted");
    let id1 = swap_events::insert_swap(
        &pool, seed.pair_id, 1, 1, Utc::now(), tx_hash, "terra1taker", None,
        seed.asset_0_id, seed.asset_1_id, &amt, &amt, None, None, None, &price, None, None, None,
        None,
    )
    .await
    .unwrap()
    .expect("second swap inserted");
    assert_ne!(id0, id1, "the two swaps must be distinct rows");

    // The fix: each fill resolves to the swap row matching ITS swap_index.
    let for_swap_0 = limit_order_fills::swap_id_for_tx_pair_index(&pool, tx_hash, seed.pair_id, 0)
        .await
        .unwrap();
    let for_swap_1 = limit_order_fills::swap_id_for_tx_pair_index(&pool, tx_hash, seed.pair_id, 1)
        .await
        .unwrap();
    assert_eq!(for_swap_0, Some(id0), "swap_index 0 fill -> first swap");
    assert_eq!(
        for_swap_1,
        Some(id1),
        "swap_index 1 fill -> SECOND swap (the old MIN(id) linkage would return the first)"
    );

    // An ordinal with no swap row -> None (no spurious link).
    let missing = limit_order_fills::swap_id_for_tx_pair_index(&pool, tx_hash, seed.pair_id, 2)
        .await
        .unwrap();
    assert_eq!(missing, None);
}

/// GitLab #331 — explicit on-chain `swap_index` wasm attrs resolve to the same swap rows as
/// parser-inferred ordinals when attrs match walk order (see `parse_limit_order_fills_*` unit tests
/// for attr parsing and forged/missing attr cases).
#[tokio::test]
async fn explicit_swap_index_attrs_link_to_matching_swap_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let tx_hash = "TX331_EXPLICIT_SWAP_INDEX";
    let amt = BigDecimal::from_str("50").unwrap();
    let price = BigDecimal::from_str("1").unwrap();

    let id0 = swap_events::insert_swap(
        &pool, seed.pair_id, 0, 1, Utc::now(), tx_hash, "terra1taker", None,
        seed.asset_0_id, seed.asset_1_id, &amt, &amt, None, None, None, &price, None, None, None,
        None,
    )
    .await
    .unwrap()
    .expect("swap 0");
    let id1 = swap_events::insert_swap(
        &pool, seed.pair_id, 1, 1, Utc::now(), tx_hash, "terra1taker", None,
        seed.asset_0_id, seed.asset_1_id, &amt, &amt, None, None, None, &price, None, None, None,
        None,
    )
    .await
    .unwrap()
    .expect("swap 1");

    // On-chain attrs `swap_index=0` / `1` (post #331) map to the same rows as inference.
    assert_eq!(
        limit_order_fills::swap_id_for_tx_pair_index(&pool, tx_hash, seed.pair_id, 0)
            .await
            .unwrap(),
        Some(id0)
    );
    assert_eq!(
        limit_order_fills::swap_id_for_tx_pair_index(&pool, tx_hash, seed.pair_id, 1)
            .await
            .unwrap(),
        Some(id1)
    );
}
