//! GitLab #689 — GET /api/v1/protocol/fees/daily flow series.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::reset_protocol_fee_series_cache;
use cl8y_dex_indexer::db::queries::protocol_fee_series as fee_s;
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::db::queries::protocol_volume as vol_q;
use cl8y_dex_indexer::indexer::protocol_fees::{FeeEventDraft, FeeSource};
use cl8y_dex_indexer::indexer::volume_aggregator;
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn insert_fee(
    pool: &sqlx::PgPool,
    source: FeeSource,
    asset_id: i32,
    raw: &str,
    usd: Option<&str>,
    at: chrono::DateTime<Utc>,
    tx: &str,
    ordinal: i64,
) {
    let draft = FeeEventDraft {
        block_height: 1,
        block_timestamp: at,
        tx_hash: tx.to_string(),
        source,
        ordinal,
        asset_id,
        amount_raw: bd(raw),
        decimals: 6,
        fee_usd: usd.map(bd),
    };
    fee_q::insert_fee_event(pool, &draft)
        .await
        .expect("insert fee");
}

#[serial]
#[tokio::test]
async fn fees_grain_allowlist_idle_unpriced_mixed() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    reset_protocol_fee_series_cache();

    let hour = vol_q::utc_hour_start(Utc::now()) - Duration::hours(2);
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        None,
        hour + Duration::minutes(10),
        "tx-unpriced",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "2000000",
        Some("5"),
        hour + Duration::hours(1) + Duration::minutes(5),
        "tx-priced",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::BookTake,
        seed.asset_1_id,
        "500000",
        None,
        hour + Duration::hours(1) + Duration::minutes(20),
        "tx-mixed-unpriced",
        0,
    )
    .await;
    fee_s::refresh_protocol_hourly(&pool)
        .await
        .expect("hourly");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/api/v1/protocol/fees/daily?grain=hourly&limit=12")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["grain"], "hourly");
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["methodology"], "protocol_catalog");
    let series = body["series"].as_array().unwrap();
    assert_eq!(series.len(), 12);

    let unpriced_h = vol_q::format_utc_hour(hour);
    let mixed_h = vol_q::format_utc_hour(hour + Duration::hours(1));
    let unpriced = series.iter().find(|p| p["utc_hour"] == unpriced_h).unwrap();
    assert!(
        unpriced["fees_usd"].is_null(),
        "all-unpriced hour must be null: {unpriced}"
    );
    assert!(unpriced["event_count"].as_i64().unwrap() > 0);

    let mixed = series.iter().find(|p| p["utc_hour"] == mixed_h).unwrap();
    assert_eq!(
        bd(mixed["fees_usd"].as_str().unwrap()),
        bd("5"),
        "mixed priced+unpriced SUMs priced, does not null the bucket"
    );

    let idle = series.iter().find(|p| {
        p["utc_hour"] == vol_q::format_utc_hour(hour - Duration::hours(1))
    });
    if let Some(p) = idle {
        assert_eq!(p["fees_usd"], "0");
        assert_eq!(p["event_count"], 0);
    }

    for bad in [
        "/api/v1/protocol/fees/daily?grain=week",
        "/api/v1/protocol/fees/daily?grain=hourly;DROP",
        "/api/v1/protocol/fees/daily?grain=daily&limit=-1",
        "/api/v1/protocol/fees/daily?grain=daily&limit=999999",
        "/api/v1/protocol/fees/daily?grain=hourly&limit=169",
        "/api/v1/protocol/fees/daily?grain=monthly&limit=25",
        "/api/v1/protocol/fees/daily?grain=daily",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&from=2020-01-01",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&to=2099-12-31",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&window=24h",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&days=7",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&metric=javascript:alert(1)",
        "/api/v1/protocol/fees/daily?grain=daily&limit=14&ticker=ustc",
    ] {
        let r = server.get(bad).await;
        assert_eq!(r.status_code(), 400, "{bad}");
    }

    // Trailing breakdown must still accept window= and ignore grain.
    let trailing = server.get("/api/v1/protocol/fees?window=24h").await;
    trailing.assert_status_ok();
}

#[serial]
#[tokio::test]
async fn fees_get_explain_does_not_scan_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    for sql in [
        "EXPLAIN (FORMAT TEXT) SELECT utc_hour, fees_usd, event_count FROM protocol_hourly_fees WHERE utc_hour >= NOW() - interval '24 hours' AND utc_hour <= NOW() ORDER BY utc_hour ASC",
        "EXPLAIN (FORMAT TEXT) SELECT utc_day, fees_usd FROM protocol_daily_fees WHERE utc_day >= CURRENT_DATE - 13 AND utc_day <= CURRENT_DATE ORDER BY utc_day ASC",
        "EXPLAIN (FORMAT TEXT) SELECT utc_month, fees_usd FROM protocol_monthly_fees WHERE utc_month >= DATE '2024-01-01' AND utc_month <= CURRENT_DATE ORDER BY utc_month ASC",
    ] {
        let rows: Vec<(String,)> = sqlx::query_as(sql).fetch_all(&pool).await.expect("explain");
        let plan = rows.into_iter().map(|(line,)| line).collect::<Vec<_>>().join("\n");
        assert!(!plan.contains("protocol_fee_events"), "{plan}");
        assert!(!plan.contains("swap_events"), "{plan}");
        assert!(!plan.contains("defillama"), "{plan}");
    }
}

#[serial]
#[tokio::test]
async fn fees_cache_ignores_extra_query_junk() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    reset_protocol_fee_series_cache();

    let hour = vol_q::utc_hour_start(Utc::now()) - Duration::hours(1);
    insert_fee(
        &pool,
        FeeSource::LimitPlace,
        seed.asset_1_id,
        "1000000",
        Some("3"),
        hour + Duration::minutes(5),
        "tx-cache",
        0,
    )
    .await;
    fee_s::refresh_protocol_hourly(&pool)
        .await
        .expect("hourly");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let first = server
        .get("/api/v1/protocol/fees/daily?grain=hourly&limit=12")
        .await;
    first.assert_status_ok();
    let body: Value = first.json();

    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        Some("77"),
        vol_q::utc_hour_start(Utc::now()) + Duration::minutes(1),
        "tx-after-cache",
        0,
    )
    .await;
    fee_s::refresh_protocol_hourly(&pool)
        .await
        .expect("refresh");
    let second = server
        .get("/api/v1/protocol/fees/daily?grain=hourly&limit=12&foo=bust")
        .await;
    let body2: Value = second.json();
    assert_eq!(body, body2, "cache key ignores extra query junk");
}

#[serial]
#[tokio::test]
async fn fees_aggregator_refreshes_series_and_hourly_prune() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_1_id,
        "1000000",
        Some("2"),
        Utc::now() - Duration::minutes(30),
        "tx-wrap",
        0,
    )
    .await;
    volume_aggregator::refresh_all_volume_windows_with_wrap(&pool, true, true).await;

    let now_hour = vol_q::utc_hour_start(Utc::now());
    let rows = fee_s::get_hourly_rows(&pool, now_hour - Duration::hours(2), now_hour)
        .await
        .expect("hourly");
    assert!(rows.iter().any(|r| r.event_count >= 1));

    let old = now_hour - Duration::hours(300);
    sqlx::query(
        "INSERT INTO protocol_hourly_fees (utc_hour, fees_usd, event_count, unpriced_count)
         VALUES ($1, 1, 1, 0)",
    )
    .bind(old)
    .execute(&pool)
    .await
    .unwrap();
    fee_s::refresh_protocol_hourly(&pool)
        .await
        .expect("prune");
    let leftover: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM protocol_hourly_fees WHERE utc_hour = $1",
    )
    .bind(old)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leftover, 0);
}

#[serial]
#[tokio::test]
async fn fees_daily_and_monthly_shape() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    reset_protocol_fee_series_cache();
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        Some("9"),
        Utc::now() - Duration::hours(2),
        "tx-shape",
        0,
    )
    .await;
    fee_s::refresh_protocol_fee_series_rollups(&pool)
        .await
        .expect("rollups");

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let daily = server
        .get("/api/v1/protocol/fees/daily?grain=daily&limit=14")
        .await;
    daily.assert_status_ok();
    let dbody: Value = daily.json();
    assert_eq!(dbody["series"].as_array().unwrap().len(), 14);
    assert_eq!(
        bd(dbody["series"].as_array().unwrap().last().unwrap()["fees_usd"]
            .as_str()
            .unwrap()),
        bd("9")
    );

    let monthly = server
        .get("/api/v1/protocol/fees/daily?grain=monthly&limit=6")
        .await;
    monthly.assert_status_ok();
    let mbody: Value = monthly.json();
    assert_eq!(mbody["series"].as_array().unwrap().len(), 6);
    assert_eq!(
        bd(mbody["series"].as_array().unwrap().last().unwrap()["fees_usd"]
            .as_str()
            .unwrap()),
        bd("9")
    );
}
