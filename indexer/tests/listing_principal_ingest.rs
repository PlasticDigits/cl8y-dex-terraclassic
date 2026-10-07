//! Historical authenticated mapper principal, replay and reserve recovery (#1437).
mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::{
    indexer::listing_principal,
    lcd::{LcdClient, TxResponse},
};
use serde_json::json;
use serial_test::serial;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn principal_tx(hash: &str, action: &str) -> TxResponse {
    let principal_field = if action == "unwrap" {
        "withdraw_amount"
    } else {
        "mint_amount"
    };
    serde_json::from_value(json!({
        "height":"10", "txhash":hash,
        "events":[
            {"type":"message","attributes":[{"key":"sender","value":"chain_sender"}]},
            {"type":"wasm","attributes":[
                {"key":"_contract_address","value":"pinned"},
                {"key":"action","value":action},
                {"key":"denom","value":"uluna"},
                {"key":"gross_amount","value":"5010000"},
                {"key":"fee","value":"10000"},
                {"key":principal_field,"value":"5000000"}
            ]}
        ]
    }))
    .unwrap()
}

async fn bank(server: &MockServer, height: &str, amount: &str, confirmed: bool) {
    let mut response = ResponseTemplate::new(200)
        .set_body_json(json!({"balance":{"denom":"uluna","amount":amount}}));
    if confirmed {
        response = response.insert_header("x-cosmos-block-height", height);
    }
    Mock::given(method("GET"))
        .and(path("/cosmos/bank/v1beta1/balances/pinned/by_denom"))
        .and(header("x-cosmos-block-height", height))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn client(server: &MockServer) -> LcdClient {
    Mock::given(method("GET"))
        .and(path("/cosmos/base/tendermint/v1beta1/node_info"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"columbus-5"}})),
        )
        .mount(server)
        .await;
    LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5")
}

#[tokio::test]
#[serial]
async fn postevent_custody_replay_and_cleanup_leave_economic_tables_untouched() {
    let pool = common::setup_pool().await;
    let server = MockServer::start().await;
    let lcd = client(&server).await;
    bank(&server, "9", "100000000", true).await;
    bank(&server, "10", "99990000", true).await;
    let txs = [
        principal_tx("WRAP_REAL_TX", "notify_deposit"),
        principal_tx("UNWRAP_REAL_TX", "unwrap"),
    ];
    let now = Utc::now();
    for _ in 0..2 {
        listing_principal::ingest_block(
            &pool,
            &lcd,
            "pinned",
            &txs,
            10,
            now,
            "CANONICAL_BLOCK_HASH",
        )
        .await
        .unwrap();
    }
    let rows = cl8y_dex_indexer::db::queries::listing_principal::get_principal_events(
        &pool,
        "pinned",
        Some(10),
        Some(10),
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 2, "unchanged source replay must be idempotent");
    assert_eq!(rows[0].principal_raw, BigDecimal::from(5_000_000));
    assert_eq!(rows[0].custody_raw, BigDecimal::from(105_000_000));
    assert_eq!(rows[1].custody_raw, BigDecimal::from(99_990_000));
    assert_eq!(rows[0].trader_address, "chain_sender");
    assert_eq!(rows[1].txn_index, 1);
    assert_eq!(rows[0].block_hash, "CANONICAL_BLOCK_HASH");
    for table in [
        "pairs",
        "swap_events",
        "protocol_fee_events",
        "pair_liquidity_usd",
        "trader_positions",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "principal projection cannot populate {table}");
    }
    sqlx::query("DELETE FROM listing_wrap_principal WHERE block_height >= 10")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM listing_mapper_custody WHERE block_height >= 10")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        cl8y_dex_indexer::db::queries::listing_principal::latest_custody(&pool, "pinned", "uluna")
            .await
            .unwrap()
            .is_none()
    );
    listing_principal::ingest_block(&pool, &lcd, "pinned", &txs, 10, now, "REPLAYED_BLOCK_HASH")
        .await
        .unwrap();
    let hashes: Vec<String> =
        sqlx::query_scalar("SELECT block_hash FROM listing_wrap_principal ORDER BY txn_index")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(hashes, vec!["REPLAYED_BLOCK_HASH", "REPLAYED_BLOCK_HASH"]);
}

#[tokio::test]
#[serial]
async fn unavailable_or_inconsistent_bank_history_cannot_publish_principal() {
    let pool = common::setup_pool().await;
    sqlx::query("TRUNCATE listing_wrap_principal,listing_mapper_custody")
        .execute(&pool)
        .await
        .unwrap();
    for (final_amount, confirmed) in [("105000000", false), ("2000000", true), ("105000001", true)]
    {
        let server = MockServer::start().await;
        let lcd = client(&server).await;
        bank(&server, "9", "100000000", true).await;
        bank(&server, "10", final_amount, confirmed).await;
        listing_principal::ingest_block(
            &pool,
            &lcd,
            "pinned",
            &[principal_tx("REAL_TX", "notify_deposit")],
            10,
            Utc::now(),
            "HASH",
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM listing_wrap_principal")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 0,
            "unconfirmed, negative initial or unrelated custody movement must omit the projection"
        );
    }
    let server = MockServer::start().await;
    let lcd = client(&server).await;
    // Even coincidentally matching custody cannot authorize an unsuccessful tx.
    bank(&server, "9", "100000000", true).await;
    bank(&server, "10", "105000000", true).await;
    let mut failed = principal_tx("FAILED_TX", "notify_deposit");
    failed.code = 5;
    listing_principal::ingest_block(&pool, &lcd, "pinned", &[failed], 10, Utc::now(), "HASH")
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM listing_wrap_principal")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "failed chain tx cannot become a listing swap");
}
