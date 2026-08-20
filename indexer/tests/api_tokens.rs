mod common;

use axum_test::TestServer;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::db::queries::volume;
use serde_json::Value;
use serial_test::serial;

#[tokio::test]
async fn list_tokens_returns_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
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
    let app = common::build_test_app(pool).await;
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
    let app = common::build_test_app(pool).await;
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
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/nonexistent").await;
    resp.assert_status_not_found();
}

#[tokio::test]
async fn list_tokens_rejects_excessive_offset() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens?offset=99999").await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn get_token_pairs() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/tokens/uluna/pairs").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty(), "LUNC should have at least one pair");
}

/// GitLab #577 **D1**: GET token volume_stats 24h is 0 after aging offer swaps past the cutoff.
#[serial]
#[tokio::test]
async fn get_token_volume_stats_24h_zeros_after_swaps_age() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh token volumes");

    sqlx::query("UPDATE swap_events SET block_timestamp = $1 WHERE pair_id = $2")
        .bind(Utc::now() - Duration::hours(25))
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .expect("age swaps");

    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh after age");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/tokens/uluna").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let stats = body["volume_stats"].as_array().expect("volume_stats");
    let w24 = stats
        .iter()
        .find(|s| s["window"] == "24h")
        .expect("24h window");
    assert_eq!(w24["volume"], "0");
    assert_eq!(w24["trade_count"].as_i64().unwrap(), 0);
    let w7 = stats
        .iter()
        .find(|s| s["window"] == "7d")
        .expect("7d window");
    let vol7: i128 = w7["volume"].as_str().unwrap_or("0").parse().unwrap_or(0);
    assert!(vol7 > 0, "25h swaps still count in 7d");
}
