//! GitLab #557 — additive offer/ask/token decimals on trade + fill JSON/CSV.

mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn pair_trades_include_offer_ask_decimals_from_assets() {
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
    assert_eq!(body[0]["offer_decimals"], 6);
    assert_eq!(body[0]["ask_decimals"], 6);
    assert_eq!(body[0]["offer_amount"], "1000");
}

#[serial]
#[tokio::test]
async fn trader_trades_same_decimals_shape_as_pair_trades() {
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
    assert_eq!(body[0]["offer_decimals"], 6);
    assert_eq!(body[0]["ask_decimals"], 6);
}

#[serial]
#[tokio::test]
async fn mixed_decimal_pair_exposes_6_and_18() {
    let pool = common::setup_pool().await;
    let _seed = common::seed_db(&pool).await;

    let asset_0: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1dec557', true, 'UST1', 'UST1', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ust1");
    let asset_1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustrdec557', true, 'USTR', 'USTR', 18)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ustr");
    let pair_addr = "terra1pair557ustrxxxxxx";
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lp557', 30)
         RETURNING id",
    )
    .bind(pair_addr)
    .bind(asset_0)
    .bind(asset_1)
    .fetch_one(&pool)
    .await
    .expect("pair");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 2000, NOW(), 'tx557mix', 'terra1traderxyz', $2, $3, 1000000, 10000000000000000000, 10)",
    )
    .bind(pair_id)
    .bind(asset_0)
    .bind(asset_1)
    .execute(&pool)
    .await
    .expect("swap");

    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, NULL, 2001, NOW(), 'tx557fill', 9, 'bid', 'terra1traderxyz', 10, 1000000, 10000000000000000000, 1)",
    )
    .bind(pair_id)
    .execute(&pool)
    .await
    .expect("fill");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let trades: Vec<Value> = server
        .get(&format!("/api/v1/pairs/{pair_addr}/trades"))
        .await
        .json();
    assert_eq!(trades[0]["offer_decimals"], 6);
    assert_eq!(trades[0]["ask_decimals"], 18);
    assert_eq!(trades[0]["offer_amount"], "1000000");
    // BigDecimal Display would be `1e+19`; tape clients parse with BigInt (#557).
    assert_eq!(trades[0]["return_amount"], "10000000000000000000");
    let ret = trades[0]["return_amount"]
        .as_str()
        .expect("return_amount string");
    assert!(!ret.contains('e') && !ret.contains('E'));

    let fills: Vec<Value> = server
        .get(&format!("/api/v1/pairs/{pair_addr}/limit-fills"))
        .await
        .json();
    assert_eq!(fills[0]["token0_decimals"], 6);
    assert_eq!(fills[0]["token1_decimals"], 18);
    assert_eq!(fills[0]["token0_amount"], "1000000");
    assert_eq!(fills[0]["token1_amount"], "10000000000000000000");
}

#[serial]
#[tokio::test]
async fn out_of_range_asset_decimals_omitted_from_trade_json() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("UPDATE assets SET decimals = 99 WHERE denom = 'uluna'")
        .execute(&pool)
        .await
        .expect("poison decimals");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Vec<Value> = server
        .get(&format!("/api/v1/pairs/{}/trades", seed.pair_address))
        .await
        .json();
    assert!(body[0].get("offer_decimals").is_none() || body[0]["offer_decimals"].is_null());
    assert_eq!(body[0]["ask_decimals"], 6);
    assert_eq!(body[0]["offer_amount"], "1000");
}

#[serial]
#[tokio::test]
async fn trader_swaps_csv_keeps_raw_amounts_and_appends_decimals() {
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
    let body = resp.text();
    let header = body.lines().next().expect("header");
    assert!(header.contains("offer_amount,return_amount"));
    assert!(header.ends_with("offer_decimals,ask_decimals"));
    let data = body.lines().nth(1).expect("row");
    assert!(data.contains(",1000,950,"));
    assert!(data.ends_with(",6,6"));
}
