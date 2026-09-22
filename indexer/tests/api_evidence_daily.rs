//! GitLab #1205 — `GET /api/v1/evidence/daily` redacted UTC-day export.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{NaiveDate, TimeZone, Utc};
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::protocol_fees::{FeeEventDraft, FeeSource};
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

const DAY: &str = "2026-06-10";
const SENDER: &str = "terra1evidencesenderaaaaaaaaaaaa";
const MAKER: &str = "terra1evidencemakeraaaaaaaaaaaaa";
const PROVIDER: &str = "terra1evidenceprovideraaaaaaaaaa";

fn ts(hour: u32) -> chrono::DateTime<Utc> {
    let d = NaiveDate::parse_from_str(DAY, "%Y-%m-%d").unwrap();
    Utc.from_utc_datetime(&d.and_hms_opt(hour, 0, 0).unwrap())
}

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn seed_evidence_day(pool: &sqlx::PgPool, pair_id: i32, asset_0: i32, asset_1: i32) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price,
          pool_return_amount, book_return_amount)
         VALUES ($1, 0, 100, $2, 'tx-swap-mid', $3, $4, $5, 1000000000000000000, 950000, 0.95, 500000, 450000)",
    )
    .bind(pair_id)
    .bind(ts(12))
    .bind(SENDER)
    .bind(asset_0)
    .bind(asset_1)
    .execute(pool)
    .await
    .expect("swap mid");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 0, 99, $2, 'tx-swap-prev', $3, $4, $5, 1000, 950, 0.95)",
    )
    .bind(pair_id)
    .bind(ts(23) - chrono::Duration::hours(1))
    .bind(SENDER)
    .bind(asset_0)
    .bind(asset_1)
    .execute(pool)
    .await
    .expect("swap prev day");

    sqlx::query(
        "INSERT INTO liquidity_events
         (pair_id, block_height, block_timestamp, tx_hash, provider, event_type,
          asset_0_amount, asset_1_amount, lp_amount)
         VALUES ($1, 101, $2, 'tx-lp-add', $3, 'add', 1000, 1000, 500),
                ($1, 102, $2, 'tx-lp-rem', $3, 'remove', 500, 500, 250)",
    )
    .bind(pair_id)
    .bind(ts(13))
    .bind(PROVIDER)
    .execute(pool)
    .await
    .expect("lp");

    sqlx::query(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner, side, price)
         VALUES ($1, 103, $2, 'tx-lim-place', 7, $3, 'bid', 1.0)",
    )
    .bind(pair_id)
    .bind(ts(14))
    .bind(SENDER)
    .execute(pool)
    .await
    .expect("place");

    sqlx::query(
        "INSERT INTO limit_order_cancellations
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner)
         VALUES ($1, 104, $2, 'tx-lim-cancel', 8, $3)",
    )
    .bind(pair_id)
    .bind(ts(15))
    .bind(SENDER)
    .execute(pool)
    .await
    .expect("cancel");

    let swap_id: i64 = sqlx::query_scalar(
        "SELECT id FROM swap_events WHERE tx_hash = 'tx-swap-mid' LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("swap id");

    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker,
          price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, 105, $3, 'tx-lim-fill', 9, 'ask', $4, 1.1, 50, 55, 1)",
    )
    .bind(pair_id)
    .bind(swap_id)
    .bind(ts(16))
    .bind(MAKER)
    .execute(pool)
    .await
    .expect("fill");

    for (source, tx, ord) in [
        (FeeSource::Wrap, "tx-wrap", 0i64),
        (FeeSource::Unwrap, "tx-unwrap", 0i64),
        (FeeSource::Ust1Mint, "tx-ust1", 0i64),
    ] {
        let draft = FeeEventDraft {
            block_height: 106,
            block_timestamp: ts(17),
            tx_hash: tx.to_string(),
            source,
            ordinal: ord,
            pair_id: None,
            asset_id: asset_1,
            amount_raw: bd("1000"),
            decimals: 6,
            fee_usd: Some(bd("0.01")),
        };
        fee_q::insert_fee_event(pool, &draft)
            .await
            .expect("fee");
    }
}

fn assert_no_actor_leak(body: &str) {
    for addr in [SENDER, MAKER, PROVIDER] {
        assert!(
            !body.contains(addr),
            "response must not contain actor bech32 {addr}"
        );
    }
    for key in ["sender", "receiver", "maker", "owner", "provider"] {
        assert!(
            !body.contains(&format!("\"{key}\"")),
            "forbidden key {key}"
        );
    }
}

#[serial]
#[tokio::test]
async fn evidence_daily_all_surfaces_and_redaction() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    seed_evidence_day(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id).await;
    let db_swap: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM swap_events WHERE block_timestamp >= $1 AND block_timestamp < $2",
    )
    .bind(ts(0))
    .bind(ts(0) + chrono::Duration::days(1))
    .fetch_one(&pool)
    .await
    .unwrap();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}"))
        .await;
    resp.assert_status_ok();
    let body = resp.text();
    assert_no_actor_leak(&body);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["day"], DAY);
    assert_eq!(v["timezone"], "UTC");
    let events = v["events"].as_array().unwrap();
    assert!(events.len() >= 7);
    let surfaces: std::collections::HashSet<_> = events
        .iter()
        .filter_map(|e| e["surface"].as_str())
        .collect();
    assert!(surfaces.contains("swap"));
    assert!(surfaces.contains("lp"));
    assert!(surfaces.contains("limit"));
    assert!(surfaces.contains("wrap"));
    assert!(!events.iter().any(|e| e["kind"].as_str() == Some("join")));
    let wrap_kinds: Vec<_> = events
        .iter()
        .filter(|e| e["surface"] == "wrap")
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(wrap_kinds.contains(&"wrap"));
    assert!(wrap_kinds.contains(&"unwrap"));
    assert!(!wrap_kinds.contains(&"ust1_mint"));
    let swap_count = events.iter().filter(|e| e["surface"] == "swap").count();
    assert_eq!(swap_count as i64, db_swap);
    let hashes: Vec<_> = events
        .iter()
        .filter_map(|e| e.get("actor_hash").and_then(|h| h.as_str()))
        .collect();
    for h in &hashes {
        assert_eq!(h.len(), 32);
    }
    let wrap_rows = events.iter().filter(|e| e["surface"] == "wrap");
    for w in wrap_rows {
        assert!(w.get("actor_hash").is_none());
    }
}

#[serial]
#[tokio::test]
async fn evidence_daily_surface_filters() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    seed_evidence_day(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    for e in v["events"].as_array().unwrap() {
        assert_eq!(e["surface"], "swap");
    }

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=limit"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    let kinds: std::collections::HashSet<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(kinds.contains("place"));
    assert!(kinds.contains("cancel"));
    assert!(kinds.contains("fill"));
    assert!(!v["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["surface"] == "swap"));
}

#[serial]
#[tokio::test]
async fn evidence_daily_bad_requests() {
    let pool = setup_pool().await;
    common::seed_db(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    server.get("/api/v1/evidence/daily").await.assert_status_bad_request();
    server
        .get("/api/v1/evidence/daily?day=2026-13-40")
        .await
        .assert_status_bad_request();
    server
        .get("/api/v1/evidence/daily?day=2099-01-01")
        .await
        .assert_status_bad_request();
    server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap'"))
        .await
        .assert_status_bad_request();
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&format=csv"
        ))
        .await
        .assert_status_bad_request();
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&cursor=not-valid"
        ))
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn evidence_daily_limit_clamp_and_pagination() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    for i in 0..5 {
        sqlx::query(
            "INSERT INTO swap_events
             (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
              offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1000, 950, 0.95)",
        )
        .bind(seed.pair_id)
        .bind(i)
        .bind(200 + i)
        .bind(ts(10))
        .bind(format!("tx-page-{i}"))
        .bind(SENDER)
        .bind(seed.asset_0_id)
        .bind(seed.asset_1_id)
        .execute(&pool)
        .await
        .expect("page swap");
    }
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap&limit=3"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    assert_eq!(v["events"].as_array().unwrap().len(), 3);
    assert_eq!(v["has_more"], true);
    let cursor = v["next_cursor"].as_str().unwrap();

    let resp2 = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=3&cursor={cursor}"
        ))
        .await;
    resp2.assert_status_ok();
    let v2: Value = resp2.json();
    assert!(v2["events"].as_array().unwrap().len() >= 2);

    let resp3 = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&limit=0"))
        .await;
    resp3.assert_status_ok();
}

#[serial]
#[tokio::test]
async fn evidence_daily_openapi_path_listed() {
    let pool = setup_pool().await;
    common::seed_db(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api-docs/openapi.json").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body["paths"]["/api/v1/evidence/daily"].is_object());
}

#[serial]
#[tokio::test]
async fn evidence_daily_post_method_not_allowed() {
    let pool = setup_pool().await;
    common::seed_db(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .post(&format!("/api/v1/evidence/daily?day={DAY}"))
        .await;
    assert!(
        resp.status_code() == 405 || resp.status_code() == 404,
        "POST must not succeed (got {})",
        resp.status_code()
    );
}
