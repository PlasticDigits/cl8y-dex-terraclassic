mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[tokio::test]
async fn oracle_price_catalog_lists_tickers() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body["metadata"].as_str().unwrap().contains("ustc"));
    assert!(body["metadata"].as_str().unwrap().contains("lunc"));
    let tickers = body["tickers"].as_array().unwrap();
    assert_eq!(tickers.len(), 2);
    assert!(tickers.iter().any(|t| t == "ustc"));
    assert!(tickers.iter().any(|t| t == "lunc"));
}

#[tokio::test]
async fn oracle_price_ustc_returns_none_when_no_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/ustc").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "ustc");
    assert!(body["price_usd"].is_null());
    assert!(body["sources"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn oracle_price_ustc_returns_cached_value() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let price = BigDecimal::from_str("0.00512").unwrap();
    let app = common::build_test_app_with_price(pool, Some(price)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/ustc").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "ustc");
    let price_str = body["price_usd"].as_str().unwrap();
    assert!(price_str.starts_with("0.005"));
}

#[tokio::test]
async fn oracle_price_lunc_returns_cached_value() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let ustc = BigDecimal::from_str("0.00512").unwrap();
    let lunc = BigDecimal::from_str("0.00005024").unwrap();
    let app = common::build_test_app_with_prices(pool, Some(ustc), Some(lunc)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/lunc").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "lunc");
    let price_str = body["price_usd"].as_str().unwrap();
    assert!(price_str.starts_with("0.00005"));
}

#[tokio::test]
async fn oracle_price_unknown_ticker_is_400() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/btc").await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn oracle_history_catalog_lists_tickers() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    let tickers = body["tickers"].as_array().unwrap();
    assert!(tickers.iter().any(|t| t == "ustc"));
    assert!(tickers.iter().any(|t| t == "lunc"));
}

#[tokio::test]
async fn oracle_history_ustc_returns_empty_when_no_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history/ustc").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "ustc");
    assert!(body["prices"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn oracle_history_returns_stored_prices_per_ticker() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ('ustc', 0.00512, 'average', NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ('ustc', 0.00513, 'average', NOW() - interval '1 minute')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ('lunc', 0.00005024, 'average', NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let ustc_resp = server.get("/api/v1/oracle/history/ustc").await;
    assert_eq!(ustc_resp.status_code(), StatusCode::OK);
    let ustc_body: serde_json::Value = ustc_resp.json();
    assert_eq!(ustc_body["prices"].as_array().unwrap().len(), 2);

    let lunc_resp = server.get("/api/v1/oracle/history/lunc").await;
    assert_eq!(lunc_resp.status_code(), StatusCode::OK);
    let lunc_body: serde_json::Value = lunc_resp.json();
    assert_eq!(lunc_body["prices"].as_array().unwrap().len(), 1);
    assert!(lunc_body["prices"][0]["price_usd"]
        .as_str()
        .unwrap()
        .starts_with("0.00005"));
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
