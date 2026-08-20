//! GitLab #569 — protocol pool TVL rollup, snapshots, and cheap `/overview` reads.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::reset_overview_cache;
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

#[serial]
#[tokio::test]
async fn t8_empty_db_total_zero_delta_null() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh empty");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_liquidity_usd, bd("0"));
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
    assert_eq!(rollup.total_liquidity_usd, bd("1.1"));
    assert_eq!(rollup.priced_pair_count, 1);
    assert!(rollup.liquidity_change_24h_pct.is_none());
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
    assert_eq!(rollup.total_liquidity_usd, bd("4"));
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
    assert_eq!(rollup.total_liquidity_usd, bd("0"));
    assert_eq!(rollup.priced_pair_count, 0);
    assert!(rollup.unpriced_pair_count >= 1);
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
        "INSERT INTO hub_prices (ticker, asset_id, price_usd, updated_at)
         VALUES ('ust1', $1, 0.98, NOW())",
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
    assert_eq!(rollup.total_liquidity_usd, bd("10.0"));
}

#[serial]
#[tokio::test]
async fn t5_ustr_uses_hub_not_2_5x() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let now = Utc::now();
    let ust1 = insert_cw20(&pool, "terra1ust1569b", "UST1", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr569", "USTR", 18).await;
    let pair = insert_pair(&pool, "terra1ustrust1", ust1, ustr).await;
    insert_reserves(&pool, pair, "10000000", "100000000000000000000", now).await;
    sqlx::query(
        "INSERT INTO hub_prices (ticker, asset_id, price_usd, updated_at)
         VALUES ('ust1', $1, 0.98, NOW()), ('ustr', $2, 0.012, NOW())",
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
    assert_eq!(rollup.total_liquidity_usd, bd("11.0"));
}

#[serial]
#[tokio::test]
async fn t6_stale_and_zero_reserves_omitted() {
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
    assert_eq!(rollup.total_liquidity_usd, bd("0"));
    assert_eq!(rollup.priced_pair_count, 0);
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
    assert_eq!(rollup.total_liquidity_usd, bd("0"));
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

    let rollup = protocol_tvl::refresh_protocol_liquidity_with_staleness(
        &pool,
        std::time::Duration::from_secs(60),
    )
    .await
    .expect("tvl");
    // current 1.1; 24h then 1.0 → +10%; 30d then 2.2 → (1.1-2.2)/2.2 * 100
    assert_eq!(rollup.total_liquidity_usd, bd("1.1"));
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
    assert!(rollup.total_liquidity_usd > bd("0"));
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
    assert_eq!(stats.total_liquidity_usd, bd("1.1"));
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
    assert!(body["total_liquidity_usd"].is_string());
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
    assert_eq!(rollup.total_liquidity_usd, bd("0"));
}
