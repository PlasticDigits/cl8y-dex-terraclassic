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

async fn seed_quorum(pool: &sqlx::PgPool, ticker: &str, price: &str) {
    sqlx::query(
        "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at)
        VALUES($1,$2,'fresh',ARRAY['mexc','kucoin'],NOW(),NOW())
        ON CONFLICT(ticker) DO UPDATE SET price_usd=EXCLUDED.price_usd,
          quality='fresh',accepted_sources=EXCLUDED.accepted_sources,as_of=NOW(),observed_at=NOW()",
    )
    .bind(ticker)
    .bind(bd(price))
    .execute(pool)
    .await
    .unwrap();
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
    assert_eq!(tickers.len(), 4);
    assert_eq!(tickers[0], "custc");
    assert_eq!(tickers[1], "lunc");
    assert_eq!(tickers[2], "ust1");
    assert_eq!(tickers[3], "ustr");
    assert_eq!(body["prices"].as_array().unwrap().len(), 4);
    for p in body["prices"].as_array().unwrap() {
        assert!(p["price_usd"].is_null());
    }
    let custc_p = body["prices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["ticker"] == "custc")
        .unwrap();
    assert_eq!(
        custc_p["asset_address"].as_str().unwrap(),
        DEFAULT_HUB_CUSTC_ADDRESS
    );
    assert!(custc_p["source_pair"].is_null());
    let lunc_p = body["prices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["ticker"] == "lunc")
        .unwrap();
    assert_eq!(
        lunc_p["asset_address"].as_str().unwrap(),
        cl8y_dex_indexer::config::DEFAULT_HUB_CLUNC_ADDRESS
    );
    assert!(lunc_p["source_pair"].is_null());
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
        "/api/v1/hub-prices/clunc",
        "/api/v1/hub-prices/cl8y",
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
async fn hub_refresh_canonical_ust1_pool_and_overview_fields() {
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
    let spoof: i32 = sqlx::query_scalar(
        "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
        VALUES('terra1spoofust1',true,'UST1','UST1',6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let thin: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1thin556', $1, $2, 'terra1lpthin', 30) RETURNING id",
    )
    .bind(spoof)
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

    // Complete cold API setup before starting the owned reserve observation's
    // unchanged twenty-second lifetime. The app certifies its separate upstream
    // oracle handle; that must not make an older pool observation fresh.
    cl8y_dex_indexer::api::reset_overview_cache();
    let app = common::build_test_app_with_price(pool.clone(), Some(bd("0.005"))).await;
    let server = TestServer::new(app);

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

    seed_quorum(&pool, "ustc", "0.005").await;
    let mut cfg = common::test_config();
    cfg.book_snapshot_interval_ms = 10_000;
    let hub_cfg = HubUsdConfig::from_indexer_config(&cfg);
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .expect("refresh");
    assert!(
        hub_prices::get_hub_price(&pool, cl8y_dex_indexer::indexer::hub_usd::HubTicker::Ust1)
            .await
            .unwrap()
            .is_none(),
        "bootstrap is observing, never immediate healthy"
    );
    // Fixture establishes a prior canonical observation without a real-time sleep.
    sqlx::query(
        "UPDATE hub_price_references SET pending_since = pending_since - INTERVAL '61 seconds'",
    )
    .execute(&pool)
    .await
    .unwrap();
    cl8y_dex_indexer::db::queries::volume::refresh_global_stats(&pool)
        .await
        .expect("global stats rollup");
    // This is a second original observation of the same owned, unchanged pool,
    // after cold rollups. It does not rewrite an admitted mark's timestamps.
    let confirmed_source_at = Utc::now();
    sqlx::query("UPDATE pair_reserves SET snapshot_at=$3 WHERE pair_id IN ($1,$2)")
        .bind(thin)
        .bind(deep)
        .bind(confirmed_source_at)
        .execute(&pool)
        .await
        .unwrap();
    let confirmed = hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .unwrap();
    assert_eq!(
        confirmed
            .ust1
            .as_ref()
            .unwrap()
            .as_of
            .unwrap()
            .timestamp_micros(),
        confirmed_source_at.timestamp_micros(),
        "scheduled mark keeps the original second reserve observation"
    );

    let body: serde_json::Value = server.get("/api/v1/hub-prices").await.json();
    let prices = body["prices"].as_array().unwrap();
    let custc_p = prices.iter().find(|p| p["ticker"] == "custc").unwrap();
    let custc_usd = custc_p["price_usd"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((custc_usd - 0.005).abs() < 1e-9, "custc usd {custc_usd}");
    assert_eq!(
        custc_p["asset_address"].as_str().unwrap(),
        DEFAULT_HUB_CUSTC_ADDRESS
    );
    assert!(custc_p["source_pair"].is_null());
    let lunc_p = prices.iter().find(|p| p["ticker"] == "lunc").unwrap();
    assert!(lunc_p["price_usd"].is_null(), "lunc oracle was not passed");
    assert!(lunc_p["source_pair"].is_null());
    let ust1_p = prices.iter().find(|p| p["ticker"] == "ust1").unwrap();
    let u = ust1_p["price_usd"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!((u - 1.0).abs() < 1e-9, "ust1 usd {u}");
    assert_eq!(ust1_p["source_pair"], "terra1deep556");
    assert_eq!(ust1_p["quality"], "bounded_spot");
    assert_eq!(ust1_p["oracle_grade"], false);
    assert_eq!(ust1_p["policy"], "hub-fixed-reference-v1");
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(ust1_p["as_of"].as_str().unwrap())
            .unwrap()
            .timestamp_micros(),
        confirmed_source_at.timestamp_micros(),
        "API must not renew the producer's original observation time"
    );
    assert_eq!(
        ust1_p["priced_leg_depth_usd"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap(),
        250.0
    );
    assert!(prices.iter().find(|p| p["ticker"] == "ustr").unwrap()["price_usd"].is_null());

    let one: serde_json::Value = server.get("/api/v1/hub-prices/ust1").await.json();
    assert_eq!(one["ticker"], "ust1");
    assert!(one["price_usd"].as_str().unwrap().starts_with("1"));

    let ov: serde_json::Value = server.get("/api/v1/overview").await.json();
    assert!(ov["ustc_price_usd"].as_str().unwrap().starts_with("0.005"));
    assert!(ov["custc_price_usd"].as_str().unwrap().starts_with("0.005"));
    assert!(ov["ust1_price_usd"].as_str().unwrap().starts_with("1"));
    assert!(ov["ustr_price_usd"].is_null());

    // Manipulate the actual canonical pair reserve ratio, never a duplicate pool.
    sqlx::query("UPDATE pair_reserves SET reserve_0=125000000,snapshot_at=NOW() WHERE pair_id=$1")
        .bind(deep)
        .execute(&pool)
        .await
        .unwrap();
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .unwrap();
    let rejected: serde_json::Value = server.get("/api/v1/hub-prices/ust1").await.json();
    assert!(rejected["price_usd"].is_null());
    assert_eq!(rejected["quality"], "divergent");
    assert_eq!(
        rejected["divergence_pct"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap(),
        100.0
    );
    assert!(hub_prices::load_quote_usd(&pool)
        .await
        .unwrap()
        .ust1
        .is_none());
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, None, None)
        .await
        .unwrap();
    let baseline: BigDecimal =
        sqlx::query_scalar("SELECT reference_ratio FROM hub_price_references WHERE ticker='ust1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        baseline,
        bd("200"),
        "outage must not reset reference toward attack"
    );

    // Restore canonical reserves; independent depth below $100 still cannot publish.
    sqlx::query("UPDATE pair_reserves SET reserve_0=50000000,reserve_1=10000000000,snapshot_at=NOW() WHERE pair_id=$1")
        .bind(deep).execute(&pool).await.unwrap();
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .unwrap();
    assert!(hub_prices::load_quote_usd(&pool)
        .await
        .unwrap()
        .ust1
        .is_none());
    sqlx::query("UPDATE pair_reserves SET reserve_0=250000000,reserve_1=50000000000,snapshot_at=NOW() WHERE pair_id=$1")
        .bind(deep).execute(&pool).await.unwrap();
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .unwrap();
    assert!(hub_prices::load_quote_usd(&pool)
        .await
        .unwrap()
        .ust1
        .is_some());

    // New no-quorum state immediately invalidates a still-fresh materialized mark.
    sqlx::query("UPDATE oracle_state SET quality='degraded',price_usd=NULL,accepted_sources='{}',as_of=NULL WHERE ticker='ustc'")
        .execute(&pool).await.unwrap();
    assert!(hub_prices::load_quote_usd(&pool)
        .await
        .unwrap()
        .ust1
        .is_none());
    let degraded: serde_json::Value = server.get("/api/v1/hub-prices/ust1").await.json();
    assert!(degraded["price_usd"].is_null());
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
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, None, None)
        .await
        .expect("refresh");
    let rows = hub_prices::get_all_hub_prices(&pool).await.unwrap();
    assert!(rows.is_empty(), "must not freeze last peg");
}

#[serial]
#[tokio::test]
async fn hub_lunc_survives_ustc_outage_and_does_not_use_ustc_price() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    sqlx::query("INSERT INTO hub_prices (ticker, price_usd, updated_at) VALUES ('ust1', 1, NOW())")
        .execute(&pool)
        .await
        .unwrap();
    let hub_cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    seed_quorum(&pool, "lunc", "0.00008").await;
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, None, Some(&bd("0.00008")))
        .await
        .expect("refresh");

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/hub-prices").await.json();
    let prices = body["prices"].as_array().unwrap();
    let lunc = prices.iter().find(|p| p["ticker"] == "lunc").unwrap();
    let lunc_usd = lunc["price_usd"].as_str().unwrap().parse::<f64>().unwrap();
    assert!((lunc_usd - 0.00008).abs() < 1e-12);
    assert!(lunc["source_pair"].is_null());
    assert_eq!(
        lunc["asset_address"].as_str().unwrap(),
        cl8y_dex_indexer::config::DEFAULT_HUB_CLUNC_ADDRESS
    );
    assert!(prices.iter().find(|p| p["ticker"] == "custc").unwrap()["price_usd"].is_null());
    assert!(prices.iter().find(|p| p["ticker"] == "ust1").unwrap()["price_usd"].is_null());

    let one: serde_json::Value = server.get("/api/v1/hub-prices/lunc").await.json();
    assert_eq!(one["ticker"], "lunc");
    assert!(one["price_usd"].as_str().unwrap().starts_with("0.00008"));

    let cex = server.get("/api/v1/oracle/price/lunc").await;
    assert_eq!(cex.status_code(), StatusCode::OK);
}

#[serial]
#[tokio::test]
async fn hub_lunc_oracle_down_does_not_zero_custc() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let hub_cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    seed_quorum(&pool, "ustc", "0.005").await;
    hub_prices::refresh_hub_prices(&pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .expect("refresh");
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/hub-prices").await.json();
    let prices = body["prices"].as_array().unwrap();
    assert!(prices.iter().find(|p| p["ticker"] == "lunc").unwrap()["price_usd"].is_null());
    let custc = prices.iter().find(|p| p["ticker"] == "custc").unwrap();
    let usd = custc["price_usd"].as_str().unwrap().parse::<f64>().unwrap();
    assert!((usd - 0.005).abs() < 1e-9);
    assert!(custc["source_pair"].is_null());
}
