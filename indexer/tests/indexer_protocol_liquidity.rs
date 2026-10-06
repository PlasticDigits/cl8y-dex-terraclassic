//! GitLab #569 — protocol pool TVL rollup, snapshots, and cheap `/overview` reads.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Datelike, Duration, Utc};
use cl8y_dex_indexer::api::reset_overview_cache;
use cl8y_dex_indexer::config::{
    DEFAULT_HUB_CUSTC_ADDRESS, DEFAULT_HUB_UST1_ADDRESS, DEFAULT_HUB_USTR_ADDRESS,
};
use cl8y_dex_indexer::db::queries::{liquidity_snapshots, volume};
use cl8y_dex_indexer::indexer::protocol_tvl;
use common::{clean_db, seed_db, setup_pool};
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
    let addr = match symbol {
        "cUSTC" => DEFAULT_HUB_CUSTC_ADDRESS,
        "UST1" => DEFAULT_HUB_UST1_ADDRESS,
        "USTR" => DEFAULT_HUB_USTR_ADDRESS,
        _ => addr,
    };
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
    let pair_id = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, $4, 30) RETURNING id",
    )
    .bind(addr)
    .bind(a0)
    .bind(a1)
    .bind(format!("{addr}lp"))
    .fetch_one(pool)
    .await
    .expect("pair");
    cl8y_dex_indexer::db::queries::state::set_state(
        pool,
        "factory_inventory_observed_at",
        &Utc::now().to_rfc3339(),
    )
    .await
    .expect("complete fixture inventory");
    pair_id
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
    cl8y_dex_indexer::db::queries::oracle::record_observation(
        pool,
        cl8y_dex_indexer::indexer::oracle::OracleTicker::parse(ticker).unwrap(),
        Some(&bd(price)),
        &["kucoin".into(), "mexc".into()],
        "fresh",
    )
    .await
    .expect("certified synthetic oracle");
}

#[serial]
#[tokio::test]
async fn t8_unknown_inventory_total_null_delta_null() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    sqlx::query("DELETE FROM indexer_state WHERE key='factory_inventory_observed_at'")
        .execute(&pool)
        .await
        .unwrap();
    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh empty");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_liquidity_usd, None);
    assert!(stats.liquidity_change_24h_pct.is_none());
    assert!(stats.liquidity_change_30d_pct.is_none());
}

#[serial]
#[tokio::test]
async fn t1_two_catalogued_legs_sum_humanized() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1tvl1", uusd, uluna).await;
    // 100 human USTC + 1000 human LUNC
    insert_reserves(&pool, pair, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // 100*0.01 + 1000*0.0001 = 1 + 0.1 = 1.1
    assert_eq!(rollup.total_liquidity_usd, Some(bd("1.1")));
    assert_eq!(rollup.priced_pair_count, 1);
    assert!(rollup.liquidity_change_24h_pct.is_none());

    let stamped: Option<BigDecimal> =
        sqlx::query_scalar("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
            .bind(pair)
            .fetch_optional(&pool)
            .await
            .expect("stamp");
    assert_eq!(stamped.as_ref(), Some(&bd("1.1")));
}

#[serial]
#[tokio::test]
async fn t2_one_catalogued_leg_doubles() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let gem = insert_cw20(&pool, "terra1gem569", "GEM", 18).await;
    let pair = insert_pair(&pool, "terra1oneside", uusd, gem).await;
    insert_reserves(&pool, pair, "200000000", "100000000000000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // 2 * 200 * 0.01 = 4
    assert_eq!(rollup.total_liquidity_usd, Some(bd("4")));
    assert_eq!(rollup.priced_pair_count, 1);
    assert_eq!(rollup.unpriced_pair_count, 0);
}

#[serial]
#[tokio::test]
async fn t3_unpriced_pair_omitted_not_zero_drag() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let g0 = insert_cw20(&pool, "terra1g0", "GEM", 6).await;
    let g1 = insert_cw20(&pool, "terra1g1", "GEM2", 6).await;
    let pair = insert_pair(&pool, "terra1gemgem", g0, g1).await;
    insert_reserves(&pool, pair, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert_eq!(rollup.total_liquidity_usd, None);
    assert_eq!(rollup.priced_pair_count, 0);
    assert!(rollup.unpriced_pair_count >= 1);
    let leftover: Option<BigDecimal> =
        sqlx::query_scalar("SELECT liquidity_usd FROM pair_liquidity_usd WHERE pair_id = $1")
            .bind(pair)
            .fetch_optional(&pool)
            .await
            .expect("leftover stamp");
    assert!(leftover.is_none(), "unpriced pair must have no rollup row");
}

#[serial]
#[tokio::test]
async fn t4_ust1_uses_hub_not_dollar() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let ust1 = insert_cw20(&pool, "terra1ust1569", "UST1", 6).await;
    let custc = insert_cw20(&pool, "terra1custc569", "cUSTC", 6).await;
    let pair = insert_pair(&pool, "terra1ust1custc", ust1, custc).await;
    insert_reserves(&pool, pair, "10000000", "20000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    sqlx::query(
        "INSERT INTO hub_prices (ticker, asset_id, price_usd, updated_at, as_of, quality)
         VALUES ('ust1', $1, 0.98, NOW(), NOW(), 'bounded_spot')",
    )
    .bind(ust1)
    .execute(&pool)
    .await
    .unwrap();

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // 10*0.98 + 20*0.01 = 10.0 — not 10*$1 + 0.2 = 10.2
    assert_eq!(rollup.total_liquidity_usd, Some(bd("10.0")));
}

#[serial]
#[tokio::test]
async fn t5_ustr_uses_hub_not_2_5x() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let ust1 = insert_cw20(&pool, "terra1ust1569b", "UST1", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr569", "USTR", 18).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    let pair = insert_pair(&pool, "terra1ustrust1", ust1, ustr).await;
    insert_reserves(&pool, pair, "10000000", "100000000000000000000", now).await;
    sqlx::query(
        "INSERT INTO hub_prices (ticker, asset_id, price_usd, updated_at, as_of, quality)
         VALUES ('ust1', $1, 0.98, NOW(), NOW(), 'bounded_spot'), ('ustr', $2, 0.012, NOW(), NOW(), 'bounded_spot')",
    )
    .bind(ust1)
    .bind(ustr)
    .execute(&pool)
    .await
    .unwrap();

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // 10*0.98 + 100*0.012 = 11.0
    assert_eq!(rollup.total_liquidity_usd, Some(bd("11.0")));
}

#[serial]
#[tokio::test]
async fn t6_stale_pool_omitted_positive_remaining_reserve_is_partial() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let stale = insert_pair(&pool, "terra1stale", uusd, uluna).await;
    let zero = insert_pair(&pool, "terra1zero", uusd, uluna).await;
    insert_reserves(
        &pool,
        stale,
        "100000000",
        "1000000000",
        now - Duration::hours(1),
    )
    .await;
    insert_reserves(&pool, zero, "0", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert_eq!(rollup.total_liquidity_usd, Some(bd("0.0001")));
    assert_eq!(rollup.quality.status, "partial");
    assert_eq!(rollup.priced_pair_count, 1);

    let leftover: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pair_liquidity_usd")
        .fetch_one(&pool)
        .await
        .expect("stamp count");
    assert_eq!(
        leftover, 1,
        "fresh positive remaining reserve is stamped while stale pair drops out"
    );
}

#[serial]
#[tokio::test]
async fn t10_hub_down_omits_ust1_without_peg() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let ust1 = insert_cw20(&pool, "terra1ust1down", "UST1", 6).await;
    let gem = insert_cw20(&pool, "terra1gemdown", "GEM", 6).await;
    let pair = insert_pair(&pool, "terra1hubdown", ust1, gem).await;
    insert_reserves(&pool, pair, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert_eq!(rollup.total_liquidity_usd, None);
}

#[serial]
#[tokio::test]
async fn h1_h2_snapshot_pct_matches_formula() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1pct", uusd, uluna).await;
    insert_reserves(&pool, pair, "100000000", "0", now).await;
    // usable requires both reserves > 0; use positive both legs
    sqlx::query("UPDATE pair_reserves SET reserve_1 = 1000000000 WHERE pair_id = $1")
        .bind(pair)
        .execute(&pool)
        .await
        .unwrap();
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    liquidity_snapshots::insert_snapshot(&pool, now - Duration::hours(24), &bd("1.0"), 1)
        .await
        .unwrap();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::days(30), &bd("2.2"), 1)
        .await
        .unwrap();

    let current = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE global_liquidity_snapshots SET quality='fresh',as_of=sampled_at,
        coverage_identity=$1,generation=1,reasons='{}' WHERE sampled_at<NOW()-interval '1 hour'",
    )
    .bind(&current.quality.coverage_identity)
    .execute(&pool)
    .await
    .unwrap();
    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // current 1.1; 24h then 1.0 → +10%; 30d then 2.2 → (1.1-2.2)/2.2 * 100
    assert_eq!(rollup.total_liquidity_usd, Some(bd("1.1")));
    let p24 = rollup.liquidity_change_24h_pct.expect("24h pct");
    let p30 = rollup.liquidity_change_30d_pct.expect("30d pct");
    assert_eq!(
        p24,
        protocol_tvl::pct_change(&bd("1.1"), &bd("1.0")).unwrap()
    );
    assert_eq!(
        p30,
        protocol_tvl::pct_change(&bd("1.1"), &bd("2.2")).unwrap()
    );
}

#[serial]
#[tokio::test]
async fn h3_no_baseline_snapshots_both_pct_null() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.liquidity_change_24h_pct.is_none());
    assert!(stats.liquidity_change_30d_pct.is_none());
}

#[serial]
#[tokio::test]
async fn h4_then_zero_pct_null_not_inf() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::hours(24), &bd("0"), 0)
        .await
        .unwrap();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1fromzero", uusd, uluna).await;
    insert_reserves(&pool, pair, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert!(rollup
        .total_liquidity_usd
        .as_ref()
        .is_some_and(|v| v > &bd("0")));
    assert!(rollup.liquidity_change_24h_pct.is_none());
}

#[serial]
#[tokio::test]
async fn h5_closer_24h_snapshot_wins_over_25h() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::hours(25), &bd("9"), 1)
        .await
        .unwrap();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::hours(24), &bd("1"), 1)
        .await
        .unwrap();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1win", uusd, uluna).await;
    insert_reserves(&pool, pair, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    let current = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE global_liquidity_snapshots SET quality='fresh',as_of=sampled_at,
        coverage_identity=$1,generation=1,reasons='{}' WHERE sampled_at<NOW()-interval '1 hour'",
    )
    .bind(&current.quality.coverage_identity)
    .execute(&pool)
    .await
    .unwrap();
    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert_eq!(
        rollup
            .total_liquidity_usd_24h_ago
            .as_ref()
            .map(|v| v.normalized()),
        Some(bd("1").normalized())
    );
    assert_eq!(
        rollup.liquidity_change_24h_pct,
        protocol_tvl::pct_change(&bd("1.1"), &bd("1"))
    );
}

#[serial]
#[tokio::test]
async fn h6_prune_drops_older_than_35d_keeps_32d() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::days(40), &bd("1"), 1)
        .await
        .unwrap();
    liquidity_snapshots::insert_snapshot(&pool, now - Duration::days(32), &bd("2"), 1)
        .await
        .unwrap();
    liquidity_snapshots::prune_snapshots(&pool, now)
        .await
        .expect("prune");
    let old: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM global_liquidity_snapshots WHERE total_liquidity_usd = 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let kept: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM global_liquidity_snapshots WHERE total_liquidity_usd = 2",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(old, 0);
    assert_eq!(kept, 1);
}

#[serial]
#[tokio::test]
async fn h7_refresh_global_stats_updates_liquidity_columns() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let uusd = insert_native(&pool, "uusd", "USTC", 6).await;
    let uluna = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1h7", uusd, uluna).await;
    insert_reserves(&pool, pair, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh must update liquidity too");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_liquidity_usd, Some(bd("1.1")));
}

#[serial]
#[tokio::test]
async fn v1_v2_overview_additive_keys() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    reset_overview_cache();
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body["total_volume_24h"].is_string());
    assert!(body["total_volume_7d_usd"].is_string());
    assert!(body["total_liquidity_usd"].is_null());
    assert!(body["liquidity_quality"].is_string());
    assert!(body.get("liquidity_change_24h_pct").is_some());
    assert!(body.get("liquidity_change_30d_pct").is_some());
    assert!(body["priced_pair_count"].is_i64());
}

#[serial]
#[tokio::test]
async fn v3_cache_miss_does_not_scan_snapshots_or_swaps() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT total_volume, total_volume_usd, total_trades,
                total_volume_7d_usd, total_volume_30d_usd,
                total_trades_7d, total_trades_30d,
                active_pairs_24h, unique_traders_24h,
                total_liquidity_usd, liquidity_change_24h_pct, liquidity_change_30d_pct,
                priced_pair_count, unpriced_pair_count,
                total_liquidity_usd_24h_ago, total_liquidity_usd_30d_ago
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_all(&pool)
    .await
    .expect("explain");
    let plan = rows
        .into_iter()
        .map(|(l,)| l)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plan.contains("swap_events"),
        "GET rollup must not scan swap_events:\n{plan}"
    );
    assert!(
        !plan.contains("global_liquidity_snapshots"),
        "GET rollup must not walk snapshot history:\n{plan}"
    );
    assert!(
        !plan.contains("pair_reserves"),
        "GET rollup must not recompute pair TVL:\n{plan}"
    );
    assert!(plan.contains("global_stats_24h"), "plan:\n{plan}");
}

#[serial]
#[tokio::test]
async fn v4_overview_cache_burst_does_not_recompute_tvl() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    reset_overview_cache();
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let first = server.get("/api/v1/overview").await;
    first.assert_status_ok();
    let body1: Value = first.json();
    let second = server.get("/api/v1/overview").await;
    second.assert_status_ok();
    let body2: Value = second.json();
    assert_eq!(body1, body2);
}

#[serial]
#[tokio::test]
async fn a14_no_backfill_from_liquidity_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    // seed_db / refresh may write a *current* snapshot; it must not invent 24h/30d history.
    let old: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM global_liquidity_snapshots
         WHERE sampled_at < NOW() - INTERVAL '1 hour'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        old, 0,
        "must not backfill aged snapshots from liquidity_events"
    );
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.liquidity_change_24h_pct.is_none());
    assert!(stats.liquidity_change_30d_pct.is_none());
}

#[serial]
#[tokio::test]
async fn a5_spoof_native_ustr_omitted() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let spoof = insert_native(&pool, "ugem", "USTR", 18).await;
    let gem = insert_cw20(&pool, "terra1gemspoof", "GEM", 6).await;
    let pair = insert_pair(&pool, "terra1spoof", spoof, gem).await;
    insert_reserves(&pool, pair, "1000000000000000000", "1000000", now).await;
    sqlx::query(
        "INSERT INTO hub_prices (ticker, price_usd, updated_at) VALUES ('ustr', 0.012, NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    assert_eq!(rollup.total_liquidity_usd, None);
}

#[serial]
#[tokio::test]
async fn q1380_outage_partial_recovery_and_real_withdrawal() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let a = insert_native(&pool, "uusd", "USTC", 6).await;
    let b = insert_native(&pool, "uluna", "LUNC", 6).await;
    let p = insert_pair(&pool, "terra1qualitylarge", a, b).await;
    let small = insert_pair(&pool, "terra1qualitysmall", a, b).await;
    let unsupported0 = insert_cw20(&pool, "terra1unsupported0", "GEM", 6).await;
    let unsupported1 = insert_cw20(&pool, "terra1unsupported1", "OTHER", 6).await;
    insert_pair(&pool, "terra1unsupported", unsupported0, unsupported1).await;
    insert_reserves(&pool, p, "100000000", "1000000000", now).await;
    insert_reserves(&pool, small, "1000000", "1000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    let refresh = || {
        protocol_tvl::refresh_protocol_liquidity_with_staleness(
            &pool,
            std::time::Duration::from_secs(60),
        )
    };
    let good = refresh().await.unwrap();
    assert_eq!(good.quality.status, "fresh");
    assert_eq!(good.total_liquidity_usd, Some(bd("1.1101")));
    assert_eq!(good.quality.reasons["unsupported_identity"], 1);
    for baseline in [now - Duration::hours(24), now - Duration::days(30)] {
        liquidity_snapshots::insert_snapshot(&pool, baseline, &bd("1.1101"), 2)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE global_liquidity_snapshots SET quality='fresh',as_of=sampled_at,
            coverage_identity=$1,reasons='{}' WHERE sampled_at=$2",
        )
        .bind(&good.quality.coverage_identity)
        .bind(baseline)
        .execute(&pool)
        .await
        .unwrap();
    }

    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()-interval '1 hour' WHERE pair_id=$1")
        .bind(p)
        .execute(&pool)
        .await
        .unwrap();
    let partial = refresh().await.unwrap();
    assert_eq!(partial.quality.status, "partial");
    assert_eq!(partial.total_liquidity_usd, Some(bd("0.0101")));
    assert!(partial.liquidity_change_24h_pct.is_none());
    assert_eq!(
        partial.quality.coverage_identity,
        good.quality.coverage_identity
    );
    sqlx::query("DELETE FROM pair_reserves WHERE pair_id=$1")
        .bind(small)
        .execute(&pool)
        .await
        .unwrap();
    let lost = refresh().await.unwrap();
    assert_eq!(lost.quality.status, "unavailable");
    assert!(lost.total_liquidity_usd.is_none());
    assert_eq!(lost.quality.reasons["missing_reserve"], 1);
    assert_eq!(lost.quality.reasons["stale_reserve"], 1);
    assert!(lost.liquidity_change_24h_pct.is_none());
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pair_liquidity_usd")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let at = Utc::now();
    insert_reserves(&pool, small, "1000000", "1000000", at).await;
    sqlx::query("UPDATE pair_reserves SET snapshot_at=$1 WHERE pair_id=$2")
        .bind(at)
        .bind(p)
        .execute(&pool)
        .await
        .unwrap();
    let recovered = refresh().await.unwrap();
    assert_eq!(recovered.total_liquidity_usd, good.total_liquidity_usd);
    assert_eq!(recovered.quality.status, "fresh");
    assert!(recovered.quality.generation > lost.quality.generation);
    sqlx::query("DELETE FROM oracle_state")
        .execute(&pool)
        .await
        .unwrap();
    let unpriced = refresh().await.unwrap();
    assert_eq!(unpriced.quality.status, "unavailable");
    assert!(unpriced.total_liquidity_usd.is_none());
    assert_eq!(unpriced.quality.reasons["missing_quote"], 2);
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    sqlx::query("UPDATE pair_reserves SET reserve_0=0,reserve_1=0,snapshot_at=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    let withdrawn = refresh().await.unwrap();
    assert_eq!(withdrawn.total_liquidity_usd, Some(bd("0")));
    assert_eq!(withdrawn.quality.status, "fresh");
    assert_eq!(withdrawn.liquidity_change_24h_pct, Some(bd("-100")));
    assert_eq!(
        withdrawn.liquidity_change_30d_pct,
        Some(bd("-100")),
        "real withdrawal-to-zero remains visible with qualified positive baselines"
    );
}

fn quality_rollup(
    status: &str,
    total: Option<BigDecimal>,
    at: chrono::DateTime<Utc>,
    generation: i64,
) -> liquidity_snapshots::LiquidityRollup {
    liquidity_snapshots::LiquidityRollup {
        total_liquidity_usd: total,
        priced_pair_count: 1,
        quality: liquidity_snapshots::LiquidityQuality {
            status: status.into(),
            as_of: Some(at),
            coverage_identity: "fixture-member-a".into(),
            generation,
            reasons: serde_json::json!({}),
        },
        ..Default::default()
    }
}

#[serial]
#[tokio::test]
async fn q1380_terminal_unknown_all_grains_throttle_and_baseline() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let boundary = cl8y_dex_indexer::db::queries::protocol_volume::utc_month_start(Utc::now())
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let at = boundary - Duration::seconds(20);
    let good = quality_rollup("fresh", Some(bd("100")), at, 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], at)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let bad = quality_rollup("unavailable", None, at + Duration::seconds(10), 2);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &bad, &[], at + Duration::seconds(10))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM global_liquidity_snapshots")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "same four-minute slot is terminal coalesced");
    for table in [
        "protocol_hourly_liquidity",
        "protocol_daily_liquidity",
        "protocol_monthly_liquidity",
    ] {
        let row: (Option<BigDecimal>, String, i64) = sqlx::query_as(&format!(
            "SELECT liquidity_usd,quality,generation FROM {table}"
        ))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            (None, "unavailable".into(), 2),
            "{table} terminal unknown must replace good close"
        );
    }
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    assert!(
        liquidity_snapshots::nearest_comparable_snapshot_tx(&mut tx, at, "fixture-member-a")
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
    // Recovery in the next month must not rewrite the previous unknown month.
    let recovery = quality_rollup(
        "fresh",
        Some(bd("100")),
        boundary + Duration::seconds(10),
        3,
    );
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(
        &mut tx,
        &recovery,
        &[],
        boundary + Duration::seconds(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let rows = cl8y_dex_indexer::db::queries::protocol_liquidity::get_monthly_rows(
        &pool,
        at.date_naive().with_day(1).unwrap(),
        boundary.date_naive(),
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].quality, "unavailable");
    assert!(rows[0].liquidity_usd.is_none());
    assert_eq!(rows[1].liquidity_usd, Some(bd("100")));
    // An outage followed by genuine recovery inside the next bucket closes fresh;
    // its terminal timestamp is the recovery, while the prior month stays unknown.
    for (seconds, status, total, generation) in [
        (20, "unavailable", None, 4),
        (30, "fresh", Some(bd("100")), 5),
    ] {
        let at = boundary + Duration::seconds(seconds);
        let observation = quality_rollup(status, total, at, generation);
        let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
            .await
            .unwrap();
        liquidity_snapshots::publish_liquidity_refresh(&mut tx, &observation, &[], at)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let rows = cl8y_dex_indexer::db::queries::protocol_liquidity::get_monthly_rows(
        &pool,
        at.date_naive().with_day(1).unwrap(),
        boundary.date_naive(),
    )
    .await
    .unwrap();
    assert_eq!(rows[0].quality, "unavailable");
    assert_eq!(rows[1].quality, "fresh");
    assert_eq!(rows[1].as_of, Some(boundary + Duration::seconds(30)));
}

#[serial]
#[tokio::test]
async fn q1380_publication_rollback_and_replayed_generation() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let at = Utc::now();
    let good = quality_rollup("fresh", Some(bd("100")), at, 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], at)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let bad = quality_rollup("unavailable", None, at + Duration::seconds(1), 2);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    let invalid = cl8y_dex_indexer::db::queries::pair_liquidity_usd::PairLiquidityStamp {
        pair_id: i32::MAX,
        liquidity_usd: bd("5"),
    };
    assert!(liquidity_snapshots::publish_liquidity_refresh(
        &mut tx,
        &bad,
        &[invalid],
        at + Duration::seconds(1)
    )
    .await
    .is_err());
    tx.rollback().await.unwrap();
    let kept = liquidity_snapshots::get_liquidity_rollup(&pool)
        .await
        .unwrap();
    assert_eq!(kept.total_liquidity_usd, Some(bd("100")));
    assert_eq!(kept.quality.generation, 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    assert!(
        liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], at)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        liquidity_snapshots::get_liquidity_rollup(&pool)
            .await
            .unwrap()
            .quality
            .generation,
        1
    );
}

#[serial]
#[tokio::test]
async fn q1380_scoped_history_repair_preserves_evidence_and_recovery() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "DELETE FROM liquidity_quality_repair_evidence WHERE repair_key='issue-1380-public-hours'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO global_liquidity_snapshots(sampled_at,total_liquidity_usd,priced_pair_count)
        VALUES('2026-10-02 05:10Z',218.57,2),('2026-10-02 05:50Z',230000,10),
        ('2026-10-04 02:59Z',0,0),('2026-10-03 02:59Z',0,0)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO protocol_hourly_liquidity(utc_hour,liquidity_usd,priced_pair_count)
        VALUES('2026-10-02 05:00Z',230000,10),('2026-10-04 02:00Z',0,0)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO protocol_daily_liquidity(utc_day,liquidity_usd,priced_pair_count)
        VALUES('2026-10-02',230000,10),('2026-10-04',0,0)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO protocol_monthly_liquidity(utc_month,liquidity_usd,priced_pair_count)
        VALUES('2026-10-01',0,0)",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let migration = include_str!("../migrations/20261004120000_liquidity_quality.sql");
    let repair = migration
        .split("-- Scoped evidence-preserving repair")
        .nth(1)
        .unwrap();
    let repair = format!("-- Scoped evidence-preserving repair{repair}");
    sqlx::raw_sql(&repair).execute(&mut *tx).await.unwrap();
    sqlx::raw_sql(&repair).execute(&mut *tx).await.unwrap();
    let evidence: (i64, Option<String>) = sqlx::query_as(
        "SELECT COUNT(*),MAX(original_row->>'total_liquidity_usd')
        FROM liquidity_quality_repair_evidence WHERE source_table='global_liquidity_snapshots'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        evidence.0, 2,
        "repair replay does not duplicate original evidence"
    );
    assert!(evidence.1.is_some());
    let recovered: Option<BigDecimal> = sqlx::query_scalar(
        "SELECT total_liquidity_usd FROM global_liquidity_snapshots
        WHERE sampled_at='2026-10-02 05:50Z'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        recovered,
        Some(bd("230000")),
        "healthy recovery in cited hour is preserved"
    );
    let other: Option<BigDecimal> = sqlx::query_scalar(
        "SELECT total_liquidity_usd FROM global_liquidity_snapshots
        WHERE sampled_at='2026-10-03 02:59Z'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        other,
        Some(bd("0")),
        "uncited zero is never assumed corrupt"
    );
    let daily: Vec<(chrono::NaiveDate, Option<BigDecimal>)> = sqlx::query_as(
        "SELECT utc_day,liquidity_usd
        FROM protocol_daily_liquidity ORDER BY utc_day",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(daily[0].1, Some(bd("230000")));
    assert!(
        daily[1].1.is_none(),
        "derived terminal from cited anomaly must be unknown"
    );
    let monthly: Option<BigDecimal> =
        sqlx::query_scalar("SELECT liquidity_usd FROM protocol_monthly_liquidity")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(monthly.is_none());
    let before_evidence: Vec<(String, String, Value)> = sqlx::query_as(
        "SELECT source_table,source_key,original_row FROM liquidity_quality_repair_evidence
        WHERE repair_key='issue-1380-public-hours' ORDER BY source_table,source_key",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    // A later observation now supplies the same day's and month's healthy close.
    // Existing audit evidence must not re-invalidate these recovered bucket rows.
    sqlx::query(
        "INSERT INTO global_liquidity_snapshots(sampled_at,total_liquidity_usd,
        priced_pair_count,quality,as_of,coverage_identity,generation,reasons)
        VALUES('2026-10-04 03:00Z',250000,11,'fresh','2026-10-04 03:00Z',
        'recovered-members',3,'{}')",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE protocol_daily_liquidity SET liquidity_usd=250000,priced_pair_count=11,
        quality='fresh',as_of='2026-10-04 03:00Z',coverage_identity='recovered-members',
        generation=3,reasons='{}' WHERE utc_day='2026-10-04'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE protocol_monthly_liquidity SET liquidity_usd=250000,priced_pair_count=11,
        quality='fresh',as_of='2026-10-04 03:00Z',coverage_identity='recovered-members',
        generation=3,reasons='{}' WHERE utc_month='2026-10-01'",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql(&repair).execute(&mut *tx).await.unwrap();
    for (table, key, bucket) in [
        ("protocol_daily_liquidity", "utc_day", "2026-10-04"),
        ("protocol_monthly_liquidity", "utc_month", "2026-10-01"),
    ] {
        let recovered: (Option<BigDecimal>, String, i64) = sqlx::query_as(&format!(
            "SELECT liquidity_usd,quality,generation FROM {table} WHERE {key}=$1::date"
        ))
        .bind(bucket)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            recovered,
            (Some(bd("250000")), "fresh".into(), 3),
            "repair replay must preserve later recovered {table} close"
        );
    }
    let after_evidence: Vec<(String, String, Value)> = sqlx::query_as(
        "SELECT source_table,source_key,original_row FROM liquidity_quality_repair_evidence
        WHERE repair_key='issue-1380-public-hours' ORDER BY source_table,source_key",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        after_evidence, before_evidence,
        "original repair evidence remains immutable"
    );
    tx.rollback().await.unwrap();
}

#[serial]
#[tokio::test]
async fn q1380_nearest_baseline_cannot_skip_degraded_or_changed_membership() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let target = Utc::now() - Duration::hours(24);
    sqlx::query(
        "INSERT INTO global_liquidity_snapshots(sampled_at,total_liquidity_usd,priced_pair_count,
        quality,as_of,coverage_identity,generation,reasons)
        VALUES($1,100,1,'fresh',$1,'member-a',1,'{}'),
        ($2,NULL,0,'unavailable',$2,'member-a',2,'{\"missing_quote\":1}')",
    )
    .bind(target - Duration::minutes(1))
    .bind(target)
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    assert!(
        liquidity_snapshots::nearest_comparable_snapshot_tx(&mut tx, target, "member-a")
            .await
            .unwrap()
            .is_none(),
        "nearest unavailable observation must not fall back to earlier fresh"
    );
    tx.rollback().await.unwrap();
    sqlx::query(
        "UPDATE global_liquidity_snapshots SET quality='fresh',total_liquidity_usd=100,
        priced_pair_count=1,coverage_identity='member-b' WHERE sampled_at=$1",
    )
    .bind(target)
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    assert!(
        liquidity_snapshots::nearest_comparable_snapshot_tx(&mut tx, target, "member-a")
            .await
            .unwrap()
            .is_none(),
        "same pool count cannot establish membership comparability"
    );
    assert_eq!(
        liquidity_snapshots::nearest_comparable_snapshot_tx(&mut tx, target, "member-b")
            .await
            .unwrap()
            .unwrap()
            .total_liquidity_usd,
        Some(bd("100"))
    );
    tx.rollback().await.unwrap();
}

#[serial]
#[tokio::test]
async fn q1380_two_refresh_entrypoints_publish_serial_generations() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let a = insert_native(&pool, "uusd", "USTC", 6).await;
    let b = insert_native(&pool, "uluna", "LUNC", 6).await;
    let p = insert_pair(&pool, "terra1concurrent", a, b).await;
    insert_reserves(&pool, p, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    // Hub and aggregator use this common publisher, with different cadence inputs.
    let (a, b) = tokio::join!(
        protocol_tvl::refresh_protocol_liquidity(&pool),
        protocol_tvl::refresh_protocol_liquidity_with_staleness(
            &pool,
            std::time::Duration::from_secs(60)
        )
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_ne!(a.quality.generation, b.quality.generation);
    let final_generation = a.quality.generation.max(b.quality.generation);
    let persisted = liquidity_snapshots::get_liquidity_rollup(&pool)
        .await
        .unwrap();
    assert_eq!(persisted.quality.generation, final_generation);
    assert_eq!(persisted.total_liquidity_usd, Some(bd("1.1")));
    let stamp_generation: i64 =
        sqlx::query_scalar("SELECT generation FROM pair_liquidity_usd WHERE pair_id=$1")
            .bind(p)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stamp_generation, final_generation);
    let source_generation: i64 =
        sqlx::query_scalar("SELECT MAX(generation) FROM global_liquidity_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(source_generation, final_generation);
    for table in [
        "protocol_hourly_liquidity",
        "protocol_daily_liquidity",
        "protocol_monthly_liquidity",
    ] {
        let generation: i64 = sqlx::query_scalar(&format!("SELECT MAX(generation) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(generation, final_generation);
    }
}

#[serial]
#[tokio::test]
async fn q1380_overview_cache_hit_qualifies_liquidity_age_independently() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_overview_cache();
    let now = Utc::now();
    let ttl = protocol_tvl::max_staleness_from_env().as_secs();
    let as_of = now - Duration::seconds(ttl.saturating_sub(1) as i64);
    let good = quality_rollup("fresh", Some(bd("100")), as_of, 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], now)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    let before: Value = server.get("/api/v1/overview").await.json();
    assert_eq!(before["liquidity_quality"], "fresh");
    assert!(before["total_liquidity_usd"].is_string());
    sqlx::query("UPDATE global_stats_24h SET updated_at=NOW() WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    let cached: Value = server.get("/api/v1/overview").await.json();
    assert_eq!(cached["liquidity_quality"], "stale");
    assert!(cached["total_liquidity_usd"].is_null());
    assert!(cached["liquidity_change_24h_pct"].is_null());
    assert_eq!(cached["liquidity_as_of"], before["liquidity_as_of"]);
    assert_eq!(
        cached["liquidity_generation"],
        before["liquidity_generation"]
    );
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_overview_cached_stats_follow_new_liquidity_generations_without_smoothing() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_overview_cache();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    let ttl = protocol_tvl::max_staleness_from_env().as_secs();
    let old_as_of = Utc::now() - Duration::seconds(ttl.saturating_sub(1) as i64);
    let initial = quality_rollup("fresh", Some(bd("100")), old_as_of, 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &initial, &[], Utc::now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE global_stats_24h SET total_volume=10,total_trades=1,total_fees_24h_usd=7,fee_event_count_24h=1 WHERE id=1")
        .execute(&pool).await.unwrap();
    let before_response = server.get("/api/v1/overview").await;
    before_response.assert_status_ok();
    let before: Value = before_response.json();
    assert_eq!(before["liquidity_quality"], "fresh");

    let mut next = quality_rollup("fresh", Some(bd("80")), Utc::now(), 2);
    next.priced_pair_count = 3;
    next.unpriced_pair_count = 0;
    next.quality.coverage_identity = "coverage-v2".into();
    next.quality.reasons = serde_json::json!({"complete":3});
    next.liquidity_change_24h_pct = Some(bd("-20"));
    next.liquidity_change_30d_pct = Some(bd("60"));
    next.total_liquidity_usd_24h_ago = Some(bd("100"));
    next.total_liquidity_usd_30d_ago = Some(bd("50"));
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &next, &[], Utc::now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::query("UPDATE global_stats_24h SET total_volume=999,total_fees_24h_usd=999 WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    // Expire the original liquidity observation while the expensive stats cache
    // remains well inside its 60s lifetime. The committed new generation is fresh.
    tokio::time::sleep(std::time::Duration::from_millis(2100)).await;
    let response = server.get("/api/v1/overview").await;
    response.assert_status_ok();
    let fresh: Value = response.json();
    assert_eq!(fresh["liquidity_quality"], "fresh");
    assert_eq!(fresh["liquidity_generation"], 2);
    assert_eq!(bd(fresh["total_liquidity_usd"].as_str().unwrap()), bd("80"));
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(fresh["liquidity_as_of"].as_str().unwrap())
            .unwrap()
            .timestamp_micros(),
        next.quality.as_of.unwrap().timestamp_micros()
    );
    assert_eq!(fresh["liquidity_coverage_identity"], "coverage-v2");
    assert_eq!(
        fresh["liquidity_reasons"],
        serde_json::json!({"complete":3})
    );
    assert_eq!(fresh["priced_pair_count"], 3);
    assert_eq!(fresh["unpriced_pair_count"], 0);
    assert_eq!(
        bd(fresh["liquidity_change_24h_pct"].as_str().unwrap()),
        bd("-20")
    );
    assert_eq!(
        bd(fresh["total_liquidity_usd_30d_ago"].as_str().unwrap()),
        bd("50")
    );
    assert_eq!(fresh["total_volume_24h"], before["total_volume_24h"]);
    assert_eq!(fresh["total_fees_24h_usd"], before["total_fees_24h_usd"]);

    for (generation, status, total, unpriced) in [
        (3, "unavailable", None, 3),
        (4, "partial", Some(bd("40")), 1),
        (5, "fresh", Some(bd("60")), 0),
    ] {
        let mut publication = quality_rollup(status, total.clone(), Utc::now(), generation);
        publication.priced_pair_count = if status == "unavailable" { 0 } else { 2 };
        publication.unpriced_pair_count = unpriced;
        publication.quality.coverage_identity = format!("coverage-{generation}");
        publication.quality.reasons = serde_json::json!({"attempt":generation});
        publication.liquidity_change_24h_pct = Some(bd("25"));
        let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
            .await
            .unwrap();
        liquidity_snapshots::publish_liquidity_refresh(&mut tx, &publication, &[], Utc::now())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let response = server.get("/api/v1/overview").await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["liquidity_generation"], generation);
        assert_eq!(body["liquidity_quality"], status);
        assert_eq!(
            body["liquidity_coverage_identity"],
            format!("coverage-{generation}")
        );
        assert_eq!(
            body["liquidity_reasons"],
            serde_json::json!({"attempt":generation})
        );
        assert_eq!(body["unpriced_pair_count"], unpriced);
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(body["liquidity_as_of"].as_str().unwrap())
                .unwrap()
                .timestamp_micros(),
            publication.quality.as_of.unwrap().timestamp_micros()
        );
        if let Some(total) = total {
            assert_eq!(bd(body["total_liquidity_usd"].as_str().unwrap()), total);
        } else {
            assert!(body["total_liquidity_usd"].is_null());
        }
        assert_eq!(
            body["liquidity_change_24h_pct"].is_null(),
            status != "fresh"
        );
        assert_eq!(body["total_volume_24h"], before["total_volume_24h"]);
        assert_eq!(body["total_fees_24h_usd"], before["total_fees_24h_usd"]);
    }
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_overview_cached_liquidity_rejects_older_publication() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_overview_cache();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    let good = quality_rollup("fresh", Some(bd("70")), Utc::now(), 10);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], Utc::now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let current: Value = server.get("/api/v1/overview").await.json();
    let older = quality_rollup("fresh", Some(bd("999")), Utc::now(), 9);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    assert!(
        liquidity_snapshots::publish_liquidity_refresh(&mut tx, &older, &[], Utc::now())
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    // Emulate a delayed lower-generation row observation in this owned QA DB.
    // The protected publisher above cannot produce this regression itself.
    sqlx::query("UPDATE global_stats_24h SET liquidity_generation=9,total_liquidity_usd=999,liquidity_as_of=NOW() WHERE id=1")
        .execute(&pool).await.unwrap();
    let cached: Value = server.get("/api/v1/overview").await.json();
    assert_eq!(cached["liquidity_generation"], 10);
    assert_eq!(
        cached["total_liquidity_usd"],
        current["total_liquidity_usd"]
    );
    assert_eq!(cached["liquidity_as_of"], current["liquidity_as_of"]);
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_overview_cached_liquidity_missing_row_does_not_serve_last_good() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    reset_overview_cache();
    let good = quality_rollup("fresh", Some(bd("70")), Utc::now(), 1);
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[], Utc::now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    server.get("/api/v1/overview").await.assert_status_ok();
    sqlx::query("DELETE FROM global_stats_24h WHERE id=1")
        .execute(&pool)
        .await
        .unwrap();
    server
        .get("/api/v1/overview")
        .await
        .assert_status_internal_server_error();
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_failures_after_source_and_each_grain_rollback_entire_publication() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let a = insert_native(&pool, "uusd", "USTC", 6).await;
    let b = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1rollbackstages", a, b).await;
    let good = quality_rollup("fresh", Some(bd("100")), now, 1);
    let stamp = cl8y_dex_indexer::db::queries::pair_liquidity_usd::PairLiquidityStamp {
        pair_id: pair,
        liquidity_usd: bd("100"),
    };
    let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
        .await
        .unwrap();
    liquidity_snapshots::publish_liquidity_refresh(&mut tx, &good, &[stamp], now)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let tables = [
        "global_liquidity_snapshots",
        "protocol_hourly_liquidity",
        "protocol_daily_liquidity",
        "protocol_monthly_liquidity",
    ];
    for failed_stage in tables {
        let mut tx = liquidity_snapshots::begin_liquidity_refresh(&pool)
            .await
            .unwrap();
        sqlx::query(&format!(
            "ALTER TABLE {failed_stage} ADD CONSTRAINT issue1380_force_failure CHECK(generation<2)"
        ))
        .execute(&mut *tx)
        .await
        .unwrap();
        let degraded = quality_rollup("unavailable", None, now + Duration::seconds(1), 2);
        assert!(
            liquidity_snapshots::publish_liquidity_refresh(
                &mut tx,
                &degraded,
                &[],
                now + Duration::seconds(1)
            )
            .await
            .is_err(),
            "failure injection {failed_stage}"
        );
        tx.rollback().await.unwrap();
        let rollup = liquidity_snapshots::get_liquidity_rollup(&pool)
            .await
            .unwrap();
        assert_eq!(rollup.quality.generation, 1);
        assert_eq!(rollup.total_liquidity_usd, Some(bd("100")));
        let kept: (BigDecimal, i64) = sqlx::query_as(
            "SELECT liquidity_usd,generation FROM pair_liquidity_usd WHERE pair_id=$1",
        )
        .bind(pair)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(kept, (bd("100"), 1));
        for table in tables {
            let total_column = if table == "global_liquidity_snapshots" {
                "total_liquidity_usd"
            } else {
                "liquidity_usd"
            };
            let kept: (Option<BigDecimal>, i64) =
                sqlx::query_as(&format!("SELECT {total_column},generation FROM {table}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(
                kept,
                (Some(bd("100")), 1),
                "{failed_stage} rollback preserves {table}"
            );
        }
    }
    // Recreate the application with an empty response cache: committed provenance
    // survives cold startup, without regenerating stamps or liquidity observation age.
    reset_overview_cache();
    let first = TestServer::new(common::build_test_app(pool.clone()).await);
    let before: Value = first.get("/api/v1/overview").await.json();
    reset_overview_cache();
    let restarted = TestServer::new(common::build_test_app(pool.clone()).await);
    let after: Value = restarted.get("/api/v1/overview").await.json();
    assert_eq!(
        before["liquidity_generation"],
        after["liquidity_generation"]
    );
    assert_eq!(before["liquidity_as_of"], after["liquidity_as_of"]);
    assert_eq!(before["total_liquidity_usd"], after["total_liquidity_usd"]);
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_verified_empty_retains_inventory_observation_and_expires() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let observed = Utc::now() - Duration::seconds(30);
    sqlx::query("INSERT INTO indexer_state(key,value,updated_at) VALUES('factory_inventory_observed_at',$1,NOW())
        ON CONFLICT(key) DO UPDATE SET value=$1,updated_at=NOW()")
        .bind(observed.to_rfc3339()).execute(&pool).await.unwrap();
    let first = protocol_tvl::refresh_protocol_liquidity(&pool)
        .await
        .unwrap();
    assert_eq!(first.quality.status, "verified_empty");
    assert_eq!(first.total_liquidity_usd, Some(bd("0")));
    assert_eq!(first.quality.as_of, Some(observed));
    let second = protocol_tvl::refresh_protocol_liquidity(&pool)
        .await
        .unwrap();
    assert_eq!(
        second.quality.as_of, first.quality.as_of,
        "revaluation must not renew factory inventory evidence"
    );
    reset_overview_cache();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    let response: Value = server.get("/api/v1/overview").await.json();
    assert_eq!(response["liquidity_quality"], "verified_empty");
    assert_eq!(response["liquidity_max_age_seconds"], 300);
    assert_eq!(
        bd(response["total_liquidity_usd"].as_str().unwrap()),
        bd("0")
    );
    sqlx::query("UPDATE indexer_state SET value=$1 WHERE key='factory_inventory_observed_at'")
        .bind((Utc::now() - Duration::seconds(301)).to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    let expired = protocol_tvl::refresh_protocol_liquidity(&pool)
        .await
        .unwrap();
    assert_eq!(expired.quality.status, "unavailable");
    assert!(expired.total_liquidity_usd.is_none());
    sqlx::query("DELETE FROM indexer_state WHERE key='factory_inventory_observed_at'")
        .execute(&pool)
        .await
        .unwrap();
    reset_overview_cache();
}

#[serial]
#[tokio::test]
async fn q1380_populated_cold_inventory_requires_completed_enumeration() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let a = insert_native(&pool, "uusd", "USTC", 6).await;
    let b = insert_native(&pool, "uluna", "LUNC", 6).await;
    let pair = insert_pair(&pool, "terra1coldsubset", a, b).await;
    insert_reserves(&pool, pair, "100000000", "1000000000", now).await;
    insert_oracle(&pool, "ustc", "0.01").await;
    insert_oracle(&pool, "lunc", "0.0001").await;
    sqlx::query("DELETE FROM indexer_state WHERE key='factory_inventory_observed_at'")
        .execute(&pool)
        .await
        .unwrap();
    let partial_inventory = protocol_tvl::refresh_protocol_liquidity(&pool)
        .await
        .unwrap();
    assert_eq!(partial_inventory.quality.status, "unavailable");
    assert_eq!(partial_inventory.quality.reasons["unknown_inventory"], 1);
    assert_eq!(partial_inventory.priced_pair_count, 1);
    assert_eq!(partial_inventory.total_liquidity_usd, None);
    assert_eq!(partial_inventory.liquidity_change_24h_pct, None);
    cl8y_dex_indexer::db::queries::state::set_state(
        &pool,
        "factory_inventory_observed_at",
        &now.to_rfc3339(),
    )
    .await
    .unwrap();
    let complete = protocol_tvl::refresh_protocol_liquidity(&pool)
        .await
        .unwrap();
    assert_eq!(complete.quality.status, "fresh");
    assert_eq!(complete.total_liquidity_usd, Some(bd("1.1")));
    assert!(complete.quality.generation > partial_inventory.quality.generation);
}
