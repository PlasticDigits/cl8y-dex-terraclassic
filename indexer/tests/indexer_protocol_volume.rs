//! GitLab #652 — volume flow Δ% + GET /api/v1/protocol/volume/daily.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::{reset_overview_cache, reset_protocol_volume_cache};
use cl8y_dex_indexer::db::queries::protocol_volume as daily_q;
use cl8y_dex_indexer::db::queries::volume;
use cl8y_dex_indexer::indexer::defillama::utc_day_start;
use cl8y_dex_indexer::indexer::protocol_fees::flow_change_pct;
use cl8y_dex_indexer::indexer::volume_aggregator;
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn wipe_swaps(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM swap_events")
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_swap(
    pool: &sqlx::PgPool,
    pair_id: i32,
    offer_id: i32,
    ask_id: i32,
    ts: chrono::DateTime<Utc>,
    tx: &str,
    volume_usd: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 6520, $2, $3, 'terra1t', $4, $5, 1000, 950, 0.95, $6)",
    )
    .bind(pair_id)
    .bind(ts)
    .bind(tx)
    .bind(offer_id)
    .bind(ask_id)
    .bind(volume_usd.map(bd))
    .execute(pool)
    .await
    .expect("insert swap");
}

#[serial]
#[tokio::test]
async fn volume_change_matches_flow_change_pct() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(1),
        "tx-cur",
        Some("150"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(25),
        "tx-prior",
        Some("100"),
    )
    .await;

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    let expected = flow_change_pct(Some(&bd("150")), Some(&bd("100")));
    assert_eq!(stats.volume_change_24h_pct, expected);
    assert_eq!(stats.volume_change_24h_pct.as_ref().map(|p| p.normalized()), Some(bd("50").normalized()));
}

#[serial]
#[tokio::test]
async fn volume_change_prior_empty_is_null() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(1),
        "tx-only-cur",
        Some("80"),
    )
    .await;

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.volume_change_24h_pct.is_none());
    assert!(stats.volume_change_7d_pct.is_none());
    assert!(stats.volume_change_30d_pct.is_none());
}

#[serial]
#[tokio::test]
async fn volume_change_idle_current_vs_prior_is_minus_100() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(30),
        "tx-prior-only",
        Some("40"),
    )
    .await;

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_trades_24h, 0);
    assert_eq!(
        stats.volume_change_24h_pct.as_ref().map(|p| p.normalized()),
        Some(bd("-100").normalized())
    );
}

#[serial]
#[tokio::test]
async fn volume_change_unpriced_activity_is_null() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(1),
        "tx-unpriced",
        None,
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(25),
        "tx-prior-priced",
        Some("10"),
    )
    .await;

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.total_trades_24h > 0);
    assert!(stats.volume_change_24h_pct.is_none());
}

#[serial]
#[tokio::test]
async fn volume_change_7d_and_30d_windows() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::days(2),
        "tx-7d-cur",
        Some("200"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::days(10),
        "tx-7d-prior",
        Some("100"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::days(20),
        "tx-30d-cur",
        Some("300"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::days(40),
        "tx-30d-prior",
        Some("150"),
    )
    .await;

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(
        stats.volume_change_7d_pct.as_ref().map(|p| p.normalized()),
        Some(bd("100").normalized())
    );
    // 30d current = 200+100+300; prior = 150.
    let expected_30d = flow_change_pct(Some(&bd("600")), Some(&bd("150"))).unwrap();
    assert_eq!(
        stats.volume_change_30d_pct.as_ref().map(|p| p.normalized()),
        Some(expected_30d.normalized())
    );
}

#[serial]
#[tokio::test]
async fn volume_change_fresh_history_is_null() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_swaps(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.volume_change_24h_pct.is_none());
    assert!(stats.volume_change_7d_pct.is_none());
    assert!(stats.volume_change_30d_pct.is_none());
}

#[serial]
#[tokio::test]
async fn volume_change_decay_updates_on_refresh_not_get() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(1),
        "tx-now",
        Some("100"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(25),
        "tx-old",
        Some("50"),
    )
    .await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let before = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(
        before.volume_change_24h_pct.as_ref().map(|p| p.normalized()),
        Some(bd("100").normalized())
    );

    sqlx::query("UPDATE swap_events SET block_timestamp = $1 WHERE tx_hash = 'tx-now'")
        .bind(Utc::now() - Duration::hours(26))
        .execute(&pool)
        .await
        .unwrap();

    let cached = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(cached.volume_change_24h_pct, before.volume_change_24h_pct);

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let after = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(after.total_trades_24h, 0);
    assert!(after.volume_change_24h_pct.is_none() || after.volume_change_24h_pct != before.volume_change_24h_pct);
}

#[serial]
#[tokio::test]
async fn overview_exposes_volume_change_keys() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    reset_overview_cache();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body.get("volume_change_24h_pct").is_some());
    assert!(body.get("volume_change_7d_pct").is_some());
    assert!(body.get("volume_change_30d_pct").is_some());
}

#[serial]
#[tokio::test]
async fn daily_days_allowlist_and_series_shape() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;
    reset_protocol_volume_cache();

    let today = utc_day_start(Utc::now());
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        today + Duration::hours(12),
        "tx-today",
        Some("25"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        today - Duration::seconds(1),
        "tx-yest-2359",
        Some("10"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        today,
        "tx-today-0000",
        Some("5"),
    )
    .await;

    daily_q::refresh_protocol_daily(&pool).await.expect("refresh daily");
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let ok7 = server.get("/api/v1/protocol/volume/daily?days=7").await;
    ok7.assert_status_ok();
    let body7: Value = ok7.json();
    assert_eq!(body7["days"], 7);
    assert_eq!(body7["timezone"], "UTC");
    assert_eq!(body7["methodology"], "protocol_catalog");
    let series7 = body7["series"].as_array().expect("series");
    assert_eq!(series7.len(), 7);
    assert_eq!(series7.last().unwrap()["utc_day"], today.date_naive().to_string());
    let today_usd = bd(series7.last().unwrap()["volume_usd"].as_str().unwrap());
    assert_eq!(today_usd, bd("30"));
    let yest = series7[series7.len() - 2]["volume_usd"].as_str().unwrap();
    assert_eq!(bd(yest), bd("10"));

    let ok30 = server.get("/api/v1/protocol/volume/daily?days=30").await;
    ok30.assert_status_ok();
    let body30: Value = ok30.json();
    assert_eq!(body30["series"].as_array().unwrap().len(), 30);

    for bad in [
        "/api/v1/protocol/volume/daily?days=1",
        "/api/v1/protocol/volume/daily?days=90",
        "/api/v1/protocol/volume/daily?days=",
        "/api/v1/protocol/volume/daily?days=7%3b",
        "/api/v1/protocol/volume/daily?days[]=",
        "/api/v1/protocol/volume/daily?days=999999",
        "/api/v1/protocol/volume/daily?days=-7",
        "/api/v1/protocol/volume/daily",
    ] {
        let resp = server.get(bad).await;
        assert_eq!(resp.status_code(), 400, "{bad}");
    }
}

#[serial]
#[tokio::test]
async fn daily_idle_zero_unpriced_null_and_cache() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;
    reset_protocol_volume_cache();

    let today = utc_day_start(Utc::now());
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        today - Duration::days(1) + Duration::hours(3),
        "tx-unpriced-day",
        None,
    )
    .await;

    daily_q::refresh_protocol_daily(&pool).await.expect("refresh");
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let first = server.get("/api/v1/protocol/volume/daily?days=7").await;
    first.assert_status_ok();
    let body: Value = first.json();
    let series = body["series"].as_array().unwrap();
    let yest = &series[series.len() - 2];
    assert!(yest["volume_usd"].is_null(), "activity+unpriced must be null, not $0");
    assert!(yest["trade_count"].as_i64().unwrap() > 0);
    let idle = &series[0];
    assert_eq!(idle["volume_usd"], "0");
    assert_eq!(idle["trade_count"], 0);

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        today + Duration::hours(1),
        "tx-after-cache",
        Some("99"),
    )
    .await;
    daily_q::refresh_protocol_daily(&pool).await.expect("refresh");
    let second = server.get("/api/v1/protocol/volume/daily?days=7").await;
    let body2: Value = second.json();
    assert_eq!(body, body2, "60s cache must ignore mid-TTL refresh");
}

#[serial]
#[tokio::test]
async fn daily_get_explain_does_not_scan_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT utc_day, volume_usd, trade_count, unpriced_trade_count, refreshed_at
         FROM protocol_daily_volume
         WHERE utc_day >= CURRENT_DATE - 29 AND utc_day <= CURRENT_DATE
         ORDER BY utc_day ASC",
    )
    .fetch_all(&pool)
    .await
    .expect("explain");
    let plan = rows.into_iter().map(|(line,)| line).collect::<Vec<_>>().join("\n");
    assert!(!plan.contains("swap_events"), "{plan}");
    assert!(plan.contains("protocol_daily_volume"), "{plan}");
}

#[serial]
#[tokio::test]
async fn daily_includes_all_pairs_not_llama_exclusions() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        utc_day_start(Utc::now()) + Duration::hours(2),
        "tx-any-pair",
        Some("12"),
    )
    .await;
    daily_q::refresh_protocol_daily(&pool).await.expect("refresh");
    let today = utc_day_start(Utc::now()).date_naive();
    let rows = daily_q::get_daily_rows(&pool, today, today).await.expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].volume_usd.as_ref().map(|v| v.normalized()), Some(bd("12").normalized()));
}

#[serial]
#[tokio::test]
async fn daily_prune_drops_rows_older_than_35d() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let old = utc_day_start(Utc::now()) - Duration::days(40);
    sqlx::query(
        "INSERT INTO protocol_daily_volume (utc_day, volume_usd, trade_count, unpriced_trade_count)
         VALUES ($1, 1, 1, 0)",
    )
    .bind(old.date_naive())
    .execute(&pool)
    .await
    .unwrap();
    daily_q::refresh_protocol_daily(&pool).await.expect("refresh");
    let leftover: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protocol_daily_volume WHERE utc_day = $1",
    )
    .bind(old.date_naive())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leftover, 0);
}

#[serial]
#[tokio::test]
async fn volume_aggregator_refreshes_daily_and_change() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        Utc::now() - Duration::hours(1),
        "tx-agg",
        Some("8"),
    )
    .await;
    volume_aggregator::refresh_all_volume_windows(&pool, true).await;
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert!(stats.volume_change_24h_pct.is_none());
    let today = utc_day_start(Utc::now()).date_naive();
    let rows = daily_q::get_daily_rows(&pool, today, today).await.expect("rows");
    assert_eq!(rows[0].trade_count, 1);
}
