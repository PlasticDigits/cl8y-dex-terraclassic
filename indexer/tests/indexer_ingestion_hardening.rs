//! Integration tests for indexer ingestion hardening (GitLab #236, #362): cursor-on-error,
//! tx pagination, reorg hash guard, replay dedup, recovery rewind.

mod common;

#[path = "scenarios/native_reference_publication_1505.rs"]
mod native_reference_publication_1505;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::db::queries::{state, swap_events};
use cl8y_dex_indexer::indexer::block_indexer;
use cl8y_dex_indexer::indexer::oracle;
use cl8y_dex_indexer::lcd::LcdClient;
use common::{clean_db, setup_pool, test_config};
use serde_json::json;
use std::str::FromStr;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn block_json(height: i64, hash: &str) -> serde_json::Value {
    json!({
        "block_id": { "hash": hash },
        "block": {
            "header": { "chain_id": "columbus-5",
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
        "columbus-5",
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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

    let failed: i64 =
        sqlx::query_scalar("SELECT height FROM indexer_failed_blocks WHERE height = $1")
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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

    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), height);
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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
async fn stale_poller_height_is_resync_not_reorg() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let lcd = lcd_client(&server);

    state::set_indexer_checkpoint(&pool, 800, "HASH800")
        .await
        .expect("seed H");
    state::set_indexer_checkpoint(&pool, 801, "HASH801")
        .await
        .expect("other writer advanced to H+1");

    let result = block_indexer::verify_checkpoint_unchanged(&lcd, &pool, 800)
        .await
        .expect("overlapping poller's stale height is not a reorg");
    assert_eq!(
        result,
        block_indexer::CheckpointVerify::Resync { db_height: 801 }
    );
}

#[tokio::test]
async fn poller_advisory_lock_is_exclusive() {
    use sqlx::Connection;

    let _pool = setup_pool().await;
    let url = test_config().database_url;

    let mut first = sqlx::PgConnection::connect(&url).await.expect("first conn");
    let mut second = sqlx::PgConnection::connect(&url)
        .await
        .expect("second conn");

    assert!(
        state::try_acquire_poller_lock(&mut first)
            .await
            .expect("first lock"),
        "first session must take the poller lock"
    );
    assert!(
        !state::try_acquire_poller_lock(&mut second)
            .await
            .expect("second try"),
        "second session must not take a held poller lock"
    );

    first.close().await.expect("close holding session");
    // A graceful client close does not acknowledge PostgreSQL backend teardown.
    // Wait for that independent session to observe the close, retaining a hard
    // bound so a leaked lock still fails rather than becoming a skipped check.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state::try_acquire_poller_lock(&mut second)
                .await
                .expect("lock after close")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("lock must release when the holding session closes");
}

#[tokio::test]
async fn swap_replay_is_idempotent_on_conflict() {
    let pool = setup_pool().await;
    clean_db(&pool).await;
    seed_minimal_pair(&pool, "terra1pair362").await;

    let pair_row: (i32, i32, i32) =
        sqlx::query_as("SELECT id, asset_0_id, asset_1_id FROM pairs WHERE contract_address = $1")
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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

    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), height);
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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

    assert_eq!(state::get_last_indexed_height(&pool).await.unwrap(), height);
}

#[tokio::test]
async fn invalid_tx_and_header_timestamp_fails_block() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
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
                "header": { "chain_id": "columbus-5",
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

    assert_eq!(
        count, 1,
        "ON CONFLICT DO NOTHING must dedupe replayed swaps"
    );
}

/// #1464: repeated upstream outages are outside the fatal accounting retry budget.
#[tokio::test]
async fn recovery1464_block_outage_resumes_same_checkpoint_without_failed_accounting() {
    use cl8y_dex_indexer::indexer::lifecycle::{IndexerHealth, IndexerStatus};
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio_util::sync::CancellationToken;

    let pool = setup_pool().await;
    clean_db(&pool).await;
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let height = 1464;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    state::record_failed_block(&pool, height, "legacy generic LCD provider outage")
        .await
        .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let reads = attempts.clone();
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            if reads.fetch_add(1, Ordering::SeqCst) < 3 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_json(block_json(height, "RECOVERED"))
            }
        })
        .mount(&server)
        .await;
    mount_empty_block_txs(&server, height).await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        100,
        1,
        "columbus-5",
        ProviderPolicy {
            max_attempts: 1,
            max_backoff_ms: 1,
            ..Default::default()
        },
    );
    let config = Config {
        block_process_max_retries: 1,
        poll_interval_ms: 1,
        ..test_config()
    };
    let health = IndexerHealth::default();
    health.progress(height - 1, height);
    let price = oracle::new_shared_price();
    let cancel = CancellationToken::new();
    let meta = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        block_indexer::index_block_with_retries_cancel(
            &pool, &lcd, &config, height, &price, &cancel, &health,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(meta.tx_count, 0);
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (height, Some("RECOVERED".into()))
    );
    let failures: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM indexer_failed_blocks WHERE height=$1")
            .bind(height)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        failures, 0,
        "availability must not be counted as failed accounting"
    );
    assert_eq!(health.snapshot().status, IndexerStatus::ProviderUnavailable);
    assert_eq!(health.snapshot().checkpoint_height, height - 1);
    health.progress(height, height);
    assert!(health.snapshot().ready);
}

#[tokio::test]
async fn recovery1464_checkpoint_outage_retries_without_advancing_or_skipping() {
    checkpoint_rejection_recovers(429).await;
}

#[tokio::test]
async fn recovery1464_checkpoint_http400_recovers_without_stopping_price_producers() {
    checkpoint_rejection_recovers(400).await;
}

async fn checkpoint_rejection_recovers(status: u16) {
    use cl8y_dex_indexer::indexer::lifecycle::IndexerHealth;
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio_util::sync::CancellationToken;

    let pool = setup_pool().await;
    clean_db(&pool).await;
    let height = 1465;
    state::set_indexer_checkpoint(&pool, height, "SAME")
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let attempts = Arc::new(AtomicUsize::new(0));
    let reads = attempts.clone();
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            if reads.fetch_add(1, Ordering::SeqCst) < 3 {
                ResponseTemplate::new(status)
            } else {
                ResponseTemplate::new(200).set_body_json(block_json(height, "SAME"))
            }
        })
        .mount(&server)
        .await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        100,
        1,
        "columbus-5",
        ProviderPolicy {
            max_attempts: 1,
            max_backoff_ms: 1,
            ..Default::default()
        },
    );
    let health = IndexerHealth::default();
    health.progress(height, height + 1);
    let cancel = CancellationToken::new();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        block_indexer::retry_availability(
            &cancel,
            &health,
            Some(height + 1),
            std::time::Duration::from_millis(1),
            || block_indexer::verify_checkpoint_unchanged(&lcd, &pool, height),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result, block_indexer::CheckpointVerify::Unchanged);
    assert_eq!(attempts.load(Ordering::SeqCst), 4);
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (height, Some("SAME".into()))
    );
    assert!(
        !health.halt_token().is_cancelled(),
        "independent price producers stay alive"
    );
    let failures: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM indexer_failed_blocks")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        failures, 0,
        "a rejected read is not a failed economic write"
    );
    assert!(!health.snapshot().ready);
    assert_eq!(health.snapshot().pending_height, Some(height + 1));
}

#[tokio::test]
async fn recovery1464_malformed_block_halts_with_cursor_and_failure_record_intact() {
    use cl8y_dex_indexer::indexer::lifecycle::IndexerHealth;
    use tokio_util::sync::CancellationToken;

    let pool = setup_pool().await;
    clean_db(&pool).await;
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let height = 1466;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    mount_block_at_height(&server, height, "BLOCK").await;
    mount_block_txs_json(&server, height, json!({"tx_responses": "malformed"})).await;
    let lcd = lcd_client(&server);
    let config = fast_retry_config(test_config());
    let health = IndexerHealth::default();
    health.progress(height - 1, height);
    let error = block_indexer::index_block_with_retries_cancel(
        &pool,
        &lcd,
        &config,
        height,
        &oracle::new_shared_price(),
        &CancellationToken::new(),
        &health,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        block_indexer::BlockIndexError::MaxRetriesExceeded {
            height: 1466,
            attempts: 2
        }
    ));
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (height - 1, Some("PREVIOUS".into()))
    );
    let attempts: i32 =
        sqlx::query_scalar("SELECT retry_count FROM indexer_failed_blocks WHERE height=$1")
            .bind(height)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(attempts, 2);
    assert!(!health.snapshot().ready);
}

/// Unknown economic pairs must not become successful omissions during provider/identity loss.
#[tokio::test]
async fn recovery1464_discovery_provider_and_identity_failures_are_not_successful_skips() {
    use bech32::ToBase32;
    use cl8y_dex_indexer::indexer::pair_discovery;
    use cl8y_dex_indexer::lcd::LcdError;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let pair =
        bech32::encode("terra", vec![41u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let factory =
        bech32::encode("terra", vec![42u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    for wrong_chain in [false, true] {
        let server = MockServer::start().await;
        common::lcd_mock::mock_chain_identity(
            &server,
            if wrong_chain {
                "localterra"
            } else {
                "columbus-5"
            },
        )
        .await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let lcd = lcd_client(&server);
        let error = pair_discovery::get_or_discover_pair(&pool, &lcd, &factory, &pair)
            .await
            .expect_err("provider failure cannot be Ok(None)");
        assert!(
            if wrong_chain {
                matches!(
                    error.downcast_ref::<LcdError>(),
                    Some(LcdError::ChainIdentity(_))
                )
            } else {
                matches!(
                    error.downcast_ref::<LcdError>(),
                    Some(LcdError::AllEndpointsFailed(_))
                )
            },
            "unexpected error {error}"
        );
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pairs WHERE contract_address=$1")
            .bind(&pair)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
    }

    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let height = 1467;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    mount_block_at_height(&server, height, "BLOCK").await;
    mount_block_txs_json(&server, height, json!({
        "tx_responses": [{ "height": height.to_string(), "txhash": "discovery-outage", "timestamp": "2024-01-01T00:00:00Z",
            "logs": [{ "events": [{ "type": "wasm", "attributes": [
                {"key": "_contract_address", "value": pair}, {"key": "action", "value": "swap"},
                {"key": "sender", "value": "terra1synthetictrader"},
                {"key": "offer_amount", "value": "100"}, {"key": "return_amount", "value": "95"},
                {"key": "offer_asset", "value": "uluna"}, {"key": "ask_asset", "value": "uusd"}
            ]}]}]
        }], "pagination": {"total": "1"}
    })).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let config = Config {
        factory_address: factory,
        ..test_config()
    };
    let error = block_indexer::index_block(
        &pool,
        &lcd_client(&server),
        &config,
        height,
        &oracle::new_shared_price(),
    )
    .await
    .expect_err("economic block must remain pending");
    assert!(block_indexer::is_retryable_availability(&error));
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (height - 1, Some("PREVIOUS".into()))
    );
    let swaps: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM swap_events WHERE tx_hash='discovery-outage'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(swaps, 0);
}

/// Exercise the production poller boundary, including its worker ownership and advisory session.
#[tokio::test]
async fn recovery1464_poller_reorg_latches_unready_and_releases_leadership() {
    struct WebhookGuard(Option<std::ffi::OsString>);
    impl Drop for WebhookGuard {
        fn drop(&mut self) {
            // This fixture and its verifier run serially; never notify an inherited operator webhook.
            unsafe {
                match &self.0 {
                    Some(value) => std::env::set_var("REORG_ALERT_WEBHOOK_URL", value),
                    None => std::env::remove_var("REORG_ALERT_WEBHOOK_URL"),
                }
            }
        }
    }
    let _webhook_guard = WebhookGuard(std::env::var_os("REORG_ALERT_WEBHOOK_URL"));
    unsafe {
        std::env::remove_var("REORG_ALERT_WEBHOOK_URL");
    }
    use cl8y_dex_indexer::indexer::{
        fee_discount_registry_health::FeeDiscountRegistryHealth,
        lifecycle::{IndexerHealth, IndexerStatus},
        poller, venus_vfdusd,
    };
    use sqlx::Connection;
    use tokio_util::sync::CancellationToken;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    state::set_indexer_checkpoint(&pool, 40, "STORED")
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path("/cosmos/base/tendermint/v1beta1/blocks/latest"))
        .respond_with(ResponseTemplate::new(200).set_body_json(block_json(41, "HEAD")))
        .mount(&server)
        .await;
    mount_block_at_height(&server, 40, "REORG").await;
    let config = Config {
        factory_address: String::new(),
        fee_discount_address: None,
        wrap_mapper_address: None,
        poll_interval_ms: 10,
        ..test_config()
    };
    let health = IndexerHealth::default();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        poller::run_indexer_with_health(
            pool.clone(),
            lcd_client(&server),
            config.clone(),
            CancellationToken::new(),
            oracle::OraclePriceHandles::new(),
            venus_vfdusd::new_shared_venus(),
            FeeDiscountRegistryHealth::from_config(None),
            health.clone(),
        ),
    )
    .await
    .unwrap_or_else(|_| panic!("poller reorg deadline: {:?}", health.snapshot()));
    assert!(matches!(
        result
            .unwrap_err()
            .downcast_ref::<block_indexer::BlockIndexError>(),
        Some(block_indexer::BlockIndexError::ReorgDetected { height: 40, .. })
    ));
    assert_eq!(health.snapshot().status, IndexerStatus::Halted);
    assert_eq!(health.snapshot().reason, Some("reorg"));
    assert!(!health.snapshot().ready);
    assert!(health.halt_token().is_cancelled());
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        (40, Some("STORED".into()))
    );
    let mut next_writer = sqlx::PgConnection::connect(&config.database_url)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        while !state::try_acquire_poller_lock(&mut next_writer)
            .await
            .unwrap()
        {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("halt must release the dedicated poller session");
}

#[tokio::test]
async fn recovery1464_absent_foreign_contract_is_pair_local_without_negative_provenance() {
    use bech32::ToBase32;
    use cl8y_dex_indexer::indexer::pair_discovery;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let pair =
        bech32::encode("terra", vec![43u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let factory =
        bech32::encode("terra", vec![44u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(
            ResponseTemplate::new(404).set_body_json(
                json!({"code":5,"message":"no such contract: not found","details":[]}),
            ),
        )
        .mount(&server)
        .await;
    let lcd = lcd_client(&server);
    for _ in 0..2 {
        let result = pair_discovery::get_or_discover_pair(&pool, &lcd, &factory, &pair)
            .await
            .unwrap();
        assert!(
            result.is_none(),
            "a demonstrably absent foreign contract cannot halt economic ingestion"
        );
    }
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pairs WHERE contract_address=$1")
        .bind(&pair)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    assert_eq!(
        lcd.pair_discovery_counters().cached_not_pair
            + lcd.pair_discovery_counters().cached_unlisted,
        0,
        "absence alone is not factory/unlisted provenance"
    );
    assert!(lcd
        .provider_counters()
        .await
        .iter()
        .all(|counter| !counter.circuit_open));
}

/// Provider recovery must preserve complete economic projections, including a new trader.
#[tokio::test]
async fn recovery1464_new_trader_registry_outage_preserves_complete_once_accounting() {
    use bech32::ToBase32;
    use cl8y_dex_indexer::indexer::lifecycle::IndexerHealth;
    use cl8y_dex_indexer::lcd::ProviderPolicy;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio_util::sync::CancellationToken;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let pair = "terra1known1464pair";
    seed_minimal_pair(&pool, pair).await;
    let trader =
        bech32::encode("terra", vec![45u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let registry =
        bech32::encode("terra", vec![46u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let height = 1468;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    let hash_attempts = Arc::new(AtomicUsize::new(0));
    let attempted = hash_attempts.clone();
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(move |_: &wiremock::Request| {
            if attempted.fetch_add(1, Ordering::SeqCst) < 3 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_json(block_json(height, "RECOVERED"))
            }
        })
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(format!(
            "^/cosmwasm/wasm/v1/contract/{registry}/smart/"
        )))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    mount_block_txs_json(&server, height, json!({
        "tx_responses": [{"height":height.to_string(),"txhash":"new-trader-recovery-1464","timestamp":"2024-01-01T00:00:00Z",
            "logs":[{"events":[{"type":"wasm","attributes":[
                {"key":"_contract_address","value":pair},{"key":"action","value":"swap"},
                {"key":"sender","value":trader},{"key":"offer_amount","value":"100"},
                {"key":"return_amount","value":"95"},{"key":"spread_amount","value":"2"},
                {"key":"commission_amount","value":"3"},{"key":"offer_asset","value":"uluna"},
                {"key":"ask_asset","value":"terra1seedustc236"}
            ]}]}]
        }],"pagination":{"total":"1"}
    })).await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        100,
        1,
        "columbus-5",
        ProviderPolicy {
            max_attempts: 1,
            max_backoff_ms: 1,
            ..Default::default()
        },
    );
    let config = Config {
        fee_discount_address: Some(registry.clone()),
        block_process_max_retries: 1,
        poll_interval_ms: 1,
        ..test_config()
    };
    let price = oracle::new_shared_price();
    let cancel = CancellationToken::new();
    let health = IndexerHealth::default();
    health.progress(height - 1, height);
    let meta = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        block_indexer::index_block_with_retries_cancel(
            &pool, &lcd, &config, height, &price, &cancel, &health,
        ),
    )
    .await
    .expect("optional registry loss must not interrupt economic accounting")
    .unwrap();
    assert_eq!(meta.tx_count, 1);
    assert_eq!(hash_attempts.load(Ordering::SeqCst), 4);
    for replay in 0..=2 {
        let trader_row = cl8y_dex_indexer::db::queries::traders::get_trader(&pool, &trader)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trader_row.total_trades, 1);
        assert_eq!(trader_row.total_volume, BigDecimal::from(100));
        assert!(
            !trader_row.registered,
            "missing optional metadata stays conservative"
        );
        let (trade_count,net,cost): (i64,BigDecimal,BigDecimal) = sqlx::query_as(
            "SELECT trade_count::bigint,net_position_quote,total_cost_base FROM trader_positions WHERE trader_address=$1")
            .bind(&trader).fetch_one(&pool).await.unwrap();
        assert_eq!(trade_count, 1);
        assert_eq!(net, BigDecimal::from(95));
        assert_eq!(cost, BigDecimal::from(100));
        let (swap_count,fee_count): (i64,i64) = sqlx::query_as(
            "SELECT (SELECT COUNT(*) FROM swap_events WHERE tx_hash=$1), (SELECT COUNT(*) FROM protocol_fee_events WHERE tx_hash=$1)")
            .bind("new-trader-recovery-1464").fetch_one(&pool).await.unwrap();
        assert_eq!((swap_count, fee_count), (1, 1));
        assert_eq!(
            state::get_indexer_checkpoint(&pool).await.unwrap(),
            (height, Some("RECOVERED".into()))
        );
        if replay < 2 {
            block_indexer::index_block(&pool, &lcd, &config, height, &price)
                .await
                .unwrap();
        }
    }
    let requests = server.received_requests().await.unwrap();
    let registry_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.url.path().contains(&registry))
        .collect();
    assert_eq!(registry_requests.len(), 1,
        "only the bounded historical cohort lookup may query the registry; replay must not retry immutable unknown attribution");
    assert_eq!(
        registry_requests[0].headers.get("x-cosmos-block-height").unwrap(),
        (height - 1).to_string().as_str(),
        "fee evidence is pinned to H-1; optional current-registry hydration stays in independent reconciliation"
    );
    let (actor, cohort): (Option<String>, String) = sqlx::query_as(
        "SELECT a.actor,a.cohort FROM protocol_fee_attributions a JOIN protocol_fee_events e ON e.id=a.fee_event_id WHERE e.tx_hash=$1")
        .bind("new-trader-recovery-1464").fetch_one(&pool).await.unwrap();
    assert_eq!(actor.as_deref(), Some(trader.as_str()));
    assert_eq!(
        cohort, "unknown",
        "provider outage must never guess retail/MM"
    );
}

/// A failed projection after a persisted swap must not be hidden by the replay guard.
#[tokio::test]
async fn recovery1464_post_insert_accounting_failure_halts_before_automatic_replay() {
    use cl8y_dex_indexer::indexer::lifecycle::IndexerHealth;
    use tokio_util::sync::CancellationToken;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let pair = "terra1known1464sqlpair";
    seed_minimal_pair(&pool, pair).await;
    let height = 1469;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(block_json(height, "BLOCK")))
        .expect(1)
        .mount(&server)
        .await;
    mount_block_txs_json(&server, height, json!({
        "tx_responses":[{"height":height.to_string(),"txhash":"post-insert-failure-1464","timestamp":"2024-01-01T00:00:00Z",
            "logs":[{"events":[{"type":"wasm","attributes":[
                {"key":"_contract_address","value":pair},{"key":"action","value":"swap"},
                {"key":"sender","value":"terra1synthetic1464sqltrader"},
                {"key":"offer_amount","value":"100"},{"key":"return_amount","value":"95"},
                {"key":"commission_amount","value":"3"},{"key":"offer_asset","value":"uluna"},
                {"key":"ask_asset","value":"terra1seedustc236"}
            ]}]}]
        }],"pagination":{"total":"1"}
    })).await;
    // The dedicated QA role owns this process's disposable database. This
    // constraint never reaches a runtime/control/shared/production database.
    sqlx::query("ALTER TABLE trader_positions ADD CONSTRAINT qa_1464_reject_position CHECK(false)")
        .execute(&pool)
        .await
        .unwrap();
    let config = Config {
        block_process_max_retries: 5,
        block_process_retry_backoff_ms: 1,
        fee_discount_address: None,
        ..test_config()
    };
    let health = IndexerHealth::default();
    health.progress(height - 1, height);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        block_indexer::index_block_with_retries_cancel(
            &pool,
            &lcd_client(&server),
            &config,
            height,
            &oracle::new_shared_price(),
            &CancellationToken::new(),
            &health,
        ),
    )
    .await;
    let checkpoint = state::get_indexer_checkpoint(&pool).await.unwrap();
    let failed_count: Option<i32> =
        sqlx::query_scalar("SELECT retry_count FROM indexer_failed_blocks WHERE height=$1")
            .bind(height)
            .fetch_optional(&pool)
            .await
            .unwrap();
    let (swaps,fees,positions): (i64,i64,i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM swap_events WHERE tx_hash=$1),
         (SELECT COUNT(*) FROM protocol_fee_events WHERE tx_hash=$1),
         (SELECT COUNT(*) FROM trader_positions WHERE trader_address='terra1synthetic1464sqltrader')")
        .bind("post-insert-failure-1464").fetch_one(&pool).await.unwrap();
    // Remove the test fault before assertions, so even a failing regression does
    // not contaminate subsequent tests in this same disposable process database.
    sqlx::query("ALTER TABLE trader_positions DROP CONSTRAINT qa_1464_reject_position")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        matches!(
            result.unwrap(),
            Err(block_indexer::BlockIndexError::ProcessingFailed { height: 1469, .. })
        ),
        "partial accounting must remain an executed failure instead of successful replay"
    );
    assert_eq!(checkpoint, (height - 1, Some("PREVIOUS".into())));
    assert_eq!(
        failed_count,
        Some(1),
        "unsafe processing failures must halt after exactly one attempt"
    );
    assert_eq!(
        (swaps, fees, positions),
        (1, 1, 0),
        "prove the fault followed durable ledger writes but blocked its projection"
    );
    assert!(!health.snapshot().ready);
    assert_eq!(
        health.snapshot().status,
        cl8y_dex_indexer::indexer::lifecycle::IndexerStatus::Halted
    );
    assert_eq!(health.snapshot().reason, Some("block_integrity_failure"));
    let marked: String =
        sqlx::query_scalar("SELECT error_message FROM indexer_failed_blocks WHERE height=$1")
            .bind(height)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(marked.starts_with(block_indexer::UNSAFE_BLOCK_FAILURE_PREFIX));
    // Model a fresh process after the SQL fault disappeared. Its fresh health/
    // cancellation/client still cannot hide the unrepaired position by replay.
    for cancellable in [true, false] {
        let restarted = IndexerHealth::default();
        restarted.progress(height - 1, height);
        let resumed = if cancellable {
            block_indexer::index_block_with_retries_cancel(
                &pool,
                &lcd_client(&server),
                &config,
                height,
                &oracle::new_shared_price(),
                &CancellationToken::new(),
                &restarted,
            )
            .await
        } else {
            block_indexer::index_block_with_retries(
                &pool,
                &lcd_client(&server),
                &config,
                height,
                &oracle::new_shared_price(),
            )
            .await
        };
        assert!(matches!(
            resumed,
            Err(block_indexer::BlockIndexError::UnsafeFailurePending { height: 1469 })
        ));
        if cancellable {
            assert_eq!(
                restarted.snapshot().status,
                cl8y_dex_indexer::indexer::lifecycle::IndexerStatus::Halted
            );
        }
        assert_eq!(
            state::get_indexer_checkpoint(&pool).await.unwrap(),
            (height - 1, Some("PREVIOUS".into()))
        );
        let (failures,positions): (i32,i64) = sqlx::query_as(
            "SELECT retry_count,(SELECT COUNT(*) FROM trader_positions WHERE trader_address='terra1synthetic1464sqltrader')
             FROM indexer_failed_blocks WHERE height=$1")
            .bind(height).fetch_one(&pool).await.unwrap();
        assert_eq!(
            (failures, positions),
            (1, 0),
            "restart neither retries nor manufactures repair"
        );
    }
    let attempts = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().ends_with(&format!("/blocks/{height}")))
        .count();
    assert_eq!(
        attempts, 1,
        "fatal retry budget must not automatically replay partial economic projections"
    );
}

/// Required raw-fee metadata cannot turn an executed SQL failure into a successful omission.
#[tokio::test]
async fn recovery1464_post_insert_fee_asset_lookup_failure_remains_halted() {
    use cl8y_dex_indexer::indexer::lifecycle::{IndexerHealth, IndexerStatus};
    use tokio_util::sync::CancellationToken;
    let pool = setup_pool().await;
    clean_db(&pool).await;
    let pair = "terra1known1464feepair";
    seed_minimal_pair(&pool, pair).await;
    let height = 1470;
    state::set_indexer_checkpoint(&pool, height - 1, "PREVIOUS")
        .await
        .unwrap();
    let server = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{height}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(block_json(height, "BLOCK")))
        .expect(1)
        .mount(&server)
        .await;
    mount_block_txs_json(&server, height, json!({
        "tx_responses":[{"height":height.to_string(),"txhash":"fee-asset-failure-1464","timestamp":"2024-01-01T00:00:00Z",
            "logs":[{"events":[{"type":"wasm","attributes":[
                {"key":"_contract_address","value":pair},{"key":"action","value":"swap"},
                {"key":"sender","value":"terra1synthetic1464feetrader"},
                {"key":"offer_amount","value":"100"},{"key":"return_amount","value":"95"},
                {"key":"commission_amount","value":"3"},{"key":"offer_asset","value":"uluna"},
                {"key":"ask_asset","value":"terra1seedustc236"}
            ]}]}]
        }],"pagination":{"total":"1"}
    })).await;
    // Dedicated QA owns this disposable child DB. Rename preserves the asset
    // table OID/FKs, while the temporary view remains readable until swap insert.
    let mut fixture = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE assets RENAME TO fixture_1464_fee_assets")
        .execute(&mut *fixture)
        .await
        .unwrap();
    sqlx::query(
        r#"CREATE FUNCTION qa_1464_fee_lookup() RETURNS BOOLEAN LANGUAGE plpgsql VOLATILE AS $$
        BEGIN
            IF EXISTS(SELECT 1 FROM swap_events WHERE tx_hash='fee-asset-failure-1464') THEN
                RAISE EXCEPTION 'qa fee asset lookup fault';
            END IF;
            RETURN TRUE;
        END $$"#,
    )
    .execute(&mut *fixture)
    .await
    .unwrap();
    sqlx::query(
        "CREATE VIEW assets AS SELECT * FROM fixture_1464_fee_assets WHERE qa_1464_fee_lookup()",
    )
    .execute(&mut *fixture)
    .await
    .unwrap();
    fixture.commit().await.unwrap();
    let config = Config {
        block_process_max_retries: 5,
        block_process_retry_backoff_ms: 1,
        fee_discount_address: None,
        ..test_config()
    };
    let health = IndexerHealth::default();
    health.progress(height - 1, height);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        block_indexer::index_block_with_retries_cancel(
            &pool,
            &lcd_client(&server),
            &config,
            height,
            &oracle::new_shared_price(),
            &CancellationToken::new(),
            &health,
        ),
    )
    .await;
    let checkpoint = state::get_indexer_checkpoint(&pool).await.unwrap();
    let failure: Option<(i32, String)> = sqlx::query_as(
        "SELECT retry_count,error_message FROM indexer_failed_blocks WHERE height=$1",
    )
    .bind(height)
    .fetch_optional(&pool)
    .await
    .unwrap();
    let (swaps,fees,positions): (i64,i64,i64) = sqlx::query_as(
        "SELECT (SELECT COUNT(*) FROM swap_events WHERE tx_hash=$1),
         (SELECT COUNT(*) FROM protocol_fee_events WHERE tx_hash=$1),
         (SELECT COUNT(*) FROM trader_positions WHERE trader_address='terra1synthetic1464feetrader')")
        .bind("fee-asset-failure-1464").fetch_one(&pool).await.unwrap();
    // Restore every fixture object before assertions, even when the regression fails.
    let mut restore = pool.begin().await.unwrap();
    sqlx::query("DROP VIEW assets")
        .execute(&mut *restore)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE fixture_1464_fee_assets RENAME TO assets")
        .execute(&mut *restore)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION qa_1464_fee_lookup()")
        .execute(&mut *restore)
        .await
        .unwrap();
    restore.commit().await.unwrap();
    assert!(
        matches!(
            result.unwrap(),
            Err(block_indexer::BlockIndexError::ProcessingFailed { height: 1470, .. })
        ),
        "required raw fee lookup must propagate its SQL failure"
    );
    assert_eq!(checkpoint, (height - 1, Some("PREVIOUS".into())));
    assert_eq!(
        (swaps, fees, positions),
        (1, 0, 0),
        "prove failure occurred after swap insert but before fee/position writes"
    );
    let (attempts, message) = failure.expect("unsafe raw fee failure must persist");
    assert_eq!(attempts, 1);
    assert!(message.starts_with(block_indexer::UNSAFE_BLOCK_FAILURE_PREFIX));
    assert!(message.contains("qa fee asset lookup fault"));
    assert_eq!(health.snapshot().status, IndexerStatus::Halted);
    assert!(!health.snapshot().ready);
    let restarted = IndexerHealth::default();
    let result = block_indexer::index_block_with_retries_cancel(
        &pool,
        &lcd_client(&server),
        &config,
        height,
        &oracle::new_shared_price(),
        &CancellationToken::new(),
        &restarted,
    )
    .await;
    assert!(matches!(
        result,
        Err(block_indexer::BlockIndexError::UnsafeFailurePending { height: 1470 })
    ));
    assert_eq!(
        state::get_indexer_checkpoint(&pool).await.unwrap(),
        checkpoint
    );
    assert_eq!(restarted.snapshot().status, IndexerStatus::Halted);
    let reads = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().ends_with(&format!("/blocks/{height}")))
        .count();
    assert_eq!(
        reads, 1,
        "removing a fee lookup fault must not authorize replay of incomplete accounting"
    );
}
