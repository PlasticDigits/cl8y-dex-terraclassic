mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn list_pairs_returns_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty(), "should have at least one pair");

    let pair = &body[0];
    assert_eq!(pair["pair_address"], seed.pair_address);
    assert!(pair["asset_0"]["symbol"].is_string());
    assert!(pair["asset_1"]["symbol"].is_string());
    assert!(pair["is_active"].as_bool().unwrap());
}

#[tokio::test]
async fn get_pair_returns_pair() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["pair_address"], seed.pair_address);
    assert_eq!(body["asset_0"]["symbol"], "LUNC");
    assert_eq!(body["asset_1"]["symbol"], "USTC");
}

#[tokio::test]
async fn get_pair_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/pairs/terra1nonexistent")
        .await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn get_pair_candles_valid_interval() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=1h",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty(), "should have candle data");
    assert!(body[0]["open_time"].is_string());
    assert!(body[0]["open"].is_string());
}

#[tokio::test]
async fn get_pair_candles_invalid_interval_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=3h",
            seed.pair_address
        ))
        .await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn get_pair_candles_default_interval() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/candles", seed.pair_address))
        .await;
    resp.assert_status_ok();
}

#[tokio::test]
async fn get_pair_trades_returns_trades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/trades", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body[0]["tx_hash"].is_string());
    assert!(body[0]["sender"].is_string());
}

#[tokio::test]
async fn get_pair_trades_with_limit() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=2",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 2);
}

#[tokio::test]
async fn get_pair_trades_pagination() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/trades?limit=2", seed.pair_address))
        .await;
    resp.assert_status_ok();
    let page1: Vec<Value> = resp.json();
    assert_eq!(page1.len(), 2);

    let last_id = page1[1]["id"].as_i64().unwrap();
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=2&before={}",
            seed.pair_address, last_id
        ))
        .await;
    resp.assert_status_ok();
    let page2: Vec<Value> = resp.json();

    for trade in &page2 {
        assert!(trade["id"].as_i64().unwrap() < last_id);
    }
}

#[tokio::test]
async fn get_pair_stats_returns_stats() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/stats", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["volume_base"].is_string());
    assert!(body["trade_count"].is_i64());
}
