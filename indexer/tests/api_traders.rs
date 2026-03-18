mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn get_trader_profile_returns_trader() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/traders/{}", seed.trader_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["address"], seed.trader_address);
    assert!(body["total_trades"].is_i64());
    assert!(body["total_volume"].is_string());
}

#[tokio::test]
async fn get_trader_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/terra1unknown").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn get_trader_trades_returns_trades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    for trade in &body {
        assert_eq!(trade["sender"], seed.trader_address);
    }
}

#[tokio::test]
async fn leaderboard_default_sort() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/leaderboard").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
}

#[tokio::test]
async fn leaderboard_valid_sort_columns() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for sort in &["total_volume", "volume_24h", "volume_7d", "volume_30d", "total_trades"] {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_ok();
    }
}

#[tokio::test]
async fn leaderboard_invalid_sort_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/traders/leaderboard?sort=hacked_column")
        .await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn leaderboard_limit_capped() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/traders/leaderboard?limit=999")
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}
