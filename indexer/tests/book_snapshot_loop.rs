//! GitLab #322: book_snapshot loop writes mirrored reserves + resting books.

mod common;

use bigdecimal::BigDecimal;
use cl8y_dex_indexer::db::queries::{pair_reserves, resting_orders};
use cl8y_dex_indexer::indexer::book_snapshot::{
    self, book_snapshot_lcd_budget, BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD,
    BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR, BOOK_SNAPSHOT_MAX_STALENESS_MS,
};
use cl8y_dex_indexer::lcd::LcdClient;
use serde_json::json;
use serial_test::serial;
use std::str::FromStr;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[tokio::test]
#[serial]
async fn snapshot_populates_reserves_and_resting_book() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let mock = common::lcd_mock::start_book_snapshot_mock(Some("12345")).await;
    // This fixture verifies storage and FIFO semantics. Strict production and
    // scaled deadlines are exercised by the failure/recovery fixtures below;
    // give this functional walk scheduling margin on a shared build host.
    let lcd = LcdClient::new_with_policy(
        vec![common::lcd_mock::lcd_base_url(&mock)],
        5000,
        30000,
        "columbus-5",
        cl8y_dex_indexer::lcd::ProviderPolicy {
            attempt_timeout_ms: 3000,
            ..Default::default()
        },
    );

    book_snapshot::snapshot_all_pairs_with_limits(
        &pool,
        &lcd,
        book_snapshot::SnapshotLimits {
            pair_timeout: std::time::Duration::from_secs(10),
            ..Default::default()
        },
    )
    .await
    .expect("snapshot cycle");

    let reserves = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("reserves row");
    assert_eq!(reserves.reserve_0, bd("1000000"));
    assert_eq!(reserves.reserve_1, bd("2000000"));
    assert_eq!(reserves.fee_bps, 25);
    assert_eq!(reserves.block_height, Some(12345));

    let bids = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "bid")
        .await
        .unwrap();
    assert_eq!(
        bids.iter().map(|o| o.order_id).collect::<Vec<_>>(),
        vec![102, 101, 100],
        "bids preserve on-chain DLL FIFO after a reprice, rather than sorting by order_id"
    );
    let bid_102 = bids.iter().find(|o| o.order_id == 102).expect("order 102");
    assert_eq!(bid_102.price, bd("1.05"));
    assert_eq!(bid_102.remaining, bd("100"));
    assert_eq!(bid_102.owner.as_deref(), Some("terra1bid2"));
    assert_eq!(bid_102.expires_at, Some(999));

    let asks = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "ask")
        .await
        .unwrap();
    assert_eq!(
        asks.iter().map(|o| o.order_id).collect::<Vec<_>>(),
        vec![201, 202],
        "asks: lowest price first"
    );
}

#[tokio::test]
#[serial]
async fn snapshot_skips_failed_pair_and_keeps_prior_snapshot() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("999"), &bd("888"), 30, Some(1))
        .await
        .unwrap();

    let good_pair = seed.pair_address.clone();
    let bad_pair = "terra1pairbroken".to_string();

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lpbroken', 30)",
    )
    .bind(&bad_pair)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .unwrap();

    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmos/base/tendermint/v1beta1/blocks/latest$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block": { "header": { "chain_id": "columbus-5", "height": "500", "time": "2026-06-05T00:00:00Z" } }
        })))
        .mount(&server)
        .await;

    let good = good_pair.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let path = req.url.path();
            if path.contains(&good) {
                let q = common::lcd_mock::smart_query_from_request(req);
                let data = if q.get("pool").is_some() {
                    json!({
                        "assets": [
                            { "info": { "native_token": { "denom": "uluna" } }, "amount": "111" },
                            { "info": { "token": { "contract_addr": "terra1ustctoken" } }, "amount": "222" }
                        ],
                        "total_share": "1"
                    })
                } else if q.get("get_fee_config").is_some() {
                    json!({ "fee_config": { "fee_bps": 25, "treasury": "terra1treasury" } })
                } else if q.get("order_book_head").is_some() {
                    json!(null)
                } else {
                    json!(null)
                };
                ResponseTemplate::new(200).insert_header("x-cosmos-block-height", common::lcd_mock::requested_height(req)).set_body_json(json!({ "data": data }))
            } else {
                ResponseTemplate::new(500).set_body_json(json!({ "message": "pool query failed" }))
            }
        })
        .mount(&server)
        .await;

    let lcd = LcdClient::new(
        vec![server.uri().trim_end_matches('/').to_string()],
        5000,
        30000,
        "columbus-5",
    );

    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .expect("cycle completes despite per-pair failure");

    let row = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("good pair updated");
    assert_eq!(row.reserve_0, bd("111"));
    assert_eq!(row.reserve_1, bd("222"));

    let broken_pair_id: i32 =
        sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = $1")
            .bind(&bad_pair)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        pair_reserves::get_pair_reserves(&pool, broken_pair_id)
            .await
            .unwrap()
            .is_none(),
        "failed pair keeps no snapshot (never written)"
    );
}

#[tokio::test]
#[serial]
async fn snapshot_block_height_none_when_lcd_height_fails() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let mock = common::lcd_mock::start_book_snapshot_mock(None).await;
    let lcd = LcdClient::new(
        vec![common::lcd_mock::lcd_base_url(&mock)],
        5000,
        0, // Test-only immediate half-open recovery after the cancelled head probe.
        "columbus-5",
    );

    book_snapshot::snapshot_all_pairs_with_limits(
        &pool,
        &lcd,
        book_snapshot::SnapshotLimits {
            height_timeout: std::time::Duration::from_millis(50),
            ..book_snapshot::SnapshotLimits::default()
        },
    )
    .await
    .expect("cycle completes without block height");

    let reserves = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("reserves still written");
    assert_eq!(reserves.block_height, None);
}

#[test]
fn lcd_budget_constant_matches_formula() {
    let pair_count = 4usize;
    let resting = 17usize;
    let expected =
        BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD + pair_count * BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR + resting;
    assert_eq!(book_snapshot_lcd_budget(pair_count, resting), expected);
    assert!(BOOK_SNAPSHOT_MAX_STALENESS_MS > 0);
}

/// A delayed book must not starve another pair or overwrite a prior atomic mirror.
#[tokio::test]
#[serial]
async fn slow_book_is_bounded_healthy_pair_refreshes_and_timeout_recovers() {
    use book_snapshot::SnapshotLimits;
    use std::time::Duration;
    let evidence: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/qa/issue-1380/localterra-reserve-fixture.json"
    ))
    .unwrap();
    let recorded_pool = evidence["pool"].clone();
    let captured_0 = bd(recorded_pool["assets"][0]["amount"].as_str().unwrap());
    let captured_1 = bd(recorded_pool["assets"][1]["amount"].as_str().unwrap());
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    // Replay the fixture with its exact contract identities, never just amounts.
    sqlx::query("UPDATE assets SET is_cw20=true, denom=NULL, contract_address=$2 WHERE id=$1")
        .bind(seed.asset_0_id)
        .bind(
            recorded_pool["assets"][0]["info"]["token"]["contract_addr"]
                .as_str()
                .unwrap(),
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE assets SET contract_address=$2 WHERE id=$1")
        .bind(seed.asset_1_id)
        .bind(
            recorded_pool["assets"][1]["info"]["token"]["contract_addr"]
                .as_str()
                .unwrap(),
        )
        .execute(&pool)
        .await
        .unwrap();
    let slow_addr = "terra1slowbook";
    let slow_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1slow_lp', 25) RETURNING id",
    )
    .bind(slow_addr)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    pair_reserves::upsert_pair_reserves(&pool, slow_id, &captured_0, &captured_1, 25, None)
        .await
        .unwrap();
    let prior = pair_reserves::get_pair_reserves(&pool, slow_id)
        .await
        .unwrap()
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmos/base/tendermint/v1beta1/blocks/latest$",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "block": { "header": { "chain_id": "columbus-5", "height": "999", "time": "2026-10-04T00:00:00Z" } }
                })),
        )
        .mount(&server)
        .await;
    let delayed_pool = recorded_pool.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let q = common::lcd_mock::smart_query_from_request(req);
            let data = if q.get("pool").is_some() {
                delayed_pool.clone()
            } else if q.get("get_fee_config").is_some() {
                json!({ "fee_config": { "fee_bps": 25, "treasury": "terra1treasury" } })
            } else {
                json!(null)
            };
            let response = ResponseTemplate::new(200)
                .insert_header(
                    "x-cosmos-block-height",
                    common::lcd_mock::requested_height(req),
                )
                .set_body_json(json!({ "data": data }));
            if req.url.path().contains(slow_addr) && q.get("order_book_head").is_some() {
                response.set_delay(Duration::from_secs(3))
            } else {
                response
            }
        })
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 5000, 0, "columbus-5");
    // Cold chain identity acquisition is separate from the component deadline.
    lcd.attest_endpoints().await.unwrap();
    let budgets = SnapshotLimits {
        concurrency: 2,
        pair_timeout: Duration::from_millis(500),
        height_timeout: Duration::from_millis(50),
    };
    let task_pool = pool.clone();
    let task_lcd = lcd.clone();
    let task = tokio::spawn(async move {
        book_snapshot::snapshot_all_pairs_with_limits(&task_pool, &task_lcd, budgets).await
    });
    // Wait on the actual database condition, not on a timing-dependent fixed sleep.
    tokio::time::timeout(Duration::from_millis(400), async {
        loop {
            if let Some(row) = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
                .await
                .unwrap()
            {
                if row.reserve_0 == captured_0 {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("healthy reserve refreshed before delayed book deadline");
    assert!(
        !task.is_finished(),
        "healthy pair refreshed while slow book still pending"
    );
    tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let failed = pair_reserves::get_pair_reserves(&pool, slow_id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        failed.snapshot_at > prior.snapshot_at,
        "pool read survives book timeout"
    );
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, slow_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        hybrid.snapshot_at, prior.snapshot_at,
        "failed book never renews hybrid generation"
    );
    assert_eq!(failed.reserve_0, captured_0);
    let healthy = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        healthy.block_height,
        Some(999),
        "pool and book use the verified pinned height"
    );

    // Remove delays: unchanged positive LCD amounts recover naturally on the next cycle.
    server.reset().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmos/base/tendermint/v1beta1/blocks/latest$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block": {"header": {"chain_id":"columbus-5", "height":"999", "time":"2026-10-09T00:00:00Z"}}
        }))).mount(&server).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let q = common::lcd_mock::smart_query_from_request(req);
            let data = if q.get("pool").is_some() {
                recorded_pool.clone()
            } else if q.get("get_fee_config").is_some() {
                json!({ "fee_config": { "fee_bps": 25, "treasury": "terra1treasury" } })
            } else {
                json!(null)
            };
            ResponseTemplate::new(200)
                .insert_header(
                    "x-cosmos-block-height",
                    common::lcd_mock::requested_height(req),
                )
                .set_body_json(json!({ "data": data }))
        })
        .mount(&server)
        .await;
    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .unwrap();
    let recovered = pair_reserves::get_pair_reserves(&pool, slow_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.reserve_0, captured_0);
    assert_eq!(recovered.reserve_1, captured_1);
    assert!(recovered.snapshot_at > prior.snapshot_at);
}

#[tokio::test]
#[serial]
async fn only_successful_full_factory_enumeration_marks_empty_inventory_known() {
    use cl8y_dex_indexer::db::queries::state;
    use cl8y_dex_indexer::indexer::pair_discovery;
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    sqlx::query("DELETE FROM indexer_state WHERE key='factory_inventory_observed_at'")
        .execute(&pool)
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmwasm/wasm/v1/contract/terra1factory/smart/",
        ))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({ "message": "failed enumeration" })),
        )
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 1000, 1, "columbus-5");
    pair_discovery::sync_all_pairs(&pool, &lcd, "terra1factory")
        .await
        .unwrap();
    assert!(state::get_state(&pool, "factory_inventory_observed_at")
        .await
        .unwrap()
        .is_none());
    server.reset().await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmwasm/wasm/v1/contract/terra1factory/smart/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": { "pairs": [] } })))
        .mount(&server)
        .await;
    pair_discovery::sync_all_pairs(&pool, &lcd, "terra1factory")
        .await
        .unwrap();
    let observed = state::get_state(&pool, "factory_inventory_observed_at")
        .await
        .unwrap()
        .unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(&observed).is_ok());
    sqlx::query("DELETE FROM indexer_state WHERE key='factory_inventory_observed_at'")
        .execute(&pool)
        .await
        .unwrap();
}

/// #1453: component failures preserve the independently committed pool and the
/// prior compatible generation, including cold missing fees and delayed books.
#[tokio::test]
#[serial]
async fn issue_1453_pool_success_survives_component_failures() {
    use cl8y_dex_indexer::db::queries::pairs;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let pair = pairs::get_pair_by_address(&pool, &seed.pair_address)
        .await
        .unwrap()
        .unwrap();
    for (index, component) in ["get_fee_config", "bid", "ask", "timeout", "get_fee_config"]
        .iter()
        .enumerate()
    {
        sqlx::query("DELETE FROM pair_fee_observations")
            .execute(&pool)
            .await
            .unwrap();
        if index == 1 {
            pair_reserves::upsert_pair_reserves(
                &pool,
                seed.pair_id,
                &bd("1000"),
                &bd("2000"),
                25,
                Some(100),
            )
            .await
            .unwrap();
        }
        let before = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap();
        let server = MockServer::start().await;
        common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
        let component = component.to_string();
        let amount = (1000 + index).to_string();
        Mock::given(method("GET"))
            .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
            .respond_with(move |req: &wiremock::Request| {
                let q = common::lcd_mock::smart_query_from_request(req);
                if q.get("pool").is_some() {
                    return ResponseTemplate::new(200).insert_header("x-cosmos-block-height", common::lcd_mock::requested_height(req)).set_body_json(json!({"data": {
                        "assets": [
                            {"info": {"native_token": {"denom": "uluna"}}, "amount": amount},
                            {"info": {"token": {"contract_addr": "terra1ustctoken"}}, "amount": "2000"}
                        ], "total_share": "1"
                    }}));
                }
                if (component == "get_fee_config" && q.get("get_fee_config").is_some())
                    || q["order_book_head"]["side"].as_str() == Some(component.as_str()) {
                    // Deterministic component rejection must not poison provider health.
                    return ResponseTemplate::new(400).set_body_json(json!({"message":"query wasm contract failed: component unavailable"}));
                }
                if q.get("get_fee_config").is_some() {
                    return ResponseTemplate::new(200).insert_header("x-cosmos-block-height", common::lcd_mock::requested_height(req)).set_body_json(json!({"data": {"fee_config":{"fee_bps":25,"treasury":"terra1treasury"}}}));
                }
                let response = ResponseTemplate::new(200).insert_header("x-cosmos-block-height", common::lcd_mock::requested_height(req)).set_body_json(json!({"data":null}));
                if component == "timeout" { response.set_delay(Duration::from_secs(1)) } else { response }
            }).mount(&server).await;
        let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
        // Cancellation below exercises the initialized pool/book component,
        // while retaining the same shared provider budget and exact deadline.
        lcd.attest_endpoints().await.unwrap();
        let result = tokio::time::timeout(
            if index == 3 {
                Duration::from_millis(200)
            } else {
                Duration::from_secs(2)
            },
            book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(100 + index as i64)),
        )
        .await;
        assert!(!matches!(result, Ok(Ok(()))), "component work must fail");
        let current = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            current.reserve_0,
            bd(&(1000 + index).to_string()),
            "case {index}: {result:?}"
        );
        assert_eq!(current.block_height, Some(100 + index as i64));
        assert!(
            chrono::Utc::now()
                .signed_duration_since(current.snapshot_at)
                .num_seconds()
                < 2
        );
        let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap();
        assert_eq!(
            hybrid.as_ref().map(|r| r.snapshot_at),
            before.as_ref().map(|r| r.snapshot_at)
        );
        if index == 0 {
            assert_eq!(current.fee_bps, -1, "unobserved fee is not quoted as zero");
        }
    }
}

#[tokio::test]
#[serial]
async fn issue_1453_monotonic_pool_and_hybrid_generations() {
    use chrono::{Duration, Utc};
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    // Maximum raw Decimal remains within the contract's human price band for 6/18.
    sqlx::query("UPDATE assets SET decimals=18 WHERE id=$1")
        .bind(seed.asset_1_id)
        .execute(&pool)
        .await
        .unwrap();
    let at = chrono::DateTime::from_timestamp_micros(
        (Utc::now() - Duration::seconds(1)).timestamp_micros(),
    )
    .unwrap();
    assert!(pair_reserves::upsert_pool_observation(
        &pool,
        seed.pair_id,
        &bd("340282366920938463463374607431768211455"),
        &bd("1"),
        Some(200),
        at
    )
    .await
    .unwrap());
    // Older response, regressed height, or unknown height cannot displace known data.
    for (height, stamp) in [
        (Some(200), at),
        (Some(201), at - Duration::milliseconds(1)),
        (Some(199), at + Duration::milliseconds(1)),
        (None, at + Duration::milliseconds(2)),
    ] {
        assert!(!pair_reserves::upsert_pool_observation(
            &pool,
            seed.pair_id,
            &bd("9"),
            &bd("9"),
            height,
            stamp
        )
        .await
        .unwrap());
    }
    let original = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(original.snapshot_at, at);
    let mut tx = pool.begin().await.unwrap();
    assert!(
        pair_reserves::upsert_hybrid_observation(&mut tx, &original, 25)
            .await
            .unwrap()
    );
    resting_orders::replace_pair_resting_orders_in_tx(
        &mut tx,
        seed.pair_id,
        original.block_height,
        &[
            resting_orders::RestingOrderInput {
                order_id: 1,
                side: "bid".to_string(),
                price: bd("1000"),
                remaining: bd("340282366920938463463374607431768211455"),
                owner: None,
                expires_at: None,
            },
            resting_orders::RestingOrderInput {
                order_id: 2,
                side: "ask".to_string(),
                price: bd("340282366920938463463.374607431768211455"),
                remaining: bd("1000000"),
                owner: None,
                expires_at: None,
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let book = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "bid")
        .await
        .unwrap();
    assert_eq!(
        book[0].remaining,
        bd("340282366920938463463374607431768211455")
    );
    let asks = resting_orders::get_pair_resting_book(&pool, seed.pair_id, "ask")
        .await
        .unwrap();
    assert_eq!(
        asks[0].price,
        bd("340282366920938463463.374607431768211455")
    );
    assert!(pair_reserves::upsert_pool_observation(
        &pool,
        seed.pair_id,
        &bd("2"),
        &bd("3"),
        Some(201),
        at + Duration::milliseconds(10)
    )
    .await
    .unwrap());
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        hybrid.reserve_0, original.reserve_0,
        "hybrid keeps its independently versioned pool"
    );
    let mut old = original.clone();
    old.block_height = Some(199);
    let mut tx = pool.begin().await.unwrap();
    assert!(!pair_reserves::upsert_hybrid_observation(&mut tx, &old, 25)
        .await
        .unwrap());
    // Repeated same-reserve generation also cannot replace a newer book attempt.
    assert!(
        !pair_reserves::upsert_hybrid_observation(&mut tx, &original, 25)
            .await
            .unwrap()
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
#[serial]
async fn issue_1453_pool_asset_spoof_preserves_original_observation() {
    use cl8y_dex_indexer::db::queries::pairs;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("123"), &bd("456"), 25, Some(1))
        .await
        .unwrap();
    let prior = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "2")
                .set_body_json(json!({"data": {
                    "assets": [
                        {"info": {"native_token": {"denom": "spoofed"}}, "amount":"0"},
                        {"info": {"token": {"contract_addr": "terra1ustctoken"}}, "amount":"0"}
                    ], "total_share":"0"
                }})),
        )
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
    let pair = pairs::get_pair_by_address(&pool, &seed.pair_address)
        .await
        .unwrap()
        .unwrap();
    assert!(
        book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(2))
            .await
            .is_err()
    );
    let after = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.reserve_0, prior.reserve_0);
    assert_eq!(after.snapshot_at, prior.snapshot_at);
}

#[tokio::test]
#[serial]
async fn issue_1453_background_lanes_refresh_pool_while_book_stalls() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let height_reads = Arc::new(AtomicUsize::new(200));
    let heights = height_reads.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmos/base/tendermint/v1beta1/blocks/latest$"))
        .respond_with(move |_req: &wiremock::Request| {
            let height = heights.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(json!({"block":{"header":{"chain_id":"columbus-5","height":height.to_string(),"time":"2026-10-09T00:00:00Z"}}}))
        }).mount(&server).await;
    let reads = Arc::new(AtomicUsize::new(0));
    let counter = reads.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let q = common::lcd_mock::smart_query_from_request(req);
            if q.get("pool").is_some() {
                let amount = counter.fetch_add(1, Ordering::SeqCst) + 1;
                return ResponseTemplate::new(200)
                    .insert_header(
                        "x-cosmos-block-height",
                        common::lcd_mock::requested_height(req),
                    )
                    .set_body_json(json!({"data":{"assets":[
                    {"info":{"native_token":{"denom":"uluna"}},"amount":amount.to_string()},
                    {"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000"}
                ],"total_share":"1"}}));
            }
            if q.get("get_fee_config").is_some() {
                return ResponseTemplate::new(200)
                    .insert_header(
                        "x-cosmos-block-height",
                        common::lcd_mock::requested_height(req),
                    )
                    .set_body_json(
                        json!({"data":{"fee_config":{"fee_bps":25,"treasury":"terra1treasury"}}}),
                    );
            }
            ResponseTemplate::new(200)
                .insert_header(
                    "x-cosmos-block-height",
                    common::lcd_mock::requested_height(req),
                )
                .set_body_json(json!({"data":null}))
                .set_delay(Duration::from_secs(2))
        })
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 3000, 1000, "columbus-5");
    // Keep cold attestation outside the one-second lane progress assertion.
    lcd.attest_endpoints().await.unwrap();
    let task = tokio::spawn(book_snapshot::run_book_snapshot_loop(pool.clone(), lcd, 40));
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if reads.load(Ordering::SeqCst) >= 4 {
                if let Some(row) = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
                    .await
                    .unwrap()
                {
                    if row.fee_bps == 25 && row.block_height.is_some_and(|h| h >= 203) {
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("pool lane continues during slow book");
    task.abort();
    let _ = task.await;
    assert!(pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .is_some());
    assert!(
        pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .is_none(),
        "unfinished book is not a hybrid generation"
    );
}

#[tokio::test]
#[serial]
async fn issue_1453_replica_lane_leadership_is_cancel_safe() {
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let server = common::lcd_mock::start_book_snapshot_mock(Some("300")).await;
    let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
    // Simulate another replica owning both lanes. No duplicate LCD writer may
    // start, and releasing leadership must allow bounded scheduled recovery.
    let mut leader = pool.begin().await.unwrap();
    for key in [1_i32, 2] {
        sqlx::query("SELECT pg_advisory_xact_lock(1453, $1)")
            .bind(key)
            .execute(&mut *leader)
            .await
            .unwrap();
    }
    let task = tokio::spawn(book_snapshot::run_book_snapshot_loop(
        pool.clone(),
        lcd.clone(),
        40,
    ));
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "standby replica never touches LCD"
    );
    leader.rollback().await.unwrap();
    // Allow one bounded head probe + pair job (4s each), with scheduler margin.
    // A one-second expectation is stronger than the production ingestion budget.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if pair_reserves::get_pair_reserves(&pool, seed.pair_id)
                .await
                .unwrap()
                .is_some()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("standby acquires released lane leadership");
    task.abort();
    let _ = task.await;
    // Cancellation drops leadership transactions, so one-shot checks can run.
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let mut tx = pool.begin().await.unwrap();
            let released: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(1453, 1)")
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            tx.rollback().await.unwrap();
            if released {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("cancelled pool leader releases advisory ownership");
}

#[tokio::test]
#[serial]
async fn issue_1453_delayed_fee_cannot_poison_newer_pool() {
    use cl8y_dex_indexer::db::queries::pairs;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let pair = pairs::get_pair_by_address(&pool, &seed.pair_address)
        .await
        .unwrap()
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let q = common::lcd_mock::smart_query_from_request(req);
            let data = if q.get("pool").is_some() {
                json!({"assets":[
                    {"info":{"native_token":{"denom":"uluna"}},"amount":"1000"},
                    {"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000"}
                ],"total_share":"1"})
            } else if q.get("get_fee_config").is_some() {
                json!({"fee_config":{"fee_bps":25,"treasury":"terra1treasury"}})
            } else {
                json!(null)
            };
            let response = ResponseTemplate::new(200)
                .insert_header(
                    "x-cosmos-block-height",
                    common::lcd_mock::requested_height(req),
                )
                .set_body_json(json!({"data":data}));
            if q.get("get_fee_config").is_some() {
                response.set_delay(Duration::from_millis(150))
            } else {
                response
            }
        })
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
    let task_pool = pool.clone();
    let task = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&task_pool, &lcd, &pair, Some(200)).await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let requests = server.received_requests().await.unwrap();
            if requests.iter().any(|r| {
                common::lcd_mock::smart_query_from_request(r)
                    .get("get_fee_config")
                    .is_some()
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let at = chrono::Utc::now();
    assert!(pair_reserves::upsert_pool_observation(
        &pool,
        seed.pair_id,
        &bd("3000"),
        &bd("4000"),
        Some(201),
        at
    )
    .await
    .unwrap());
    assert!(
        task.await.unwrap().is_err(),
        "newer current pool requires its own compatible fee before hybrid eligibility"
    );
    let latest = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(latest.block_height, Some(201));
    assert_eq!(latest.reserve_0, bd("3000"));
    assert_eq!(
        latest.fee_bps, -1,
        "old height200 fee is not reused for height201 reserves"
    );
    assert!(
        pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .is_none(),
        "old fee never forms a hybrid with newer reserves"
    );
}

#[tokio::test]
#[serial]
async fn issue_1453_lost_backend_cancels_lane_and_recovers() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let server = common::lcd_mock::start_book_snapshot_mock(Some("300")).await;
    let reads = Arc::new(AtomicUsize::new(0));
    let counter = reads.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |req: &wiremock::Request| {
            let q = common::lcd_mock::smart_query_from_request(req);
            let mut first_pool = false;
            let data = if q.get("pool").is_some() {
                first_pool = counter.fetch_add(1, Ordering::SeqCst) == 0;
                json!({"assets":[
                    {"info":{"native_token":{"denom":"uluna"}},"amount":"1000"},
                    {"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000"}
                ],"total_share":"1"})
            } else if q.get("get_fee_config").is_some() {
                json!({"fee_config":{"fee_bps":25,"treasury":"terra1treasury"}})
            } else {
                json!(null)
            };
            let response = ResponseTemplate::new(200)
                .insert_header(
                    "x-cosmos-block-height",
                    common::lcd_mock::requested_height(req),
                )
                .set_body_json(json!({"data":data}));
            if first_pool {
                response.set_delay(Duration::from_secs(2))
            } else {
                response
            }
        })
        .with_priority(1)
        .mount(&server)
        .await;
    // Test-only immediate recovery isolates leadership cancellation from circuit
    // cooldown, which is exercised independently by the provider tests.
    let lcd = LcdClient::new(vec![server.uri()], 3000, 0, "columbus-5");
    let active = tokio::spawn(book_snapshot::run_book_snapshot_loop(
        pool.clone(),
        lcd.clone(),
        100,
    ));
    tokio::time::timeout(Duration::from_secs(1), async {
        while reads.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let standby = tokio::spawn(book_snapshot::run_book_snapshot_loop(
        pool.clone(),
        lcd.clone(),
        100,
    ));
    // pg_locks covers the entire cluster. Check database and login in the same
    // statement as termination so parallel binaries cannot kill each other's lanes.
    let terminated: bool = sqlx::query_scalar(
        "SELECT pg_terminate_backend(l.pid)
         FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid
         WHERE l.locktype='advisory' AND l.classid=1453 AND l.objid=1 AND l.granted
           AND l.database=(SELECT oid FROM pg_database WHERE datname=current_database())
           AND a.datname=current_database() AND a.usename=current_user
           AND l.pid<>pg_backend_pid() LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(terminated);
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(row) = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
                .await
                .unwrap()
            {
                if row.fee_bps == 25 {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("live lane acquires released leadership and publishes compatible fee");
    assert!(
        reads.load(Ordering::SeqCst) >= 2,
        "replacement read starts after leader loss"
    );
    active.abort();
    standby.abort();
    let _ = active.await;
    let _ = standby.await;
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if lcd
                .provider_counters()
                .await
                .iter()
                .all(|c| c.in_flight == 0)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("leadership cancellation releases outstanding provider permits");
}

/// #1453 the bounded head deadline must reach a healthy third endpoint after
/// two attempt deadlines; otherwise successful pool reads cannot advance known height.
#[tokio::test]
#[serial]
async fn issue_1453_third_provider_recovers_known_height_within_head_budget() {
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    pair_reserves::upsert_pair_reserves(&pool, seed.pair_id, &bd("999"), &bd("888"), 25, Some(699))
        .await
        .unwrap();
    let original = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    let first = MockServer::start().await;
    let second = MockServer::start().await;
    for server in [&first, &second] {
        common::lcd_mock::mock_chain_identity(server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"^/cosmos/base/tendermint/v1beta1/blocks/latest$",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"block":{"header":{
                        "chain_id":"columbus-5","height":"700","time":"2026-10-09T00:00:00Z"
                    }}}))
                    .set_delay(Duration::from_millis(300)),
            )
            .mount(server)
            .await;
    }
    let healthy = common::lcd_mock::start_book_snapshot_mock(Some("700")).await;
    let lcd = LcdClient::new_with_policy(
        vec![first.uri(), second.uri(), healthy.uri()],
        1000,
        1000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 100,
            deadline_ms: 400,
            ..ProviderPolicy::default()
        },
    );
    book_snapshot::snapshot_all_pairs_with_limits(
        &pool,
        &lcd,
        book_snapshot::SnapshotLimits {
            concurrency: 2,
            pair_timeout: Duration::from_millis(400),
            height_timeout: Duration::from_millis(400),
        },
    )
    .await
    .unwrap();
    let current = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(current.block_height, Some(700));
    assert_eq!(current.reserve_0, bd("1000000"));
    assert_eq!(current.fee_bps, 25);
    assert!(current.snapshot_at > original.snapshot_at);
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hybrid.block_height, Some(700));
    for server in [&first, &second] {
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.url.path().ends_with("/blocks/latest"))
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.url.path().contains("/smart/"))
                .count(),
            0,
            "failed head providers remain cooled down for pool/fee/book work"
        );
    }
}

/// #1453 healthy shared admission pressure must wait inside pair deadlines,
/// rather than drain the pair list while the only provider permit is occupied.
#[tokio::test]
#[serial]
async fn issue_1453_shared_provider_capacity_preserves_all_four_pairs() {
    use base64::Engine;
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let mut ids = vec![seed.pair_id];
    for index in 1..4 {
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
             VALUES ($1,$2,$3,'terra1capacitylp',25) RETURNING id",
        )
        .bind(format!("terra1capacitypair{index}"))
        .bind(seed.asset_0_id)
        .bind(seed.asset_1_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        ids.push(id);
    }
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let in_flight = active.clone();
    let max_flight = maximum.clone();
    let app = axum::Router::new()
        .route("/cosmos/base/tendermint/v1beta1/node_info", axum::routing::get(|| async {
            axum::Json(json!({"default_node_info":{"network":"columbus-5"}}))
        }))
        .route("/cosmos/base/tendermint/v1beta1/blocks/latest", axum::routing::get(|| async {
            axum::Json(json!({"block":{"header":{"chain_id":"columbus-5","height":"900","time":"2026-10-09T00:00:00Z"}}}))
        }))
        .fallback(axum::routing::get(move |uri: axum::http::Uri, request_headers: axum::http::HeaderMap| {
            let active = in_flight.clone(); let maximum = max_flight.clone();
            async move {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(now, Ordering::SeqCst);
                let encoded = uri.path().split("/smart/").nth(1).unwrap();
                let query: serde_json::Value = serde_json::from_slice(
                    &base64::engine::general_purpose::STANDARD.decode(encoded).unwrap()).unwrap();
                tokio::time::sleep(Duration::from_millis(20)).await;
                let data = if query.get("pool").is_some() {
                    json!({"assets":[
                        {"info":{"native_token":{"denom":"uluna"}},"amount":"1000"},
                        {"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000"}
                    ],"total_share":"1"})
                } else if query.get("get_fee_config").is_some() {
                    json!({"fee_config":{"fee_bps":25,"treasury":"terra1treasury"}})
                } else { assert!(query.get("order_book_head").is_some()); json!(null) };
                let mut response_headers = axum::http::HeaderMap::new();
                response_headers.insert("x-cosmos-block-height", request_headers["x-cosmos-block-height"].clone());
                active.fetch_sub(1, Ordering::SeqCst);
                (response_headers, axum::Json(json!({"data":data})))
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    let lcd = LcdClient::new_with_policy(
        vec![url],
        1000,
        1000,
        "columbus-5",
        ProviderPolicy {
            concurrency: 1,
            ..ProviderPolicy::default()
        },
    );
    book_snapshot::snapshot_all_pairs_with_limits(
        &pool,
        &lcd,
        book_snapshot::SnapshotLimits {
            concurrency: 4,
            pair_timeout: Duration::from_secs(4),
            height_timeout: Duration::from_secs(4),
        },
    )
    .await
    .unwrap();
    for id in ids {
        let raw = pair_reserves::get_pair_reserves(&pool, id)
            .await
            .unwrap()
            .expect("every healthy pool observed");
        let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, id)
            .await
            .unwrap()
            .expect("every healthy empty book compatible");
        assert_eq!(
            (raw.reserve_0, raw.fee_bps, raw.block_height),
            (bd("1000"), 25, Some(900))
        );
        assert_eq!(hybrid.block_height, Some(900));
        assert_eq!(hybrid.snapshot_at, raw.snapshot_at);
    }
    let counters = lcd.provider_counters().await;
    assert!(counters.iter().any(|c| c.concurrency_deferrals > 0));
    assert!(counters.iter().all(|c| c.in_flight == 0 && c.failures == 0));
    assert_eq!(
        maximum.load(Ordering::SeqCst),
        1,
        "actual simultaneous provider data handlers remain bounded"
    );
    shutdown_tx.send(()).unwrap();
    server.await.unwrap();
}

/// Production reserve/book lanes must leave provider capacity for independently
/// scheduled quote admission; publication keeps the producer's observation age.
#[tokio::test]
#[serial]
async fn recovery1464_production_lanes_overlap_scheduled_admission() {
    use base64::Engine;
    use cl8y_dex_indexer::{api, lcd::ProviderPolicy};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let mut seed = common::seed_route_solve_with_mirror(&pool).await;
    seed.token_a = "terra1fairnessaa000000000000000000000000".into();
    seed.token_b = "terra1fairnessbb000000000000000000000000".into();
    sqlx::query(
        "UPDATE assets SET contract_address=$1 WHERE contract_address='terra1routesolveaaa'",
    )
    .bind(&seed.token_a)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE assets SET contract_address=$1 WHERE contract_address='terra1routesolvebbb'",
    )
    .bind(&seed.token_b)
    .execute(&pool)
    .await
    .unwrap();
    // Four pairs share just two ordinary CW20 assets; this exercises the actual
    // production four-pool/two-book lanes without a larger policy fixture.
    let (a, b): (i32, i32) = sqlx::query_as("SELECT asset_0_id,asset_1_id FROM pairs")
        .fetch_one(&pool)
        .await
        .unwrap();
    for i in 1..4 {
        let id: i32 = sqlx::query_scalar("INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) VALUES($1,$2,$3,'terra1capacitylp',30) RETURNING id")
            .bind(format!("terra1fairnesspair{i}")).bind(a).bind(b).fetch_one(&pool).await.unwrap();
        pair_reserves::upsert_pair_reserves(
            &pool,
            id,
            &bd("10000000000000"),
            &bd("10000000000000"),
            30,
            Some(900),
        )
        .await
        .unwrap();
    }
    sqlx::query("UPDATE pair_reserves SET block_height=900")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE hybrid_pair_reserves SET block_height=900")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO pair_fee_observations(pair_id,fee_bps,block_height,observed_at) SELECT id,30,900,NOW() FROM pairs")
        .execute(&pool).await.unwrap();
    common::reference_marks::seed_reference_marks(
        &pool,
        &cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig::from_indexer_config(
            &common::test_config(),
        ),
        &[],
    )
    .await;
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let pools = Arc::new(AtomicUsize::new(0));
    let books = Arc::new(AtomicUsize::new(0));
    let completed_books = Arc::new(AtomicUsize::new(0));
    let max_pools = Arc::new(AtomicUsize::new(0));
    let max_books = Arc::new(AtomicUsize::new(0));
    let overlap = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    // A deterministic gate demonstrates overlap without depending on slow fake
    // responses or extending production provider/pair deadlines.
    let (release_lanes, lane_release) = tokio::sync::watch::channel(false);
    let app = axum::Router::new()
        .route("/cosmos/base/tendermint/v1beta1/node_info", axum::routing::get(|| async { axum::Json(json!({"default_node_info":{"network":"columbus-5"}})) }))
        .route("/cosmos/base/tendermint/v1beta1/blocks/latest", axum::routing::get(|| async { axum::Json(json!({"block":{"header":{"chain_id":"columbus-5","height":"900","time":"2026-10-10T00:00:00Z"}}})) }))
        .fallback(axum::routing::get({
            let (active,maximum,pools,books,completed_books,max_pools,max_books,overlap,requests) = (active.clone(),maximum.clone(),pools.clone(),books.clone(),completed_books.clone(),max_pools.clone(),max_books.clone(),overlap.clone(),requests.clone());
            let (token_a,token_b) = (seed.token_a.clone(),seed.token_b.clone());
            let (release_lanes,lane_release) = (release_lanes.clone(),lane_release.clone());
            move |uri: axum::http::Uri, request_headers: axum::http::HeaderMap| {
                let (active,maximum,pools,books,completed_books,max_pools,max_books,overlap,requests) = (active.clone(),maximum.clone(),pools.clone(),books.clone(),completed_books.clone(),max_pools.clone(),max_books.clone(),overlap.clone(),requests.clone());
                let (token_a,token_b) = (token_a.clone(),token_b.clone());
                let (release_lanes,mut lane_release) = (release_lanes.clone(),lane_release.clone());
                async move {
                    requests.fetch_add(1,Ordering::SeqCst);
                    maximum.fetch_max(active.fetch_add(1,Ordering::SeqCst)+1,Ordering::SeqCst);
                    let query = uri.path().split("/smart/").nth(1).map(|encoded| serde_json::from_slice::<serde_json::Value>(&base64::engine::general_purpose::STANDARD.decode(encoded).unwrap()).unwrap());
                    let is_pool = query.as_ref().is_some_and(|q|q.get("pool").is_some());
                    let is_book = query.as_ref().is_some_and(|q|q.get("order_book_head").is_some());
                    if is_pool { max_pools.fetch_max(pools.fetch_add(1,Ordering::SeqCst)+1,Ordering::SeqCst); }
                    if is_book { max_books.fetch_max(books.fetch_add(1,Ordering::SeqCst)+1,Ordering::SeqCst); }
                    if query.is_none() && pools.load(Ordering::SeqCst)>0 && books.load(Ordering::SeqCst)>0 {
                        overlap.fetch_add(1,Ordering::SeqCst);
                        release_lanes.send_replace(true);
                    }
                    if (is_pool || is_book) && !*lane_release.borrow() {
                        tokio::time::timeout(Duration::from_secs(2),lane_release.changed()).await.expect("scheduled metadata releases both lanes").unwrap();
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    let body = if let Some(q) = query {
                        let data = if is_pool {json!({"assets":[{"info":{"token":{"contract_addr":token_a}},"amount":"10000000000000"},{"info":{"token":{"contract_addr":token_b}},"amount":"10000000000000"}],"total_share":"1"})}
                            else if is_book {json!(null)}
                            else if q.get("get_fee_config").is_some() {json!({"fee_config":{"fee_bps":30,"treasury":"terra1treasury"}})}
                            else if q.get("is_paused").is_some() {json!({"paused":false})}
                            else if q.get("get_asset_code_ids").is_some() {json!({"code_ids":[10184,10184]})}
                            else if q.get("blacklist_check").is_some() {json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]})}
                            else if q.get("is_code_id_whitelisted").is_some() {json!({"whitelisted":true})}
                            else if q.get("get_hooks").is_some() {json!({"hooks":[]})}
                            else if q.get("get_discount_registry").is_some() {json!({"registry":null})}
                            else {panic!("unexpected fixture query: {q}")};
                        json!({"data":data})
                    } else {json!({"contract_info":{"code_id":"10184","admin":""}})};
                    if is_pool {pools.fetch_sub(1,Ordering::SeqCst);}
                    if is_book {books.fetch_sub(1,Ordering::SeqCst);completed_books.fetch_add(1,Ordering::SeqCst);}
                    active.fetch_sub(1,Ordering::SeqCst);
                    let mut headers = axum::http::HeaderMap::new();
                    if let Some(height) = request_headers.get("x-cosmos-block-height") {headers.insert("x-cosmos-block-height",height.clone());}
                    (headers,axum::Json(body))
                }
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let upstream = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let lcd = LcdClient::new_with_policy(
        vec![endpoint],
        2_500,
        1,
        "columbus-5",
        ProviderPolicy {
            burst: 256,
            attempt_timeout_ms: 2_500,
            deadline_ms: 8_000,
            ..Default::default()
        },
    );
    let lane_start_at = chrono::Utc::now();
    let liquidity = tokio::spawn(book_snapshot::run_book_snapshot_loop(
        pool.clone(),
        lcd.clone(),
        10_000,
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while pools.load(Ordering::SeqCst) < 4 || books.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("actual production lanes overlap");
    let mut config = common::test_config();
    config.route_solver_db_hybrid = true;
    let state = common::local_quotes::state(pool.clone(), &config, lcd.clone());
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut workers = api::local_quote::start_ingestion_with_cancel(state.clone(), cancel.clone());
    let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api_base = format!("http://{}", api_listener.local_addr().unwrap());
    let api_server = tokio::spawn(async move {
        axum::serve(api_listener, api::build_router(state, &config))
            .await
            .unwrap()
    });
    let client = reqwest::Client::new();
    let quote_url = format!(
        "{api_base}/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&max_spread=0.05",
        seed.token_a, seed.token_b
    );
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if client
                .get(&quote_url)
                .send()
                .await
                .unwrap()
                .status()
                .is_success()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("scheduled ordinary-token admission initializes useful quote");
    // A first useful quote may use seeded or already-complete sibling views.
    // Shutdown retention is tested only after all four actual trading pools
    // have completed both original production lanes, with no live fixture I/O.
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let completed: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pairs p
                JOIN pair_reserves r ON r.pair_id=p.id
                JOIN hybrid_pair_reserves h ON h.pair_id=p.id
                WHERE p.asset_0_id=$1 AND p.asset_1_id=$2
                  AND r.block_height=900 AND h.block_height=900
                  AND r.snapshot_at >= $3",
            )
            .bind(a)
            .bind(b)
            .bind(lane_start_at)
            .fetch_one(&pool)
            .await
            .unwrap();
            if completed == 4
                && completed_books.load(Ordering::SeqCst) >= 8
                && active.load(Ordering::SeqCst) == 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("all four real pool/hybrid observations complete before shutdown");
    let counters = lcd.provider_counters().await;
    assert!(
        counters.iter().all(|c| c.failures == 0),
        "healthy overlapping observations succeed before explicit shutdown: {counters:?}"
    );
    cancel.cancel();
    while workers.join_next().await.is_some() {}
    liquidity.abort();
    let _ = liquidity.await;
    tokio::time::sleep(Duration::from_millis(450)).await;
    let count = requests.load(Ordering::SeqCst);
    let response = client.get(&quote_url).send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert!(
        status.is_success(),
        "completed scheduled observations remain usable: status={status}, body={text}"
    );
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        count,
        "API quote causes zero provider requests"
    );
    assert_eq!(max_pools.load(Ordering::SeqCst), 4);
    assert_eq!(max_books.load(Ordering::SeqCst), 2);
    assert!(maximum.load(Ordering::SeqCst) <= 8);
    assert!(
        overlap.load(Ordering::SeqCst) > 0,
        "ordinary token metadata is read while both real liquidity lanes are active"
    );
    for row in cl8y_dex_indexer::db::queries::pairs::get_all_pairs(&pool)
        .await
        .unwrap()
        .into_iter()
        .filter(|row| row.asset_0_id == a && row.asset_1_id == b)
    {
        let raw = pair_reserves::get_pair_reserves(&pool, row.id)
            .await
            .unwrap()
            .unwrap();
        let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, row.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(raw.block_height, Some(900));
        assert_eq!(hybrid.block_height, Some(900));
        assert!(
            hybrid.snapshot_at <= raw.snapshot_at,
            "book publication cannot renew pool observation time"
        );
        assert!(
            chrono::Utc::now()
                .signed_duration_since(raw.snapshot_at)
                .num_seconds()
                < 20
        );
    }
    assert!(
        lcd.provider_counters()
            .await
            .iter()
            .all(|c| c.in_flight == 0),
        "explicit shutdown releases even active upstream attempts"
    );
    api_server.abort();
    upstream.abort();
}

/// A healthy fallback whose own attempt budget is cut short by the pair's
/// deadline must remain available to the next independently scheduled cycle.
#[tokio::test]
#[serial]
async fn recovery1464_pair_deadline_cancellation_preserves_healthy_next_cycle() {
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let healthy = common::lcd_mock::start_book_snapshot_mock(Some("900")).await;
    let bad = common::lcd_mock::start_book_snapshot_mock(Some("900")).await;
    for (server, delay) in [(&healthy, 2_000), (&bad, 3_000)] {
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
            .and(|request: &wiremock::Request| {
                common::lcd_mock::smart_query_from_request(request)
                    .get("pool")
                    .is_some()
            })
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "900")
                    .set_body_json(json!({"data":{"assets":[
                    {"info":{"native_token":{"denom":"uluna"}},"amount":"1000000"},
                    {"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000000"}
                ],"total_share":"1"}}))
                    .set_delay(Duration::from_millis(delay)),
            )
            .with_priority(1)
            .mount(server)
            .await;
    }
    // The successful head read begins on healthy index0, advancing the shared
    // round-robin cursor to bad index1 for this pair's first pool attempt.
    let lcd = LcdClient::new_with_policy(
        vec![healthy.uri(), bad.uri()],
        2_500,
        5_000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2_500,
            deadline_ms: 8_000,
            max_backoff_ms: 5_000,
            ..Default::default()
        },
    );
    let namespace = lcd.chain_cache_namespace();
    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .unwrap();
    assert!(
        pair_reserves::get_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .is_none(),
        "the cancelled fallback never publishes an incomplete observation"
    );
    let first = lcd.provider_counters().await;
    assert_eq!(
        (
            first[0].failures,
            first[0].caller_cancellations,
            first[0].in_flight
        ),
        (0, 1, 0)
    );
    assert!(
        !first[0].circuit_open,
        "a 4s caller budget is not the healthy fallback's own 2.5s timeout"
    );
    assert!(first[1].circuit_open);
    assert_eq!(
        first[1].failures, 1,
        "the first bad provider still reaches its real timeout"
    );
    let cycle_started = chrono::Utc::now();
    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .unwrap();
    let raw = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (raw.reserve_0, raw.reserve_1, raw.block_height),
        (bd("1000000"), bd("2000000"), Some(900))
    );
    assert_eq!(
        hybrid.snapshot_at, raw.snapshot_at,
        "book publication preserves the original pool observation"
    );
    assert!(raw.snapshot_at >= cycle_started);
    assert!(
        chrono::Utc::now()
            .signed_duration_since(raw.snapshot_at)
            .num_seconds()
            < 20
    );
    assert_eq!(
        lcd.chain_cache_namespace(),
        namespace,
        "snapshot views retain the same provider/cache namespace"
    );
    let recovered = lcd.provider_counters().await;
    assert_eq!(
        (
            recovered[0].failures,
            recovered[0].caller_cancellations,
            recovered[0].in_flight
        ),
        (0, 1, 0)
    );
    assert!(recovered[0].successes > first[0].successes);
}

/// A temporary full provider can defer the head read without producing an
/// unknown-height pool generation once capacity returns inside the head budget.
#[tokio::test]
#[serial]
async fn recovery1464_owned_workers_wait_for_head_capacity_and_publish_pinned_quotes() {
    use base64::Engine;
    use cl8y_dex_indexer::{api, lcd::ProviderPolicy};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let mut seed = common::seed_route_solve_with_mirror(&pool).await;
    seed.token_a = "terra1headwaitaaa000000000000000000000000".into();
    seed.token_b = "terra1headwaitbbb000000000000000000000000".into();
    sqlx::query("UPDATE assets SET contract_address=$1 WHERE symbol='RTEA'")
        .bind(&seed.token_a)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE assets SET contract_address=$1 WHERE symbol='RTEB'")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    // An expired hybrid cannot hide a poisoned current raw generation.
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()-INTERVAL '30 seconds'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE hybrid_pair_reserves SET snapshot_at=NOW()-INTERVAL '30 seconds'")
        .execute(&pool)
        .await
        .unwrap();
    common::reference_marks::seed_reference_marks(
        &pool,
        &cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig::from_indexer_config(
            &common::test_config(),
        ),
        &[],
    )
    .await;
    let held = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let unpinned_pools = Arc::new(AtomicUsize::new(0));
    let (release_tx, release_rx) = tokio::sync::watch::channel(false);
    let app = axum::Router::new().fallback(axum::routing::get({
        let (held,requests,unpinned_pools)=(held.clone(),requests.clone(),unpinned_pools.clone());
        let (a,b)=(seed.token_a.clone(),seed.token_b.clone());
        move |uri:axum::http::Uri, request_headers:axum::http::HeaderMap| {
            let (held,requests,unpinned_pools)=(held.clone(),requests.clone(),unpinned_pools.clone());
            let (a,b)=(a.clone(),b.clone());let mut release=release_rx.clone();
            async move {
                requests.fetch_add(1,Ordering::SeqCst);
                let body=if uri.path().ends_with("/node_info") {
                    json!({"default_node_info":{"network":"columbus-5"}})
                } else if uri.path().ends_with("/blocks/latest") {
                    json!({"block":{"header":{"chain_id":"columbus-5","height":"900","time":"2026-10-11T00:00:00Z"}}})
                } else if uri.path()=="/held" {
                    held.fetch_add(1,Ordering::SeqCst);
                    if !*release.borrow() {release.changed().await.unwrap();}
                    json!({"ok":true})
                } else if let Some(encoded)=uri.path().split("/smart/").nth(1) {
                    let q:serde_json::Value=serde_json::from_slice(&base64::engine::general_purpose::STANDARD.decode(encoded).unwrap()).unwrap();
                    let data=if q.get("pool").is_some() {
                        if request_headers.get("x-cosmos-block-height").is_none() {unpinned_pools.fetch_add(1,Ordering::SeqCst);}
                        json!({"assets":[{"info":{"token":{"contract_addr":a}},"amount":"10000000000000"},{"info":{"token":{"contract_addr":b}},"amount":"10000000000000"}],"total_share":"1"})
                    } else if q.get("get_fee_config").is_some() {json!({"fee_config":{"fee_bps":30,"treasury":"terra1treasury"}})}
                    else if q.get("order_book_head").is_some() {json!(null)}
                    else if q.get("is_paused").is_some() {json!({"paused":false})}
                    else if q.get("get_asset_code_ids").is_some() {json!({"code_ids":[10184,10184]})}
                    else if q.get("blacklist_check").is_some() {json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]})}
                    else if q.get("is_code_id_whitelisted").is_some() {json!({"whitelisted":true})}
                    else if q.get("get_hooks").is_some() {json!({"hooks":[]})}
                    else if q.get("get_discount_registry").is_some() {json!({"registry":null})}
                    else {panic!("unmodeled fixture query");};
                    json!({"data":data})
                } else {json!({"contract_info":{"code_id":"10184","admin":""}})};
                let mut headers=axum::http::HeaderMap::new();
                headers.insert("x-cosmos-block-height",axum::http::HeaderValue::from_static("900"));
                (headers,axum::Json(body))
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let upstream = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let lcd = LcdClient::new_with_policy(
        vec![url],
        2_500,
        1_000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2_500,
            deadline_ms: 8_000,
            ..Default::default()
        },
    );
    let mut holders = tokio::task::JoinSet::new();
    for _ in 0..8 {
        let client = lcd.clone();
        holders.spawn(async move { client.get::<serde_json::Value>("/held").await });
    }
    tokio::time::timeout(Duration::from_secs(1), async {
        while held.load(Ordering::SeqCst) < 8 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(lcd.provider_counters().await[0].in_flight, 8);
    let liquidity = tokio::spawn(book_snapshot::run_book_snapshot_loop(
        pool.clone(),
        lcd.clone(),
        10_000,
    ));
    // The initial book is stale and issues no provider call; the first new
    // deferral is therefore the actual production pool head prelude.
    tokio::time::timeout(Duration::from_secs(1), async {
        while lcd.provider_counters().await[0].concurrency_deferrals == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let released_at = chrono::Utc::now();
    release_tx.send_replace(true);
    while let Some(result) = holders.join_next().await {
        result.unwrap().unwrap();
    }
    let pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs")
        .fetch_one(&pool)
        .await
        .unwrap();
    let first = tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            let row = pair_reserves::get_pair_reserves(&pool, pair_id)
                .await
                .unwrap()
                .unwrap();
            if row.fee_bps == 30 && row.block_height == Some(900) {
                break row;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("released provider supplies a pinned usable pool in the first cycle");
    assert!(
        first.snapshot_at < released_at,
        "capacity wait must preserve the original pre-head timestamp"
    );
    assert_eq!(unpinned_pools.load(Ordering::SeqCst), 0);
    // Stop the head-capacity worker before creating any quote qualification,
    // then complete a bounded pool/book pass with its leadership released.
    // Aborting a later incomplete fee phase after qualification must fail closed;
    // it cannot be used as the setup for a positive cached-quote assertion.
    liquidity.abort();
    let _ = liquidity.await;
    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .unwrap();
    let mut config = common::test_config();
    config.route_solver_db_hybrid = true;
    let state = common::local_quotes::state(pool.clone(), &config, lcd.clone());
    let cancel = tokio_util::sync::CancellationToken::new();
    let mut workers = api::local_quote::start_ingestion_with_cancel(state.clone(), cancel.clone());
    let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", api_listener.local_addr().unwrap());
    let api_server = tokio::spawn(async move {
        axum::serve(api_listener, api::build_router(state, &config))
            .await
            .unwrap()
    });
    let quote = format!(
        "{base}/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&pool_only=true",
        seed.token_a, seed.token_b
    );
    let client = reqwest::Client::new();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, pair_id)
                .await
                .unwrap()
                .unwrap();
            if hybrid.block_height == Some(900)
                && hybrid.snapshot_at >= first.snapshot_at
                && client
                    .get(&quote)
                    .send()
                    .await
                    .unwrap()
                    .status()
                    .is_success()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("actual book and scheduled quote workers initialize compatible generation");
    cancel.cancel();
    while workers.join_next().await.is_some() {}
    let count = requests.load(Ordering::SeqCst);
    let response = client.get(&quote).send().await.unwrap();
    assert!(response.status().is_success());
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert!(
        body["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        count,
        "cached request triggers zero provider RPC"
    );
    assert_eq!(unpinned_pools.load(Ordering::SeqCst), 0);
    let counters = lcd.provider_counters().await;
    assert_eq!(counters[0].failures, 0);
    assert_eq!(counters[0].in_flight, 0);
    api_server.abort();
    upstream.abort();
}

#[tokio::test]
#[serial]
async fn recovery1464_head_floor_tries_valid_alternative_without_renewing_all_old_pools() {
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::time::Duration;
    for all_old in [false, true] {
        let pool = common::setup_pool().await;
        let seed = common::seed_db(&pool).await;
        pair_reserves::upsert_pair_reserves(
            &pool,
            seed.pair_id,
            &bd("7000"),
            &bd("8000"),
            25,
            Some(110),
        )
        .await
        .unwrap();
        let prior = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .unwrap();
        // Both endpoints can serve pinned111, but their cached latest envelopes
        // can be below the last stored generation even inside the lag threshold.
        let old = common::lcd_mock::start_book_snapshot_mock(Some("111")).await;
        let alternative = common::lcd_mock::start_book_snapshot_mock(Some("111")).await;
        for (server, height) in [(&old, 100), (&alternative, if all_old { 101 } else { 111 })] {
            Mock::given(method("GET")).and(wiremock::matchers::path("/cosmos/base/tendermint/v1beta1/blocks/latest"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"block":{"header":{"chain_id":"columbus-5","height":height.to_string(),"time":"2026-10-11T00:00:00Z"}}})))
                .with_priority(1).mount(server).await;
        }
        let lcd = LcdClient::new_with_policy(
            vec![old.uri(), alternative.uri()],
            2_500,
            1_000,
            "columbus-5",
            ProviderPolicy {
                attempt_timeout_ms: 2_500,
                deadline_ms: 8_000,
                ..Default::default()
            },
        );
        let started = chrono::Utc::now();
        book_snapshot::snapshot_all_pairs_with_limits(
            &pool,
            &lcd,
            book_snapshot::SnapshotLimits {
                height_timeout: Duration::from_secs(4),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let raw = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
            .await
            .unwrap()
            .unwrap();
        if all_old {
            assert_eq!(
                (
                    raw.reserve_0,
                    raw.reserve_1,
                    raw.fee_bps,
                    raw.block_height,
                    raw.snapshot_at
                ),
                (
                    prior.reserve_0,
                    prior.reserve_1,
                    prior.fee_bps,
                    prior.block_height,
                    prior.snapshot_at
                ),
                "all-old heads cannot renew or replace a stored generation"
            );
        } else {
            assert_eq!(
                (raw.reserve_0, raw.reserve_1, raw.fee_bps, raw.block_height),
                (bd("1000000"), bd("2000000"), 25, Some(111))
            );
            assert!(raw.snapshot_at >= started);
            let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(hybrid.snapshot_at, raw.snapshot_at);
        }
        assert!(
            lcd.provider_counters()
                .await
                .iter()
                .all(|c| c.failures == 0),
            "valid older heads are query-local outcomes, not provider outages"
        );
        for server in [&old, &alternative] {
            assert_eq!(
                server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .filter(|r| r.url.path().ends_with("/blocks/latest"))
                    .count(),
                1,
                "head alternatives share one bounded scan, no cycle retry resets"
            );
        }
    }
}

#[tokio::test]
#[serial]
async fn recovery1464_cached_same_height_fee_reapplies_without_renewing_pool_time() {
    use std::sync::Arc;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let pair = cl8y_dex_indexer::db::queries::pairs::get_pair_by_address(&pool, &seed.pair_address)
        .await
        .unwrap()
        .unwrap();
    let server = common::lcd_mock::start_book_snapshot_mock(Some("900")).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let signal = started.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|request:&wiremock::Request|common::lcd_mock::smart_query_from_request(request).get("pool").is_some())
        .respond_with(move |_:&wiremock::Request| {
            signal.notify_one();
            ResponseTemplate::new(200).insert_header("x-cosmos-block-height","900")
                .set_body_json(json!({"data":{"assets":[{"info":{"native_token":{"denom":"uluna"}},"amount":"1000000"},{"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000000"}],"total_share":"1"}}))
                .set_delay(Duration::from_millis(150))
        }).with_priority(1).mount(&server).await;
    let lcd = LcdClient::new(vec![server.uri()], 2_500, 1_000, "columbus-5");
    let worker_pool = pool.clone();
    let worker_lcd = lcd.clone();
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&worker_pool, &worker_lcd, &pair, Some(900)).await
    });
    tokio::time::timeout(Duration::from_secs(1), started.notified())
        .await
        .unwrap();
    // The fee becomes visible after the pool's original observation began, so
    // raw upsert deliberately cannot copy it. The later cache hit must reapply.
    sqlx::query("INSERT INTO pair_fee_observations(pair_id,fee_bps,observed_at,block_height) VALUES($1,25,NOW(),900)").bind(seed.pair_id).execute(&pool).await.unwrap();
    worker.await.unwrap().unwrap();
    let raw = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    let fee_time: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT observed_at FROM pair_fee_observations WHERE pair_id=$1")
            .bind(seed.pair_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((raw.fee_bps, raw.block_height), (25, Some(900)));
    assert!(
        raw.snapshot_at < fee_time,
        "cached fee publication cannot renew the pool's pre-response observation time"
    );
    let hybrid = pair_reserves::get_hybrid_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(hybrid.snapshot_at, raw.snapshot_at);
    assert_eq!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| common::lcd_mock::smart_query_from_request(r)
                .get("get_fee_config")
                .is_some())
            .count(),
        0,
        "cached reapplication needs no redundant fee RPC"
    );
}

#[tokio::test]
#[serial]
async fn recovery1464_cached_fee_reapplication_rechecks_locked_current_fee_height() {
    use std::sync::Arc;
    use std::time::Duration;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let pair = cl8y_dex_indexer::db::queries::pairs::get_pair_by_address(&pool, &seed.pair_address)
        .await
        .unwrap()
        .unwrap();
    let server = common::lcd_mock::start_book_snapshot_mock(Some("900")).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let signal = started.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|request:&wiremock::Request|common::lcd_mock::smart_query_from_request(request).get("pool").is_some())
        .respond_with(move |_:&wiremock::Request| {
            signal.notify_one();
            ResponseTemplate::new(200).insert_header("x-cosmos-block-height","900")
                .set_body_json(json!({"data":{"assets":[{"info":{"native_token":{"denom":"uluna"}},"amount":"1000000"},{"info":{"token":{"contract_addr":"terra1ustctoken"}},"amount":"2000000"}],"total_share":"1"}}))
                .set_delay(Duration::from_millis(150))
        }).with_priority(1).mount(&server).await;
    let lcd = LcdClient::new(vec![server.uri()], 2_500, 1_000, "columbus-5");
    let worker_pool = pool.clone();
    let worker_lcd = lcd.clone();
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&worker_pool, &worker_lcd, &pair, Some(900)).await
    });
    tokio::time::timeout(Duration::from_secs(1), started.notified())
        .await
        .unwrap();
    sqlx::query("INSERT INTO pair_fee_observations(pair_id,fee_bps,observed_at,block_height) VALUES($1,25,NOW(),900)").bind(seed.pair_id).execute(&pool).await.unwrap();
    let mut update = pool.begin().await.unwrap();
    sqlx::query("SELECT pair_id FROM pair_fee_observations WHERE pair_id=$1 FOR UPDATE")
        .bind(seed.pair_id)
        .fetch_one(&mut *update)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2),async {
        loop {
            let blocked:i64=sqlx::query_scalar("SELECT COUNT(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%FOR SHARE%'").fetch_one(&pool).await.unwrap();
            if blocked>0 {break;}
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }).await.expect("cached reapplication waits on the fee row it must revalidate");
    let before = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((before.fee_bps, before.block_height), (-1, Some(900)));
    sqlx::query("UPDATE pair_fee_observations SET block_height=901 WHERE pair_id=$1")
        .bind(seed.pair_id)
        .execute(&mut *update)
        .await
        .unwrap();
    update.commit().await.unwrap();
    assert!(
        worker.await.unwrap().is_err(),
        "a superseded cached fee is not a validated publication"
    );
    let after = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (after.fee_bps, after.block_height, after.snapshot_at),
        (-1, Some(900), before.snapshot_at)
    );
    assert_eq!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| common::lcd_mock::smart_query_from_request(r)
                .get("get_fee_config")
                .is_some())
            .count(),
        0
    );
}
