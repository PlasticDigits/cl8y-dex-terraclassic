mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn list_tokens_returns_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() >= 2, "should have at least two tokens");
    assert!(body[0]["symbol"].is_string());
    assert!(body[0]["decimals"].is_i64());
}

#[tokio::test]
async fn get_token_by_denom() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/uluna").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["token"]["symbol"], "LUNC");
    assert!(body["volume_stats"].is_array());
}

#[tokio::test]
async fn get_token_by_contract() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/terra1ustctoken").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["token"]["symbol"], "USTC");
}

#[tokio::test]
async fn get_token_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/nonexistent").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn get_token_pairs() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/uluna/pairs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty(), "LUNC should have at least one pair");
}
