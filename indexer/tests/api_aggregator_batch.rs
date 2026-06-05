//! GitLab #288 — set-based CG/CMC aggregator endpoints (no per-pair N+1).

mod common;

use axum_test::TestServer;
use chrono::Utc;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn batch_stats_match_per_pair_queries() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let per_pair = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_for_pair(
        &pool, seed.pair_id,
    )
    .await
    .expect("per-pair stats");
    let all = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_all_pairs(&pool)
        .await
        .expect("batch stats");
    let batch = all.get(&seed.pair_id).expect("pair in batch map");

    assert_eq!(batch.volume_base, per_pair.volume_base);
    assert_eq!(batch.volume_quote, per_pair.volume_quote);
    assert_eq!(batch.trade_count, per_pair.trade_count);
    assert_eq!(batch.close_price, per_pair.close_price);
}

#[serial]
#[tokio::test]
async fn cg_tickers_default_limit_is_100() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/tickers").await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body.len() <= 100);
}

#[serial]
#[tokio::test]
async fn cg_tickers_limit_offset_pagination() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let page0: Vec<Value> = server.get("/cg/tickers?limit=1&offset=0").await.json();
    let page1: Vec<Value> = server.get("/cg/tickers?limit=1&offset=1").await.json();
    assert_eq!(page0.len(), 1);
    if !page1.is_empty() {
        assert_ne!(page0[0]["ticker_id"], page1[0]["ticker_id"]);
    }
}

#[serial]
#[tokio::test]
async fn cg_tickers_rejects_excessive_offset() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/tickers?offset=99999").await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn cmc_summary_sorted_by_volume_desc() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    // Second pair with higher quote volume in the 24h window.
    let pair2_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id)
         VALUES ('terra1lowvolpair', $1, $2)
         RETURNING id",
    )
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("insert low-vol pair");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 3000, $2, 'high_vol_tx', $3, $4, $5, 10, 5000, 0.5)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now())
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("high vol swap on seed pair");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 3001, $2, 'low_vol_tx', $3, $4, $5, 10, 1, 0.5)",
    )
    .bind(pair2_id)
    .bind(Utc::now())
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("low vol swap on second pair");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let body: Vec<Value> = server.get("/cmc/summary?limit=2").await.json();
    assert!(body.len() >= 2);
    assert_eq!(body[0]["trading_pairs"], "LUNC_USTC");
}
