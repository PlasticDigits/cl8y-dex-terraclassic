//! GitLab #705 — GET `/candles` returns newest `limit` bars, chronological JSON.

mod common;

use axum_test::TestServer;
use chrono::{DateTime, Duration, SecondsFormat, SubsecRound, Utc};
use serde_json::Value;
use serial_test::serial;
use sqlx::PgPool;

async fn insert_series(
    pool: &PgPool,
    pair_id: i32,
    interval: &str,
    newest: DateTime<Utc>,
    count: i32,
    step_mins: i32,
) {
    sqlx::query(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close,
                             volume_base, volume_quote, trade_count)
         SELECT $1, $2, $3 - make_interval(mins => (n * $5)::int),
                1, 1.1, 0.9, 1.05,
                CASE WHEN n = 0 THEN 0 ELSE 100 END,
                CASE WHEN n = 0 THEN 0 ELSE 100 END,
                CASE WHEN n = 0 THEN 0 ELSE 1 END
         FROM generate_series(0, $4 - 1) AS n",
    )
    .bind(pair_id)
    .bind(interval)
    .bind(newest)
    .bind(count)
    .bind(step_mins)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("insert {count} {interval} candles: {e}"));
}

fn parse_open_time(v: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(v.as_str().expect("open_time string"))
        .expect("open_time RFC3339")
        .with_timezone(&Utc)
}

fn assert_strictly_increasing(times: &[DateTime<Utc>]) {
    for w in times.windows(2) {
        assert!(
            w[0] < w[1],
            "open_time must be strictly increasing, got {} then {}",
            w[0],
            w[1]
        );
    }
}

#[serial]
#[tokio::test]
async fn default_15m_returns_newest_200_chronological() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();

    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "15m", newest, 400, 15).await;
    insert_series(&pool, seed.pair_id, "1m", newest, 250, 1).await;
    insert_series(&pool, seed.pair_id, "5m", newest, 250, 5).await;
    insert_series(&pool, seed.pair_id, "1h", newest, 250, 60).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let base = format!("/api/v1/pairs/{}/candles", seed.pair_address);

    let resp = server.get(&format!("{base}?interval=15m")).await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 200);
    let times: Vec<_> = body.iter().map(|r| parse_open_time(&r["open_time"])).collect();
    assert_strictly_increasing(&times);
    let oldest_seed = newest - Duration::minutes(15 * 399);
    assert!(
        times[0] > oldest_seed,
        "first bar must not be the oldest seeded 15m (got {})",
        times[0]
    );
    assert_eq!(times.last().copied(), Some(newest));
    assert_eq!(body.last().unwrap()["trade_count"], 0);

    for (iv, step, n) in [("1m", 1i32, 250i32), ("5m", 5, 250), ("1h", 60, 250)] {
        let resp = server.get(&format!("{base}?interval={iv}")).await;
        resp.assert_status_ok();
        let body: Vec<Value> = resp.json();
        assert_eq!(body.len(), 200, "{iv} default limit");
        let times: Vec<_> = body.iter().map(|r| parse_open_time(&r["open_time"])).collect();
        assert_strictly_increasing(&times);
        let oldest_seed = newest - Duration::minutes(i64::from(step) * i64::from(n - 1));
        assert!(times[0] > oldest_seed, "{iv} first bar is oldest-N");
        assert_eq!(times.last().copied(), Some(newest), "{iv} last must be newest");
    }
}

#[serial]
#[tokio::test]
async fn coarse_intervals_still_return_newest_not_oldest() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    // 4h: 90d window holds >200 bars. 1d/1w stay under the default 200 cap.
    insert_series(&pool, seed.pair_id, "4h", newest, 250, 240).await;
    insert_series(&pool, seed.pair_id, "1d", newest, 80, 1440).await;
    // 10 weekly bars ≈ 9 weeks, inside the 90-day default window.
    insert_series(&pool, seed.pair_id, "1w", newest, 10, 10080).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let base = format!("/api/v1/pairs/{}/candles", seed.pair_address);

    for (iv, expect_len) in [("4h", 200usize), ("1d", 80), ("1w", 10)] {
        let resp = server.get(&format!("{base}?interval={iv}")).await;
        resp.assert_status_ok();
        let body: Vec<Value> = resp.json();
        assert_eq!(body.len(), expect_len, "{iv} row count");
        let times: Vec<_> = body.iter().map(|r| parse_open_time(&r["open_time"])).collect();
        assert_strictly_increasing(&times);
        assert_eq!(times.last().copied(), Some(newest), "{iv} last must be newest");
    }
}

#[serial]
#[tokio::test]
async fn limit_10_is_newest_10_ascending() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "15m", newest, 40, 15).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m&limit=10",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 10);
    let times: Vec<_> = body.iter().map(|r| parse_open_time(&r["open_time"])).collect();
    assert_strictly_increasing(&times);
    assert_eq!(times[0], newest - Duration::minutes(15 * 9));
    assert_eq!(times[9], newest);
}

#[serial]
#[tokio::test]
async fn limit_99999_clamps_to_1000_newest() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "15m", newest, 1100, 15).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m&limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1000);
    let last = parse_open_time(&body.last().unwrap()["open_time"]);
    assert_eq!(last, newest);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m&limit=1000",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1000);
}

#[serial]
#[tokio::test]
async fn explicit_from_to_newest_n_stays_inside_window() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "15m", newest, 400, 15).await;

    // Old slice: bars 300..=350 from newest (not the live tail).
    let slice_newest = newest - Duration::minutes(15 * 300);
    let slice_oldest = newest - Duration::minutes(15 * 350);
    let from = slice_oldest - Duration::minutes(1);
    let to = slice_newest + Duration::minutes(1);

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m&limit=10&from={}&to={}",
            seed.pair_address,
            from.to_rfc3339_opts(SecondsFormat::Secs, true),
            to.to_rfc3339_opts(SecondsFormat::Secs, true)
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 10);
    let times: Vec<_> = body.iter().map(|r| parse_open_time(&r["open_time"])).collect();
    assert_strictly_increasing(&times);
    assert_eq!(times.last().copied(), Some(slice_newest));
    assert!(
        times.iter().all(|t| *t >= from && *t <= to),
        "newest-N leaked outside from/to"
    );
    assert!(
        times.iter().all(|t| *t < newest),
        "old-slice GET must not return the live newest bar"
    );
}

#[serial]
#[tokio::test]
async fn inverted_or_unparseable_from_to_do_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let base = format!("/api/v1/pairs/{}/candles?interval=1h", seed.pair_address);

    let now = Utc::now();
    let later = now + Duration::days(1);
    let earlier = now - Duration::days(2);
    let inverted = server
        .get(&format!(
            "{base}&from={}&to={}",
            later.to_rfc3339_opts(SecondsFormat::Secs, true),
            earlier.to_rfc3339_opts(SecondsFormat::Secs, true)
        ))
        .await;
    inverted.assert_status_ok();
    let body: Vec<Value> = inverted.json();
    assert!(body.is_empty() || body.len() <= 200);

    let junk = server.get(&format!("{base}&from=not-a-date&to=%%")).await;
    junk.assert_status_ok();
}

#[serial]
#[tokio::test]
async fn omitted_interval_defaults_1h_newest_n() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "1h", newest, 250, 60).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/pairs/{}/candles", seed.pair_address))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 200);
    let last = parse_open_time(&body.last().unwrap()["open_time"]);
    assert_eq!(last, newest);
}

#[serial]
#[tokio::test]
async fn empty_pair_returns_empty_array() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.is_empty());
}

#[serial]
#[tokio::test]
async fn unknown_pair_404_and_hostile_interval_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    server
        .get("/api/v1/pairs/terra1notapair705/candles?interval=15m")
        .await
        .assert_status_not_found();

    for iv in [
        "3h",
        "javascript:alert(1)",
        "1m;drop",
        "1h%20OR%201=1",
        "../15m",
    ] {
        let resp = server
            .get(&format!(
                "/api/v1/pairs/{}/candles?interval={iv}",
                seed.pair_address
            ))
            .await;
        resp.assert_status_bad_request();
    }
}

#[serial]
#[tokio::test]
async fn cross_pair_candles_do_not_leak() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();

    let pair_b: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pair705other', $1, $2, 'terra1lp705b', 30)
         RETURNING id",
    )
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair b");

    let newest_a = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    let newest_b = newest_a - Duration::minutes(15);
    insert_series(&pool, seed.pair_id, "15m", newest_a, 5, 15).await;
    insert_series(&pool, pair_b, "15m", newest_b, 5, 15).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=15m",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 5);
    let last = parse_open_time(&body.last().unwrap()["open_time"]);
    assert_eq!(last, newest_a);
    assert_ne!(last, newest_b);
}

#[serial]
#[tokio::test]
async fn negative_and_zero_limit_clamp_to_one_newest() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query("DELETE FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .unwrap();
    let newest = (Utc::now() - Duration::minutes(2)).trunc_subsecs(0);
    insert_series(&pool, seed.pair_id, "15m", newest, 20, 15).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let base = format!("/api/v1/pairs/{}/candles?interval=15m", seed.pair_address);

    for limit in ["-1", "0"] {
        let resp = server.get(&format!("{base}&limit={limit}")).await;
        resp.assert_status_ok();
        let body: Vec<Value> = resp.json();
        assert_eq!(body.len(), 1, "limit={limit}");
        assert_eq!(parse_open_time(&body[0]["open_time"]), newest);
    }
}
