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
    assert!(body["metadata"].as_str().unwrap().contains("vfdusd"));
    let tickers = body["tickers"].as_array().unwrap();
    assert_eq!(tickers.len(), 3);
    assert!(tickers.iter().any(|t| t == "ustc"));
    assert!(tickers.iter().any(|t| t == "lunc"));
    assert!(tickers.iter().any(|t| t == "vfdusd"));
    assert!(body.get("price_usd").is_none());
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
async fn oracle_price_fdusd_without_v_is_400() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/fdusd").await;
    assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn oracle_price_path_injection_is_400_or_not_found() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for path in [
        "/api/v1/oracle/price/..%2f",
        "/api/v1/oracle/price/ustc%00",
        "/api/v1/oracle/price/ustc%2f..%2f",
    ] {
        let resp = server.get(path).await;
        assert!(
            resp.status_code() == StatusCode::BAD_REQUEST
                || resp.status_code() == StatusCode::NOT_FOUND,
            "{path} => {}",
            resp.status_code()
        );
    }
}

#[tokio::test]
async fn oracle_price_vfdusd_returns_none_when_no_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/vfdusd").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "vfdusd");
    assert!(body["price_usd"].is_null());
}

#[tokio::test]
async fn oracle_price_vfdusd_returns_cached_value_not_hardcoded_peg() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let vfdusd = BigDecimal::from_str("0.87").unwrap();
    let app = common::build_test_app_with_vfdusd(pool, Some(vfdusd)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/vfdusd").await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert_eq!(body["ticker"], "vfdusd");
    let price_str = body["price_usd"].as_str().unwrap();
    assert!(
        price_str.starts_with("0.87"),
        "depeg must display, got {price_str}"
    );
    assert!(!price_str.starts_with("1.0"));
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
    assert!(tickers.iter().any(|t| t == "vfdusd"));
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

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at) VALUES ('vfdusd', 0.87, 'average', NOW())",
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

    let vfdusd_resp = server.get("/api/v1/oracle/history/vfdusd").await;
    assert_eq!(vfdusd_resp.status_code(), StatusCode::OK);
    let vfdusd_body: serde_json::Value = vfdusd_resp.json();
    assert_eq!(vfdusd_body["ticker"], "vfdusd");
    assert_eq!(vfdusd_body["prices"].as_array().unwrap().len(), 1);
    assert!(vfdusd_body["prices"][0]["price_usd"]
        .as_str()
        .unwrap()
        .starts_with("0.87"));
}

#[tokio::test]
async fn oracle_price_ustc_omits_venus() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/ustc").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["venus"].is_null());
}

#[tokio::test]
async fn oracle_price_vfdusd_includes_independent_venus_snapshot() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let cex = BigDecimal::from_str("0.87").unwrap();
    let venus = cl8y_dex_indexer::indexer::venus_vfdusd::VenusVfdusdSnapshot::new_now(
        BigDecimal::from_str("0.023").unwrap(),
    );
    let app = common::build_test_app_with_venus(pool, Some(cex), Some(venus)).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/vfdusd").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["price_usd"].as_str().unwrap().starts_with("0.87"));
    assert!(body["venus"]["fdusd_per_vfdusd"]
        .as_str()
        .unwrap()
        .starts_with("0.023"));
    assert_eq!(body["venus"]["source"], "venus_bsc");
    assert!(!body["venus"]["vtoken"].as_str().unwrap().contains("http"));
}

#[tokio::test]
async fn oracle_venus_route_requires_vfdusd_ticker() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let ok = server.get("/api/v1/oracle/price/vfdusd/venus").await;
    assert_eq!(ok.status_code(), StatusCode::OK);
    let body: serde_json::Value = ok.json();
    assert!(body["fdusd_per_vfdusd"].is_null());
    assert_eq!(body["source"], "venus_bsc");

    for path in [
        "/api/v1/oracle/price/ustc/venus",
        "/api/v1/oracle/price/lunc/venus",
        "/api/v1/oracle/price/fdusd/venus",
        "/api/v1/oracle/price/btc/venus",
    ] {
        let resp = server.get(path).await;
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST, "{path}");
    }
}

#[tokio::test]
async fn oracle_venus_outage_does_not_hide_cex() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let cex = BigDecimal::from_str("0.87").unwrap();
    let app = common::build_test_app_with_venus(pool, Some(cex), None).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/price/vfdusd").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(body["price_usd"].as_str().unwrap().starts_with("0.87"));
    assert!(body["venus"]["fdusd_per_vfdusd"].is_null());
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
    common::clean_db(&pool).await;
    let seed = common::seed_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!("/api/v1/pairs/{}/stats", seed.pair_address);
    let resp = server.get(&url).await;
    assert_eq!(resp.status_code(), StatusCode::OK);

    let body: serde_json::Value = resp.json();
    assert!(body.get("volume_usd").is_some());
}
