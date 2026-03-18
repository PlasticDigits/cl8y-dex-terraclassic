mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn cmc_summary_returns_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/summary").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body[0]["trading_pairs"].is_string());
    assert!(body[0]["base_currency"].is_string());
    assert!(body[0]["last_price"].is_string());
}

#[tokio::test]
async fn cmc_assets_returns_map() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/assets").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body.is_object());
    assert!(body["LUNC"].is_object());
    assert!(body["USTC"].is_object());
    assert!(body["LUNC"]["name"].is_string());
}

#[tokio::test]
async fn cmc_ticker_returns_map() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/ticker").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body.is_object());
    assert!(body["LUNC_USTC"].is_object());
    assert!(body["LUNC_USTC"]["last_price"].is_string());
}

#[tokio::test]
async fn cmc_trades_returns_trades() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/trades/LUNC_USTC").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body[0]["trade_id"].is_i64());
    assert!(body[0]["price"].is_string());
}

#[tokio::test]
async fn cmc_trades_invalid_pair_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/trades/INVALID").await;
    resp.assert_status_bad_request();
}
