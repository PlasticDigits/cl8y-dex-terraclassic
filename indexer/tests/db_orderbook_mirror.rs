//! GitLab #279 (Phase 1a): the mirrored pair-reserves + resting-book tables and their queries.

mod common;

use bigdecimal::BigDecimal;
use serial_test::serial;
use cl8y_dex_indexer::db::queries::{pair_reserves, resting_orders};
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[tokio::test]
async fn pair_reserves_upsert_replaces_snapshot() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("1000000"), &bd("2000000"), 30, Some(123))
        .await
        .unwrap();
    let row = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("reserves row");
    assert_eq!(row.reserve_0, bd("1000000"));
    assert_eq!(row.reserve_1, bd("2000000"));
    assert_eq!(row.fee_bps, 30);
    assert_eq!(row.block_height, Some(123));

    // A second upsert replaces the prior snapshot in place.
    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("1500000"), &bd("2000000"), 25, Some(456))
        .await
        .unwrap();
    let row = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("reserves row");
    assert_eq!(row.reserve_0, bd("1500000"));
    assert_eq!(row.fee_bps, 25);
    assert_eq!(row.block_height, Some(456));
}

#[tokio::test]
async fn pair_reserves_missing_is_none() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    // No snapshot taken yet -> degrade-not-error contract returns None.
    assert!(pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn resting_book_replace_and_walk_order() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let mk = |order_id: i64, side: &str, price: &str, rem: &str| resting_orders::RestingOrderInput {
        order_id,
        side: side.to_string(),
        price: bd(price),
        remaining: bd(rem),
        owner: Some("terra1maker".to_string()),
        expires_at: None,
    };

    let orders = vec![
        mk(1, "bid", "1.00", "100"),
        mk(2, "bid", "1.05", "100"), // higher bid -> first
        mk(3, "bid", "1.05", "100"), // same price, later id -> after #2 (FIFO)
        mk(4, "ask", "1.20", "100"),
        mk(5, "ask", "1.10", "100"), // lower ask -> first
    ];
    resting_orders::replace_pair_resting_orders(&pool, seed.pair_id, Some(10), &orders)
        .await
        .unwrap();

    let bids = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "bid")
        .await
        .unwrap();
    assert_eq!(
        bids.iter().map(|o| o.order_id).collect::<Vec<_>>(),
        vec![2, 3, 1],
        "bids: best price first, FIFO at equal price"
    );

    let asks = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "ask")
        .await
        .unwrap();
    assert_eq!(
        asks.iter().map(|o| o.order_id).collect::<Vec<_>>(),
        vec![5, 4],
        "asks: lowest price first"
    );

    // A fresh snapshot wholesale-replaces the prior book.
    resting_orders::replace_pair_resting_orders(&pool, seed.pair_id, Some(11), &[mk(9, "bid", "0.9", "50")])
        .await
        .unwrap();
    let bids = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "bid")
        .await
        .unwrap();
    assert_eq!(
        bids.iter().map(|o| o.order_id).collect::<Vec<_>>(),
        vec![9],
        "replace wipes the prior snapshot"
    );
}

#[tokio::test]
#[serial]
async fn resting_book_replace_rolls_back_on_constraint_failure() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let mk = |order_id: i64, side: &str, price: &str| resting_orders::RestingOrderInput {
        order_id,
        side: side.to_string(),
        price: bd(price),
        remaining: bd("100"),
        owner: None,
        expires_at: None,
    };

    let initial = vec![mk(1, "bid", "1.0"), mk(2, "ask", "1.2")];
    resting_orders::replace_pair_resting_orders(&pool, seed.pair_id, Some(1), &initial)
        .await
        .unwrap();

    let bad = vec![
        mk(3, "bid", "0.9"),
        resting_orders::RestingOrderInput {
            side: "buy".to_string(),
            ..mk(4, "bid", "0.8")
        },
    ];
    let err = resting_orders::replace_pair_resting_orders(&pool, seed.pair_id, Some(2), &bad)
        .await
        .expect_err("invalid side should fail");
    assert!(
        err.to_string().contains("resting_limit_orders") || err.to_string().contains("check"),
        "expected CHECK constraint failure, got: {err}"
    );

    let bids = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "bid")
        .await
        .unwrap();
    let asks = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "ask")
        .await
        .unwrap();
    assert_eq!(bids.len(), 1);
    assert_eq!(asks.len(), 1);
    assert_eq!(bids[0].order_id, 1);
    assert_eq!(asks[0].order_id, 2);
}
