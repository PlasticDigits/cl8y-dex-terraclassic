//! GitLab #142 — parked-expired and refunded lifecycle on `limit_order_placements` + HTTP.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::indexer::oracle;
use cl8y_dex_indexer::indexer::parser::process_block_txs;
use cl8y_dex_indexer::lcd::{Attribute, Event, LcdClient, TxLog, TxResponse};
use serde_json::Value;
use serial_test::serial;

fn wasm_park_tx(pair: &str, order_id: &str, remaining: &str, txhash: &str) -> TxResponse {
    TxResponse {
        height: "5001".into(),
        txhash: txhash.into(),
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: vec![
                    Attribute {
                        // GitLab #285: lifecycle events are scoped by the runtime-reserved
                        // `_contract_address` key only (wasm_contract_addr), so fixtures must use
                        // it — the unreserved `contract_address` is no longer matched.
                        key: "_contract_address".into(),
                        value: pair.into(),
                    },
                    Attribute {
                        key: "action".into(),
                        value: "limit_order_expired_parked".into(),
                    },
                    Attribute {
                        key: "order_id".into(),
                        value: order_id.into(),
                    },
                    Attribute {
                        key: "maker".into(),
                        value: "terra1maker".into(),
                    },
                    Attribute {
                        key: "side".into(),
                        value: "bid".into(),
                    },
                    Attribute {
                        key: "remaining".into(),
                        value: remaining.into(),
                    },
                ],
            }],
        }]),
        timestamp: None,
        events: None,
    }
}

fn wasm_claim_tx(pair: &str, order_id: &str, txhash: &str) -> TxResponse {
    TxResponse {
        height: "5002".into(),
        txhash: txhash.into(),
        logs: Some(vec![TxLog {
            events: vec![Event {
                event_type: "wasm".into(),
                attributes: vec![
                    Attribute {
                        // GitLab #285: lifecycle events are scoped by the runtime-reserved
                        // `_contract_address` key only (wasm_contract_addr), so fixtures must use
                        // it — the unreserved `contract_address` is no longer matched.
                        key: "_contract_address".into(),
                        value: pair.into(),
                    },
                    Attribute {
                        key: "action".into(),
                        value: "claim_expired_limit_order".into(),
                    },
                    Attribute {
                        key: "order_id".into(),
                        value: order_id.into(),
                    },
                    Attribute {
                        key: "owner".into(),
                        value: "terra1maker".into(),
                    },
                ],
            }],
        }]),
        timestamp: None,
        events: None,
    }
}

#[serial]
#[tokio::test]
async fn park_event_then_claim_updates_db_and_api_filters() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner, side, price)
         VALUES ($1, 5000, NOW(), 'tx_place900', 900, $2, 'bid', 2.0)",
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .execute(&pool)
    .await
    .expect("insert placement 900");

    let config: Config = common::test_config();
    let lcd = LcdClient::new(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
    );
    let ustc = oracle::new_shared_price();
    let block_time = Utc::now();

    process_block_txs(
        &pool,
        &lcd,
        &config,
        &[wasm_park_tx(
            &seed.pair_address,
            "900",
            "123456789",
            "tx_park900",
        )],
        5001,
        block_time,
        &ustc,
    )
    .await
    .expect("park ingest");

    let status: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM limit_order_placements WHERE pair_id = $1 AND order_id = 900",
    )
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status.as_str(), "parked_expired");

    let rem: BigDecimal = sqlx::query_scalar(
        "SELECT remaining_escrow FROM limit_order_placements WHERE pair_id = $1 AND order_id = 900",
    )
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rem.normalized().to_string(), "123456789");

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    let parked: Vec<&Value> = body
        .iter()
        .filter(|r| {
            r["order_id"].as_i64() == Some(900)
                && r["lifecycle_status"].as_str() == Some("parked_expired")
        })
        .collect();
    assert_eq!(parked.len(), 1);
    assert_eq!(parked[0]["remaining_escrow"], "123456789");

    process_block_txs(
        &pool,
        &lcd,
        &config,
        &[wasm_claim_tx(&seed.pair_address, "900", "tx_claim900")],
        5002,
        block_time,
        &ustc,
    )
    .await
    .expect("claim ingest");

    let status2: String = sqlx::query_scalar(
        "SELECT lifecycle_status FROM limit_order_placements WHERE pair_id = $1 AND order_id = 900",
    )
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status2.as_str(), "refunded");

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(
        !body.iter().any(|r| r["order_id"].as_i64() == Some(900)),
        "default feed excludes refunded"
    );

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements?status=refunded",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let rf: Vec<Value> = resp.json();
    assert!(rf.iter().any(|r| {
        r["order_id"].as_i64() == Some(900) && r["lifecycle_status"].as_str() == Some("refunded")
    }));
}
