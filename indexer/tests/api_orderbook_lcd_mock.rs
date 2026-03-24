//! `/cg/orderbook` and `/cmc/orderbook/*` against a wiremock LCD (no real chain).

mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn cg_orderbook_200_simulated_depth_matches_query() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=50")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 50);
    assert_eq!(asks.len(), 50);
    assert!(bids.len() <= 100);
    assert!(asks.len() <= 100);
}

#[tokio::test]
async fn cg_orderbook_depth_capped_at_100_with_lcd_mock() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=9999")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 100);
    assert_eq!(asks.len(), 100);
}

#[tokio::test]
async fn cg_orderbook_second_identical_request_hits_cache_not_lcd() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = "/cg/orderbook?ticker_id=LUNC_USTC&depth=20";
    server.get(url).await.assert_status_ok();
    server.get(url).await.assert_status_ok();

    assert_eq!(
        mock.received_requests()
            .await
            .expect("mock server should expose request log")
            .len(),
        1,
        "orderbook cache should avoid a second LCD smart-query for same pair+depth"
    );
}

#[tokio::test]
async fn cmc_orderbook_200_with_lcd_mock() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/LUNC_USTC?depth=15").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 15);
    assert_eq!(asks.len(), 15);
}
