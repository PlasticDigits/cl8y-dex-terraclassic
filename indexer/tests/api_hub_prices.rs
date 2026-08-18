//! GitLab #556 — DEX hub-prices API (not CEX oracle).

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::config::{DEFAULT_HUB_CUSTC_ADDRESS, DEFAULT_HUB_UST1_ADDRESS};
use cl8y_dex_indexer::db::queries::hub_prices;
use cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[tokio::test]
async fn hub_prices_catalog_is_dex_not_cex() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/hub-prices").await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    let meta = body["metadata"].as_str().unwrap();
    assert!(meta.contains("DEX"));
    assert!(meta.contains("not CEX"));
    assert!(meta.contains("not settlement"));
    let tickers = body["tickers"].as_array().unwrap();
    assert_eq!(tickers.len(), 3);
    assert_eq!(tickers[0], "custc");
    assert_eq!(body["prices"].as_array().unwrap().len(), 3);
    for p in body["prices"].as_array().unwrap() {
        assert!(p["price_usd"].is_null());
    }
}

#[tokio::test]
async fn hub_prices_unknown_ticker_is_400() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for path in [
        "/api/v1/hub-prices/ustc",
        "/api/v1/hub-prices/ustr_",
        "/api/v1/hub-prices/javascript:alert(1)",
        "/api/v1/hub-prices/vfdusd",
    ] {
        let resp = server.get(path).await;
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST, "{path}");
    }
    // Axum does not bind `{ticker}` across `/../`; traversal must not succeed as ustr.
    let traversal = server.get("/api/v1/hub-prices/../ustr").await;
    assert_ne!(
        traversal.status_code(),
        StatusCode::OK,
        "path traversal must not return a hub mark"
    );
}

#[tokio::test]
async fn oracle_price_still_rejects_hub_tickers() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    for t in ["ustr", "ust1", "custc"] {
        let resp = server.get(&format!("/api/v1/oracle/price/{t}")).await;
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST, "{t}");
    }
}

#[serial]
#[tokio::test]
async fn hub_refresh_picks_deepest_ust1_pool_and_overview_fields() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let custc: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'cUSTC', 'cUSTC', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_CUSTC_ADDRESS)
    .fetch_one(&pool)
    .await
    .unwrap();
    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_UST1_ADDRESS)
    .fetch_one(&pool)
    .await
    .unwrap();
    let thin: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1thin556', $1, $2, 'terra1lpthin', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(custc)
    .fetch_one(&pool)
    .await
    .unwrap();
    let deep: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1deep556', $1, $2, 'terra1lpdeep', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(custc)
    .fetch_one(&pool)
    .await
    .unwrap();

    let now = Utc::now();
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 10000000, 10000000, 30, $3), ($2, 250000000, 50000000000, 30, $3)",
    )
    .bind(thin)
    .bind(deep)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ('ustc', 0.005, 'average', NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut cfg = common::test_config();
    cfg.book_snapshot_interval_ms = 10_000;
    let hub_cfg = HubUsdConfig::from_indexer_config(&cfg);
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")))
        .await
        .expect("refresh");
    cl8y_dex_indexer::db::queries::volume::refresh_global_stats(&pool)
        .await
        .expect("global stats rollup");

    cl8y_dex_indexer::api::reset_overview_cache();
    let app = common::build_test_app_with_price(pool.clone(), Some(bd("0.005"))).await;
    let server = TestServer::new(app);

    let body: serde_json::Value = server.get("/api/v1/hub-prices").await.json();
    let prices = body["prices"].as_array().unwrap();
    let custc_p = prices.iter().find(|p| p["ticker"] == "custc").unwrap();
    let custc_usd = custc_p["price_usd"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((custc_usd - 0.005).abs() < 1e-9, "custc usd {custc_usd}");
    let ust1_p = prices.iter().find(|p| p["ticker"] == "ust1").unwrap();
    let u = ust1_p["price_usd"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((u - 1.0).abs() < 1e-9, "ust1 usd {u}");
    assert_eq!(ust1_p["source_pair"], "terra1deep556");
    assert!(prices.iter().find(|p| p["ticker"] == "ustr").unwrap()["price_usd"].is_null());

    let one: serde_json::Value = server.get("/api/v1/hub-prices/ust1").await.json();
    assert_eq!(one["ticker"], "ust1");
    assert!(one["price_usd"].as_str().unwrap().starts_with("1"));

    let ov: serde_json::Value = server.get("/api/v1/overview").await.json();
    assert!(ov["ustc_price_usd"].as_str().unwrap().starts_with("0.005"));
    assert!(ov["custc_price_usd"].as_str().unwrap().starts_with("0.005"));
    assert!(ov["ust1_price_usd"].as_str().unwrap().starts_with("1"));
    assert!(ov["ustr_price_usd"].is_null());
}

#[serial]
#[tokio::test]
async fn hub_refresh_oracle_down_clears_marks() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    sqlx::query("INSERT INTO hub_prices (ticker, price_usd, updated_at) VALUES ('ust1', 1, NOW())")
        .execute(&pool)
        .await
        .unwrap();
    let hub_cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, None)
        .await
        .expect("refresh");
    let rows = hub_prices::get_all_hub_prices(&pool).await.unwrap();
    assert!(rows.is_empty(), "must not freeze last peg");
}
