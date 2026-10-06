//! Forgejo #1397 / #1205 — public-chain-linked UTC-day evidence without actor identifiers.

mod common;

use std::net::SocketAddr;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum_test::TestServer;
use base64::Engine;
use bigdecimal::BigDecimal;
use chrono::{NaiveDate, TimeZone, Utc};
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::protocol_fees::{FeeEventDraft, FeeSource};
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use tower::ServiceExt;

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
    .bind(ts(0) - chrono::Duration::hours(1))
    .bind(SENDER)
    .bind(asset_0)
    .bind(asset_1)
    .execute(pool)
    .await
    .expect("swap prev day");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 0, 200, $2, 'tx-swap-next', $3, $4, $5, 1000, 950, 0.95)",
    )
    .bind(pair_id)
    .bind(ts(0) + chrono::Duration::days(1))
    .bind(SENDER)
    .bind(asset_0)
    .bind(asset_1)
    .execute(pool)
    .await
    .expect("swap next day");

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

    let swap_id: i64 =
        sqlx::query_scalar("SELECT id FROM swap_events WHERE tx_hash = 'tx-swap-mid' LIMIT 1")
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
        fee_q::insert_fee_event(pool, &draft).await.expect("fee");
    }
}

fn assert_no_actor_leak(body: &str) {
    for addr in [SENDER, MAKER, PROVIDER] {
        assert!(
            !body.contains(addr),
            "response must not contain actor bech32 {addr}"
        );
    }
    for key in [
        "actor",
        "actor_hash",
        "sender",
        "receiver",
        "maker",
        "owner",
        "provider",
    ] {
        assert!(!body.contains(&format!("\"{key}\"")), "forbidden key {key}");
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
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}"))
        .await;
    resp.assert_status_ok();
    let body = resp.text();
    assert_no_actor_leak(&body);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["schema_version"], 2);
    assert_eq!(
        v["privacy_policy"],
        "public-chain-linked-no-actor-identifiers"
    );
    assert_eq!(v["day"], DAY);
    assert_eq!(v["timezone"], "UTC");
    assert_eq!(v["complete"], true);
    assert!(!body.contains("tx-swap-prev"));
    assert!(!body.contains("tx-swap-next"));
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
    assert!(events.iter().all(|e| e.get("actor_hash").is_none()));
    let offer = events
        .iter()
        .find(|e| e["tx_hash"] == "tx-swap-mid")
        .unwrap()["offer_amount"]
        .as_str()
        .unwrap();
    assert_eq!(offer, "1000000000000000000");
    assert!(!offer.contains('e') && !offer.contains('E'));

    // Schema 1's unsalted digest was reversible by enumerating public actors.
    let legacy_digest = |actor: &str| {
        Sha256::digest(actor.as_bytes())[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let published_legacy_hash = legacy_digest(SENDER);
    let dictionary_match = [PROVIDER, MAKER, SENDER]
        .into_iter()
        .find(|candidate| legacy_digest(candidate) == published_legacy_hash);
    assert_eq!(dictionary_match, Some(SENDER));
    assert!(!body.contains(&published_legacy_hash));

    // Removing this shortcut does not anonymize retained public transaction IDs.
    // The indexed public transaction fixture still joins directly to its actor.
    let public_tx = events
        .iter()
        .find(|e| e["tx_hash"] == "tx-swap-mid")
        .unwrap()["tx_hash"]
        .as_str()
        .unwrap();
    let linked_actor: String =
        sqlx::query_scalar("SELECT sender FROM swap_events WHERE tx_hash = $1 LIMIT 1")
            .bind(public_tx)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(linked_actor, SENDER);

    assert_eq!(
        events
            .iter()
            .filter(|e| e["tx_hash"] == "tx-swap-mid" && e["surface"] == "swap")
            .count(),
        1
    );
    assert!(body.contains("terra1paircontractabc"));
    let keys: Vec<_> = events
        .iter()
        .map(|e| {
            (
                e["block_height"].as_i64().unwrap(),
                e["tx_hash"].as_str().unwrap().to_string(),
                e["surface"].as_str().unwrap().to_string(),
                e["kind"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
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

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=lp"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    let kinds: std::collections::HashSet<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["add", "remove"].into_iter().collect());
    assert!(v["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["surface"] == "lp"));

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=wrap"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    let kinds: Vec<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"wrap"));
    assert!(kinds.contains(&"unwrap"));
    assert!(!kinds.contains(&"ust1_mint"));
    assert!(v["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e.get("actor_hash").is_none()));

    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap,lp"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    let surfaces: std::collections::HashSet<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["surface"].as_str().unwrap())
        .collect();
    assert_eq!(surfaces, ["swap", "lp"].into_iter().collect());

    let resp = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&surface=wrap"
        ))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    let surfaces: std::collections::HashSet<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["surface"].as_str().unwrap())
        .collect();
    assert_eq!(surfaces, ["swap", "wrap"].into_iter().collect());

    let first = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap,lp&limit=1"
        ))
        .await;
    first.assert_status_ok();
    let first: Value = first.json();
    let cursor = first["next_cursor"].as_str().unwrap();
    let reference = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap,lp&cursor={cursor}"
        ))
        .await;
    reference.assert_status_ok();
    let reference: Value = reference.json();
    for canonical_equivalent in ["surface=lp,swap", "surface=LP&surface=swap&surface=lp"] {
        let replay = server
            .get(&format!(
                "/api/v1/evidence/daily?day={DAY}&{canonical_equivalent}&cursor={cursor}"
            ))
            .await;
        replay.assert_status_ok();
        assert_eq!(replay.json::<Value>()["events"], reference["events"]);
    }
}

#[serial]
#[tokio::test]
async fn evidence_daily_bad_requests() {
    let pool = setup_pool().await;
    common::seed_db(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    server
        .get("/api/v1/evidence/daily")
        .await
        .assert_status_bad_request();
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
        .get(&format!("/api/v1/evidence/daily?day={DAY}&format=csv"))
        .await
        .assert_status_bad_request();
    server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&format=xml"))
        .await
        .assert_status_bad_request();
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&cursor=not-valid"
        ))
        .await
        .assert_status_bad_request();
    server
        .get("/api/v1/evidence/daily?day=2026-09-03%27%20OR%201=1--")
        .await
        .assert_status_bad_request();
    server
        .get("/api/v1/evidence/daily?day=2026-09-03;DROP%20TABLE%20swap_events")
        .await
        .assert_status_bad_request();
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap%27%20UNION%20SELECT%20sender%20FROM%20swap_events"
        ))
        .await
        .assert_status_bad_request();

    let oversized_cursor = format!("e2.{}", "A".repeat(2046));
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&cursor={oversized_cursor}"
        ))
        .await
        .assert_status_bad_request();

    let resp = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&redact=0&unredacted=1&sender={SENDER}"
        ))
        .await;
    resp.assert_status_ok();
    assert_no_actor_leak(&resp.text());
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
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=3"
        ))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    assert_eq!(v["events"].as_array().unwrap().len(), 3);
    assert_eq!(v["has_more"], true);
    let cursor = v["next_cursor"].as_str().unwrap();

    let page1: std::collections::HashSet<_> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["tx_hash"].as_str().unwrap().to_string())
        .collect();

    let resp2 = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=3&cursor={cursor}"
        ))
        .await;
    resp2.assert_status_ok();
    let v2: Value = resp2.json();
    let page2 = v2["events"].as_array().unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(v2["has_more"], false);
    assert!(cursor.starts_with("e2."));
    // Stateless cursors permit repeat requests within their original scope.
    let replay = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=3&cursor={cursor}"
        ))
        .await;
    replay.assert_status_ok();
    assert_eq!(replay.json::<Value>()["events"], v2["events"]);
    for scope in [
        "day=2026-06-09&surface=swap",
        "day=2026-06-11&surface=swap",
        "day=2026-06-10&surface=lp",
        "day=2026-06-10&surface=swap,lp",
        "day=2026-06-10",
    ] {
        server
            .get(&format!("/api/v1/evidence/daily?{scope}&cursor={cursor}"))
            .await
            .assert_status_bad_request();
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor.strip_prefix("e2.").unwrap())
        .unwrap();
    let original: Value = serde_json::from_slice(&payload).unwrap();
    for (field, invalid) in [
        ("block_height", serde_json::json!(-1)),
        ("row_id", serde_json::json!(0)),
        ("ordinal", serde_json::json!(-1)),
        ("tx_hash", serde_json::json!("")),
        ("surface", serde_json::json!("invalid")),
        ("kind", serde_json::json!("invalid")),
    ] {
        let mut forged = original.clone();
        forged[field] = invalid;
        let forged = format!(
            "e2.{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(serde_json::to_vec(&forged).unwrap())
        );
        server
            .get(&format!(
                "/api/v1/evidence/daily?day={DAY}&surface=swap&cursor={forged}"
            ))
            .await
            .assert_status_bad_request();
    }
    let legacy_cursor = cursor.replacen("e2.", "e1.", 1);
    server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&cursor={legacy_cursor}"
        ))
        .await
        .assert_status_bad_request();
    for e in page2 {
        assert!(!page1.contains(e["tx_hash"].as_str().unwrap()));
    }

    let resp3 = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=0"
        ))
        .await;
    resp3.assert_status_ok();
    let v3: Value = resp3.json();
    assert_eq!(v3["events"].as_array().unwrap().len(), 1);

    let resp4 = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=10000"
        ))
        .await;
    resp4.assert_status_ok();
    let v4: Value = resp4.json();
    assert_eq!(v4["events"].as_array().unwrap().len(), 5);
    assert_eq!(v4["has_more"], false);
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
    let path = &body["paths"]["/api/v1/evidence/daily"];
    assert!(path.is_object());
    let tags = path["get"]["tags"].as_array().expect("tags");
    assert!(tags.iter().any(|t| t.as_str() == Some("Evidence")));
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
    assert_eq!(resp.status_code(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        server
            .put(&format!("/api/v1/evidence/daily?day={DAY}"))
            .await
            .status_code(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        server
            .delete(&format!("/api/v1/evidence/daily?day={DAY}"))
            .await
            .status_code(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[serial]
#[tokio::test]
async fn evidence_daily_empty_past_and_incomplete_today() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/evidence/daily?day=2020-01-01").await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    assert_eq!(v["events"].as_array().unwrap().len(), 0);
    assert_eq!(v["complete"], true);
    assert_eq!(v["has_more"], false);

    let today = Utc::now().date_naive();
    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={today}"))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    assert_eq!(v["complete"], false);
}

#[serial]
#[tokio::test]
async fn evidence_daily_same_tx_swap_index_tie_break() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    let pair2: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairtwocontractaa', $1, $2, 'terra1lptoken2', 30)
         RETURNING id",
    )
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair2");

    for pair in [seed.pair_id, pair2] {
        sqlx::query(
            "INSERT INTO swap_events
             (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
              offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
             VALUES ($1, 0, 300, $2, 'tx-tie', $3, $4, $5, 10, 9, 0.9)",
        )
        .bind(pair)
        .bind(ts(11))
        .bind(SENDER)
        .bind(seed.asset_0_id)
        .bind(seed.asset_1_id)
        .execute(&pool)
        .await
        .expect("tie swap");
    }

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=1"
        ))
        .await;
    resp.assert_status_ok();
    let v: Value = resp.json();
    assert_eq!(v["events"].as_array().unwrap().len(), 1);
    assert_eq!(v["has_more"], true);
    let first_pair = v["events"][0]["pair_address"].as_str().unwrap().to_string();
    let cursor = v["next_cursor"].as_str().unwrap();

    let resp2 = server
        .get(&format!(
            "/api/v1/evidence/daily?day={DAY}&surface=swap&limit=1&cursor={cursor}"
        ))
        .await;
    resp2.assert_status_ok();
    let v2: Value = resp2.json();
    assert_eq!(v2["events"].as_array().unwrap().len(), 1);
    let second_pair = v2["events"][0]["pair_address"].as_str().unwrap();
    assert_ne!(first_pair, second_pair);
    assert_eq!(v2["events"][0]["tx_hash"], "tx-tie");
}

#[serial]
#[tokio::test]
async fn evidence_daily_includes_gem_swap() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    let gem_asset: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1gemembercontractaaaa', true, 'Ember', 'EMBER', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("gem asset");
    let gem_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1gempaircontractaaa', $1, $2, 'terra1gemlp', 30)
         RETURNING id",
    )
    .bind(gem_asset)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("gem pair");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 0, 310, $2, 'tx-gem', $3, $4, $5, 10, 9, 0.9)",
    )
    .bind(gem_pair)
    .bind(ts(12))
    .bind(SENDER)
    .bind(gem_asset)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("gem swap");

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap"))
        .await;
    resp.assert_status_ok();
    let body = resp.text();
    assert!(body.contains("tx-gem"));
    assert!(body.contains("terra1gempaircontractaaa"));
    assert_no_actor_leak(&body);
}

#[serial]
#[tokio::test]
async fn evidence_daily_db_error_is_sanitized() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let closer = pool.clone();
    let app = build_test_app(pool).await;
    closer.close().await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}"))
        .await;
    assert_eq!(resp.status_code(), StatusCode::SERVICE_UNAVAILABLE);
    let body = resp.text();
    assert_eq!(body, "Evidence export unavailable");
    assert!(!body.to_ascii_lowercase().contains("sqlx"));
}

#[serial]
#[tokio::test]
async fn evidence_daily_global_rate_limit_returns_429() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 2;
    config.rate_limit_lcd_heavy_rps = 0;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    let url = format!("/api/v1/evidence/daily?day={DAY}");
    let mut saw_429 = false;
    for i in 0..40 {
        let resp = server
            .get(&url)
            .add_header("x-forwarded-for", format!("192.0.2.{}", i + 1))
            .add_header("x-real-ip", format!("198.51.100.{}", i + 1))
            .await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            assert!(resp.headers().get("retry-after").is_some());
            break;
        }
    }
    assert!(saw_429, "global governor should 429 the evidence route");
}

fn evidence_request(surface: &str) -> Request<Body> {
    Request::builder()
        .uri(format!(
            "/api/v1/evidence/daily?day={DAY}&surface={surface}"
        ))
        .body(Body::empty())
        .unwrap()
}

/// Wait for actual PostgreSQL lock contention rather than assuming that spawned
/// requests already acquired their export permits after an arbitrary sleep.
/// Match the prefix: pg_stat_activity truncates SQL before the FROM table name.
async fn wait_for_blocked_exports(pool: &sqlx::PgPool, count: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let blocked: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_activity
                 WHERE datname = current_database() AND state = 'active'
                   AND wait_event_type = 'Lock' AND query LIKE 'SELECT * FROM (%'",
            )
            .fetch_one(pool)
            .await
            .unwrap();
            if blocked >= count {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("exports should reach the deliberately locked evidence query");
}

#[serial]
#[tokio::test]
async fn evidence_daily_admission_timeout_isolated_from_ordinary_api_and_other_routers() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let app = build_test_app(pool.clone()).await;
    let independent_app = build_test_app(pool.clone()).await;
    let server = TestServer::builder().http_transport().build(
        app.clone()
            .into_make_service_with_connect_info::<SocketAddr>(),
    );
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let swap_url = server
        .server_url(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap"))
        .unwrap();

    let mut locked = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE swap_events IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *locked)
        .await
        .unwrap();
    let started = std::time::Instant::now();
    let first = tokio::spawn(
        client
            .get(swap_url.clone())
            .header("x-forwarded-for", "192.0.2.1")
            .send(),
    );
    let second = tokio::spawn(
        client
            .get(swap_url)
            .header("x-forwarded-for", "192.0.2.2")
            .send(),
    );
    wait_for_blocked_exports(&pool, 2).await;

    // Forwarded-header spoofing cannot buy another export slot on the same
    // socket server. Admission rejects promptly instead of queueing a query.
    for forwarded in ["192.0.2.3", "192.0.2.4"] {
        let overload = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            client
                .get(
                    server
                        .server_url(&format!("/api/v1/evidence/daily?day={DAY}&surface=wrap"))
                        .unwrap(),
                )
                .header("x-forwarded-for", forwarded)
                .header("x-real-ip", forwarded)
                .send(),
        )
        .await
        .expect("saturated export admission must fail promptly")
        .unwrap();
        assert_eq!(overload.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(overload.headers().get("retry-after").is_some());
    }

    // A real ordinary DB-backed API request retains access to pool connections.
    let pairs = server.get("/api/v1/pairs").await;
    pairs.assert_status_ok();
    let pairs: Value = pairs.json();
    assert!(!pairs["items"].as_array().unwrap().is_empty());
    // Independently built routers have their own export admission slots.
    let other_router = independent_app
        .oneshot(evidence_request("wrap"))
        .await
        .unwrap();
    assert_eq!(other_router.status(), StatusCode::OK);

    for task in [first, second] {
        let response = task.await.unwrap().unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = response.text().await.unwrap();
        assert!(!body.is_empty());
        for internal in ["sqlx", "postgres", "swap_events", "SELECT", "connection"] {
            assert!(!body.contains(internal), "timeout must sanitize {internal}");
        }
    }
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
    // Timeout releases both permits while the source table remains locked.
    let recovered = app.clone().oneshot(evidence_request("wrap")).await.unwrap();
    assert_eq!(recovered.status(), StatusCode::OK);
    locked.rollback().await.unwrap();
    assert_eq!(
        app.oneshot(evidence_request("swap"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[serial]
#[tokio::test]
async fn evidence_daily_cancelled_export_holds_slot_until_bounded_work_settles() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::builder().http_transport().build(
        app.clone()
            .into_make_service_with_connect_info::<SocketAddr>(),
    );
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let swap_url = server
        .server_url(&format!("/api/v1/evidence/daily?day={DAY}&surface=swap"))
        .unwrap();
    let mut locked = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE swap_events IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *locked)
        .await
        .unwrap();
    let cancelled = tokio::spawn(client.get(swap_url.clone()).send());
    let survivor = tokio::spawn(client.get(swap_url).send());
    wait_for_blocked_exports(&pool, 2).await;
    cancelled.abort();
    assert!(cancelled.await.unwrap_err().is_cancelled());

    // The cancelled HTTP caller must not free its slot while the shielded
    // database worker is still blocked. Otherwise repeated aborts bypass the
    // export limit by accumulating abandoned SQL queries.
    let busy = server
        .get(&format!("/api/v1/evidence/daily?day={DAY}&surface=wrap"))
        .await;
    assert_eq!(busy.status_code(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        survivor.await.unwrap().unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    tokio::time::timeout(std::time::Duration::from_secs(4), async {
        loop {
            let replacement = app.clone().oneshot(evidence_request("wrap")).await.unwrap();
            match replacement.status() {
                StatusCode::OK => break,
                StatusCode::TOO_MANY_REQUESTS => {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await
                }
                status => panic!("unexpected recovery status: {status}"),
            }
        }
    })
    .await
    .expect("cancelled export must release its permit after bounded work settles");
    locked.rollback().await.unwrap();
    assert_eq!(
        app.oneshot(evidence_request("swap"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[serial]
#[tokio::test]
async fn evidence_daily_large_day_caps_pages_without_gaps_or_duplicates() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         SELECT $1, i, 1000 + i, $2, 'tx-bulk-' || lpad(i::text, 4, '0'), $3,
                $4, $5, 1000000000000000000, 950000, 0.95
         FROM generate_series(1, 2500) AS i",
    )
    .bind(seed.pair_id)
    .bind(ts(12))
    .bind(SENDER)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .unwrap();
    let server = TestServer::new(build_test_app(pool).await);
    let mut cursor = None;
    let mut collected = Vec::new();
    for expected_size in [1000, 1000, 500] {
        let mut url = format!("/api/v1/evidence/daily?day={DAY}&surface=swap&limit=10000");
        if let Some(cursor) = &cursor {
            url.push_str(&format!("&cursor={cursor}"));
        }
        let response = server.get(&url).await;
        response.assert_status_ok();
        let body = response.text();
        assert_no_actor_leak(&body);
        let page: Value = response.json();
        let events = page["events"].as_array().unwrap();
        assert_eq!(events.len(), expected_size);
        for event in events {
            assert_eq!(event["offer_amount"], "1000000000000000000");
            collected.push(event["tx_hash"].as_str().unwrap().to_owned());
        }
        assert_eq!(page["has_more"], expected_size == 1000);
        cursor = page["next_cursor"].as_str().map(str::to_owned);
    }
    assert!(cursor.is_none());
    let expected = (1..=2500)
        .map(|i| format!("tx-bulk-{i:04}"))
        .collect::<Vec<_>>();
    assert_eq!(collected, expected);
}
