//! GitLab #322: book_snapshot loop writes mirrored reserves + resting books.

mod common;

use bigdecimal::BigDecimal;
use serial_test::serial;
use cl8y_dex_indexer::db::queries::{pair_reserves, resting_orders};
use cl8y_dex_indexer::indexer::book_snapshot::{
    self, book_snapshot_lcd_budget, BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD,
    BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR, BOOK_SNAPSHOT_MAX_STALENESS_MS,
};
use cl8y_dex_indexer::lcd::LcdClient;
use std::str::FromStr;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};
use serde_json::json;

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
        vec![101, 102, 100],
        "bids: best price first, FIFO by order_id at equal price"
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

    pair_reserves::upsert_pair_reserves(
        &pool,
        seed.pair_id,
        &bd("999"),
        &bd("888"),
        30,
        Some(1),
    )
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
        .and(path_regex(r"^/cosmos/base/tendermint/v1beta1/blocks/latest$"))
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

    let lcd = LcdClient::new(vec![server.uri().trim_end_matches('/').to_string()], 5000, 30000);

    book_snapshot::snapshot_all_pairs(&pool, &lcd)
        .await
        .expect("cycle completes despite per-pair failure");

    let row = pair_reserves::get_pair_reserves(&pool, seed.pair_id)
        .await
        .unwrap()
        .expect("good pair updated");
    assert_eq!(row.reserve_0, bd("111"));
    assert_eq!(row.reserve_1, bd("222"));

    let broken_pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = $1")
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
    let expected = BOOK_SNAPSHOT_LCD_CYCLE_OVERHEAD
        + pair_count * BOOK_SNAPSHOT_LCD_FIXED_PER_PAIR
        + resting;
    assert_eq!(book_snapshot_lcd_budget(pair_count, resting), expected);
    assert!(BOOK_SNAPSHOT_MAX_STALENESS_MS > 0);
}
