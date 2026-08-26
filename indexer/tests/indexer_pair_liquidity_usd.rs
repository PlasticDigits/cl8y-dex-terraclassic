//! GitLab #655 — per-pair AMM TVL rollup on `GET /api/v1/pairs`.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::indexer::protocol_tvl;
use common::{clean_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn insert_native(pool: &sqlx::PgPool, denom: &str, symbol: &str, decimals: i16) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
         VALUES ($1, false, $2, $2, $3) RETURNING id",
    )
    .bind(denom)
    .bind(symbol)
    .bind(decimals)
    .fetch_one(pool)
    .await
    .expect("native asset")
}

async fn insert_cw20(pool: &sqlx::PgPool, addr: &str, symbol: &str, decimals: i16) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, $2, $2, $3) RETURNING id",
    )
    .bind(addr)
    .bind(symbol)
    .bind(decimals)
    .fetch_one(pool)
    .await
    .expect("cw20 asset")
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

async fn insert_reserves(
    pool: &sqlx::PgPool,
    pair_id: i32,
    r0: &str,
    r1: &str,
    snapshot_at: chrono::DateTime<Utc>,
) {
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, $2, $3, 30, $4)",
    )
    .bind(pair_id)
    .bind(bd(r0))
    .bind(bd(r1))
    .bind(snapshot_at)
    .execute(pool)
    .await
    .expect("reserves");
}

async fn insert_oracle(pool: &sqlx::PgPool, ticker: &str, price: &str) {
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ($1, $2, 'average', NOW())",
    )
    .bind(ticker)
    .bind(bd(price))
    .execute(pool)
    .await
    .expect("oracle");
}

fn item_for<'a>(body: &'a Value, addr: &str) -> &'a Value {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pair_address"] == addr)
        .unwrap_or_else(|| panic!("missing pair {addr}"))
}

fn parse_usd(v: &Value) -> Option<BigDecimal> {
    v["liquidity_usd"].as_str().map(bd)
}

#[serial]
#[tokio::test]
async fn i1_i2_i3_priced_one_sided_and_unpriced() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let gem = insert_cw20(&pool, "terra1gem655", "GEM", 18).await;
    let g0 = insert_cw20(&pool, "terra1g0_655", "EMBER", 6).await;
    let g1 = insert_cw20(&pool, "terra1g1_655", "CORAL", 6).await;

    let both = insert_pair(&pool, "terra1both655", uusd, uluna).await;
    insert_reserves(&pool, both, "100000000", "1000000000", now).await;
    let one = insert_pair(&pool, "terra1one655", uusd, gem).await;
    insert_reserves(&pool, one, "200000000", "100000000000000000000", now).await;
    let none = insert_pair(&pool, "terra1none655", g0, g1).await;
    insert_reserves(&pool, none, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();

    // 100*0.01 + 1000*0.0001 = 1.1
    assert_eq!(
        parse_usd(item_for(&body, "terra1both655")).unwrap(),
        bd("1.1")
    );
    // 2 * 200 * 0.01 = 4 (one-sided CPAMM; not 1e24 gem)
    assert_eq!(parse_usd(item_for(&body, "terra1one655")).unwrap(), bd("4"));
    assert!(
        item_for(&body, "terra1none655")["liquidity_usd"].is_null()
            || item_for(&body, "terra1none655")
                .as_object()
                .unwrap()
                .get("liquidity_usd")
                .is_none()
    );
}

#[serial]
#[tokio::test]
async fn i4_i5_hub_marks_not_pegs() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let ust1 = insert_cw20(&pool, "terra1ust1655", "UST1", 6).await;
    let custc = insert_cw20(&pool, "terra1custc655", "cUSTC", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr655", "USTR", 18).await;
    let p_custc = insert_pair(&pool, "terra1ust1custc655", ust1, custc).await;
    insert_reserves(&pool, p_custc, "10000000", "20000000", now).await;
    let p_ustr = insert_pair(&pool, "terra1ust1ustr655", ust1, ustr).await;
    insert_reserves(&pool, p_ustr, "10000000", "100000000000000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    sqlx::query(
        "INSERT INTO hub_prices (ticker, asset_id, price_usd, updated_at)
         VALUES ('ust1', $1, 0.98, NOW()), ('ustr', $2, 0.012, NOW())",
    )
    .bind(ust1)
    .bind(ustr)
    .execute(&pool)
    .await
    .unwrap();

    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();

    // 10*0.98 + 20*0.01 = 10.0 — not 10*$1 + 0.2 = 10.2
    let ust1_custc = parse_usd(item_for(&body, "terra1ust1custc655")).unwrap();
    assert_eq!(ust1_custc, bd("10.0"));
    assert_ne!(ust1_custc, bd("10.2"));
    // 10*0.98 + 100*0.012 = 11.0 — not 100 * 2.5 * 0.01
    let ust1_ustr = parse_usd(item_for(&body, "terra1ust1ustr655")).unwrap();
    assert_eq!(ust1_ustr, bd("11.0"));
    assert_ne!(ust1_ustr, bd("2.5"));
}

#[serial]
#[tokio::test]
async fn i6_i7_stale_zero_and_hub_down_omit() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let ust1 = insert_cw20(&pool, "terra1ust1down655", "UST1", 6).await;
    let gem = insert_cw20(&pool, "terra1gemdown655", "GEM", 6).await;

    let stale = insert_pair(&pool, "terra1stale655", uusd, uluna).await;
    insert_reserves(
        &pool,
        stale,
        "100000000",
        "1000000000",
        now - Duration::hours(1),
    )
    .await;
    let zero = insert_pair(&pool, "terra1zero655", uusd, uluna).await;
    insert_reserves(&pool, zero, "0", "1000000", now).await;
    let hub_down = insert_pair(&pool, "terra1hubdown655", ust1, gem).await;
    insert_reserves(&pool, hub_down, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();
    for addr in ["terra1stale655", "terra1zero655", "terra1hubdown655"] {
        let row = item_for(&body, addr);
        assert!(
            row.get("liquidity_usd").is_none() || row["liquidity_usd"].is_null(),
            "{addr} must omit liquidity_usd, got {row}"
        );
    }
}

#[serial]
#[tokio::test]
async fn i8_i9_sort_nulls_last_and_i12_matches_stamp() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let g0 = insert_cw20(&pool, "terra1ga655", "EMBER", 6).await;
    let g1 = insert_cw20(&pool, "terra1gb655", "CORAL", 6).await;
    let hi = insert_pair(&pool, "terra1hi655", uusd, uluna).await;
    insert_reserves(&pool, hi, "1000000000", "1000000000", now).await; // 10 + 0.1 = 10.1
    let lo = insert_pair(&pool, "terra1lo655", uusd, uluna).await;
    insert_reserves(&pool, lo, "100000000", "1000000000", now).await; // 1.1
    let un = insert_pair(&pool, "terra1un655", g0, g1).await;
    insert_reserves(&pool, un, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let stamped: Option<BigDecimal> =
        sqlx::query_scalar("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
            .bind(hi)
            .fetch_optional(&pool)
            .await
            .expect("stamp row");
    let stamped = stamped.expect("priced pair must have a rollup row");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let desc: Value = server
        .get("/api/v1/pairs?sort=liquidity_usd&order=desc")
        .await
        .json();
    let d = desc["items"].as_array().unwrap();
    assert_eq!(d[0]["pair_address"], "terra1hi655");
    assert_eq!(d[1]["pair_address"], "terra1lo655");
    assert_eq!(d[2]["pair_address"], "terra1un655");
    assert_eq!(parse_usd(&d[0]).unwrap().normalized(), stamped.normalized());

    let asc: Value = server
        .get("/api/v1/pairs?sort=liquidity_usd&order=asc")
        .await
        .json();
    let a = asc["items"].as_array().unwrap();
    assert_eq!(a[0]["pair_address"], "terra1lo655");
    assert_eq!(a[1]["pair_address"], "terra1hi655");
    assert_eq!(a[2]["pair_address"], "terra1un655");
}

#[serial]
#[tokio::test]
async fn i10_a5_invalid_sort_400_and_i14_caps() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for q in [
        "/api/v1/pairs?sort=liquidity_usd;DROP",
        "/api/v1/pairs?sort=__proto__",
        "/api/v1/pairs?sort=tvl",
        "/api/v1/pairs?sort=liquidity_usd;DELETE",
        "/api/v1/pairs?sort=id%20FROM%20pairs--",
    ] {
        server.get(q).await.assert_status_bad_request();
    }

    let neg: Value = server.get("/api/v1/pairs?limit=-1").await.json();
    assert_eq!(neg["limit"].as_i64().unwrap(), 1);

    server
        .get("/api/v1/pairs?offset=10001")
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn i11_i15_a10_additive_and_search_usd_sort() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let add = insert_pair(&pool, "terra1add655", uusd, uluna).await;
    insert_reserves(&pool, add, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let id_sort: Value = server.get("/api/v1/pairs?sort=id").await.json();
    assert!(parse_usd(item_for(&id_sort, "terra1add655")).is_some());

    let vol: Value = server.get("/api/v1/pairs?sort=volume_24h").await.json();
    assert!(parse_usd(item_for(&vol, "terra1add655")).is_some());

    let q_usd: Value = server
        .get("/api/v1/pairs?q=LUNC&sort=liquidity_usd&order=desc")
        .await
        .json();
    assert!(!q_usd["items"].as_array().unwrap().is_empty());
    assert!(parse_usd(item_for(&q_usd, "terra1add655")).is_some());

    let wild: Value = server
        .get("/api/v1/pairs?q=%25&sort=liquidity_usd")
        .await
        .json();
    assert!(wild["items"].as_array().unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn i16_a1_a2_double_count_spoof_and_vfdusd() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pa = insert_pair(&pool, "terra1p655a", uusd, uluna).await;
    insert_reserves(&pool, pa, "100000000", "1000000000", now).await;
    let pb = insert_pair(&pool, "terra1p655b", uusd, uluna).await;
    insert_reserves(&pool, pb, "100000000", "1000000000", now).await;

    let spoof = insert_native(&pool, "ugem", "USTR", 6).await;
    let spoof_pair = insert_pair(&pool, "terra1spoof655", spoof, uusd).await;
    insert_reserves(&pool, spoof_pair, "999999999999", "1000000", now).await;

    let vfd = insert_cw20(&pool, "terra1vfd655", "VFDUSD", 8).await;
    let vfd_pair = insert_pair(&pool, "terra1vfdpair655", vfd, uusd).await;
    insert_reserves(&pool, vfd_pair, "100000000", "1000000", now).await;

    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();
    let a = parse_usd(item_for(&body, "terra1p655a")).unwrap();
    let b = parse_usd(item_for(&body, "terra1p655b")).unwrap();
    assert_eq!(a, b);
    assert_eq!(a, bd("1.1"));

    let spoof_row = item_for(&body, "terra1spoof655");
    // one-sided 2× USTC only (1 human * 0.01 * 2 = 0.02), not USTR-spoofed native
    assert_eq!(parse_usd(spoof_row).unwrap(), bd("0.02"));

    let vfd_row = item_for(&body, "terra1vfdpair655");
    assert_eq!(parse_usd(vfd_row).unwrap(), bd("0.02"));
}

#[serial]
#[tokio::test]
async fn a7_explain_sort_does_not_scan_swaps_or_reserves() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT p.id, pl.liquidity_usd
         FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_volume_24h pv ON pv.pair_id = p.id
         LEFT JOIN pair_liquidity_usd pl ON pl.pair_id = p.id
         ORDER BY pl.liquidity_usd DESC NULLS LAST, p.id ASC
         LIMIT 50 OFFSET 0",
    )
    .fetch_all(&pool)
    .await
    .expect("explain");

    let plan: String = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plan.contains("swap_events"),
        "sort=liquidity_usd must not scan swap_events:\n{plan}"
    );
    assert!(
        !plan.contains("pair_reserves"),
        "sort=liquidity_usd must not live-join pair_reserves:\n{plan}"
    );
    assert!(
        !plan.contains("oracle_prices"),
        "sort=liquidity_usd must not join oracle_prices:\n{plan}"
    );
}

#[serial]
#[tokio::test]
async fn a11_unpriced_is_not_zero_on_rollup() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let g0 = insert_cw20(&pool, "terra1z0", "GEM", 6).await;
    let g1 = insert_cw20(&pool, "terra1z1", "GEM2", 6).await;
    let unpriced = insert_pair(&pool, "terra1unpriced0", g0, g1).await;
    insert_reserves(&pool, unpriced, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;

    protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("stamp");

    let stamped: Option<BigDecimal> =
        sqlx::query_scalar("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
            .bind(unpriced)
            .fetch_optional(&pool)
            .await
            .expect("lookup");
    assert!(
        stamped.is_none(),
        "unpriced must have no rollup row (not $0)"
    );
}
