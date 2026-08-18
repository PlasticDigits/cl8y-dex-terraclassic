//! GitLab #553 — trader `total_volume_usd` from catalog `swap_events.volume_usd`.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::db::queries::traders;
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

const PRICED: &str = "terra1priced000000000000000000000000000000000";
const UNPRICED: &str = "terra1unpriced0000000000000000000000000000000";
const RAW_WHALE: &str = "terra1rawwhale0000000000000000000000000000000";

#[serial]
#[tokio::test]
async fn upsert_accumulates_priced_usd_and_ignores_unpriced() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    traders::upsert_trader(&pool, PRICED, &bd("1000"), Some(&bd("12.5")))
        .await
        .expect("first");
    traders::upsert_trader(&pool, PRICED, &bd("2000"), None)
        .await
        .expect("unpriced hop");
    traders::upsert_trader(&pool, PRICED, &bd("500"), Some(&bd("7.5")))
        .await
        .expect("second priced");

    let row = traders::get_trader(&pool, PRICED)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(row.total_trades, 3);
    assert_eq!(row.total_volume.normalized(), bd("3500").normalized());
    assert_eq!(
        row.total_volume_usd.as_ref().map(|v| v.normalized()),
        Some(bd("20").normalized())
    );
}

#[serial]
#[tokio::test]
async fn refresh_sums_swap_volume_usd_not_offer_amount() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query("UPDATE swap_events SET volume_usd = 1.25 WHERE sender = $1")
        .bind(&seed.trader_address)
        .execute(&pool)
        .await
        .expect("price swaps");

    traders::refresh_trader_total_volume_usd(&pool)
        .await
        .expect("refresh");

    let row = traders::get_trader(&pool, &seed.trader_address)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(
        row.total_volume_usd.as_ref().map(|v| v.normalized()),
        Some(bd("6.25").normalized())
    );
}

#[serial]
#[tokio::test]
async fn profile_and_leaderboard_expose_usd_null_when_unpriced() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    traders::upsert_trader(&pool, UNPRICED, &bd("10000000000000000000"), None)
        .await
        .expect("unpriced trader");
    traders::upsert_trader(&pool, PRICED, &bd("1"), Some(&bd("711.2")))
        .await
        .expect("priced trader");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let unpriced: Value = server
        .get(&format!("/api/v1/traders/{UNPRICED}"))
        .await
        .json();
    assert!(unpriced["total_volume"].is_string());
    assert!(
        unpriced["total_volume_usd"].is_null(),
        "unpriced trades → JSON null, not 0"
    );

    let priced: Value = server
        .get(&format!("/api/v1/traders/{PRICED}"))
        .await
        .json();
    let priced_usd = priced["total_volume_usd"]
        .as_str()
        .expect("priced usd string");
    assert!(
        (priced_usd.parse::<f64>().unwrap() - 711.2).abs() < 1e-8,
        "priced usd was {priced_usd}"
    );

    let board: Vec<Value> = server
        .get("/api/v1/traders/leaderboard?sort=total_volume_usd&limit=10")
        .await
        .json();
    assert_eq!(board[0]["address"], PRICED);
    let board_usd = board[0]["total_volume_usd"].as_str().expect("board usd");
    assert!((board_usd.parse::<f64>().unwrap() - 711.2).abs() < 1e-8);
    let unpriced_row = board.iter().find(|r| r["address"] == UNPRICED).unwrap();
    assert!(unpriced_row["total_volume_usd"].is_null());
}

#[serial]
#[tokio::test]
async fn usd_sort_matches_priced_volume_not_raw_offer_amount() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    // Raw 18-dec whale would win `total_volume` sort but has tiny USD.
    traders::upsert_trader(
        &pool,
        RAW_WHALE,
        &bd("10000000000000000000"),
        Some(&bd("1")),
    )
    .await
    .expect("raw whale");
    traders::upsert_trader(&pool, PRICED, &bd("1000"), Some(&bd("500")))
        .await
        .expect("priced");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let raw: Vec<Value> = server
        .get("/api/v1/traders/leaderboard?sort=total_volume&limit=10")
        .await
        .json();
    assert_eq!(raw[0]["address"], RAW_WHALE);

    let usd: Vec<Value> = server
        .get("/api/v1/traders/leaderboard?sort=total_volume_usd&limit=10")
        .await
        .json();
    assert_eq!(usd[0]["address"], PRICED);
    let top_usd = usd[0]["total_volume_usd"].as_str().expect("top usd");
    assert!((top_usd.parse::<f64>().unwrap() - 500.0).abs() < 1e-8);
    assert_eq!(usd[1]["address"], RAW_WHALE);
}
