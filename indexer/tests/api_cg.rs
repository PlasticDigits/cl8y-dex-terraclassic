mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

#[serial]
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

#[serial]
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

#[serial]
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
    if let Some(trade) = body["buy"].as_array().and_then(|a| a.first()) {
        let ts = trade["trade_timestamp"].as_i64().unwrap();
        assert!(
            ts > 1_700_000_000 && ts < 1_700_000_000_000,
            "trade_timestamp must be Unix seconds: {ts}"
        );
    }
}

#[serial]
#[tokio::test]
async fn cg_historical_trades_invalid_ticker() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/historical_trades?ticker_id=INVALID").await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn cg_historical_trades_invalid_type_rejected() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&type=notbuyorsell")
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn cg_historical_trades_limit_capped_at_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&limit=99999")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let buys = body["buy"].as_array().unwrap();
    let sells = body["sell"].as_array().unwrap();
    assert!(buys.len() + sells.len() <= 500);
}

#[serial]
#[tokio::test]
async fn cg_orderbook_invalid_ticker_format() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/orderbook?ticker_id=ONLYONE").await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn cg_orderbook_depth_capped() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=9999")
        .await;
    // LCD may fail (dummy endpoint); if we get 200, total depth 100 → at most 50 per side.
    if resp.status_code().is_success() {
        let body: Value = resp.json();
        let bids = body["bids"].as_array().unwrap();
        let asks = body["asks"].as_array().unwrap();
        assert!(bids.len() <= 50);
        assert!(asks.len() <= 50);
    }
}

#[serial]
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
