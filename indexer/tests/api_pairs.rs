mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn list_pairs_returns_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    let items = body["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "should have at least one pair");
    assert!(body["total"].as_i64().unwrap() >= 1);
    assert_eq!(body["limit"].as_i64().unwrap(), 50);
    assert_eq!(body["offset"].as_i64().unwrap(), 0);

    let pair = &items[0];
    assert_eq!(pair["pair_address"], seed.pair_address);
    assert!(pair["asset_0"]["symbol"].is_string());
    assert!(pair["asset_1"]["symbol"].is_string());
    assert!(pair["is_active"].as_bool().unwrap());
    assert!(pair["volume_quote_24h"].is_string());

    // Pagination, sort, search (same server / DB to avoid parallel seed conflicts)
    let resp = server
        .get("/api/v1/pairs?limit=1&offset=0&sort=id&order=asc")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert!(body["total"].as_i64().unwrap() >= 1);

    let resp = server.get("/api/v1/pairs?sort=volume_24h&order=desc").await;
    resp.assert_status_ok();

    let resp = server.get("/api/v1/pairs?q=LUNC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(!body["items"].as_array().unwrap().is_empty());

    let resp = server.get("/api/v1/pairs?sort=bad_sort").await;
    resp.assert_status_bad_request();
}

#[serial]
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

#[serial]
#[tokio::test]
async fn get_pair_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs/terra1nonexistent").await;
    resp.assert_status_not_found();
}

#[serial]
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

#[serial]
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

#[serial]
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

#[serial]
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

#[serial]
#[tokio::test]
async fn get_pair_limit_fills_returns_indexed_fills() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, NULL, 1001, NOW(), $2, 7, 'bid', 'terra1maker', 1.5, 100, 150, 1)",
    )
    .bind(seed.pair_id)
    .bind("lofilltx0001")
    .execute(&pool)
    .await
    .expect("insert limit_order_fills");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/limit-fills", seed.pair_address))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["order_id"], 7);
    assert_eq!(body[0]["side"], "bid");
    assert_eq!(body[0]["maker"], "terra1maker");
}

#[serial]
#[tokio::test]
async fn get_pair_order_limit_fills_filters_by_order_id() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    for (tid, oid) in [("lofilltx_a", 1i64), ("lofilltx_b", 2i64)] {
        sqlx::query(
            "INSERT INTO limit_order_fills
             (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
             VALUES ($1, NULL, 1002, NOW(), $2, $3, 'ask', 'terra1mk', 2, 10, 20, 0)",
        )
        .bind(seed.pair_id)
        .bind(tid)
        .bind(oid)
        .execute(&pool)
        .await
        .expect("insert limit_order_fills");
    }

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-orders/2/fills",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["order_id"], 2);
}

#[serial]
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

#[serial]
#[tokio::test]
async fn get_pair_trades_pagination() {
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

#[serial]
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

#[serial]
#[tokio::test]
async fn get_pair_liquidity_events_returns_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/liquidity-events",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["event_type"], "add");
    assert!(body[0]["lp_amount"].is_string());
}

#[serial]
#[tokio::test]
async fn get_pair_limit_placements_and_cancellations() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let p: Vec<Value> = resp.json();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0]["order_id"], 7);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-cancellations",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let c: Vec<Value> = resp.json();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0]["order_id"], 7);
}
