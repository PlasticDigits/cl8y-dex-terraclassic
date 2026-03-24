mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[tokio::test]
async fn oracle_price_returns_none_when_no_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body["price_usd"].is_null());
    assert!(body["sources"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn oracle_price_returns_cached_value() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let price = BigDecimal::from_str("0.00512").unwrap();
    let app = common::build_test_app_with_price(pool, Some(price)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    let price_str = body["price_usd"].as_str().unwrap();
    assert!(price_str.starts_with("0.005"));
}

#[tokio::test]
async fn oracle_history_returns_empty_when_no_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body["prices"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn oracle_history_returns_stored_prices() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    sqlx::query(
        "INSERT INTO ustc_prices (price_usd, source, fetched_at) VALUES (0.00512, 'average', NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO ustc_prices (price_usd, source, fetched_at) VALUES (0.00513, 'average', NOW() - interval '1 minute')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    let prices = body["prices"].as_array().unwrap();
    assert_eq!(prices.len(), 2);
}

#[tokio::test]
async fn overview_includes_usd_fields() {
    let pool = common::setup_pool().await;
    let _seed = common::seed_db(&pool).await;

    let price = BigDecimal::from_str("0.00512").unwrap();
    let app = common::build_test_app_with_price(pool, Some(price)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/overview").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body.get("total_volume_24h_usd").is_some());
    assert!(body.get("ustc_price_usd").is_some());
    assert!(body["ustc_price_usd"]
        .as_str()
        .unwrap()
        .starts_with("0.005"));
}

#[tokio::test]
async fn pair_stats_includes_volume_usd() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!("/api/v1/pairs/{}/stats", seed.pair_address);
    let resp = server.get(&url).await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body.get("volume_usd").is_some());
}
