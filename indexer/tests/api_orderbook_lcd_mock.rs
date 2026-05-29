//! `/cg/orderbook` and `/cmc/orderbook/*` against a wiremock LCD (no real chain).

mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::api::orderbook_sim;
use serde_json::Value;

fn parse_level_price(level: &[String; 2]) -> f64 {
    level[0].parse().expect("price string")
}

#[tokio::test]
async fn cg_orderbook_200_simulated_depth_matches_query() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=50")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 25);
    assert_eq!(asks.len(), 25);
    assert!(bids.len() <= 50);
    assert!(asks.len() <= 50);
}

#[tokio::test]
async fn cg_orderbook_depth_capped_at_100_with_lcd_mock() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=9999")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 50);
    assert_eq!(asks.len(), 50);
}

#[tokio::test]
async fn cg_orderbook_second_identical_request_hits_cache_not_lcd() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = "/cg/orderbook?ticker_id=LUNC_USTC&depth=20";
    server.get(url).await.assert_status_ok();
    let lcd_calls_after_first = mock
        .received_requests()
        .await
        .expect("mock server should expose request log")
        .len();
    server.get(url).await.assert_status_ok();
    let lcd_calls_after_second = mock.received_requests().await.expect("request log").len();
    assert_eq!(
        lcd_calls_after_first, lcd_calls_after_second,
        "orderbook cache should avoid extra LCD smart-queries on repeat pair+depth"
    );
}

#[tokio::test]
async fn cg_orderbook_includes_resting_limit_levels_when_mocked() {
    let mock = common::lcd_mock::start_hybrid_orderbook_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=20")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert!(
        bids.iter().any(|l| l[0].as_str() == Some("999999.5")),
        "merged book should include synthetic limit bid: {bids:?}"
    );
    assert!(
        asks.iter().any(|l| l[0].as_str() == Some("0.0000001")),
        "merged book should include synthetic limit ask: {asks:?}"
    );
}

#[tokio::test]
async fn cg_and_cmc_orderbook_levels_match_for_same_pair() {
    let mock = common::lcd_mock::start_hybrid_orderbook_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let cg: Value = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=15")
        .await
        .json();
    let cmc: Value = server.get("/cmc/orderbook/LUNC_USTC?depth=15").await.json();
    assert_eq!(cg["bids"], cmc["bids"]);
    assert_eq!(cg["asks"], cmc["asks"]);
}

#[tokio::test]
async fn cmc_orderbook_200_with_lcd_mock() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server.get("/cmc/orderbook/LUNC_USTC?depth=15").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 7);
    assert_eq!(asks.len(), 7);
}

#[tokio::test]
async fn cg_orderbook_bid_prices_decrease_ask_prices_increase() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=20")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let bids = body["bids"].as_array().unwrap();
    let asks = body["asks"].as_array().unwrap();
    assert_eq!(bids.len(), 10);
    assert_eq!(asks.len(), 10);

    let bid_prices: Vec<f64> = bids
        .iter()
        .map(|l| l[0].as_str().unwrap().parse().unwrap())
        .collect();
    for w in bid_prices.windows(2) {
        assert!(w[0] > w[1], "bids should be decreasing: {bid_prices:?}");
    }

    let ask_prices: Vec<f64> = asks
        .iter()
        .map(|l| l[0].as_str().unwrap().parse().unwrap())
        .collect();
    for w in ask_prices.windows(2) {
        assert!(w[0] < w[1], "asks should be increasing: {ask_prices:?}");
    }
}

#[tokio::test]
async fn cg_orderbook_with_db_fee_worse_than_zero_fee_baseline() {
    let r0 = 10_000_000_000u128;
    let r1 = 5_000_000_000_000_000u128;
    let depth = 10;
    let zero = orderbook_sim::walk_amm_book(r0, r1, depth, 0);
    // Seeded pair fee_bps = 30 (see common::seed_db).
    let fee = orderbook_sim::walk_amm_book(r0, r1, depth, 30);

    for i in 0..depth {
        let z: f64 = parse_level_price(&zero.bids[i]);
        let f: f64 = parse_level_price(&fee.bids[i]);
        assert!(f < z, "fee should worsen bid at {i}");
        let z: f64 = parse_level_price(&zero.asks[i]);
        let f: f64 = parse_level_price(&fee.asks[i]);
        assert!(f > z, "fee should worsen ask at {i}");
    }
}

#[tokio::test]
async fn cg_orderbook_depth_one_openware_split() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/cg/orderbook?ticker_id=LUNC_USTC&depth=1")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["bids"].as_array().unwrap().len(), 1);
    assert_eq!(body["asks"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cg_orderbook_default_depth_is_ten_per_side() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server.get("/cg/orderbook?ticker_id=LUNC_USTC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["bids"].as_array().unwrap().len(), 10);
    assert_eq!(body["asks"].as_array().unwrap().len(), 10);
}
