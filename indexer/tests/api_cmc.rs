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
async fn cmc_orderbook_returns_openware_array_wrapper() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/LUNC_USTC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        body.is_array(),
        "CMC orderbook root must be JSON array (Openware Peatio; #223)"
    );
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1, "exactly one book object per request");
    assert!(arr[0]["timestamp"].is_number());
    assert!(arr[0]["bids"].is_array());
    assert!(arr[0]["asks"].is_array());
}

#[tokio::test]
async fn cmc_orderbook_unknown_pair_returns_404_not_array() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/LUNC_NOPE").await;
    resp.assert_status_not_found();
    let text = resp.text();
    assert!(
        !text.trim_start().starts_with('['),
        "404 must not return Openware array wrapper: {text}"
    );
}

#[tokio::test]
async fn cg_orderbook_stays_object_not_array() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/orderbook?ticker_id=LUNC_USTC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        body.is_object() && body.get("ticker_id").is_some(),
        "CG orderbook must remain a single object with ticker_id (#223 guardrail)"
    );
    assert!(!body.is_array());
}

#[tokio::test]
async fn cmc_orderbook_invalid_market_pair_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/BADPAIR").await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn cmc_orderbook_depth_capped_when_ok() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/LUNC_USTC?depth=9999").await;
    if resp.status_code().is_success() {
        let body: Value = resp.json();
        let ob = body
            .as_array()
            .and_then(|a| a.first())
            .expect("array wrapper (#223)");
        let bids = ob["bids"].as_array().unwrap();
        let asks = ob["asks"].as_array().unwrap();
        assert!(bids.len() <= 50);
        assert!(asks.len() <= 50);
    }
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
