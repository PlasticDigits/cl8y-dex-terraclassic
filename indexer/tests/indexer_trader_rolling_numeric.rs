//! GitLab #1277 — trader rolling / lifetime raw volume is NUMERIC(38, 0).
//!
//! Decay tests in `indexer_volume_window_decay.rs` use 6-dec seeds that never hit
//! `10^20`. Catalog overflow in `volume_usd_catalog.rs` covers global/pair only.
//! Mutate `block_timestamp` — no 24h wall-clock sleeps (V3).

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::db::queries::traders;
use cl8y_dex_indexer::indexer::volume_aggregator;
use serial_test::serial;
use sqlx::PgPool;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

fn zero() -> BigDecimal {
    BigDecimal::from(0)
}

/// 1000 human 18-dec tokens. Overflows `NUMERIC(38, 18)` (`|x| < 10^20`).
fn offer_1e21() -> BigDecimal {
    bd("1000000000000000000000")
}

fn cap_1e38_minus_1() -> BigDecimal {
    bd("99999999999999999999999999999999999999")
}

async fn column_typmod(pool: &PgPool, column: &str) -> String {
    sqlx::query_scalar(
        "SELECT format_type(a.atttypid, a.atttypmod)
         FROM pg_attribute a
         JOIN pg_class c ON c.oid = a.attrelid
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'public' AND c.relname = 'traders' AND a.attname = $1",
    )
    .bind(column)
    .fetch_one(pool)
    .await
    .expect("column type")
}

struct HugeSeed {
    pair_id: i32,
    sender: String,
    offer: BigDecimal,
    six_sender: String,
}

/// 18-dec sender with one in-window `10^21` offer. Optional 6-dec seed trader.
async fn seed_huge(pool: &PgPool, with_six_dec: bool) -> HugeSeed {
    let six = if with_six_dec {
        common::seed_db(pool).await.trader_address
    } else {
        common::clean_db(pool).await;
        String::new()
    };

    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1r1277ust1', true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("ust1");
    let ustr: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1r1277ustr', true, 'USTR', 'USTR', 18) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("ustr");
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1r1277pair', $1, $2, 'terra1r1277lp', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(ustr)
    .fetch_one(pool)
    .await
    .expect("pair");

    let sender = "terra1r1277huge".to_string();
    let offer = offer_1e21();
    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ($1, 1, 0, 0, 0, 0, false)",
    )
    .bind(&sender)
    .execute(pool)
    .await
    .expect("trader");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 12771, $2, 'tx1277huge', $3, $4, $5, $6, 1000000, 1)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(&sender)
    .bind(ustr)
    .bind(ust1)
    .bind(&offer)
    .execute(pool)
    .await
    .expect("huge swap");

    HugeSeed {
        pair_id,
        sender,
        offer,
        six_sender: six,
    }
}

async fn age_sender(pool: &PgPool, sender: &str, ts: chrono::DateTime<Utc>) {
    sqlx::query("UPDATE swap_events SET block_timestamp = $1 WHERE sender = $2")
        .bind(ts)
        .bind(sender)
        .execute(pool)
        .await
        .expect("age sender swaps");
}

async fn trader_rolling(
    pool: &PgPool,
    address: &str,
) -> (
    BigDecimal,
    BigDecimal,
    BigDecimal,
    BigDecimal,
    Option<BigDecimal>,
) {
    let row = traders::get_trader(pool, address)
        .await
        .expect("trader")
        .expect("trader exists");
    (
        row.volume_24h,
        row.volume_7d,
        row.volume_30d,
        row.total_volume,
        row.total_volume_usd,
    )
}

/// R6 / schema: raw columns are (38, 0); USD stays (38, 18).
#[serial]
#[tokio::test]
async fn traders_raw_volume_columns_are_numeric_38_0() {
    let pool = common::setup_pool().await;
    for col in ["volume_24h", "volume_7d", "volume_30d", "total_volume"] {
        assert_eq!(
            column_typmod(&pool, col).await,
            "numeric(38,0)",
            "{col} must be NUMERIC(38, 0) (#1277)"
        );
    }
    assert_eq!(
        column_typmod(&pool, "total_volume_usd").await,
        "numeric(38,18)",
        "total_volume_usd stays NUMERIC(38, 18) (#553 / R5)"
    );
}

/// I1 / R1: in-window 10^21 offer refreshes without overflow.
#[serial]
#[tokio::test]
async fn refresh_rolling_volumes_stores_1e21_raw_offer() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, false).await;

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I1 refresh must not numeric-overflow");
    let (v24, v7, v30, _life, usd) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(v24.normalized(), seed.offer.normalized());
    assert_eq!(v7.normalized(), seed.offer.normalized());
    assert_eq!(v30.normalized(), seed.offer.normalized());
    assert!(usd.is_none(), "I1 unpriced 18-dec must not invent USD (R5)");
}

/// I2: 25h ages 24h to 0; 7d/30d still hold the raw integer.
#[serial]
#[tokio::test]
async fn huge_offer_aged_25h_zeros_24h_keeps_7d() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, false).await;
    age_sender(&pool, &seed.sender, Utc::now() - Duration::hours(25)).await;

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I2 refresh");
    let (v24, v7, v30, _, _) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert_eq!(v7.normalized(), seed.offer.normalized());
    assert_eq!(v30.normalized(), seed.offer.normalized());
}

/// I3 / R3: 31d zeros all rolling; lifetime raw is not rewritten by the idle UPDATE.
#[serial]
#[tokio::test]
async fn huge_offer_aged_31d_zeros_rolling_keeps_lifetime() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, false).await;
    sqlx::query("UPDATE traders SET total_volume = $1 WHERE address = $2")
        .bind(&seed.offer)
        .bind(&seed.sender)
        .execute(&pool)
        .await
        .expect("stamp lifetime");
    age_sender(&pool, &seed.sender, Utc::now() - Duration::days(31)).await;

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I3 refresh");
    let (v24, v7, v30, life, usd) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert_eq!(v7.normalized(), zero().normalized());
    assert_eq!(v30.normalized(), zero().normalized());
    assert_eq!(life.normalized(), seed.offer.normalized());
    assert!(usd.is_none(), "D2 must not zero/rewrite total_volume_usd");
}

/// I4 / R2: mixed 6-dec seed + 18-dec 10^21 in one refresh.
#[serial]
#[tokio::test]
async fn mixed_6dec_and_18dec_senders_refresh_without_overflow() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, true).await;
    assert!(!seed.six_sender.is_empty());

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I4 mixed refresh");
    let (six_24, _, _, _, _) = trader_rolling(&pool, &seed.six_sender).await;
    // seed_db inserts five 1000-offer swaps inside 24h.
    assert_eq!(six_24.normalized(), bd("5000").normalized());
    let (huge_24, _, _, _, _) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(huge_24.normalized(), seed.offer.normalized());
}

/// I5: no swaps → rolling zeros, no error.
#[serial]
#[tokio::test]
async fn empty_swap_events_refresh_zeros_without_error() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ('terra1r1277empty', 2, 50, 10, 20, 30, false)",
    )
    .execute(&pool)
    .await
    .expect("idle trader");

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I5 empty");
    let (v24, v7, v30, life, _) = trader_rolling(&pool, "terra1r1277empty").await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert_eq!(v7.normalized(), zero().normalized());
    assert_eq!(v30.normalized(), zero().normalized());
    assert_eq!(life.normalized(), bd("50").normalized());
}

/// I6 / R4: upsert_trader 10^21 then rolling refresh.
#[serial]
#[tokio::test]
async fn upsert_trader_1e21_then_refresh_does_not_overflow() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, false).await;
    sqlx::query("DELETE FROM traders WHERE address = $1")
        .bind(&seed.sender)
        .execute(&pool)
        .await
        .expect("drop stamped trader");

    traders::upsert_trader(&pool, &seed.sender, &seed.offer, None)
        .await
        .expect("I6 upsert must not overflow");
    let row = traders::get_trader(&pool, &seed.sender)
        .await
        .expect("get")
        .expect("inserted");
    assert_eq!(row.total_volume.normalized(), seed.offer.normalized());
    assert!(row.total_volume_usd.is_none());

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("I6 refresh");
    let (v24, _, _, life, usd) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(v24.normalized(), seed.offer.normalized());
    assert_eq!(life.normalized(), seed.offer.normalized());
    assert!(usd.is_none());
}

/// A1: additive lifetime raw caps at 10^38-1 (no wrap / panic).
#[serial]
#[tokio::test]
async fn upsert_trader_caps_lifetime_raw_at_1e38_minus_1() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let over = bd("100000000000000000000000000000000000000"); // 10^38
    traders::upsert_trader(&pool, "terra1r1277cap", &over, None)
        .await
        .expect("A1 insert cap");
    let row = traders::get_trader(&pool, "terra1r1277cap")
        .await
        .expect("get")
        .expect("row");
    assert_eq!(
        row.total_volume.normalized(),
        cap_1e38_minus_1().normalized()
    );

    traders::upsert_trader(&pool, "terra1r1277cap", &over, None)
        .await
        .expect("A1 update cap");
    let row = traders::get_trader(&pool, "terra1r1277cap")
        .await
        .expect("get")
        .expect("row");
    assert_eq!(
        row.total_volume.normalized(),
        cap_1e38_minus_1().normalized()
    );
    assert!(row.total_volume >= zero(), "A1 must not wrap negative");
}

/// I7 / A7: leaderboard sort=volume_24h ranks the 18-dec sender; JSON is plain digits.
#[serial]
#[tokio::test]
async fn leaderboard_volume_24h_ranks_1e21_as_plain_decimal() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, true).await;
    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh");
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/api/v1/traders/leaderboard?sort=volume_24h&limit=5")
        .await;
    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    assert_eq!(body[0]["address"], seed.sender);
    let v24 = body[0]["volume_24h"].as_str().expect("string");
    assert_eq!(v24, "1000000000000000000000");
    assert!(
        !v24.contains('e') && !v24.contains('E'),
        "R1 JSON must be plain decimal, got {v24}"
    );
    let six = body
        .iter()
        .find(|t| t["address"] == seed.six_sender)
        .expect("6-dec sender on board");
    assert_eq!(six["volume_24h"].as_str().unwrap(), "5000");
}

/// I8 / R7: aggregator loop still runs pair/global when trader refresh used to be the failure.
#[serial]
#[tokio::test]
async fn refresh_all_volume_windows_succeeds_with_1e21_trader() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, false).await;

    volume_aggregator::refresh_all_volume_windows(&pool, false).await;

    let (v24, _, _, _, _) = trader_rolling(&pool, &seed.sender).await;
    assert_eq!(
        v24.normalized(),
        seed.offer.normalized(),
        "trader refresh must not be the aggregator failure"
    );
    let global: Option<BigDecimal> =
        sqlx::query_scalar("SELECT total_volume FROM global_stats_24h WHERE id = 1")
            .fetch_optional(&pool)
            .await
            .expect("global");
    assert!(
        global.is_some(),
        "R7 pair/global refresh still runs alongside trader rolling"
    );
}

/// I9 / R5: priced 6-dec USD is unchanged by the 18-dec raw refresh; USD type stays (38, 18).
#[serial]
#[tokio::test]
async fn huge_raw_refresh_does_not_smash_total_volume_usd() {
    let pool = common::setup_pool().await;
    let seed = seed_huge(&pool, true).await;
    let usd = bd("12.34");
    sqlx::query("UPDATE swap_events SET volume_usd = $1 WHERE sender = $2")
        .bind(&usd)
        .bind(&seed.six_sender)
        .execute(&pool)
        .await
        .expect("price 6-dec swaps");
    traders::refresh_trader_total_volume_usd(&pool)
        .await
        .expect("usd refresh");
    let before: Option<BigDecimal> =
        sqlx::query_scalar("SELECT total_volume_usd FROM traders WHERE address = $1")
            .bind(&seed.six_sender)
            .fetch_one(&pool)
            .await
            .expect("usd before");

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("rolling");
    let after: Option<BigDecimal> =
        sqlx::query_scalar("SELECT total_volume_usd FROM traders WHERE address = $1")
            .bind(&seed.six_sender)
            .fetch_one(&pool)
            .await
            .expect("usd after");
    assert_eq!(
        before.as_ref().map(|v| v.normalized()),
        after.as_ref().map(|v| v.normalized()),
        "rolling UPDATE must not rewrite total_volume_usd"
    );
    let huge_usd: Option<BigDecimal> =
        sqlx::query_scalar("SELECT total_volume_usd FROM traders WHERE address = $1")
            .bind(&seed.sender)
            .fetch_one(&pool)
            .await
            .expect("huge usd");
    assert!(huge_usd.is_none(), "I1 18-dec raw must not smash USD");
    assert_eq!(
        column_typmod(&pool, "total_volume_usd").await,
        "numeric(38,18)"
    );
    let _ = seed.pair_id;
}
