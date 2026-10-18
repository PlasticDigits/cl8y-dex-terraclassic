//! #1206: real HTTP and isolated PostgreSQL digest acceptance.
mod common;
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{NaiveDate, TimeZone, Utc};
use cl8y_dex_indexer::{
    db::queries::protocol_fees as fee_q,
    indexer::protocol_fees::{FeeEventDraft, FeeSource},
};
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::{net::SocketAddr, str::FromStr};
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
    .bind(ts(0) + chrono::Duration::minutes(30))
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
    .bind(ts(0) - chrono::Duration::minutes(30))
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

#[tokio::test]
#[serial]
async fn all_surfaces_calendar_privacy_sources_and_token_mix() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    seed_evidence_day(&pool, seed.pair_id, seed.asset_0_id, seed.asset_1_id).await;
    // Add every remaining current source, including the three post-issue invoice paths.
    for source in FeeSource::ALL {
        if matches!(
            source,
            FeeSource::Wrap | FeeSource::Unwrap | FeeSource::Ust1Mint
        ) {
            continue;
        }
        fee_q::insert_fee_event(
            &pool,
            &FeeEventDraft {
                block_height: 110,
                block_timestamp: ts(18),
                tx_hash: format!("fee-{}", source.as_str()),
                source,
                ordinal: 0,
                pair_id: None,
                asset_id: seed.asset_1_id,
                amount_raw: bd("1000"),
                decimals: 6,
                fee_usd: if source == FeeSource::SwapAmm {
                    None
                } else {
                    Some(bd("0.02"))
                },
            },
        )
        .await
        .unwrap();
    }
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let response = server
        .get(&format!("/api/v1/evidence/digest?day={DAY}"))
        .await;
    response.assert_status_ok();
    let body = response.text();
    for forbidden in [
        SENDER,
        MAKER,
        PROVIDER,
        "tx-swap-mid",
        "actor_hash",
        "tx_hash",
        "swap_event_id",
        "unique_traders",
        "events\":",
    ] {
        assert!(!body.contains(forbidden), "{forbidden}");
    }
    let value: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(value["timezone"], "UTC");
    assert_eq!(value["complete"], true);
    assert_eq!(value["counts"]["swap"]["swap"], 1);
    assert_eq!(value["counts"]["lp"]["add"], 1);
    assert_eq!(value["counts"]["lp"]["remove"], 1);
    for kind in ["place", "cancel", "fill"] {
        assert_eq!(value["counts"]["limit"][kind], 1);
    }
    for kind in ["wrap", "unwrap"] {
        assert_eq!(value["counts"]["wrap"][kind], 1);
    }
    for kind in ["ust1_mint", "ust1_redeem"] {
        assert_eq!(value["counts"]["window"][kind], 1);
    }
    let rows = value["fees"]["by_source"].as_array().unwrap();
    assert_eq!(rows.len(), FeeSource::ALL.len());
    for (row, source) in rows.iter().zip(FeeSource::ALL) {
        assert_eq!(row["source"], source.as_str());
        assert_eq!(row["event_count"], 1);
    }
    assert!(rows[0]["amount_usd"].is_null());
    assert!(value["fees"]["total_usd"].as_str().is_some());
    assert!(value["fees"]["by_token"].as_array().unwrap().len() <= 9);
    let spec = server.get("/api-docs/openapi.json").await.json::<Value>();
    assert!(spec["paths"]["/api/v1/evidence/digest"].is_object());
    // Empty/past and current days, without the listing adapter's 404 gate.
    let empty = server.get("/api/v1/evidence/digest?day=2020-01-01").await;
    empty.assert_status_ok();
    let empty = empty.json::<Value>();
    assert_eq!(empty["fees"]["total_usd"], "0");
    assert!(empty["fees"]["by_source"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["event_count"] == 0 && r["amount_usd"] == "0" && r["share_pct"].is_null()));
    let today = server
        .get(&format!(
            "/api/v1/evidence/digest?day={}",
            Utc::now().date_naive()
        ))
        .await
        .json::<Value>();
    assert_eq!(today["complete"], false);
}

#[tokio::test]
#[serial]
async fn query_abuse_methods_and_sanitized_database_error() {
    let pool = setup_pool().await;
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app.clone());
    for query in [
        "",
        "day=today",
        "day=2026-02-30",
        "day=2099-01-01",
        "day=2026-06-10%27%20OR%201=1--",
        "day=2026-06-10;DROP%20TABLE%20swap_events",
    ] {
        server
            .get(&format!("/api/v1/evidence/digest?{query}"))
            .await
            .assert_status_bad_request();
    }
    for key in [
        "window",
        "timestamp",
        "from",
        "to",
        "sender",
        "trader",
        "maker",
        "address",
        "tx",
        "tx_hash",
        "include_txs",
        "events",
        "unredacted",
        "redact",
        "join",
        "format",
        "limit",
        "cursor",
        "surface",
    ] {
        server
            .get(&format!("/api/v1/evidence/digest?day={DAY}&{key}=csv"))
            .await
            .assert_status_bad_request();
    }
    server
        .get(&format!("/api/v1/evidence/digest?day={DAY}&day={DAY}"))
        .await
        .assert_status_bad_request();
    for method in ["POST", "PUT", "DELETE"] {
        let request = Request::builder()
            .method(method)
            .uri(format!("/api/v1/evidence/digest?day={DAY}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
    pool.close().await;
    let response = server
        .get(&format!("/api/v1/evidence/digest?day={DAY}"))
        .await;
    response.assert_status_internal_server_error();
    assert_eq!(response.text(), "Internal server error");
}

#[tokio::test]
#[serial]
async fn partial_pricing_and_bounded_other_bucket_preserve_headline() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    for n in 0..10 {
        let asset: i32 = sqlx::query_scalar("INSERT INTO assets (denom, is_cw20, name, symbol, decimals) VALUES ($1, false, 'synthetic', $2, 6) RETURNING id")
            .bind(format!("udigest{n}")).bind("terra1notanasset").fetch_one(&pool).await.unwrap();
        for (ordinal, fee_usd) in [(0, Some(bd("1"))), (1, None)] {
            fee_q::insert_fee_event(
                &pool,
                &FeeEventDraft {
                    block_height: 100,
                    block_timestamp: ts(12),
                    tx_hash: format!("partial-{n}"),
                    source: FeeSource::SwapAmm,
                    ordinal,
                    pair_id: Some(seed.pair_id),
                    asset_id: asset,
                    amount_raw: bd("1000"),
                    decimals: 6,
                    fee_usd,
                },
            )
            .await
            .unwrap();
        }
    }
    sqlx::query("INSERT INTO swap_events (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender, offer_asset_id, ask_asset_id, offer_amount, return_amount, price) SELECT $1, n, 100, $2, 'digest-large', $3, $4, $5, 1000, 1000, 1 FROM generate_series(0, 9999) n")
        .bind(seed.pair_id).bind(ts(12)).bind(SENDER).bind(seed.asset_0_id).bind(seed.asset_1_id).execute(&pool).await.unwrap();
    let server = TestServer::new(build_test_app(pool).await);
    let response = server
        .get(&format!("/api/v1/evidence/digest?day={DAY}"))
        .await;
    response.assert_status_ok();
    let body = response.text();
    assert!(!body.contains(SENDER));
    assert!(!body.contains("terra1notanasset"));
    let value: Value = serde_json::from_str(&body).unwrap();
    assert!(body.len() < 8_000);
    assert_eq!(value["counts"]["swap"]["swap"], 10_000);
    assert!(value.get("events").is_none());
    assert_eq!(
        value["fees"]["total_usd"]
            .as_str()
            .unwrap()
            .parse::<BigDecimal>()
            .unwrap(),
        bd("10")
    );
    let source = &value["fees"]["by_source"][0];
    assert_eq!(source["event_count"], 20);
    assert!(source["amount_usd"].is_null());
    assert!(source["share_pct"].is_null());
    let tokens = value["fees"]["by_token"].as_array().unwrap();
    assert_eq!(tokens.len(), 9);
    assert!(tokens
        .iter()
        .all(|r| r["amount_usd"].is_null() && r["share_pct"].is_null()));
    let other = tokens.last().unwrap();
    assert_eq!(other["is_other"], true);
    assert_eq!(other["event_count"], 4);
    assert!(other["contract_or_denom"].is_null());
}

#[tokio::test]
#[serial]
async fn digest_socket_peer_governor_returns_429_with_retry_after() {
    let pool = setup_pool().await;
    let mut config = common::test_config();
    config.rate_limit_rps = 1;
    config.rate_limit_lcd_heavy_rps = 0;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    // Exercise the real production peer-IP extractor: mocked HTTP lacks the
    // socket ConnectInfo required by the global governor.
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());
    let mut successful = 0;
    let mut limited = false;
    for _ in 0..12 {
        let response = server
            .get(&format!("/api/v1/evidence/digest?day={DAY}"))
            .await;
        match response.status_code() {
            StatusCode::OK => successful += 1,
            StatusCode::TOO_MANY_REQUESTS => {
                let retry = response
                    .headers()
                    .get(header::RETRY_AFTER)
                    .expect("digest governor must provide Retry-After")
                    .to_str()
                    .expect("Retry-After must be an HTTP integer");
                assert!(
                    retry.parse::<u64>().is_ok(),
                    "Retry-After must be delay-seconds"
                );
                limited = true;
                break;
            }
            status => panic!("unexpected digest status before rate limit: {status}"),
        }
    }
    assert!(
        successful > 0,
        "burst must exercise the actual digest handler first"
    );
    assert!(
        limited,
        "digest route must remain behind the global socket-peer governor"
    );
}
