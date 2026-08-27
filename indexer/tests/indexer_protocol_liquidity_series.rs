//! GitLab #689 — GET /api/v1/protocol/liquidity/daily stock series.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::reset_protocol_liquidity_cache;
use cl8y_dex_indexer::db::queries::liquidity_snapshots;
use cl8y_dex_indexer::db::queries::protocol_liquidity as liq_q;
use cl8y_dex_indexer::db::queries::protocol_volume as vol_q;
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

/// `seed_db` → `refresh_global_stats` → TVL snapshot + grain rollups (often $0 now).
/// Tests that need missing-sample `null` must start from empty snapshots + grain tables.
async fn wipe_liquidity_history(pool: &sqlx::PgPool) {
    sqlx::query(
        "TRUNCATE TABLE protocol_hourly_liquidity, protocol_daily_liquidity,
                        protocol_monthly_liquidity, global_liquidity_snapshots",
    )
    .execute(pool)
    .await
    .expect("wipe liquidity history");
}

async fn snap(
    pool: &sqlx::PgPool,
    at: chrono::DateTime<Utc>,
    usd: &str,
    priced: i32,
) {
    liquidity_snapshots::insert_snapshot(pool, at, &bd(usd), priced)
        .await
        .expect("snapshot");
}

#[serial]
#[tokio::test]
async fn liquidity_grain_allowlist_and_stock_null_fill() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_liquidity_history(&pool).await;
    reset_protocol_liquidity_cache();

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let ok = server
        .get("/api/v1/protocol/liquidity/daily?grain=daily&limit=14")
        .await;
    ok.assert_status_ok();
    let body: Value = ok.json();
    assert_eq!(body["grain"], "daily");
    assert_eq!(body["limit"], 14);
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["methodology"], "protocol_catalog");
    let series = body["series"].as_array().expect("series");
    assert_eq!(series.len(), 14);
    assert!(series.iter().all(|p| p["liquidity_usd"].is_null()));
    assert!(body.get("days").is_none());

    for bad in [
        "/api/v1/protocol/liquidity/daily?grain=week",
        "/api/v1/protocol/liquidity/daily?grain=hourly;DROP",
        "/api/v1/protocol/liquidity/daily?grain=daily%3b",
        "/api/v1/protocol/liquidity/daily?grain[]=",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=-1",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=999999",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=",
        "/api/v1/protocol/liquidity/daily?grain=hourly&limit=169",
        "/api/v1/protocol/liquidity/daily?grain=monthly&limit=25",
        "/api/v1/protocol/liquidity/daily?grain=daily",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&from=2020-01-01",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&to=2099-12-31",
        "/api/v1/protocol/liquidity/daily?grain=hourly&from=1970-01-01&to=2099-12-31",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&window=24h",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&days=7",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&metric=javascript:alert(1)",
        "/api/v1/protocol/liquidity/daily?grain=daily&limit=14&ticker=ustc",
        "/api/v1/protocol/liquidity/daily?grain=../&limit=7",
    ] {
        let resp = server.get(bad).await;
        assert_eq!(resp.status_code(), 400, "{bad}");
    }
}

#[serial]
#[tokio::test]
async fn liquidity_last_snapshot_in_hour_not_sum() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_liquidity_history(&pool).await;
    reset_protocol_liquidity_cache();

    let hour = vol_q::utc_hour_start(Utc::now()) - Duration::hours(3);
    snap(&pool, hour + Duration::minutes(10), "10", 1).await;
    snap(&pool, hour + Duration::minutes(59), "40", 2).await;
    snap(&pool, hour + Duration::hours(1) + Duration::minutes(1), "70", 3).await;
    liq_q::refresh_protocol_hourly(&pool)
        .await
        .expect("hourly");

    let rows = liq_q::get_hourly_rows(&pool, hour, hour + Duration::hours(1))
        .await
        .expect("rows");
    let start = rows.iter().find(|r| r.utc_hour == hour).expect("10:00");
    assert_eq!(
        start.liquidity_usd.as_ref().map(|v| v.normalized()),
        Some(bd("40").normalized()),
        "last-in-bucket, not SUM(10+40)"
    );
    assert_eq!(start.priced_pair_count, 2);
    let next = rows
        .iter()
        .find(|r| r.utc_hour == hour + Duration::hours(1))
        .expect("11:00");
    assert_eq!(
        next.liquidity_usd.as_ref().map(|v| v.normalized()),
        Some(bd("70").normalized())
    );

    let empty_hour = hour + Duration::hours(2);
    let empty = rows
        .iter()
        .find(|r| r.utc_hour == empty_hour)
        .or_else(|| {
            // generate_series may not include empty_hour in get range; query GET fill
            None
        });
    if let Some(row) = empty {
        assert!(row.liquidity_usd.is_none(), "empty bucket is null not 0");
    }

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/api/v1/protocol/liquidity/daily?grain=hourly&limit=12")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let series = body["series"].as_array().unwrap();
    let want_hour = vol_q::format_utc_hour(hour);
    let point = series
        .iter()
        .find(|p| p["utc_hour"] == want_hour)
        .expect("hour point");
    assert_eq!(bd(point["liquidity_usd"].as_str().unwrap()), bd("40"));
}

#[serial]
#[tokio::test]
async fn liquidity_empty_bucket_is_null_not_zero() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_liquidity_history(&pool).await;
    reset_protocol_liquidity_cache();
    liq_q::refresh_protocol_hourly(&pool)
        .await
        .expect("hourly");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/api/v1/protocol/liquidity/daily?grain=hourly&limit=12")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let series = body["series"].as_array().unwrap();
    assert!(
        series.iter().all(|p| p["liquidity_usd"].is_null()),
        "missing samples must be JSON null, not \"0\": {body}"
    );
    assert!(series.iter().all(|p| p["liquidity_usd"] != "0"));
}

#[serial]
#[tokio::test]
async fn liquidity_get_explain_does_not_walk_snapshots() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    for sql in [
        "EXPLAIN (FORMAT TEXT) SELECT utc_hour, liquidity_usd, priced_pair_count FROM protocol_hourly_liquidity WHERE utc_hour >= NOW() - interval '24 hours' AND utc_hour <= NOW() ORDER BY utc_hour ASC",
        "EXPLAIN (FORMAT TEXT) SELECT utc_day, liquidity_usd FROM protocol_daily_liquidity WHERE utc_day >= CURRENT_DATE - 13 AND utc_day <= CURRENT_DATE ORDER BY utc_day ASC",
        "EXPLAIN (FORMAT TEXT) SELECT utc_month, liquidity_usd FROM protocol_monthly_liquidity WHERE utc_month >= DATE '2024-01-01' AND utc_month <= CURRENT_DATE ORDER BY utc_month ASC",
    ] {
        let rows: Vec<(String,)> = sqlx::query_as(sql).fetch_all(&pool).await.expect("explain");
        let plan = rows.into_iter().map(|(line,)| line).collect::<Vec<_>>().join("\n");
        assert!(!plan.contains("global_liquidity_snapshots"), "{plan}");
        assert!(!plan.contains("pair_reserves"), "{plan}");
        assert!(!plan.contains("defillama"), "{plan}");
    }
}

#[serial]
#[tokio::test]
async fn liquidity_cache_ignores_extra_query_junk() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_liquidity_history(&pool).await;
    reset_protocol_liquidity_cache();

    let hour = vol_q::utc_hour_start(Utc::now()) - Duration::hours(1);
    snap(&pool, hour + Duration::minutes(30), "15", 1).await;
    liq_q::refresh_protocol_hourly(&pool)
        .await
        .expect("hourly");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let first = server
        .get("/api/v1/protocol/liquidity/daily?grain=hourly&limit=12")
        .await;
    first.assert_status_ok();
    let body: Value = first.json();

    snap(&pool, vol_q::utc_hour_start(Utc::now()) + Duration::minutes(1), "99", 1).await;
    liq_q::refresh_protocol_hourly(&pool)
        .await
        .expect("refresh");
    let second = server
        .get("/api/v1/protocol/liquidity/daily?grain=hourly&limit=12&foo=bust")
        .await;
    let body2: Value = second.json();
    assert_eq!(body, body2, "cache key ignores extra query junk");
}

#[serial]
#[tokio::test]
async fn liquidity_monthly_survives_snapshot_prune() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_liquidity_history(&pool).await;

    let past = Utc::now() - Duration::days(10);
    snap(&pool, past, "88", 4).await;
    liq_q::refresh_protocol_monthly(&pool)
        .await
        .expect("monthly");
    let month = vol_q::utc_month_start(past);
    let before = liq_q::get_monthly_rows(&pool, month, month)
        .await
        .expect("month");
    assert_eq!(before.len(), 1);
    assert_eq!(
        before[0].liquidity_usd.as_ref().map(|v| v.normalized()),
        Some(bd("88").normalized())
    );

    sqlx::query("DELETE FROM global_liquidity_snapshots")
        .execute(&pool)
        .await
        .unwrap();
    liq_q::refresh_protocol_monthly(&pool)
        .await
        .expect("refresh after prune");
    let after = liq_q::get_monthly_rows(&pool, month, month)
        .await
        .expect("month after");
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].liquidity_usd.as_ref().map(|v| v.normalized()),
        Some(bd("88").normalized()),
        "monthly stock must survive snapshot wipe"
    );
}

#[serial]
#[tokio::test]
async fn liquidity_hourly_prune_drops_old_rows() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let old = vol_q::utc_hour_start(Utc::now()) - Duration::hours(300);
    sqlx::query(
        "INSERT INTO protocol_hourly_liquidity (utc_hour, liquidity_usd, priced_pair_count)
         VALUES ($1, 1, 1)",
    )
    .bind(old)
    .execute(&pool)
    .await
    .unwrap();
    liq_q::refresh_protocol_hourly(&pool)
        .await
        .expect("refresh");
    let leftover: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protocol_hourly_liquidity WHERE utc_hour = $1",
    )
    .bind(old)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leftover, 0);
}
