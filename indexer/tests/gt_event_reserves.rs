//! GitLab #684 — persist / backfill post-event AMM reserves for `/gt/events`.
mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::{liquidity, pair_reserves, swap_events};
use cl8y_dex_indexer::indexer::gt_event_reserves;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[serial]
#[tokio::test]
async fn i3_a14_backfill_reverse_apply_not_snapshot_stamp() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();

    let price = bd("1");
    let amt = bd("40");
    let ret = bd("30");
    swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        10,
        Utc::now(),
        "tx_old",
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &amt,
        &ret,
        None,
        Some(&bd("1")),
        None,
        &price,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("old");
    swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        20,
        Utc::now(),
        "tx_new",
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &amt,
        &ret,
        None,
        Some(&bd("1")),
        None,
        &price,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("new");

    // Tip snapshot = post-state after the newest swap.
    pair_reserves::upsert_pair_reserves(
        &pool,
        seed.pair_id,
        &bd("1000"),
        &bd("2000"),
        30,
        Some(20),
    )
    .await
    .unwrap();

    let stats = gt_event_reserves::backfill_all(&pool)
        .await
        .expect("backfill");
    assert!(stats.swaps_filled >= 2);

    let rows: Vec<(i64, BigDecimal, BigDecimal)> = sqlx::query_as(
        "SELECT block_height, reserve_0, reserve_1 FROM swap_events
         WHERE pair_id = $1 ORDER BY block_height",
    )
    .bind(seed.pair_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    let (h_old, r0_old, r1_old) = &rows[0];
    let (h_new, r0_new, r1_new) = &rows[1];
    assert_eq!(*h_old, 10);
    assert_eq!(*h_new, 20);
    assert_eq!(r0_new.to_string(), "1000");
    assert_eq!(r1_new.to_string(), "2000");
    assert_ne!(
        r0_old.to_string(),
        "1000",
        "A14: oldest reconstructed row must not copy the live snapshot"
    );
    // Legacy pool-only: invert newest (offer 40, return+commission 31) → old post = 960 / 2031
    assert_eq!(r0_old.to_string(), "960");
    assert_eq!(r1_old.to_string(), "2031");
}

#[serial]
#[tokio::test]
async fn i5_backfill_join_then_exit() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    liquidity::insert_liquidity_event(
        &pool,
        seed.pair_id,
        1,
        Utc::now(),
        "txadd",
        "terra1lp",
        "add",
        &bd("100"),
        &bd("200"),
        &bd("10"),
        None,
        None,
    )
    .await
    .unwrap();
    liquidity::insert_liquidity_event(
        &pool,
        seed.pair_id,
        2,
        Utc::now(),
        "txrem",
        "terra1lp",
        "remove",
        &bd("10"),
        &bd("20"),
        &bd("1"),
        None,
        None,
    )
    .await
    .unwrap();
    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("90"), &bd("180"), 30, Some(2))
        .await
        .unwrap();
    gt_event_reserves::backfill_all(&pool).await.unwrap();
    let rows: Vec<(String, BigDecimal, BigDecimal)> = sqlx::query_as(
        "SELECT event_type, reserve_0, reserve_1 FROM liquidity_events
         WHERE pair_id = $1 ORDER BY block_height",
    )
    .bind(seed.pair_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows[1].0, "remove");
    assert_eq!(rows[1].1.to_string(), "90");
    assert_eq!(rows[0].0, "add");
    assert_eq!(rows[0].1.to_string(), "100");
    assert_eq!(rows[0].2.to_string(), "200");
}

#[serial]
#[tokio::test]
async fn i6_conflict_does_not_overwrite_reserves() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let tx = "tx_conflict_684";
    sqlx::query("DELETE FROM swap_events WHERE tx_hash = $1")
        .bind(tx)
        .execute(&pool)
        .await
        .ok();
    let first = swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        30,
        Utc::now(),
        tx,
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &bd("10"),
        &bd("9"),
        None,
        None,
        None,
        &bd("0.9"),
        None,
        None,
        None,
        None,
        None,
        Some(&bd("111")),
        Some(&bd("222")),
    )
    .await
    .unwrap();
    assert!(first.is_some());
    let second = swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        30,
        Utc::now(),
        tx,
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &bd("10"),
        &bd("9"),
        None,
        None,
        None,
        &bd("0.9"),
        None,
        None,
        None,
        None,
        None,
        Some(&bd("1")),
        Some(&bd("2")),
    )
    .await
    .unwrap();
    assert!(second.is_none());
    let (r0, r1): (BigDecimal, BigDecimal) = sqlx::query_as(
        "SELECT reserve_0, reserve_1 FROM swap_events WHERE tx_hash = $1 AND pair_id = $2",
    )
    .bind(tx)
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(r0.to_string(), "111");
    assert_eq!(r1.to_string(), "222");
}

#[serial]
#[tokio::test]
async fn last_persisted_prefers_later_swap() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        1,
        Utc::now(),
        "tx1",
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &bd("1"),
        &bd("1"),
        None,
        None,
        None,
        &bd("1"),
        None,
        None,
        None,
        None,
        None,
        Some(&bd("10")),
        Some(&bd("20")),
    )
    .await
    .unwrap();
    swap_events::insert_swap(
        &pool,
        seed.pair_id,
        0,
        2,
        Utc::now(),
        "tx2",
        "terra1taker",
        None,
        seed.asset_0_id,
        seed.asset_1_id,
        &bd("1"),
        &bd("1"),
        None,
        None,
        None,
        &bd("1"),
        None,
        None,
        None,
        None,
        None,
        Some(&bd("11")),
        Some(&bd("19")),
    )
    .await
    .unwrap();
    let last = gt_event_reserves::last_persisted_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("last");
    assert_eq!(last.0.to_string(), "11");
    assert_eq!(last.1.to_string(), "19");
}
