//! GitLab #586 / #613 — protocol treasury fee ingest, rollup, overview scalars, GET /protocol/fees.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::reset_overview_cache;
use cl8y_dex_indexer::api::reset_protocol_fees_cache;
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::protocol_fees::{
    overview_fee_usd_field, parse_ust1_window_fees, parse_wrap_fees, parse_wrap_mapper_address,
    FeeEventDraft, FeeSource,
};
use cl8y_dex_indexer::lcd::{Attribute, Event, TxLog, TxResponse};
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
    hours_ago: i64,
    tx: &str,
    ordinal: i64,
) {
    let draft = FeeEventDraft {
        block_height: 1,
        block_timestamp: Utc::now() - Duration::hours(hours_ago),
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
async fn pool_only_swap_amm_no_book() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        Some("5"),
        1,
        "tx-amm",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("5"));
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    let amm = sources.iter().find(|s| s.source == "swap_amm").unwrap();
    assert_eq!(amm.event_count, 1);
    assert!(sources
        .iter()
        .all(|s| s.source != "book_take" || s.event_count == 0));
    assert!(sources.iter().all(|s| s.source != "wrap"));
}

#[serial]
#[tokio::test]
async fn hybrid_counts_amm_and_book_once() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        Some("4"),
        1,
        "tx-hyb",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::BookTake,
        seed.asset_1_id,
        "200000",
        Some("1"),
        1,
        "tx-hyb",
        42,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("5"));
}

#[serial]
#[tokio::test]
async fn replay_unique_constraint() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("1"),
        1,
        "tx-dup",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("1"),
        1,
        "tx-dup",
        0,
    )
    .await;
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM protocol_fee_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[serial]
#[tokio::test]
async fn windows_decay_and_change_pct() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("150"),
        1,
        "tx-now",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("100"),
        25,
        "tx-prior",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("9"),
        61,
        "tx-old",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("150"));
    assert_eq!(rollup.fees_change_24h_pct.unwrap(), bd("50"));
    assert_eq!(rollup.fee_event_count_24h, 1);
}

#[serial]
#[tokio::test]
async fn prior_zero_change_is_null() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("10"),
        1,
        "tx-only",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert!(rollup.fees_change_24h_pct.is_none());
}

#[serial]
#[tokio::test]
async fn unpriced_activity_headline_null() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000",
        None,
        1,
        "tx-gem",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert!(rollup.total_fees_24h_usd.is_none());
    assert_eq!(
        overview_fee_usd_field(1, rollup.total_fees_24h_usd.as_ref()),
        None
    );
}

#[serial]
#[tokio::test]
async fn token_cap_other_remainder() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    for i in 0..20 {
        let asset: i32 = sqlx::query_scalar(
            "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
             VALUES ($1, false, $2, $3, 6) RETURNING id",
        )
        .bind(format!("ugem{i}"))
        .bind(format!("Gem {i}"))
        .bind(format!("GEM{i}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        insert_fee(
            &pool,
            FeeSource::SwapAmm,
            asset,
            "1000000",
            Some(&(20 - i).to_string()),
            1,
            &format!("tx-cap-{i}"),
            0,
        )
        .await;
    }
    let _ = seed;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let tokens = fee_q::get_fees_by_token(&pool, "24h").await.unwrap();
    assert!(tokens.len() <= 9);
    assert!(tokens.iter().any(|t| t.is_other));
    let other = tokens.iter().find(|t| t.is_other).unwrap();
    assert!(other.amount_usd.is_some());
}

#[serial]
#[tokio::test]
async fn overview_and_fees_get_are_o1() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    reset_overview_cache();
    reset_protocol_fees_cache();

    let ov_plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT total_fees_24h_usd, total_fees_7d_usd, total_fees_30d_usd,
                fees_change_24h_pct, fee_event_count_24h
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let ov = ov_plan
        .into_iter()
        .map(|(l,)| l)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!ov.contains("swap_events"), "{ov}");
    assert!(!ov.contains("protocol_fee_events"), "{ov}");
    assert!(!ov.contains("limit_order_fills"), "{ov}");

    let fee_plan: Vec<(String,)> = sqlx::query_as(
        r#"EXPLAIN (FORMAT TEXT)
           SELECT source, amount_usd FROM protocol_fee_stats_by_source WHERE "window" = '24h'"#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let fp = fee_plan
        .into_iter()
        .map(|(l,)| l)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!fp.contains("protocol_fee_events"), "{fp}");
    assert!(!fp.contains("swap_events"), "{fp}");
}

#[serial]
#[tokio::test]
async fn fees_get_window_allowlist() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    reset_protocol_fees_cache();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    let ok = server.get("/api/v1/protocol/fees?window=24h").await;
    ok.assert_status_ok();
    let body: Value = ok.json();
    assert_eq!(body["window"], "24h");
    assert!(body["by_source"].is_array());
    assert!(body["by_token"].is_array());
    assert!(body["ust1_window_configured"].is_boolean());
    assert_eq!(body["ust1_window_configured"], false);

    for bad in [
        "window=1';drop",
        "window=javascript:",
        "window=../",
        "window=1h",
    ] {
        let resp = server.get(&format!("/api/v1/protocol/fees?{bad}")).await;
        assert_eq!(resp.status_code(), 400, "{bad}");
    }
}

#[serial]
#[tokio::test]
async fn overview_additive_fee_keys() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    reset_overview_cache();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body.get("total_volume_24h_usd").is_some());
    assert!(body.get("total_liquidity_usd").is_some());
    assert!(body.get("total_fees_24h_usd").is_some());
    assert!(body.get("total_fees_7d_usd").is_some());
    assert!(body.get("total_fees_30d_usd").is_some());
    assert!(body.get("fees_change_24h_pct").is_some());
}

#[test]
fn wrap_pin_and_spoof() {
    let pin = parse_wrap_mapper_address(
        "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2",
    )
    .unwrap();
    let tx = TxResponse {
        height: "1".into(),
        txhash: "T".into(),
        timestamp: None,
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: vec![
                    Attribute {
                        key: "_contract_address".into(),
                        value: "terra1attackerxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".into(),
                    },
                    Attribute {
                        key: "action".into(),
                        value: "wrap".into(),
                    },
                    Attribute {
                        key: "fee_amount".into(),
                        value: "9".into(),
                    },
                    Attribute {
                        key: "denom".into(),
                        value: "uusd".into(),
                    },
                ],
            }],
        }]),
        events: None,
    };
    assert!(parse_wrap_fees(&tx, &pin).is_empty());
}

fn attr(key: &str, value: &str) -> Attribute {
    Attribute {
        key: key.to_string(),
        value: value.to_string(),
    }
}

/// Captured ustr-cmm wrap-mapper fixture (GitLab #613): `notify_deposit` + `fee`.
fn captured_wrap_tx(pin: &str, denom: &str, fee: &str) -> TxResponse {
    TxResponse {
        height: "1".into(),
        txhash: "WRAP-CAPTURED".into(),
        timestamp: None,
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: vec![
                    attr("_contract_address", pin),
                    attr("action", "notify_deposit"),
                    attr("denom", denom),
                    attr("gross_amount", "1000000"),
                    attr("fee", fee),
                    attr("fee_wrap_bps", "200"),
                    attr("mint_amount", "980000"),
                ],
            }],
        }]),
        events: None,
    }
}

#[serial]
#[tokio::test]
async fn captured_wrap_ingest_rollup_and_get() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    let pin = parse_wrap_mapper_address(
        "terra1xuuuhpmyd5t29ry7mydg7ra2q2phrwhx7j28nx7x9sjw6zznkumsz0nmd2",
    )
    .unwrap();
    let fees = parse_wrap_fees(&captured_wrap_tx(&pin, "uluna", "20000"), &pin);
    assert_eq!(fees.len(), 1);
    assert_eq!(fees[0].source, FeeSource::Wrap);
    assert_eq!(fees[0].token, "uluna");
    insert_fee(
        &pool,
        fees[0].source,
        seed.asset_0_id,
        &fees[0].amount_raw.to_string(),
        Some("0.12"),
        1,
        "tx-wrap-custc",
        fees[0].ordinal,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, true,
    )
    .await;
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    let wrap = sources.iter().find(|s| s.source == "wrap").unwrap();
    assert_eq!(wrap.event_count, 1);
    assert!(wrap.amount_usd.as_ref().unwrap() > &bd("0"));

    reset_protocol_fees_cache();
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/protocol/fees?window=24h").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["wrap_mapper_configured"], true);
    let wrap_row = body["by_source"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["source"] == "wrap")
        .unwrap();
    assert!(wrap_row["event_count"].as_i64().unwrap() >= 1);
}

#[serial]
#[tokio::test]
async fn captured_unwrap_and_combo_with_swap_amm() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Unwrap,
        seed.asset_0_id,
        "5100",
        Some("0.03"),
        1,
        "tx-combo",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "3000",
        Some("0.05"),
        1,
        "tx-combo",
        1,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, true,
    )
    .await;
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    let unwrap = sources.iter().find(|s| s.source == "unwrap").unwrap();
    let amm = sources.iter().find(|s| s.source == "swap_amm").unwrap();
    assert_eq!(unwrap.event_count, 1);
    assert_eq!(amm.event_count, 1);
}

#[serial]
#[tokio::test]
async fn unconfigured_mapper_omits_wrap_family() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_0_id,
        "20000",
        Some("0.1"),
        1,
        "tx-wrap-omit",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1",
        Some("5"),
        1,
        "tx-amm-keep",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, false,
    )
    .await;
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    assert!(sources
        .iter()
        .all(|s| s.source != "wrap" && s.source != "unwrap"));
    assert!(sources
        .iter()
        .any(|s| s.source == "swap_amm" && s.event_count == 1));
}

#[serial]
#[tokio::test]
async fn wrap_window_decay_and_unpriced_null() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_0_id,
        "20000",
        Some("0.2"),
        25,
        "tx-wrap-old",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Unwrap,
        seed.asset_0_id,
        "5100",
        None,
        1,
        "tx-unwrap-unpriced",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_wrap(
        &pool, true, true,
    )
    .await;
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    let wrap = sources.iter().find(|s| s.source == "wrap").unwrap();
    let unwrap = sources.iter().find(|s| s.source == "unwrap").unwrap();
    assert_eq!(wrap.event_count, 0);
    assert_eq!(unwrap.event_count, 1);
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert!(rollup.total_fees_24h_usd.is_none());
}

#[serial]
#[tokio::test]
async fn ust1_window_fees_included_in_totals_when_configured() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        "10000",
        Some("2.5"),
        1,
        "tx-ust1-mint",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Ust1Redeem,
        seed.asset_1_id,
        "10000",
        Some("1.5"),
        1,
        "tx-ust1-redeem",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_pins(
        &pool, true, false, true,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert!(rollup.ust1_window_configured);
    assert_eq!(rollup.fee_event_count_24h, 2);
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("4"));
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    let mint = sources.iter().find(|s| s.source == "ust1_mint").unwrap();
    let redeem = sources.iter().find(|s| s.source == "ust1_redeem").unwrap();
    assert_eq!(mint.event_count, 1);
    assert_eq!(redeem.event_count, 1);
    assert_eq!(mint.amount_usd.as_ref().unwrap(), &bd("2.5"));
}

#[serial]
#[tokio::test]
async fn ust1_window_unconfigured_omits_sources_not_fake_zero() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        "1000000",
        Some("5"),
        1,
        "tx-amm-only",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        "10000",
        Some("2"),
        1,
        "tx-window-hidden",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_pins(
        &pool, true, false, false,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert!(!rollup.ust1_window_configured);
    // Totals still include stored events (census); Source table omits window keys.
    assert_eq!(rollup.fee_event_count_24h, 2);
    let sources = fee_q::get_fees_by_source(&pool, "24h").await.unwrap();
    assert!(sources
        .iter()
        .all(|s| s.source != "ust1_mint" && s.source != "ust1_redeem"));
    assert!(sources
        .iter()
        .any(|s| s.source == "swap_amm" && s.event_count == 1));
}

#[serial]
#[tokio::test]
async fn ust1_window_unpriced_activity_is_null_not_zero() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        "10000",
        None,
        1,
        "tx-unpriced-window",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_pins(
        &pool, true, false, true,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert!(rollup.total_fees_24h_usd.is_none());
}

#[serial]
#[tokio::test]
async fn ust1_window_replay_does_not_double_count() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        "10000",
        Some("3"),
        1,
        "tx-replay-window",
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        "10000",
        Some("3"),
        1,
        "tx-replay-window",
        0,
    )
    .await;
    cl8y_dex_indexer::indexer::volume_aggregator::refresh_all_volume_windows_with_pins(
        &pool, true, false, true,
    )
    .await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("3"));
}

#[test]
fn ust1_window_parse_exported_for_fixtures() {
    let pin = "terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2";
    let ust1 = "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72";
    let tx = TxResponse {
        height: "1".into(),
        txhash: "W".into(),
        timestamp: None,
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: vec![
                    Attribute {
                        key: "_contract_address".into(),
                        value: pin.into(),
                    },
                    Attribute {
                        key: "action".into(),
                        value: "deposit".into(),
                    },
                    Attribute {
                        key: "fee_amount".into(),
                        value: "10000".into(),
                    },
                    Attribute {
                        key: "fee_asset".into(),
                        value: ust1.into(),
                    },
                ],
            }],
        }]),
        events: None,
    };
    let fees = parse_ust1_window_fees(&tx, pin);
    assert_eq!(fees.len(), 1);
    assert_eq!(fees[0].source, FeeSource::Ust1Mint);
}
