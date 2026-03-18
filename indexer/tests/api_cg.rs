mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn cg_pairs_returns_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/pairs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["ticker_id"], "LUNC_USTC");
    assert_eq!(body[0]["base"], "LUNC");
    assert_eq!(body[0]["target"], "USTC");
}

#[tokio::test]
async fn cg_tickers_returns_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/tickers").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body[0]["ticker_id"].is_string());
    assert!(body[0]["last_price"].is_string());
    assert!(body[0]["base_volume"].is_string());
}

#[tokio::test]
async fn cg_historical_trades_returns_trades() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC")
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["buy"].is_array());
    assert!(body["sell"].is_array());
}

#[tokio::test]
async fn cg_historical_trades_invalid_ticker() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/historical_trades?ticker_id=INVALID")
        .await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn cg_historical_trades_filter_buy() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&type=buy")
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["sell"].as_array().unwrap().is_empty());
}
