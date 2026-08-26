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
async fn daily_prune_drops_rows_older_than_retain_window() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let old = utc_day_start(Utc::now()) - Duration::days(100);
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
    let now_hour = daily_q::utc_hour_start(Utc::now());
    let hourly = daily_q::get_hourly_rows(&pool, now_hour - Duration::hours(2), now_hour)
        .await
        .expect("hourly");
    assert!(hourly.iter().any(|r| r.trade_count >= 1));
    let this_month = daily_q::utc_month_start(Utc::now());
    let monthly = daily_q::get_monthly_rows(&pool, this_month, this_month)
        .await
        .expect("monthly");
    assert_eq!(monthly[0].trade_count, 1);
}

#[serial]
#[tokio::test]
async fn grain_limit_allowlist_and_series_shape() {
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
        "tx-grain-today",
        Some("40"),
    )
    .await;
    daily_q::refresh_protocol_volume_rollups(&pool)
        .await
        .expect("rollups");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let ok = server
        .get("/api/v1/protocol/volume/daily?grain=daily&limit=14")
        .await;
    ok.assert_status_ok();
    let body: Value = ok.json();
    assert_eq!(body["grain"], "daily");
    assert_eq!(body["limit"], 14);
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["methodology"], "protocol_catalog");
    assert!(body.get("days").is_none());
    let series = body["series"].as_array().expect("series");
    assert_eq!(series.len(), 14);
    assert_eq!(series.last().unwrap()["utc_day"], today.date_naive().to_string());
    assert_eq!(bd(series.last().unwrap()["volume_usd"].as_str().unwrap()), bd("40"));

    let hourly = server
        .get("/api/v1/protocol/volume/daily?grain=hourly&limit=24")
        .await;
    hourly.assert_status_ok();
    let hbody: Value = hourly.json();
    assert_eq!(hbody["grain"], "hourly");
    assert_eq!(hbody["series"].as_array().unwrap().len(), 24);
    assert!(hbody["series"].as_array().unwrap()[0]["utc_hour"].as_str().unwrap().contains('T'));

    let monthly = server
        .get("/api/v1/protocol/volume/daily?grain=monthly&limit=6")
        .await;
    monthly.assert_status_ok();
    let mbody: Value = monthly.json();
    assert_eq!(mbody["grain"], "monthly");
    assert_eq!(mbody["series"].as_array().unwrap().len(), 6);
    assert_eq!(
        mbody["series"].as_array().unwrap().last().unwrap()["utc_month"],
        daily_q::format_utc_month(daily_q::utc_month_start(Utc::now()))
    );

    let alias = server.get("/api/v1/protocol/volume/daily?days=7").await;
    alias.assert_status_ok();
    let alias_body: Value = alias.json();
    assert_eq!(alias_body["days"], 7);
    assert_eq!(alias_body["series"].as_array().unwrap().len(), 7);

    for bad in [
        "/api/v1/protocol/volume/daily?grain=week",
        "/api/v1/protocol/volume/daily?grain=daily%3b",
        "/api/v1/protocol/volume/daily?grain[]=",
        "/api/v1/protocol/volume/daily?grain=daily&limit=-1",
        "/api/v1/protocol/volume/daily?grain=daily&limit=999999",
        "/api/v1/protocol/volume/daily?grain=daily&limit=",
        "/api/v1/protocol/volume/daily?grain=daily&limit=1e308",
        "/api/v1/protocol/volume/daily?grain=hourly&limit=169",
        "/api/v1/protocol/volume/daily?grain=monthly&limit=25",
        "/api/v1/protocol/volume/daily?grain=daily",
        "/api/v1/protocol/volume/daily?days=90",
        "/api/v1/protocol/volume/daily?grain=hourly&from=1970-01-01&to=2099-12-31",
        "/api/v1/protocol/volume/daily?grain=daily&limit=14&from=2020-01-01",
        "/api/v1/protocol/volume/daily?grain=daily%27%20OR%201%3D1&limit=7",
    ] {
        let resp = server.get(bad).await;
        assert_eq!(resp.status_code(), 400, "{bad}");
    }
}

#[serial]
#[tokio::test]
async fn grain_idle_unpriced_and_cache_ignores_extra_query() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;
    reset_protocol_volume_cache();

    let now_hour = daily_q::utc_hour_start(Utc::now());
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        now_hour - Duration::hours(1) + Duration::minutes(10),
        "tx-hour-unpriced",
        None,
    )
    .await;
    daily_q::refresh_protocol_hourly(&pool).await.expect("hourly");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let first = server
        .get("/api/v1/protocol/volume/daily?grain=hourly&limit=12")
        .await;
    first.assert_status_ok();
    let body: Value = first.json();
    let series = body["series"].as_array().unwrap();
    assert_eq!(series.len(), 12);
    let prev = &series[series.len() - 2];
    assert!(prev["volume_usd"].is_null(), "activity+unpriced must be null");
    assert!(prev["trade_count"].as_i64().unwrap() > 0);
    let idle = &series[0];
    assert_eq!(idle["volume_usd"], "0");
    assert_eq!(idle["trade_count"], 0);

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        now_hour + Duration::minutes(1),
        "tx-hour-after-cache",
        Some("77"),
    )
    .await;
    daily_q::refresh_protocol_hourly(&pool).await.expect("refresh");
    let second = server
        .get("/api/v1/protocol/volume/daily?grain=hourly&limit=12&foo=bust")
        .await;
    let body2: Value = second.json();
    assert_eq!(body, body2, "cache key ignores extra query junk");
}

#[serial]
#[tokio::test]
async fn grain_get_explain_does_not_scan_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    for sql in [
        "EXPLAIN (FORMAT TEXT) SELECT utc_hour, volume_usd, trade_count FROM protocol_hourly_volume WHERE utc_hour >= NOW() - interval '24 hours' AND utc_hour <= NOW() ORDER BY utc_hour ASC",
        "EXPLAIN (FORMAT TEXT) SELECT utc_month, volume_usd, trade_count FROM protocol_monthly_volume WHERE utc_month >= DATE '2024-01-01' AND utc_month <= CURRENT_DATE ORDER BY utc_month ASC",
    ] {
        let rows: Vec<(String,)> = sqlx::query_as(sql).fetch_all(&pool).await.expect("explain");
        let plan = rows.into_iter().map(|(line,)| line).collect::<Vec<_>>().join("\n");
        assert!(!plan.contains("swap_events"), "{plan}");
    }
}

#[serial]
#[tokio::test]
async fn hourly_bucket_is_hour_half_open_and_monthly_is_calendar() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_swaps(&pool).await;

    let hour = daily_q::utc_hour_start(Utc::now()) - Duration::hours(3);
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        hour,
        "tx-hour-start",
        Some("5"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        hour + Duration::hours(1),
        "tx-next-hour",
        Some("9"),
    )
    .await;
    daily_q::refresh_protocol_hourly(&pool).await.expect("hourly");
    let rows = daily_q::get_hourly_rows(&pool, hour, hour + Duration::hours(1))
        .await
        .expect("rows");
    let start_row = rows.iter().find(|r| r.utc_hour == hour).expect("start hour");
    assert_eq!(start_row.volume_usd.as_ref().map(|v| v.normalized()), Some(bd("5").normalized()));
    let next = rows
        .iter()
        .find(|r| r.utc_hour == hour + Duration::hours(1))
        .expect("next hour");
    assert_eq!(next.volume_usd.as_ref().map(|v| v.normalized()), Some(bd("9").normalized()));

    daily_q::refresh_protocol_monthly(&pool).await.expect("monthly");
    let this_month = daily_q::utc_month_start(Utc::now());
    let months = daily_q::get_monthly_rows(&pool, this_month, this_month)
        .await
        .expect("month");
    assert_eq!(months.len(), 1);
    assert_eq!(
        months[0].volume_usd.as_ref().map(|v| v.normalized()),
        Some(bd("14").normalized())
    );
}

#[serial]
#[tokio::test]
async fn hourly_prune_drops_rows_older_than_retain_window() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let old = daily_q::utc_hour_start(Utc::now()) - Duration::hours(300);
    sqlx::query(
        "INSERT INTO protocol_hourly_volume (utc_hour, volume_usd, trade_count, unpriced_trade_count)
         VALUES ($1, 1, 1, 0)",
    )
    .bind(old)
    .execute(&pool)
    .await
    .unwrap();
    daily_q::refresh_protocol_hourly(&pool).await.expect("refresh");
    let leftover: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protocol_hourly_volume WHERE utc_hour = $1",
    )
    .bind(old)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leftover, 0);
}
