//! GitLab #189 — consolidated CG/CMC hybrid + pool-only reporting.

mod common;

use axum_test::TestServer;
use chrono::Utc;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn cg_tickers_include_consolidated_extensions() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    insert_hybrid_swap(&pool, &seed, "hybrid_consolidated_tx").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/cg/tickers").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    let ext = &body[0]["cl8y_extensions"];
    assert_eq!(ext["consolidated"], true);
    assert_eq!(ext["hybrid_trade_count_24h"], "1");
    assert!(ext["pool_only_trade_count_24h"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap()
        >= 5);
    assert_eq!(ext["book_leg_volume_quote_24h"], "55");
    assert_eq!(ext["pool_leg_volume_quote_24h"], "40");
}

#[serial]
#[tokio::test]
async fn cg_historical_trades_include_hybrid_leg_volumes() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    insert_hybrid_swap(&pool, &seed, "hybrid_trade_legs_tx").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC")
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    let sells = body["sell"].as_array().expect("sell trades");
    let hybrid = sells
        .iter()
        .find(|t| t["pool_leg_volume"].as_str() == Some("40"))
        .expect("hybrid trade with pool_leg_volume");
    assert_eq!(hybrid["book_leg_volume"], "55");
}

#[serial]
#[tokio::test]
async fn cmc_summary_includes_consolidated_extensions() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    insert_hybrid_swap(&pool, &seed, "hybrid_cmc_summary_tx").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/cmc/summary").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body[0]["cl8y_extensions"]["hybrid_trade_count_24h"], "1");
    assert_eq!(body[0]["cl8y_extensions"]["consolidated"], true);
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
