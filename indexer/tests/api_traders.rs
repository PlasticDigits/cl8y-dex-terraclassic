mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

// Shared `dex_indexer_test` DB: serialize seed_db + HTTP assertions (see api_pairs.rs).
#[serial]
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

#[serial]
#[tokio::test]
async fn get_trader_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/terra1unknown").await;
    resp.assert_status_not_found();
}

#[serial]
#[tokio::test]
async fn get_trader_trades_returns_trades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/traders/{}/trades", seed.trader_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    for trade in &body {
        assert_eq!(trade["sender"], seed.trader_address);
    }
}

#[serial]
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

#[serial]
#[tokio::test]
async fn leaderboard_valid_sort_columns() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for sort in &[
        "total_volume",
        "volume_24h",
        "volume_7d",
        "volume_30d",
        "total_trades",
    ] {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_ok();
    }
}

#[serial]
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

#[serial]
#[tokio::test]
async fn leaderboard_limit_capped() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/leaderboard?limit=999").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[serial]
#[tokio::test]
async fn get_trader_trades_pair_filter_returns_subset() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    for trade in &body {
        assert_eq!(trade["sender"], seed.trader_address);
        assert_eq!(trade["pair_address"], seed.pair_address);
    }
}

#[serial]
#[tokio::test]
async fn get_trader_trades_unknown_pair_returns_404() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?pair=terra1nosuchpairxxxxxxxx",
            seed.trader_address
        ))
        .await;
    resp.assert_status_not_found();
}

#[serial]
#[tokio::test]
async fn get_trader_trades_csv_returns_text_csv() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?format=csv&limit=3",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let content_type = resp.header("content-type");
    let ct = content_type.to_str().unwrap_or("");
    assert!(ct.contains("text/csv"), "unexpected content-type: {ct}");
    let body = resp.text();
    assert!(body.contains("id,pair_address,block_height"));
    assert!(body.contains("tx_hash"));
}

#[serial]
#[tokio::test]
async fn get_trader_limit_fills_returns_maker_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-fills?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 2);
    for row in &body {
        assert_eq!(row["maker"], seed.trader_address);
        assert_eq!(row["pair_address"], seed.pair_address);
    }
}

#[serial]
#[tokio::test]
async fn get_trader_limit_cancellations_returns_owner_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-cancellations?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["owner"], seed.trader_address);
    assert_eq!(body[0]["order_id"], 7);
}

#[serial]
#[tokio::test]
async fn get_trader_limit_placements_returns_owner_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-placements",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["owner"], seed.trader_address);
    assert_eq!(body[0]["order_id"], 8);
    assert_eq!(body[0]["pair_address"], seed.pair_address);
    assert_eq!(body[0]["lifecycle_status"], "active");
}

#[serial]
#[tokio::test]
async fn get_trader_limit_placements_bad_status_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-placements?status=invalid",
            seed.trader_address
        ))
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn get_trader_positions_returns_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO trader_positions
         (trader_address, pair_id, net_position_quote, avg_entry_price, total_cost_base, realized_pnl, trade_count)
         VALUES ($1, $2, 100, 0.5, 50, 0, 1)",
    )
    .bind(&seed.trader_address)
    .bind(seed.pair_id)
    .execute(&pool)
    .await
    .expect("insert position");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/positions",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["pair_address"], seed.pair_address);
    assert!(body[0]["net_position_quote"].is_string());
    assert_eq!(body[0]["asset_0_decimals"], 6);
    assert_eq!(body[0]["asset_1_decimals"], 6);
    assert_eq!(body[0]["asset_0_symbol"], "LUNC");
    assert_eq!(body[0]["asset_1_symbol"], "USTC");
    assert_eq!(body[0]["asset_0_denom"], "uluna");
    assert!(body[0]["asset_1_denom"].is_null() || body[0].get("asset_1_denom").is_none());
}
