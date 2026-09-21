//! Forgejo #1263 — Protocol top-5 trailing 30d pair USD volume + current TVL + vol/LP.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::api::reset_protocol_top_pairs_cache;
use cl8y_dex_indexer::db::queries::protocol_top_pairs::LIST_TOP_PAIRS_SQL;
use cl8y_dex_indexer::db::queries::volume;
use cl8y_dex_indexer::indexer::defillama::COLUMBUS5_GEM_ADDRESSES;
use common::{build_test_app, clean_db, setup_pool};
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
         VALUES ($1, 12630, NOW(), $2, 'terra1trader1263', $3, $4, 1000000, 950000, 0.95, $5)",
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

async fn stamp_liquidity(pool: &sqlx::PgPool, pair_id: i32, usd: &str) {
    sqlx::query(
        "INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (pair_id) DO UPDATE SET liquidity_usd = EXCLUDED.liquidity_usd, updated_at = NOW()",
    )
    .bind(pair_id)
    .bind(bd(usd))
    .execute(pool)
    .await
    .expect("liquidity stamp");
}

fn items(body: &Value) -> &[Value] {
    body["items"].as_array().expect("items array")
}

fn addrs(body: &Value) -> Vec<&str> {
    items(body)
        .iter()
        .map(|i| i["pair_address"].as_str().unwrap())
        .collect()
}

async fn get_top(server: &TestServer, qs: &str) -> axum_test::TestResponse {
    let path = if qs.is_empty() {
        "/api/v1/protocol/top-pairs".to_string()
    } else {
        format!("/api/v1/protocol/top-pairs?{qs}")
    };
    server.get(&path).await
}

#[serial]
#[tokio::test]
async fn ranks_priced_economic_pairs_by_30d_usd_then_pair_id() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;

    let a = insert_pair(&pool, "terra1top1263a", uusd, uluna).await;
    insert_swap(&pool, a, uusd, uluna, "tx1263a", Some("30")).await;
    stamp_liquidity(&pool, a, "100").await;

    let b = insert_pair(&pool, "terra1top1263b", uusd, uluna).await;
    insert_swap(&pool, b, uusd, uluna, "tx1263b", Some("10")).await;
    stamp_liquidity(&pool, b, "50").await;

    let c = insert_pair(&pool, "terra1top1263c", uusd, uluna).await;
    insert_swap(&pool, c, uusd, uluna, "tx1263c", Some("20")).await;
    stamp_liquidity(&pool, c, "80").await;

    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("refresh 30d");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = get_top(&server, "").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(
        addrs(&body),
        vec!["terra1top1263a", "terra1top1263c", "terra1top1263b"]
    );
    let first = &items(&body)[0];
    assert_eq!(first["asset_0"]["symbol"], "USTC");
    assert_eq!(first["asset_1"]["symbol"], "LUNC");
    assert_eq!(first["volume_usd_30d"], "30");
    assert_eq!(first["liquidity_usd"], "100");
    assert_eq!(first["volume_per_tvl"], "0.3");
}

#[serial]
#[tokio::test]
async fn caps_at_five_and_tie_breaks_on_pair_id() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;

    for i in 0..7 {
        let addr = format!("terra1cap1263{i}");
        let pid = insert_pair(&pool, &addr, uusd, uluna).await;
        let vol = if i < 2 { "50" } else { "40" };
        insert_swap(&pool, pid, uusd, uluna, &format!("txcap1263{i}"), Some(vol)).await;
        stamp_liquidity(&pool, pid, "10").await;
    }

    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("refresh 30d");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = get_top(&server, "").await.json();
    let ranked = addrs(&body);
    assert_eq!(ranked.len(), 5);
    assert_eq!(ranked[0], "terra1cap12630");
    assert_eq!(ranked[1], "terra1cap12631");
    assert!(!ranked.iter().any(|a| *a == "terra1cap12636"));
}

#[serial]
#[tokio::test]
async fn idle_unpriced_overflow_and_zero_tvl() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;

    let live = insert_pair(&pool, "terra1live1263", uusd, uluna).await;
    insert_swap(&pool, live, uusd, uluna, "txlive1263", Some("12")).await;
    stamp_liquidity(&pool, live, "0").await;

    let missing_tvl = insert_pair(&pool, "terra1notvl1263", uusd, uluna).await;
    insert_swap(&pool, missing_tvl, uusd, uluna, "txnotvl1263", Some("11")).await;

    let unpriced = insert_pair(&pool, "terra1unp1263", uusd, uluna).await;
    insert_swap(&pool, unpriced, uusd, uluna, "txunp1263", None).await;
    stamp_liquidity(&pool, unpriced, "999").await;

    let idle = insert_pair(&pool, "terra1idle1263", uusd, uluna).await;
    insert_swap(&pool, idle, uusd, uluna, "txidle1263", Some("8")).await;
    stamp_liquidity(&pool, idle, "40").await;

    let overflow = insert_pair(&pool, "terra1ovf1263", uusd, uluna).await;
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 12631, NOW(), 'txovf1263a', 'terra1trader1263', $2, $3, 1, 1, 1, $4),
                ($1, 12632, NOW(), 'txovf1263b', 'terra1trader1263', $2, $3, 1, 1, 1, $4)",
    )
    .bind(overflow)
    .bind(uusd)
    .bind(uluna)
    .bind(bd("6e19"))
    .execute(&pool)
    .await
    .expect("overflow swaps");

    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("stamp while live");
    sqlx::query(
        "UPDATE swap_events SET block_timestamp = NOW() - INTERVAL '31 days' WHERE tx_hash = 'txidle1263'",
    )
    .execute(&pool)
    .await
    .expect("age idle");
    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("refresh after age");

    let idle_usd: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_30d WHERE pair_id = $1")
            .bind(idle)
            .fetch_one(&pool)
            .await
            .expect("idle row");
    assert_eq!(idle_usd.unwrap().normalized(), bd("0").normalized());

    let ovf: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM pair_volume_30d WHERE pair_id = $1")
            .bind(overflow)
            .fetch_one(&pool)
            .await
            .expect("overflow row");
    assert!(ovf.is_none(), "overflow ≥ 10^20 must stamp NULL");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = get_top(&server, "").await.json();
    let ranked = addrs(&body);
    assert_eq!(ranked, vec!["terra1live1263", "terra1notvl1263"]);
    let live_item = &items(&body)[0];
    assert!(
        live_item["liquidity_usd"].is_null()
            || live_item.as_object().unwrap().get("liquidity_usd").is_none()
    );
    assert!(
        live_item["volume_per_tvl"].is_null()
            || live_item.as_object().unwrap().get("volume_per_tvl").is_none()
    );
    let missing = &items(&body)[1];
    assert!(
        missing["volume_per_tvl"].is_null()
            || missing.as_object().unwrap().get("volume_per_tvl").is_none()
    );
}

#[serial]
#[tokio::test]
async fn gems_lose_to_lower_volume_economic_pairs() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let gem0 = insert_cw20(&pool, COLUMBUS5_GEM_ADDRESSES[0], "UST1", 6).await;
    let gem1 = insert_cw20(&pool, COLUMBUS5_GEM_ADDRESSES[1], "USTR", 6).await;

    let gem_pair = insert_pair(&pool, "terra1gem1263", gem0, gem1).await;
    insert_swap(&pool, gem_pair, gem0, gem1, "txgem1263", Some("900")).await;
    stamp_liquidity(&pool, gem_pair, "10").await;

    let econ = insert_pair(&pool, "terra1econ1263", uusd, uluna).await;
    insert_swap(&pool, econ, uusd, uluna, "txecon1263", Some("5")).await;
    stamp_liquidity(&pool, econ, "20").await;

    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("refresh 30d");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = get_top(&server, "").await.json();
    assert_eq!(addrs(&body), vec!["terra1econ1263"]);
    assert_eq!(items(&body)[0]["volume_usd_30d"], "5");
    assert_eq!(items(&body)[0]["volume_per_tvl"], "0.25");
}

#[serial]
#[tokio::test]
async fn empty_priced_window_returns_no_items() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: Value = get_top(&server, "").await.json();
    assert_eq!(items(&body).len(), 0);
}

#[serial]
#[tokio::test]
async fn get_400_on_disallowed_params_and_cache_ignores_junk() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pid = insert_pair(&pool, "terra1cache1263", uusd, uluna).await;
    insert_swap(&pool, pid, uusd, uluna, "txcache1263", Some("7")).await;
    stamp_liquidity(&pool, pid, "70").await;
    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("refresh 30d");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    get_top(&server, "limit=5").await.assert_status_ok();
    get_top(&server, "window=30d").await.assert_status_ok();
    get_top(&server, "limit=5&window=30d")
        .await
        .assert_status_ok();

    for qs in [
        "limit=1",
        "limit=6",
        "limit=100000",
        "window=24h",
        "window=7d",
        "from=2026-01-01",
        "to=2026-02-01",
        "sort=volume_usd_24h",
        "ticker=ustc",
    ] {
        let resp = get_top(&server, qs).await;
        assert_eq!(resp.status_code(), 400, "{qs} must 400");
    }

    reset_protocol_top_pairs_cache();
    let first = get_top(&server, "").await;
    first.assert_status_ok();
    let first_body: Value = first.json();
    assert_eq!(items(&first_body)[0]["volume_usd_30d"], "7");

    sqlx::query("UPDATE pair_volume_30d SET volume_usd = 99 WHERE pair_id = $1")
        .bind(pid)
        .execute(&pool)
        .await
        .expect("mutate stamp");
    let junk = get_top(&server, "foo=bust&bar=1").await;
    junk.assert_status_ok();
    let junk_body: Value = junk.json();
    assert_eq!(
        items(&junk_body)[0]["volume_usd_30d"], "7",
        "junk query keys must not bust the 60s cache"
    );
}

#[serial]
#[tokio::test]
async fn explain_top_pairs_does_not_scan_swap_events_or_reserves() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let sql = format!(
        "EXPLAIN (FORMAT TEXT) {}",
        LIST_TOP_PAIRS_SQL.replace("$1", "'{}'::text[]")
    );
    let rows: Vec<(String,)> = sqlx::query_as(&sql)
        .fetch_all(&pool)
        .await
        .expect("explain");
    let plan = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plan.contains("swap_events"),
        "GET top-pairs must not scan swap_events:\n{plan}"
    );
    assert!(
        !plan.contains("pair_reserves"),
        "GET top-pairs must not scan pair_reserves:\n{plan}"
    );
}

#[serial]
#[tokio::test]
async fn overview_stays_rollup_after_30d_stamp() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_protocol_top_pairs_cache();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pid = insert_pair(&pool, "terra1ov1263", uusd, uluna).await;
    insert_swap(&pool, pid, uusd, uluna, "txov1263", Some("3")).await;
    volume::refresh_pair_volumes_30d(&pool)
        .await
        .expect("30d");
    volume::refresh_global_stats(&pool)
        .await
        .expect("global");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let overview = server.get("/api/v1/overview").await;
    overview.assert_status_ok();
    let body: Value = overview.json();
    assert!(body.get("total_volume_30d_usd").is_some());
    assert!(body.get("items").is_none());
}
