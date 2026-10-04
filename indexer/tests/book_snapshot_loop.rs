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
    let lcd = LcdClient::new(vec![common::lcd_mock::lcd_base_url(&mock)], 5000, 30000);

    book_snapshot::snapshot_all_pairs(&pool, &lcd)
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
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmos/base/tendermint/v1beta1/blocks/latest$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block": { "header": { "height": "500", "time": "2026-06-05T00:00:00Z" } }
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
                ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
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
    let lcd = LcdClient::new(vec![common::lcd_mock::lcd_base_url(&mock)], 5000, 30000);

    book_snapshot::snapshot_all_pairs(&pool, &lcd)
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
    pair_reserves::upsert_pair_reserves(&pool, slow_id, &captured_0, &captured_1, 25, Some(1))
        .await
        .unwrap();
    let prior = pair_reserves::get_pair_reserves(&pool, slow_id)
        .await
        .unwrap()
        .unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmos/base/tendermint/v1beta1/blocks/latest$",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({
                    "block": { "header": { "height": "999", "time": "2026-10-04T00:00:00Z" } }
                }))
                .set_delay(Duration::from_secs(3)),
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
            let response = ResponseTemplate::new(200).set_body_json(json!({ "data": data }));
            if req.url.path().contains(slow_addr) && q.get("order_book_head").is_some() {
                response.set_delay(Duration::from_secs(3))
            } else {
                response
            }
        })
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
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
    assert_eq!(failed.snapshot_at, prior.snapshot_at);
    assert_eq!(failed.reserve_0, captured_0);
    let healthy = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        healthy.block_height, None,
        "timed out height is never invented"
    );

    // Remove delays: unchanged positive LCD amounts recover naturally on the next cycle.
    server.reset().await;
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
            ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
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
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmwasm/wasm/v1/contract/terra1factory/smart/",
        ))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(json!({ "message": "failed enumeration" })),
        )
        .mount(&server)
        .await;
    let lcd = LcdClient::new(vec![server.uri()], 1000, 1000);
    pair_discovery::sync_all_pairs(&pool, &lcd, "terra1factory")
        .await
        .unwrap();
    assert!(state::get_state(&pool, "factory_inventory_observed_at")
        .await
        .unwrap()
        .is_none());
    server.reset().await;
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
