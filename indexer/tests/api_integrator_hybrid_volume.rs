//! GitLab #216 — internal trades API leg volumes + null-leg headline volume.

mod common;

use axum_test::TestServer;
use chrono::Utc;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn pair_trades_expose_leg_volume_aliases_matching_cg() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    insert_hybrid_swap(&pool, &seed, "internal_trades_hybrid_tx").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/pairs/{}/trades", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    let trade = body
        .iter()
        .find(|t| t["tx_hash"] == "internal_trades_hybrid_tx")
        .expect("hybrid trade row");

    assert_eq!(trade["offer_amount"], "100");
    assert_eq!(trade["return_amount"], "95");
    assert_eq!(trade["pool_return_amount"], "40");
    assert_eq!(trade["book_return_amount"], "55");
    assert_eq!(trade["pool_leg_volume"], "40");
    assert_eq!(trade["book_leg_volume"], "55");
    assert_eq!(trade["limit_book_offer_consumed"], "60");

    let pool_leg: i64 = trade["pool_leg_volume"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let book_leg: i64 = trade["book_leg_volume"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let ret: i64 = trade["return_amount"].as_str().unwrap().parse().unwrap();
    assert_eq!(pool_leg + book_leg, ret);
}

#[serial]
#[tokio::test]
async fn pair_trades_pool_only_omits_leg_aliases_not_zero_volume() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    insert_pool_only_swap(&pool, &seed, "internal_trades_pool_only_tx").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/pairs/{}/trades", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    let trade = body
        .iter()
        .find(|t| t["tx_hash"] == "internal_trades_pool_only_tx")
        .expect("pool-only trade");

    assert_eq!(trade["offer_amount"], "50");
    assert_eq!(trade["return_amount"], "49");
    assert!(trade.get("pool_leg_volume").is_none());
    assert!(trade.get("book_leg_volume").is_none());
    assert!(trade.get("pool_return_amount").is_none());
}

async fn insert_hybrid_swap(pool: &sqlx::PgPool, seed: &common::SeedData, tx_hash: &str) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price,
          pool_return_amount, book_return_amount, limit_book_offer_consumed)
         VALUES ($1, 2000, $2, $3, $4, $5, $6, 100, 95, 0.95, 40, 55, 60)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now())
    .bind(tx_hash)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(pool)
    .await
    .expect("insert hybrid swap");
}

async fn insert_pool_only_swap(pool: &sqlx::PgPool, seed: &common::SeedData, tx_hash: &str) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 2001, $2, $3, $4, $5, $6, 50, 49, 0.98)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now())
    .bind(tx_hash)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(pool)
    .await
    .expect("insert pool-only swap");
}
