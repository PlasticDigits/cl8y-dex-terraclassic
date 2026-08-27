//! GitLab #685 — CG/CMC listing field truthfulness (TVL stamp, bid/ask, gems, freeze, ids).

mod common;

use std::collections::HashSet;
use std::str::FromStr;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::api::reset_aggregator_cache;
use cl8y_dex_indexer::indexer::asset_code_id_freeze::{
    replace_frozen_pair_addresses, snapshot_frozen_pair_addresses,
};
use cl8y_dex_indexer::indexer::defillama::COLUMBUS5_GEM_ADDRESSES;
use serde_json::Value;
use serial_test::serial;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn stamp_tvl(pool: &sqlx::PgPool, pair_id: i32, usd: &str) {
    sqlx::query(
        "INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (pair_id) DO UPDATE SET liquidity_usd = EXCLUDED.liquidity_usd",
    )
    .bind(pair_id)
    .bind(bd(usd))
    .execute(pool)
    .await
    .expect("stamp tvl");
}

async fn insert_reserves(pool: &sqlx::PgPool, pair_id: i32, r0: &str, r1: &str) {
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, $2, $3, 30, NOW())
         ON CONFLICT (pair_id) DO UPDATE SET
            reserve_0 = EXCLUDED.reserve_0,
            reserve_1 = EXCLUDED.reserve_1,
            fee_bps = EXCLUDED.fee_bps,
            snapshot_at = NOW()",
    )
    .bind(pair_id)
    .bind(bd(r0))
    .bind(bd(r1))
    .execute(pool)
    .await
    .expect("reserves");
}

fn ticker_for<'a>(body: &'a [Value], pool_id: &str) -> &'a Value {
    body.iter()
        .find(|t| t["pool_id"] == pool_id)
        .unwrap_or_else(|| panic!("missing pool_id {pool_id}"))
}

fn assert_liq_eq(row: &Value, expected: &str) {
    let got = BigDecimal::from_str(row["liquidity_in_usd"].as_str().unwrap()).unwrap();
    assert_eq!(
        got,
        bd(expected),
        "liquidity_in_usd={}",
        row["liquidity_in_usd"]
    );
}

#[serial]
#[tokio::test]
async fn cg_tickers_liquidity_is_tvl_stamp_not_volume() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();

    sqlx::query("UPDATE swap_events SET volume_usd = 999999 WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .expect("wash volume");
    stamp_tvl(&pool, seed.pair_id, "42.5").await;
    insert_reserves(&pool, seed.pair_id, "1000000", "1000000").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Vec<Value> = server.get("/cg/tickers").await.json();
    let row = ticker_for(&body, &seed.pair_address);
    assert_liq_eq(row, "42.5");
    assert_ne!(
        BigDecimal::from_str(row["liquidity_in_usd"].as_str().unwrap()).unwrap(),
        bd("999999")
    );
    assert_ne!(row["base_volume"], "0");

    let last = BigDecimal::from_str(row["last_price"].as_str().unwrap()).unwrap();
    let bid = BigDecimal::from_str(row["bid"].as_str().unwrap()).unwrap();
    let ask = BigDecimal::from_str(row["ask"].as_str().unwrap()).unwrap();
    assert!(
        bid <= last && last <= ask,
        "bid={bid} last={last} ask={ask}"
    );
    assert!(!row["bid"].as_str().unwrap().contains("0.999"));
}

#[serial]
#[tokio::test]
async fn cg_tickers_unpriced_is_zero_not_volume() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();
    sqlx::query("UPDATE swap_events SET volume_usd = 777 WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Vec<Value> = server.get("/cg/tickers").await.json();
    let row = ticker_for(&body, &seed.pair_address);
    assert_eq!(row["liquidity_in_usd"], "0");
}

#[serial]
#[tokio::test]
async fn cg_tickers_idle_priced_pool_has_tvl() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .expect("idle");
    stamp_tvl(&pool, seed.pair_id, "1000").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Vec<Value> = server.get("/cg/tickers").await.json();
    let row = ticker_for(&body, &seed.pair_address);
    assert_liq_eq(row, "1000");
    assert_eq!(row["base_volume"], "0");
}

#[serial]
#[tokio::test]
async fn cmc_summary_bid_ask_not_toy_spread() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();
    insert_reserves(&pool, seed.pair_id, "1000000", "1000000").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Vec<Value> = server.get("/cmc/summary").await.json();
    let row = body
        .iter()
        .find(|r| r["trading_pairs"] == "LUNC_USTC")
        .expect("summary row");
    assert!(row.get("liquidity_in_usd").is_none());
    let last = BigDecimal::from_str(row["last_price"].as_str().unwrap()).unwrap();
    let bid = BigDecimal::from_str(row["highest_bid"].as_str().unwrap()).unwrap();
    let ask = BigDecimal::from_str(row["lowest_ask"].as_str().unwrap()).unwrap();
    assert!(bid <= last && last <= ask);
}

#[serial]
#[tokio::test]
async fn cmc_ticker_frozen_and_numeric_ids() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();
    sqlx::query("UPDATE assets SET cmc_id = 4172 WHERE id = $1")
        .bind(seed.asset_0_id)
        .execute(&pool)
        .await
        .expect("cmc id");

    let prev = snapshot_frozen_pair_addresses();
    replace_frozen_pair_addresses(HashSet::from([seed.pair_address.clone()]));

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/cmc/ticker").await.json();
    let row = &body["LUNC_USTC"];
    assert_eq!(row["isFrozen"], "1");
    assert!(row["base_id"].is_number());
    assert_eq!(row["base_id"], 4172);
    assert_eq!(row["quote_id"], 0);
    assert_eq!(row["cl8y_base_address"], "uluna");
    replace_frozen_pair_addresses(prev);
}

#[serial]
#[tokio::test]
async fn gems_omitted_from_cg_cmc_lists() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();

    let gem_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'EMBER', 'EMBER', 6) RETURNING id",
    )
    .bind(COLUMBUS5_GEM_ADDRESSES[0])
    .fetch_one(&pool)
    .await
    .expect("gem asset");
    let gem_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, fee_bps)
         VALUES ('terra1gempair', $1, $2, 30) RETURNING id",
    )
    .bind(gem_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("gem pair");
    stamp_tvl(&pool, gem_pair, "50").await;

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let pairs: Vec<Value> = server.get("/cg/pairs").await.json();
    assert!(pairs.iter().all(|p| p["pool_id"] != "terra1gempair"));
    assert!(pairs.iter().any(|p| p["pool_id"] == seed.pair_address));

    let tickers: Vec<Value> = server.get("/cg/tickers").await.json();
    assert!(tickers.iter().all(|p| p["pool_id"] != "terra1gempair"));

    let summary: Vec<Value> = server.get("/cmc/summary").await.json();
    assert!(summary.iter().all(|p| p["trading_pairs"] != "EMBER_USTC"));

    let ticker: Value = server.get("/cmc/ticker").await.json();
    assert!(ticker.get("EMBER_USTC").is_none());

    let assets: Value = server.get("/cmc/assets").await.json();
    assert!(assets.get("EMBER").is_none());
}

#[serial]
#[tokio::test]
async fn duplicate_ticker_id_does_not_bind_wrong_pair() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();

    let _: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, fee_bps)
         VALUES ('terra1otherluncustc', $1, $2, 30) RETURNING id",
    )
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("dup pair");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC")
        .await;
    resp.assert_status_not_found();

    let pairs: Vec<Value> = server.get("/cg/pairs").await.json();
    let lunc_ustc: Vec<_> = pairs
        .iter()
        .filter(|p| p["ticker_id"] == "LUNC_USTC")
        .collect();
    assert_eq!(lunc_ustc.len(), 2);
    assert_ne!(lunc_ustc[0]["pool_id"], lunc_ustc[1]["pool_id"]);
}

#[serial]
#[tokio::test]
async fn cmc_assets_prefers_economic_pin_on_symbol_collision() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    reset_aggregator_cache();

    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1spoofcl8y', true, 'Spoof', 'CL8Y', 6)",
    )
    .execute(&pool)
    .await
    .expect("spoof");
    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'CL8Y', 'CL8Y', 6)",
    )
    .bind(cl8y_dex_indexer::config::DEFAULT_HUB_CL8Y_ADDRESS)
    .execute(&pool)
    .await
    .expect("real cl8y");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let assets: Value = server.get("/cmc/assets").await.json();
    assert_eq!(assets["CL8Y"]["name"], "CL8Y");
}

#[serial]
#[tokio::test]
async fn historical_trades_buy_is_offer_quote() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 9000, $2, 'buy_quote_offer', $3, $4, $5, 100, 90, 1.1)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now())
    .bind(&seed.trader_address)
    .bind(seed.asset_1_id)
    .bind(seed.asset_0_id)
    .execute(&pool)
    .await
    .expect("buy swap");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&type=buy")
        .await
        .json();
    let buys = body["buy"].as_array().unwrap();
    assert!(!buys.is_empty());
    assert!(body["sell"].as_array().unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn cg_tickers_offset_over_max_is_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    server
        .get("/cg/tickers?offset=10001")
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn cg_tickers_cache_returns_same_tvl_stamp() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    reset_aggregator_cache();
    stamp_tvl(&pool, seed.pair_id, "88").await;

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let first: Vec<Value> = server.get("/cg/tickers").await.json();
    stamp_tvl(&pool, seed.pair_id, "1").await;
    let second: Vec<Value> = server.get("/cg/tickers").await.json();
    let a = ticker_for(&first, &seed.pair_address);
    let b = ticker_for(&second, &seed.pair_address);
    assert_liq_eq(a, "88");
    assert_liq_eq(b, "88");
}
