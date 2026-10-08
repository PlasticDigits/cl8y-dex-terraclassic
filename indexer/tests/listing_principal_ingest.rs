//! Historical authenticated treasury custody, principal, replay and recovery (#1437).
mod common;

use base64::Engine;
use bech32::ToBase32;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::{
    db::queries::{listing_principal as db_principal, state},
    indexer::listing_principal,
    lcd::{LcdClient, TxResponse},
};
use serde_json::json;
use serial_test::serial;
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn address(seed: u8) -> String {
    bech32::encode("terra", [seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
}
fn mapper() -> String {
    address(1)
}
fn treasury() -> String {
    address(2)
}

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
                {"key":"_contract_address","value":mapper()},
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

async fn mapper_config(
    server: &MockServer,
    height: &str,
    custody_address: Option<&str>,
    confirmed: bool,
) {
    let data = match custody_address {
        Some(address) => json!({"treasury":address}),
        None => json!({}),
    };
    let mut response = ResponseTemplate::new(200).set_body_json(json!({"data":data}));
    if confirmed {
        response = response.insert_header("x-cosmos-block-height", height);
    }
    let query = base64::engine::general_purpose::STANDARD.encode(br#"{"config":{}}"#);
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmwasm/wasm/v1/contract/{}/smart/{query}",
            mapper()
        )))
        .and(header("x-cosmos-block-height", height))
        .respond_with(response)
        .mount(server)
        .await;
}

async fn stable_config(server: &MockServer) {
    for height in ["9", "10"] {
        mapper_config(server, height, Some(&treasury()), true).await;
    }
}

async fn bank(
    server: &MockServer,
    owner: &str,
    denom: &str,
    height: &str,
    amount: &str,
    confirmed: bool,
) {
    let mut response = ResponseTemplate::new(200)
        .set_body_json(json!({"balance":{"denom":denom,"amount":amount}}));
    if confirmed {
        response = response.insert_header("x-cosmos-block-height", height);
    }
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/bank/v1beta1/balances/{owner}/by_denom"
        )))
        .and(wiremock::matchers::query_param("denom", denom))
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
async fn treasury_postevent_custody_replay_and_cleanup_leave_economic_tables_untouched() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let server = MockServer::start().await;
    let lcd = client(&server).await;
    stable_config(&server).await;
    bank(&server, &treasury(), "uluna", "9", "100000000", true).await;
    // Treasury keeps the gross wrap deposit, then pays the principal unwrap.
    bank(&server, &treasury(), "uluna", "10", "100010000", true).await;
    bank(&server, &mapper(), "uluna", "10", "0", true).await;
    let txs = [
        principal_tx("WRAP_REAL_TX", "notify_deposit"),
        principal_tx("UNWRAP_REAL_TX", "unwrap"),
    ];
    let now = Utc::now();
    for _ in 0..2 {
        listing_principal::ingest_block(
            &pool,
            &lcd,
            &mapper(),
            &txs,
            10,
            now,
            "CANONICAL_BLOCK_HASH",
        )
        .await
        .unwrap();
    }
    let rows = db_principal::get_principal_events(&pool, &mapper(), Some(10), Some(10))
        .await
        .unwrap();
    assert_eq!(rows.len(), 2, "unchanged source replay must be idempotent");
    assert_eq!(rows[0].principal_raw, BigDecimal::from(5_000_000));
    assert_eq!(rows[0].custody_raw, BigDecimal::from(105_010_000));
    assert_eq!(rows[1].custody_raw, BigDecimal::from(100_010_000));
    assert_eq!(
        rows[0].custody_address.as_deref(),
        Some(treasury().as_str())
    );
    assert_eq!(rows[0].trader_address, "chain_sender");
    assert_eq!(rows[1].txn_index, 1);
    assert_eq!(rows[0].block_hash, "CANONICAL_BLOCK_HASH");
    assert_eq!(
        db_principal::latest_custody(&pool, &mapper(), "uluna")
            .await
            .unwrap()
            .unwrap()
            .custody_address,
        Some(treasury())
    );
    let requests = server.received_requests().await.unwrap();
    assert!(requests
        .iter()
        .any(|r| r.url.path().contains(&format!("/balances/{}/", treasury()))));
    assert!(
        !requests
            .iter()
            .any(|r| r.url.path().contains(&format!("/balances/{}/", mapper()))),
        "zero mapper bank is not the source of native custody"
    );
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
    sqlx::query("DELETE FROM listing_wrap_principal WHERE block_height>=10")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM listing_mapper_custody WHERE block_height>=10")
        .execute(&pool)
        .await
        .unwrap();
    assert!(db_principal::latest_custody(&pool, &mapper(), "uluna")
        .await
        .unwrap()
        .is_none());
    listing_principal::ingest_block(&pool, &lcd, &mapper(), &txs, 10, now, "REPLAYED_BLOCK_HASH")
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
async fn unavailable_or_inconsistent_treasury_history_cannot_publish_principal() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    for (final_amount, confirmed) in [("105010000", false), ("2000000", true), ("105010001", true)]
    {
        let server = MockServer::start().await;
        let lcd = client(&server).await;
        stable_config(&server).await;
        bank(&server, &treasury(), "uluna", "9", "100000000", true).await;
        bank(&server, &treasury(), "uluna", "10", final_amount, confirmed).await;
        listing_principal::ingest_block(
            &pool,
            &lcd,
            &mapper(),
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
        let latest = db_principal::latest_custody(&pool, &mapper(), "uluna")
            .await
            .unwrap()
            .unwrap();
        assert!(
            latest.custody_address.is_none(),
            "unavailable source cannot resurrect older positive backing"
        );
    }
    let server = MockServer::start().await;
    let lcd = client(&server).await;
    stable_config(&server).await;
    bank(&server, &treasury(), "uluna", "9", "100000000", true).await;
    bank(&server, &treasury(), "uluna", "10", "105010000", true).await;
    let mut failed = principal_tx("FAILED_TX", "notify_deposit");
    failed.code = 5;
    listing_principal::ingest_block(&pool, &lcd, &mapper(), &[failed], 10, Utc::now(), "HASH")
        .await
        .unwrap();
    assert!(
        db_principal::get_principal_events(&pool, &mapper(), None, None)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
#[serial]
async fn missing_malformed_rotated_or_unattested_mapper_config_fails_closed() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let previous = treasury();
    let rotated = address(3);
    let uppercase = treasury().to_ascii_uppercase();
    let current_cases = [
        None,
        Some("terra1notchecksummed"),
        Some("https://host/treasury"),
        Some(uppercase.as_str()),
        Some(rotated.as_str()),
        Some(previous.as_str()),
    ];
    for (index, current) in current_cases.into_iter().enumerate() {
        sqlx::query("TRUNCATE listing_wrap_principal,listing_mapper_custody")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO listing_mapper_custody(mapper_address,custody_address,native_denom,custody_raw,block_height,block_timestamp) VALUES($1,$2,'uluna',100000000,9,NOW())")
            .bind(mapper()).bind(&previous).execute(&pool).await.unwrap();
        let server = MockServer::start().await;
        let lcd = client(&server).await;
        mapper_config(&server, "9", Some(&previous), true).await;
        mapper_config(&server, "10", current, index != 5).await;
        bank(&server, &previous, "uluna", "9", "100000000", true).await;
        bank(&server, &previous, "uluna", "10", "105010000", true).await;
        listing_principal::ingest_block(
            &pool,
            &lcd,
            &mapper(),
            &[principal_tx("REAL_TX", "notify_deposit")],
            10,
            Utc::now(),
            "HASH",
        )
        .await
        .unwrap();
        assert!(
            db_principal::get_principal_events(&pool, &mapper(), None, None)
                .await
                .unwrap()
                .is_empty()
        );
        let latest = db_principal::latest_custody(&pool, &mapper(), "uluna")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.block_height, 10);
        assert!(latest.custody_address.is_none());
        assert!(
            !server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .any(|r| r.url.path().contains("/balances/")),
            "invalid identity cannot choose a bank source"
        );
    }
}

#[tokio::test]
#[serial]
async fn refresh_resolves_treasury_for_both_denoms_and_legacy_rows_remain_unproven() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let server = MockServer::start().await;
    let lcd = client(&server).await;
    stable_config(&server).await;
    state::set_indexer_checkpoint(&pool, 10, "HASH")
        .await
        .unwrap();
    Mock::given(method("GET")).and(path("/cosmos/base/tendermint/v1beta1/blocks/10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"block_id":{"hash":"HASH"},"block":{"header":{"chain_id":"columbus-5","height":"10","time":"2026-10-07T12:00:00Z"}}})))
        .mount(&server).await;
    bank(&server, &treasury(), "uluna", "10", "100000000", true).await;
    bank(&server, &treasury(), "uusd", "10", "200000000", true).await;
    sqlx::query("INSERT INTO listing_mapper_custody(mapper_address,native_denom,custody_raw,block_height,block_timestamp) VALUES($1,'uluna',999999999,9,NOW())")
        .bind(mapper()).execute(&pool).await.unwrap();
    listing_principal::refresh_custody(&pool, &lcd, &mapper())
        .await
        .unwrap();
    for (denom, amount) in [("uluna", 100000000), ("uusd", 200000000)] {
        let snapshot = db_principal::latest_custody(&pool, &mapper(), denom)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.custody_address, Some(treasury()));
        assert_eq!(snapshot.custody_raw, BigDecimal::from(amount));
    }
    let old_source: Option<String> = sqlx::query_scalar(
        "SELECT custody_address FROM listing_mapper_custody WHERE block_height=9",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        old_source, None,
        "migration must not invent a source for old data"
    );
    sqlx::query("INSERT INTO listing_wrap_principal(mapper_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,block_height,block_timestamp) VALUES($1,'uluna','wrap',1,999999999,'LEGACY','legacy_sender',9,NOW())")
        .bind(mapper()).execute(&pool).await.unwrap();
    assert!(
        db_principal::get_principal_events(&pool, &mapper(), None, None)
            .await
            .unwrap()
            .is_empty(),
        "legacy principal has no proven treasury source"
    );
}
