//! GitLab #568 — no as-of-now hub rewrite; idle mark-to-market USD candles.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::config::{
    DEFAULT_HUB_CUSTC_ADDRESS, DEFAULT_HUB_UST1_ADDRESS, DEFAULT_HUB_USTR_ADDRESS,
};
use cl8y_dex_indexer::db::queries::{candles, hub_prices, usd_as_of};
use cl8y_dex_indexer::indexer::candle_builder;
use cl8y_dex_indexer::indexer::candle_mark;
use cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig;
use cl8y_dex_indexer::indexer::pair_price_usd::HubQuoteUsd;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

fn f64_bd(v: &BigDecimal) -> f64 {
    use bigdecimal::ToPrimitive;
    v.to_f64().unwrap()
}

struct HubTrio {
    ust1: i32,
    custc: i32,
    ust1_custc_pair: i32,
    ust1_custc_addr: String,
    ust1_ustr_pair: i32,
}

async fn insert_cw20(
    pool: &sqlx::PgPool,
    contract: &str,
    name: &str,
    symbol: &str,
    decimals: i16,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, $2, $3, $4) RETURNING id",
    )
    .bind(contract)
    .bind(name)
    .bind(symbol)
    .bind(decimals)
    .fetch_one(pool)
    .await
    .expect("cw20")
}

async fn insert_pair(pool: &sqlx::PgPool, addr: &str, a0: i32, a1: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, $4, 30) RETURNING id",
    )
    .bind(addr)
    .bind(a0)
    .bind(a1)
    .bind(format!("{addr}lp"))
    .fetch_one(pool)
    .await
    .expect("pair")
}

async fn seed_hub_trio(pool: &sqlx::PgPool) -> HubTrio {
    let ust1 = insert_cw20(pool, DEFAULT_HUB_UST1_ADDRESS, "UST1", "UST1", 6).await;
    let custc = insert_cw20(pool, DEFAULT_HUB_CUSTC_ADDRESS, "cUSTC", "cUSTC", 6).await;
    let ustr = insert_cw20(pool, DEFAULT_HUB_USTR_ADDRESS, "USTR", "USTR", 18).await;
    let ust1_custc_addr = "terra1ust1custc568".to_string();
    let ust1_custc_pair = insert_pair(pool, &ust1_custc_addr, ust1, custc).await;
    let ust1_ustr_pair = insert_pair(pool, "terra1ust1ustr568", ust1, ustr).await;
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 250000000, 50000000000, 30, $3),
                ($2, 1000000000, 80000000000000000000, 30, $3)",
    )
    .bind(ust1_custc_pair)
    .bind(ust1_ustr_pair)
    .bind(now)
    .execute(pool)
    .await
    .expect("reserves");
    HubTrio {
        ust1,
        custc,
        ust1_custc_pair,
        ust1_custc_addr,
        ust1_ustr_pair,
    }
}

async fn insert_swap(
    pool: &sqlx::PgPool,
    pair_id: i32,
    height: i64,
    ts: chrono::DateTime<Utc>,
    tx: &str,
    offer: i32,
    ask: i32,
    human: &BigDecimal,
    price_usd: Option<&BigDecimal>,
) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, $2, $3, $4, 'terra1t', $5, $6, 1000000, 200000000, $7, $8)",
    )
    .bind(pair_id)
    .bind(height)
    .bind(ts)
    .bind(tx)
    .bind(offer)
    .bind(ask)
    .bind(human)
    .bind(price_usd)
    .execute(pool)
    .await
    .expect("swap");
}

fn hub_cfg() -> HubUsdConfig {
    HubUsdConfig::from_indexer_config(&common::test_config())
}

fn invert(close: &BigDecimal, close_human: &BigDecimal) -> f64 {
    f64_bd(close) / f64_bd(close_human)
}

/// Hold the shared test-DB flock for the whole test so sibling cargo processes cannot
/// TRUNCATE `pairs` mid-mark upsert (FK `candles_pair_id_fkey`).
async fn isolated_db() -> (sqlx::PgPool, std::fs::File) {
    let pool = common::setup_pool().await;
    let lock = common::lock_shared_test_db();
    common::clean_db_holding(&pool).await;
    (pool, lock)
}

#[serial]
#[tokio::test]
async fn hub_refresh_does_not_rewrite_historical_usd() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;

    let past = Utc::now() - Duration::hours(3);
    let human = bd("200");
    let stamped = bd("1.0"); // 200 × USTC 0.005
    insert_swap(
        &pool,
        trio.ust1_custc_pair,
        5680,
        past,
        "tx568hist",
        trio.ust1,
        trio.custc,
        &human,
        Some(&stamped),
    )
    .await;

    let open_time = candle_builder::truncate_to_interval(past, "1h");
    candles::upsert_candle(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        open_time,
        &stamped,
        &stamped,
        &stamped,
        &stamped,
        Some(&human),
        Some(&human),
        Some(&human),
        Some(&human),
        &bd("1000"),
        &bd("200000"),
        1,
    )
    .await
    .expect("hist candle");

    hub_prices::refresh_hub_prices(&pool, &hub_cfg(), Some(&bd("0.004")), None)
        .await
        .expect("refresh");

    let stored: BigDecimal =
        sqlx::query_scalar("SELECT price_usd FROM swap_events WHERE tx_hash = 'tx568hist'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        (f64_bd(&stored) - 1.0).abs() < 1e-9,
        "historical swap price_usd rewritten to {stored}"
    );

    let rows = candles::get_candles(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        past - Duration::hours(1),
        past + Duration::minutes(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(
        (f64_bd(&rows[0].close) - 1.0).abs() < 1e-9,
        "historical candle USD rewritten to {}",
        rows[0].close
    );
    assert_eq!(rows[0].trade_count, 1);
}

#[serial]
#[tokio::test]
async fn two_ingest_stamps_keep_as_of_quote_usd() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    let human = bd("200");
    let t0 = Utc::now() - Duration::hours(5);
    let t1 = Utc::now() - Duration::hours(4);
    insert_swap(
        &pool,
        trio.ust1_custc_pair,
        5681,
        t0,
        "tx568a",
        trio.ust1,
        trio.custc,
        &human,
        Some(&bd("1.0")),
    )
    .await;
    insert_swap(
        &pool,
        trio.ust1_custc_pair,
        5682,
        t1,
        "tx568b",
        trio.ust1,
        trio.custc,
        &human,
        Some(&bd("0.8")),
    )
    .await;

    hub_prices::refresh_hub_prices(&pool, &hub_cfg(), Some(&bd("0.003")), None)
        .await
        .expect("refresh");

    let a: BigDecimal =
        sqlx::query_scalar("SELECT price_usd FROM swap_events WHERE tx_hash = 'tx568a'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let b: BigDecimal =
        sqlx::query_scalar("SELECT price_usd FROM swap_events WHERE tx_hash = 'tx568b'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!((f64_bd(&a) - 1.0).abs() < 1e-9);
    assert!((f64_bd(&b) - 0.8).abs() < 1e-9);
}

#[serial]
#[tokio::test]
async fn idle_ustc_tick_writes_mark_bars() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    let now = Utc::now();
    let hub = HubQuoteUsd {
        ust1: Some(bd("1")),
        ustr: Some(bd("0.01")),
            ..Default::default()
        };

    let n =
        candle_mark::apply_idle_usd_marks(&pool, &hub_cfg(), now, Some(&bd("0.005")), None, &hub)
            .await
            .expect("marks");
    assert!(n >= 1);

    let rows = candles::get_candles(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        now - Duration::hours(2),
        now + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    let c0 = &rows[0];
    assert_eq!(c0.trade_count, 0);
    assert_eq!(c0.volume_base, bd("0"));
    assert_eq!(c0.volume_quote, bd("0"));
    let human = c0.close_human.as_ref().unwrap();
    assert!((f64_bd(human) - 200.0).abs() < 1e-6, "human {human}");
    assert!((f64_bd(&c0.close) - 1.0).abs() < 1e-6, "usd {}", c0.close);
    assert!((invert(&c0.close, human) - 0.005).abs() < 1e-9);

    candle_mark::apply_idle_usd_marks(&pool, &hub_cfg(), now, Some(&bd("0.0045")), None, &hub)
        .await
        .expect("marks 2");

    let rows = candles::get_candles(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        now - Duration::hours(2),
        now + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    let c1 = &rows[0];
    assert_eq!(c1.trade_count, 0);
    let human = c1.close_human.as_ref().unwrap();
    assert!((f64_bd(&c1.close) - 0.9).abs() < 1e-6, "usd {}", c1.close);
    assert!((invert(&c1.close, human) - 0.0045).abs() < 1e-9);
    assert!(c1.open <= c1.close || c1.high >= c1.low);
    assert!(c1.high >= c1.low);
}

#[serial]
#[tokio::test]
async fn idle_ustr_and_lunc_marks() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;

    let clunc = insert_cw20(&pool, "terra1clunc568", "cLUNC", "cLUNC", 6).await;
    let gem = insert_cw20(&pool, "terra1gem568", "GEM", "GEMX", 6).await;
    let lunc_pair = insert_pair(&pool, "terra1gemclunc568", gem, clunc).await;
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 1000000, 2000000, 30, NOW())",
    )
    .bind(lunc_pair)
    .execute(&pool)
    .await
    .unwrap();

    hub_prices::refresh_hub_prices(&pool, &hub_cfg(), Some(&bd("0.005")), Some(&bd("0.00005")))
        .await
        .expect("refresh");

    let ustr_rows = candles::get_candles(
        &pool,
        trio.ust1_ustr_pair,
        "1h",
        Utc::now() - Duration::hours(2),
        Utc::now() + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(ustr_rows.len(), 1);
    assert_eq!(ustr_rows[0].trade_count, 0);
    let ustr_human = ustr_rows[0].close_human.as_ref().unwrap();
    let ustr_inv = invert(&ustr_rows[0].close, ustr_human);
    assert!(ustr_inv > 0.0);

    let lunc_rows = candles::get_candles(
        &pool,
        lunc_pair,
        "1h",
        Utc::now() - Duration::hours(2),
        Utc::now() + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(lunc_rows.len(), 1);
    let lh = lunc_rows[0].close_human.as_ref().unwrap();
    assert!((invert(&lunc_rows[0].close, lh) - 0.00005).abs() < 1e-12);

    let before = lunc_rows[0].close.clone();
    candle_mark::apply_idle_usd_marks(
        &pool,
        &hub_cfg(),
        Utc::now(),
        Some(&bd("0.009")),
        Some(&bd("0.00005")),
        &HubQuoteUsd {
            ust1: Some(bd("1")),
            ustr: Some(bd("0.01")),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let lunc_after = candles::get_candles(
        &pool,
        lunc_pair,
        "1h",
        Utc::now() - Duration::hours(2),
        Utc::now() + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert!(
        (f64_bd(&lunc_after[0].close) - f64_bd(&before)).abs() < 1e-12,
        "USTC tick must not reprice a LUNC quote"
    );
}

#[serial]
#[tokio::test]
async fn swap_bar_keeps_trade_count_when_marked() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    let now = Utc::now();
    let human = bd("200");
    candle_builder::update_candles_for_swap(
        &pool,
        trio.ust1_custc_pair,
        now,
        Some(&bd("1.0")),
        &human,
        &bd("1000"),
        &bd("200000"),
    )
    .await
    .unwrap();

    candle_mark::apply_idle_usd_marks(
        &pool,
        &hub_cfg(),
        now,
        Some(&bd("0.004")),
        None,
        &HubQuoteUsd {
            ust1: Some(bd("0.8")),
            ustr: None,
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let rows = candles::get_candles(
        &pool,
        trio.ust1_custc_pair,
        "1m",
        now - Duration::minutes(2),
        now + Duration::minutes(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].trade_count, 1);
    assert!(rows[0].volume_base > bd("0"));
    let h = rows[0].close_human.as_ref().unwrap();
    assert!((f64_bd(h) - 200.0).abs() < 1e-9);
    assert!((f64_bd(&rows[0].close) - 0.8).abs() < 1e-6);
}

#[serial]
#[tokio::test]
async fn oracle_down_writes_no_usd_marks() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    hub_prices::refresh_hub_prices(&pool, &hub_cfg(), None, None)
        .await
        .expect("refresh");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id = $1")
        .bind(trio.ust1_custc_pair)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    let leftover: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM hub_prices")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(leftover, 0);
}

#[serial]
#[tokio::test]
async fn skip_unknown_quote_and_spoof_native_ustr() {
    let (pool, _lock) = isolated_db().await;
    let gem = insert_cw20(&pool, "terra1gemx568", "GEMX", "GEMX", 6).await;
    let other = insert_cw20(&pool, "terra1gemy568", "GEMY", "GEMY", 6).await;
    let gem_pair = insert_pair(&pool, "terra1gempair568", gem, other).await;
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 1000000, 1000000, 30, NOW())",
    )
    .bind(gem_pair)
    .execute(&pool)
    .await
    .unwrap();

    let spoof_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
         VALUES ('ugem', false, 'Spoof', 'USTR', 18) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let spoof_pair = insert_pair(&pool, "terra1spoofustr568", gem, spoof_id).await;
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 1000000, 1000000000000000000, 30, NOW())",
    )
    .bind(spoof_pair)
    .execute(&pool)
    .await
    .unwrap();

    let fake_ust1 = insert_cw20(&pool, "terra1fakeust1568", "UST1", "UST1", 6).await;
    let fake_pair = insert_pair(&pool, "terra1fakeust1pair568", gem, fake_ust1).await;
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 1000000, 1000000, 30, NOW())",
    )
    .bind(fake_pair)
    .execute(&pool)
    .await
    .unwrap();

    let n = candle_mark::apply_idle_usd_marks(
        &pool,
        &hub_cfg(),
        Utc::now(),
        Some(&bd("0.005")),
        Some(&bd("0.00005")),
        &HubQuoteUsd {
            ust1: Some(bd("1")),
            ustr: Some(bd("0.01")),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(n, 0);
    let c: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id IN ($1, $2, $3)")
            .bind(gem_pair)
            .bind(spoof_pair)
            .bind(fake_pair)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(c, 0);
}

#[serial]
#[tokio::test]
async fn get_candles_includes_mark_bars() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    candle_mark::apply_idle_usd_marks(
        &pool,
        &hub_cfg(),
        Utc::now(),
        Some(&bd("0.005")),
        None,
        &HubQuoteUsd {
            ust1: Some(bd("1")),
            ustr: Some(bd("0.01")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=1h",
            trio.ust1_custc_addr
        ))
        .await;
    assert_eq!(resp.status_code(), StatusCode::OK);
    let body: serde_json::Value = resp.json();
    assert!(!body.as_array().unwrap().is_empty());
    assert_eq!(body[0]["trade_count"], 0);

    let junk = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=javascript:alert(1)",
            trio.ust1_custc_addr
        ))
        .await;
    assert_eq!(junk.status_code(), StatusCode::BAD_REQUEST);

    let missing = server
        .get("/api/v1/pairs/terra1doesnotexist568/candles?interval=1h")
        .await;
    assert_eq!(missing.status_code(), StatusCode::NOT_FOUND);
}

#[serial]
#[tokio::test]
async fn repair_restores_as_of_ustc_oracle() {
    let (pool, _lock) = isolated_db().await;
    let trio = seed_hub_trio(&pool).await;
    let t0 = Utc::now() - Duration::hours(2);
    let open_time = candle_builder::truncate_to_interval(t0, "1h");
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ('ustc', 0.005, 'average', $1)",
    )
    .bind(open_time - Duration::minutes(1))
    .execute(&pool)
    .await
    .unwrap();
    let human = bd("200");
    insert_swap(
        &pool,
        trio.ust1_custc_pair,
        5690,
        t0,
        "tx568repair",
        trio.ust1,
        trio.custc,
        &human,
        Some(&bd("0.8")),
    )
    .await;
    candles::upsert_candle(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        open_time,
        &bd("0.8"),
        &bd("0.8"),
        &bd("0.8"),
        &bd("0.8"),
        Some(&human),
        Some(&human),
        Some(&human),
        Some(&human),
        &bd("1"),
        &bd("1"),
        1,
    )
    .await
    .unwrap();

    usd_as_of::repair_ustc_lunc_usd_as_of_oracle(&pool)
        .await
        .expect("repair");

    let usd: BigDecimal =
        sqlx::query_scalar("SELECT price_usd FROM swap_events WHERE tx_hash = 'tx568repair'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!((f64_bd(&usd) - 1.0).abs() < 1e-9, "repaired {usd}");
    let rows = candles::get_candles(
        &pool,
        trio.ust1_custc_pair,
        "1h",
        t0 - Duration::hours(1),
        t0 + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "expected repaired 1h candle");
    assert!((f64_bd(&rows[0].close) - 1.0).abs() < 1e-9);
    assert_eq!(rows[0].trade_count, 1);
}

#[serial]
#[tokio::test]
async fn clunc_ust1_factory_usd_still_tracks_human() {
    let (pool, _lock) = isolated_db().await;
    let _trio = seed_hub_trio(&pool).await;
    let clunc = insert_cw20(&pool, "terra1cluncust1568", "cLUNC", "cLUNC", 6).await;
    let ust1: i32 = sqlx::query_scalar("SELECT id FROM assets WHERE contract_address = $1")
        .bind(DEFAULT_HUB_UST1_ADDRESS)
        .fetch_one(&pool)
        .await
        .unwrap();
    let pair = insert_pair(&pool, "terra1cluncust1pair568", clunc, ust1).await;
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 1000000000, 47000, 30, NOW())",
    )
    .bind(pair)
    .execute(&pool)
    .await
    .unwrap();

    hub_prices::refresh_hub_prices(&pool, &hub_cfg(), Some(&bd("0.005")), None)
        .await
        .expect("refresh");
    let hub = cl8y_dex_indexer::db::queries::hub_prices::load_quote_usd(&pool)
        .await
        .expect("hub");
    assert!(hub.ust1.is_some(), "hub UST1 required for cLUNC/UST1 marks");
    let written = candle_mark::apply_idle_usd_marks(
        &pool,
        &hub_cfg(),
        Utc::now(),
        Some(&bd("0.005")),
        None,
        &hub,
    )
    .await
    .expect("marks");
    assert!(
        written >= 1,
        "cLUNC/UST1 must receive a current-bucket mark (wrote {written})"
    );

    let rows = candles::get_candles(
        &pool,
        pair,
        "1h",
        Utc::now() - Duration::hours(2),
        Utc::now() + Duration::hours(1),
        10,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    let h = rows[0].close_human.as_ref().unwrap();
    assert!(f64_bd(&rows[0].close) > 0.0);
    let implied_ust1 = invert(&rows[0].close, h);
    assert!(
        implied_ust1 > 0.5 && implied_ust1 < 1.5,
        "implied UST1 USD {implied_ust1}"
    );
}
