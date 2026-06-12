//! Integration tests for indexer ingestion hardening (GitLab #236, #362): cursor-on-error,
//! tx pagination, reorg hash guard, replay dedup, recovery rewind.

mod common;

use cl8y_dex_indexer::config::Config;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::{state, swap_events};
use std::str::FromStr;
use cl8y_dex_indexer::indexer::block_indexer;
use cl8y_dex_indexer::indexer::oracle;
use cl8y_dex_indexer::lcd::LcdClient;
use common::{clean_db, setup_pool, test_config};
use serde_json::json;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn block_json(height: i64, hash: &str) -> serde_json::Value {
    json!({
        "block_id": { "hash": hash },
        "block": {
            "header": {
                "height": height.to_string(),
                "time": "2024-01-01T00:00:00Z"
            }
        }
    })
}

fn tx_json(height: i64, idx: u32) -> serde_json::Value {
    json!({
        "height": height.to_string(),
        "txhash": format!("tx{}_{}", height, idx),
        "timestamp": "2024-01-01T00:00:00Z",
        "logs": [],
        "events": []
    })
}

async fn mount_latest_block(_server: &MockServer, _height: i64, _hash: &str) {
    // Reserved for full poller e2e tests.
}

async fn mount_empty_block_txs(server: &MockServer, height: i64) {
    mount_block_txs_json(
        server,
        height,
        json!({ "tx_responses": [], "pagination": { "total": "0" } }),
    )
    .await;
}

async fn mount_block_at_height(server: &MockServer, height: i64, hash: &str) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{}",
            height
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(block_json(height, hash)))
        .mount(server)
        .await;
}

async fn mount_block_txs_json(server: &MockServer, height: i64, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path("/cosmos/tx/v1beta1/txs"))
        .and(query_param("query", format!("tx.height={}", height)))
        .and(query_param("page", "1"))
        .and(query_param("limit", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn seed_minimal_pair(pool: &sqlx::PgPool, pair_address: &str) {
    let asset_0: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
         VALUES ('uluna', false, 'Luna', 'LUNC', 6)
         RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("asset_0");

    let asset_1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1seedustc236', true, 'USTC', 'USTC', 6)
         RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("asset_1");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lp236', 30)",
    )
    .bind(pair_address)
    .bind(asset_0)
    .bind(asset_1)
    .execute(pool)
    .await
    .expect("pair");
}

fn lcd_client(server: &MockServer) -> LcdClient {
    LcdClient::new(
        vec![server.uri().trim_end_matches('/').to_string()],
        5000,
        30000,
    )
}

fn fast_retry_config(base: Config) -> Config {
    Config {
        block_process_max_retries: 2,
        block_process_retry_backoff_ms: 1,
        ..base
    }
}

#[tokio::test]
async fn cursor_does_not_advance_on_parser_failure() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 500i64;
    let hash = "ABC500HASH";
    let pair_address = "terra1pair236test";

    seed_minimal_pair(&pool, pair_address).await;
    mount_block_at_height(&server, height, hash).await;

    let unknown_cw20 = "terra1unknowncw20token236xxxxxxxxxxxxxxxxxxx";
    mount_block_txs_json(
        &server,
        height,
        json!({
            "tx_responses": [{
                "height": height.to_string(),
                "txhash": "badtx",
                "timestamp": "2024-01-01T00:00:00Z",
                "logs": [{ "events": [{ "type": "wasm", "attributes": [
                    { "key": "_contract_address", "value": pair_address },
                    { "key": "action", "value": "swap" },
                    { "key": "sender", "value": "terra1trader" },
                    { "key": "offer_amount", "value": "100" },
                    { "key": "return_amount", "value": "95" },
                    { "key": "offer_asset", "value": unknown_cw20 },
                    { "key": "ask_asset", "value": "uluna" }
                ]}]}]
            }],
            "pagination": { "total": "1" }
        }),
    )
    .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(502).set_body_string("LCD unavailable"))
        .mount(&server)
        .await;

    state::set_indexer_checkpoint(&pool, height - 1, "PREVHASH")
        .await
        .expect("seed checkpoint");

    let lcd = lcd_client(&server);
    let config = fast_retry_config(test_config());
    let price = oracle::new_shared_price();
    let err = block_indexer::index_block_with_retries(&pool, &lcd, &config, height, &price)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("failed after"),
        "expected max retries halt, got: {err}"
    );

    let cursor = state::get_last_indexed_height(&pool).await.unwrap();
    assert_eq!(cursor, height - 1, "cursor must not advance on failure");

    let failed: i64 = sqlx::query_scalar(
        "SELECT height FROM indexer_failed_blocks WHERE height = $1",
    )
    .bind(height)
    .fetch_one(&pool)
    .await
    .expect("failed_blocks row");
    assert_eq!(failed, height);
}

#[tokio::test]
async fn cursor_advances_on_empty_block_success() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 601i64;
    let hash = "HASH601";

    mount_block_at_height(&server, height, hash).await;
    mount_empty_block_txs(&server, height).await;

    let lcd = lcd_client(&server);
    let config = test_config();

    let price = oracle::new_shared_price();
    block_indexer::index_block(&pool, &lcd, &config, height, &price)
        .await
        .expect("index empty block");

    assert_eq!(
        state::get_last_indexed_height(&pool).await.unwrap(),
        height
    );
    assert_eq!(
        state::get_last_indexed_block_hash(&pool)
            .await
            .unwrap()
            .as_deref(),
        Some(hash)
    );
}

#[tokio::test]
async fn reorg_detection_halts_on_hash_mismatch() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 700i64;
    let stored_hash = "ORIGINAL_HASH";
    let canonical_hash = "REORG_HASH";

    state::set_indexer_checkpoint(&pool, height, stored_hash)
        .await
        .expect("seed checkpoint");

    mount_block_at_height(&server, height, canonical_hash).await;

    let lcd = lcd_client(&server);
    let err = block_indexer::verify_checkpoint_unchanged(&lcd, &pool, height)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("reorg"), "got: {err}");
}

#[tokio::test]
async fn swap_replay_is_idempotent_on_conflict() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    seed_minimal_pair(&pool, "terra1pair362").await;

    let pair_row: (i32, i32, i32) = sqlx::query_as(
        "SELECT id, asset_0_id, asset_1_id FROM pairs WHERE contract_address = $1",
    )
    .bind("terra1pair362")
    .fetch_one(&pool)
    .await
    .expect("pair");

    let (pair_id, asset_0_id, asset_1_id) = pair_row;
    let amt = BigDecimal::from_str("1000").unwrap();
    let price = BigDecimal::from_str("1").unwrap();
    let tx_hash = "TX362_REPLAY_DEDUP";

    let first = swap_events::insert_swap(
        &pool,
        pair_id,
        0,
        500,
        Utc::now(),
        tx_hash,
        "terra1taker",
        None,
        asset_0_id,
        asset_1_id,
        &amt,
        &amt,
        None,
        None,
        None,
        &price,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap()
    .expect("first insert");

    let second = swap_events::insert_swap(
        &pool,
        pair_id,
        0,
        500,
        Utc::now(),
        tx_hash,
        "terra1taker",
        None,
        asset_0_id,
        asset_1_id,
        &amt,
        &amt,
        None,
        None,
        None,
        &price,
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(second, None, "replay must not insert duplicate swap row");

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM swap_events WHERE tx_hash = $1")
        .bind(tx_hash)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 1);
    assert!(first > 0);
}

#[tokio::test]
async fn reorg_recovery_rewind_allows_catch_up() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 900i64;
    let old_hash = "OLD900";
    let new_hash = "NEW900";

    state::set_indexer_checkpoint(&pool, height, old_hash)
        .await
        .expect("seed checkpoint");

    mount_block_at_height(&server, height, new_hash).await;
    let lcd = lcd_client(&server);
    assert!(
        block_indexer::verify_checkpoint_unchanged(&lcd, &pool, height)
            .await
            .is_err(),
        "mismatch must halt before catch-up"
    );

    // Mirrors scripts/indexer-reorg-recover.sh --height 900 --apply
    state::set_last_indexed_height(&pool, height - 1)
        .await
        .expect("rewind cursor");
    state::set_state(&pool, state::KEY_LAST_INDEXED_BLOCK_HASH, "")
        .await
        .expect("clear hash");

    block_indexer::verify_checkpoint_unchanged(&lcd, &pool, height - 1)
        .await
        .expect("legacy cursor without hash skips reorg guard");

    mount_empty_block_txs(&server, height).await;
    let price = oracle::new_shared_price();
    block_indexer::index_block(&pool, &lcd, &test_config(), height, &price)
        .await
        .expect("re-index after recovery");

    assert_eq!(
        state::get_last_indexed_height(&pool).await.unwrap(),
        height
    );
    assert_eq!(
        state::get_last_indexed_block_hash(&pool)
            .await
            .unwrap()
            .as_deref(),
        Some(new_hash)
    );
}

#[tokio::test]
async fn multi_page_block_txs_ingested_count_matches_lcd_total() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 800i64;
    let hash = "HASH800";

    mount_block_at_height(&server, height, hash).await;

    let page1: Vec<_> = (0..100).map(|i| tx_json(height, i)).collect();
    let page2: Vec<_> = (100..155).map(|i| tx_json(height, i)).collect();

    Mock::given(method("GET"))
        .and(path("/cosmos/tx/v1beta1/txs"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tx_responses": page1,
            "total": "155"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cosmos/tx/v1beta1/txs"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tx_responses": page2,
            "total": "155"
        })))
        .mount(&server)
        .await;

    let lcd = lcd_client(&server);
    let result = lcd.get_block_txs(height, 100, 10).await.expect("paginate");
    assert_eq!(result.txs.len(), 155);
    assert_eq!(result.page_count, 2);
}

#[tokio::test]
async fn missing_tx_timestamp_uses_block_header_time() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 900i64;
    let hash = "HASH900";

    mount_block_at_height(&server, height, hash).await;

    let mut tx = tx_json(height, 0);
    tx.as_object_mut().unwrap().remove("timestamp");

    mount_block_txs_json(
        &server,
        height,
        json!({ "tx_responses": [tx], "pagination": { "total": "1" } }),
    )
    .await;

    let lcd = lcd_client(&server);
    let config = test_config();
    let price = oracle::new_shared_price();

    block_indexer::index_block(&pool, &lcd, &config, height, &price)
        .await
        .expect("index block with header fallback");

    assert_eq!(
        state::get_last_indexed_height(&pool).await.unwrap(),
        height
    );
}

#[tokio::test]
async fn invalid_tx_and_header_timestamp_fails_block() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 901i64;
    let hash = "HASH901";

    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{}",
            height
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block_id": { "hash": hash },
            "block": {
                "header": {
                    "height": height.to_string(),
                    "time": "not-a-valid-time"
                }
            }
        })))
        .mount(&server)
        .await;

    let mut tx = tx_json(height, 0);
    tx.as_object_mut().unwrap().remove("timestamp");

    mount_block_txs_json(
        &server,
        height,
        json!({ "tx_responses": [tx], "pagination": { "total": "1" } }),
    )
    .await;

    let lcd = lcd_client(&server);
    let config = test_config();
    let price = oracle::new_shared_price();

    let cursor_before = state::get_last_indexed_height(&pool).await.unwrap();

    let err = block_indexer::index_block(&pool, &lcd, &config, height, &price)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("no usable chain timestamp"),
        "got: {err}"
    );

    assert_eq!(
        state::get_last_indexed_height(&pool).await.unwrap(),
        cursor_before,
        "cursor must not advance when chain time is unavailable"
    );
}

#[tokio::test]
async fn swap_replay_does_not_duplicate_rows() {
    use bigdecimal::BigDecimal;
    use chrono::Utc;
    use cl8y_dex_indexer::db::queries::swap_events;

    let pool = setup_pool().await;
    clean_db(&pool).await;
    seed_minimal_pair(&pool, "terra1pairreplay362").await;

    let pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("pair");

    let offer_asset: i32 = sqlx::query_scalar("SELECT asset_0_id FROM pairs WHERE id = $1")
        .bind(pair_id)
        .fetch_one(&pool)
        .await
        .expect("offer");
    let ask_asset: i32 = sqlx::query_scalar("SELECT asset_1_id FROM pairs WHERE id = $1")
        .bind(pair_id)
        .fetch_one(&pool)
        .await
        .expect("ask");

    let height = 900i64;
    let ts = Utc::now();
    let amount = BigDecimal::from(1_000_000i64);
    let price = BigDecimal::from(1i64);

    for _ in 0..2 {
        swap_events::insert_swap(
            &pool,
            pair_id,
            0,
            height,
            ts,
            "replay_tx_362",
            "terra1sender",
            None,
            offer_asset,
            ask_asset,
            &amount,
            &amount,
            None,
            None,
            None,
            &price,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("insert");
    }

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM swap_events WHERE tx_hash = 'replay_tx_362' AND pair_id = $1",
    )
    .bind(pair_id)
    .fetch_one(&pool)
    .await
    .expect("count");

    assert_eq!(count, 1, "ON CONFLICT DO NOTHING must dedupe replayed swaps");
}
