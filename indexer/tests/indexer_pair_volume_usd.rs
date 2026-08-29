//! GitLab #692 — pair-list 24h USD volume stamp (`volume_usd_24h` on GET /api/v1/pairs).

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::db::queries::volume;
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

async fn insert_swap(
    pool: &sqlx::PgPool,
    pair_id: i32,
    offer: i32,
    ask: i32,
    tx: &str,
    volume_usd: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 6920, NOW(), $2, 'terra1trader692', $3, $4, 1000000, 950000, 0.95, $5)",
    )
    .bind(pair_id)
    .bind(tx)
    .bind(offer)
    .bind(ask)
    .bind(volume_usd.map(bd))
    .execute(pool)
    .await
    .expect("swap");
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
    v["volume_usd_24h"].as_str().map(bd)
}

fn usd_omitted(v: &Value) -> bool {
    v["volume_usd_24h"].is_null() || v.as_object().unwrap().get("volume_usd_24h").is_none()
}

#[serial]
#[tokio::test]
async fn refresh_stamps_sum_of_priced_swaps_and_omits_unpriced() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let g0 = insert_cw20(&pool, "terra1g0_692", "EMBER", 6).await;
    let g1 = insert_cw20(&pool, "terra1g1_692", "CORAL", 6).await;

    let priced = insert_pair(&pool, "terra1priced692", uusd, uluna).await;
    insert_swap(&pool, priced, uusd, uluna, "tx692a", Some("12.5")).await;
    insert_swap(&pool, priced, uluna, uusd, "tx692b", Some("7.5")).await;
    let unpriced = insert_pair(&pool, "terra1unpriced692", g0, g1).await;
    insert_swap(&pool, unpriced, g0, g1, "tx692c", None).await;

    volume::refresh_pair_volumes(&pool)
        .await
        .expect("refresh pair volumes");

    let stamped: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_24h WHERE pair_id = $1")
            .bind(priced)
            .fetch_one(&pool)
            .await
            .expect("priced rollup");
    assert_eq!(stamped.unwrap().normalized(), bd("20").normalized());

    let gem_usd: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_24h WHERE pair_id = $1")
            .bind(unpriced)
            .fetch_one(&pool)
            .await
            .expect("unpriced rollup row");
    assert!(
        gem_usd.is_none(),
        "unpriced activity must stamp NULL, not $0"
    );

    let quote: BigDecimal =
        sqlx::query_scalar("SELECT volume_quote FROM pair_volume_24h WHERE pair_id = $1")
            .bind(unpriced)
            .fetch_one(&pool)
            .await
            .expect("quote still rolls");
    assert!(
        quote > bd("0"),
        "volume_quote_24h stays raw for unpriced rows"
    );

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();

    assert_eq!(
        parse_usd(item_for(&body, "terra1priced692"))
            .unwrap()
            .normalized(),
        bd("20").normalized()
    );
    assert!(item_for(&body, "terra1priced692")["volume_quote_24h"].is_string());
    assert!(usd_omitted(item_for(&body, "terra1unpriced692")));
    assert!(item_for(&body, "terra1unpriced692")["volume_quote_24h"].is_string());
}

#[serial]
#[tokio::test]
async fn sort_volume_usd_24h_nulls_last_and_volume_24h_stays_quote() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let g0 = insert_cw20(&pool, "terra1g0s_692", "EMBER", 6).await;
    let g1 = insert_cw20(&pool, "terra1g1s_692", "CORAL", 6).await;

    let hi = insert_pair(&pool, "terra1hi692", uusd, uluna).await;
    insert_swap(&pool, hi, uusd, uluna, "tx692hi", Some("90")).await;
    let lo = insert_pair(&pool, "terra1lo692", uusd, uluna).await;
    insert_swap(&pool, lo, uusd, uluna, "tx692lo", Some("10")).await;
    let un = insert_pair(&pool, "terra1un692", g0, g1).await;
    insert_swap(&pool, un, g0, g1, "tx692un", None).await;
    // Huge raw quote so quote-sort would rank the gem first if USD sort leaked.
    sqlx::query("UPDATE swap_events SET offer_amount = 999999999999, return_amount = 999999999999 WHERE tx_hash = 'tx692un'")
        .execute(&pool)
        .await
        .expect("inflate gem quote");

    volume::refresh_pair_volumes(&pool).await.expect("refresh");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let desc: Value = server
        .get("/api/v1/pairs?sort=volume_usd_24h&order=desc")
        .await
        .json();
    let d = desc["items"].as_array().unwrap();
    assert_eq!(d[0]["pair_address"], "terra1hi692");
    assert_eq!(d[1]["pair_address"], "terra1lo692");
    assert_eq!(d[2]["pair_address"], "terra1un692");

    let asc: Value = server
        .get("/api/v1/pairs?sort=volume_usd_24h&order=asc")
        .await
        .json();
    let a = asc["items"].as_array().unwrap();
    assert_eq!(a[0]["pair_address"], "terra1lo692");
    assert_eq!(a[1]["pair_address"], "terra1hi692");
    assert_eq!(a[2]["pair_address"], "terra1un692");

    let quote: Value = server
        .get("/api/v1/pairs?sort=volume_24h&order=desc")
        .await
        .json();
    let q = quote["items"].as_array().unwrap();
    assert_eq!(
        q[0]["pair_address"], "terra1un692",
        "sort=volume_24h must stay raw quote"
    );
}

#[serial]
#[tokio::test]
async fn invalid_sort_volume_usd_and_injection_400_caps() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for q in [
        "/api/v1/pairs?sort=volume_usd",
        "/api/v1/pairs?sort=volume_usd_24h;DROP",
        "/api/v1/pairs?sort=tvl",
        "/api/v1/pairs?sort=volume_usd_24h;DELETE",
        "/api/v1/pairs?order=desc--",
        "/api/v1/pairs?sort=id%20FROM%20pairs--",
    ] {
        server.get(q).await.assert_status_bad_request();
    }

    let neg: Value = server.get("/api/v1/pairs?limit=-1").await.json();
    assert_eq!(neg["limit"].as_i64().unwrap(), 1);

    let zero: Value = server.get("/api/v1/pairs?limit=0").await.json();
    assert_eq!(zero["limit"].as_i64().unwrap(), 1);

    server
        .get("/api/v1/pairs?offset=10001")
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn explain_volume_usd_sort_does_not_scan_swap_events() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    insert_pair(&pool, "terra1ex692", uusd, uluna).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT p.id, pv.volume_usd
         FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_volume_24h pv ON pv.pair_id = p.id
         ORDER BY pv.volume_usd DESC NULLS LAST
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
        "sort=volume_usd_24h must use pair_volume_24h, not scan swap_events:\n{plan}"
    );
}

#[serial]
#[tokio::test]
async fn idle_zeros_usd_and_overflow_omits() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let idle = insert_pair(&pool, "terra1idle692", uusd, uluna).await;
    insert_swap(&pool, idle, uusd, uluna, "tx692idle", Some("5")).await;
    volume::refresh_pair_volumes(&pool)
        .await
        .expect("stamp while live");
    sqlx::query(
        "UPDATE swap_events SET block_timestamp = NOW() - INTERVAL '48 hours' WHERE tx_hash = 'tx692idle'",
    )
    .execute(&pool)
    .await
    .expect("age");

    let overflow = insert_pair(&pool, "terra1ovf692", uusd, uluna).await;
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 6921, NOW(), 'tx692ovf1', 'terra1trader692', $2, $3, 1, 1, 1, $4),
                ($1, 6922, NOW(), 'tx692ovf2', 'terra1trader692', $2, $3, 1, 1, 1, $4)",
    )
    .bind(overflow)
    .bind(uusd)
    .bind(uluna)
    .bind(bd("6e19"))
    .execute(&pool)
    .await
    .expect("overflow swaps");

    volume::refresh_pair_volumes(&pool).await.expect("refresh");

    let idle_usd: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_24h WHERE pair_id = $1")
            .bind(idle)
            .fetch_one(&pool)
            .await
            .expect("idle row");
    assert_eq!(
        idle_usd.unwrap().normalized(),
        bd("0").normalized(),
        "idle must zero USD (#577 D3)"
    );

    let ovf: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_24h WHERE pair_id = $1")
            .bind(overflow)
            .fetch_one(&pool)
            .await
            .expect("overflow row");
    assert!(ovf.is_none(), "overflow ≥ 10^20 must omit USD, not 500");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs?limit=100").await.json();
    // Idle serializes "0"; UI treats that as —.
    let idle_json = item_for(&body, "terra1idle692")["volume_usd_24h"].as_str();
    assert!(
        idle_json.is_none() || idle_json == Some("0"),
        "idle JSON is 0 or omitted, got {idle_json:?}"
    );
    assert!(usd_omitted(item_for(&body, "terra1ovf692")));
}

#[serial]
#[tokio::test]
async fn single_pair_get_omits_24h_usd() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1one692", uusd, uluna).await;
    insert_swap(&pool, pair, uusd, uluna, "tx692one", Some("3")).await;
    volume::refresh_pair_volumes(&pool).await.expect("refresh");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/api/v1/pairs/terra1one692").await.json();
    assert!(usd_omitted(&body), "single GET must not live-scan 24h USD");
    assert!(
        body.get("volume_quote_24h").is_none() || body["volume_quote_24h"].is_null(),
        "single GET volume_quote_24h stays None"
    );
}
