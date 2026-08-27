//! GitLab #683 — factory economic CW20 fee USD (CL8Y first) + NULL-only backfill.

mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::api::{reset_overview_cache, reset_protocol_fees_cache};
use cl8y_dex_indexer::config::{
    DEFAULT_HUB_CL8Y_ADDRESS, DEFAULT_HUB_CUSTC_ADDRESS, DEFAULT_HUB_UST1_ADDRESS,
};
use cl8y_dex_indexer::db::queries::hub_prices;
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::defillama::COLUMBUS5_GEM_ADDRESSES;
use cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig;
use cl8y_dex_indexer::db::queries::assets as asset_q;
use cl8y_dex_indexer::indexer::protocol_fees::{fee_usd_for_raw, FeeEventDraft, FeeSource};
use cl8y_dex_indexer::indexer::volume_aggregator;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

struct EconSeed {
    cl8y_id: i32,
    ust1_id: i32,
    gem_id: i32,
    cl8y_pair: i32,
}

async fn seed_cl8y_hub(pool: &sqlx::PgPool) -> EconSeed {
    common::clean_db(pool).await;

    let custc: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'cUSTC', 'cUSTC', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_CUSTC_ADDRESS)
    .fetch_one(pool)
    .await
    .unwrap();
    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_UST1_ADDRESS)
    .fetch_one(pool)
    .await
    .unwrap();
    let cl8y: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'CeramicLiberty.com', 'CL8Y-cb', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_CL8Y_ADDRESS)
    .fetch_one(pool)
    .await
    .unwrap();
    let gem: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Ember', 'EMBER', 6) RETURNING id",
    )
    .bind(COLUMBUS5_GEM_ADDRESSES[0])
    .fetch_one(pool)
    .await
    .unwrap();

    let ust1_pool: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1deep683', $1, $2, 'terra1lpdeep683', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(custc)
    .fetch_one(pool)
    .await
    .unwrap();
    let cl8y_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1cl8yust1683', $1, $2, 'terra1lpcl8y683', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(cl8y)
    .fetch_one(pool)
    .await
    .unwrap();
    let gem_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1gempire683', $1, $2, 'terra1lpgem683', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(gem)
    .fetch_one(pool)
    .await
    .unwrap();

    let now = Utc::now();
    // UST1/cUSTC: 250 UST1 + 50_000 cUSTC → usd(UST1)=$1, TVL $500
    // CL8Y/UST1: 10_000 CL8Y + 100 UST1 → usd(CL8Y)=$0.01, TVL $200
    // Gem/UST1: same size — gem still unpriced
    sqlx::query(
        "INSERT INTO pair_reserves (pair_id, reserve_0, reserve_1, fee_bps, snapshot_at)
         VALUES ($1, 250000000, 50000000000, 30, $4),
                ($2, 100000000, 10000000000, 30, $4),
                ($3, 100000000, 10000000000, 30, $4)",
    )
    .bind(ust1_pool)
    .bind(cl8y_pair)
    .bind(gem_pair)
    .bind(now)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ('ustc', 0.005, 'average', NOW())",
    )
    .execute(pool)
    .await
    .unwrap();

    let mut cfg = common::test_config();
    cfg.book_snapshot_interval_ms = 10_000;
    let hub_cfg = HubUsdConfig::from_indexer_config(&cfg);
    hub_prices::refresh_hub_prices(pool, &hub_cfg, Some(&bd("0.005")), None)
        .await
        .expect("hub+economic refresh");

    EconSeed {
        cl8y_id: cl8y,
        ust1_id: ust1,
        gem_id: gem,
        cl8y_pair,
    }
}

async fn insert_fee(
    pool: &sqlx::PgPool,
    source: FeeSource,
    asset_id: i32,
    raw: &str,
    usd: Option<&str>,
    hours_ago: i64,
    tx: &str,
    decimals: i16,
) {
    let draft = FeeEventDraft {
        block_height: 1,
        block_timestamp: Utc::now() - Duration::hours(hours_ago),
        tx_hash: tx.to_string(),
        source,
        ordinal: 0,
        asset_id,
        amount_raw: bd(raw),
        decimals,
        fee_usd: usd.map(bd),
    };
    fee_q::insert_fee_event(pool, &draft)
        .await
        .expect("insert fee");
}

#[serial]
#[tokio::test]
async fn cl8y_ask_fee_stamps_usd_and_rollup() {
    let pool = common::setup_pool().await;
    let seed = seed_cl8y_hub(&pool).await;

    let hub = hub_prices::load_quote_usd(&pool).await.unwrap();
    let mark = hub
        .economic_usd_per_human(DEFAULT_HUB_CL8Y_ADDRESS)
        .expect("CL8Y mark");
    assert!((mark - bd("0.01")).abs() < bd("0.0000001"));

    // Ingest path: fee_usd_for_raw (same helper as parser::ingest_protocol_fee).
    let cl8y_asset = asset_q::get_asset_by_id(&pool, seed.cl8y_id)
        .await
        .unwrap()
        .expect("CL8Y asset");
    let stamped = fee_usd_for_raw(
        &cl8y_asset,
        &bd("1298000"),
        Some(&bd("0.005")),
        None,
        None,
        Some(&hub),
    )
    .expect("CL8Y fee USD");
    assert_eq!(stamped, bd("0.01298"));

    // 1.298 human CL8Y × $0.01 = $0.01298
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        Some("0.01298"),
        1,
        "tx-cl8y-amm",
        6,
    )
    .await;
    volume_aggregator::refresh_all_volume_windows_with_wrap(&pool, true, false).await;

    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("0.01298"));
    assert_eq!(rollup.total_fees_7d_usd.unwrap(), bd("0.01298"));
    assert_eq!(rollup.total_fees_30d_usd.unwrap(), bd("0.01298"));

    reset_protocol_fees_cache();
    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/protocol/fees?window=24h").await.json();
    let tokens = body["by_token"].as_array().unwrap();
    let row = tokens
        .iter()
        .find(|t| t["symbol"] == "CL8Y-cb")
        .expect("CL8Y-cb token row");
    assert_eq!(bd(row["amount_human"].as_str().unwrap()), bd("1.298"));
    assert_eq!(bd(row["amount_usd"].as_str().unwrap()), bd("0.01298"));

    let hub_body: serde_json::Value = server.get("/api/v1/hub-prices").await.json();
    assert_eq!(hub_body["tickers"].as_array().unwrap().len(), 4);
    assert_eq!(hub_body["prices"].as_array().unwrap().len(), 4);
    let cl8y = server.get("/api/v1/hub-prices/cl8y").await;
    assert_eq!(cl8y.status_code(), StatusCode::BAD_REQUEST);
    let js = server.get("/api/v1/hub-prices/javascript:").await;
    assert_eq!(js.status_code(), StatusCode::BAD_REQUEST);
    let _ = seed.cl8y_pair;
}

#[serial]
#[tokio::test]
async fn no_qualifying_pair_leaves_fee_usd_null() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let cl8y: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'CL8Y', 'CL8Y', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_CL8Y_ADDRESS)
    .fetch_one(&pool)
    .await
    .unwrap();
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        cl8y,
        "1298000",
        None,
        1,
        "tx-unpriced",
        6,
    )
    .await;
    volume_aggregator::refresh_all_volume_windows_with_wrap(&pool, true, false).await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 1);
    assert!(rollup.total_fees_24h_usd.is_none());
}

#[serial]
#[tokio::test]
async fn mixed_window_priced_sum_drops_gem() {
    let pool = common::setup_pool().await;
    let seed = seed_cl8y_hub(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.ust1_id,
        "1000000",
        Some("0.97"),
        1,
        "tx-ust1",
        6,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        Some("0.01298"),
        1,
        "tx-cl8y",
        6,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.gem_id,
        "1000000",
        None,
        1,
        "tx-gem",
        6,
    )
    .await;
    volume_aggregator::refresh_all_volume_windows_with_wrap(&pool, true, false).await;
    let rollup = fee_q::get_fee_rollup(&pool).await.unwrap();
    assert_eq!(rollup.fee_event_count_24h, 3);
    assert_eq!(rollup.total_fees_24h_usd.unwrap(), bd("0.98298"));
}

#[serial]
#[tokio::test]
async fn null_only_backfill_does_not_rewrite_stamp() {
    let pool = common::setup_pool().await;
    let seed = seed_cl8y_hub(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        None,
        1,
        "tx-null",
        6,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1000000",
        Some("99.99"),
        1,
        "tx-stamped",
        6,
    )
    .await;

    let hub = hub_prices::load_quote_usd(&pool).await.unwrap();
    let filled = fee_q::backfill_null_fee_usd(&pool, Some(&bd("0.005")), None, None, &hub)
        .await
        .unwrap();
    assert_eq!(filled, 1);

    let null_row: Option<BigDecimal> = sqlx::query_scalar(
        "SELECT fee_usd FROM protocol_fee_events WHERE tx_hash = 'tx-null'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(null_row.is_some());
    assert_eq!(null_row.unwrap(), bd("0.01298"));

    let stamped: BigDecimal = sqlx::query_scalar(
        "SELECT fee_usd FROM protocol_fee_events WHERE tx_hash = 'tx-stamped'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stamped, bd("99.99"));

    // Second backfill must not change the original stamp when the live mark moves.
    sqlx::query("UPDATE economic_token_marks SET price_usd = 50")
        .execute(&pool)
        .await
        .unwrap();
    let hub2 = hub_prices::load_quote_usd(&pool).await.unwrap();
    let filled2 = fee_q::backfill_null_fee_usd(&pool, Some(&bd("0.005")), None, None, &hub2)
        .await
        .unwrap();
    assert_eq!(filled2, 0);
    let stamped2: BigDecimal = sqlx::query_scalar(
        "SELECT fee_usd FROM protocol_fee_events WHERE tx_hash = 'tx-stamped'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stamped2, bd("99.99"));
}

#[serial]
#[tokio::test]
async fn replay_does_not_double_count() {
    let pool = common::setup_pool().await;
    let seed = seed_cl8y_hub(&pool).await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        Some("0.01298"),
        1,
        "tx-dup-683",
        6,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        Some("0.01298"),
        1,
        "tx-dup-683",
        6,
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
async fn fees_window_allowlist_and_overview_o1() {
    let pool = common::setup_pool().await;
    let _seed = seed_cl8y_hub(&pool).await;
    volume_aggregator::refresh_all_volume_windows_with_wrap(&pool, true, false).await;
    reset_overview_cache();
    reset_protocol_fees_cache();
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for bad in ["window=javascript:", "window=ust1-window", "window="] {
        let resp = server.get(&format!("/api/v1/protocol/fees?{bad}")).await;
        assert_eq!(resp.status_code(), StatusCode::BAD_REQUEST, "{bad}");
    }
    let ok = server.get("/api/v1/protocol/fees?window=24h").await;
    ok.assert_status_ok();
}

#[serial]
#[tokio::test]
async fn llama_daily_inherits_cl8y_fee_excludes_gem_pair() {
    let pool = common::setup_pool().await;
    let seed = seed_cl8y_hub(&pool).await;
    let ts = Utc::now() - Duration::hours(2);
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.cl8y_id,
        "1298000",
        Some("0.01298"),
        2,
        "tx-llama-cl8y",
        6,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.gem_id,
        "1000000",
        Some("9.99"),
        2,
        "tx-llama-gem",
        6,
    )
    .await;
    sqlx::query(
        "UPDATE protocol_fee_events SET block_timestamp = $1
         WHERE tx_hash IN ('tx-llama-cl8y', 'tx-llama-gem')",
    )
    .bind(ts)
    .execute(&pool)
    .await
    .unwrap();
    let gem_pair: i32 = sqlx::query_scalar(
        "SELECT id FROM pairs WHERE contract_address = 'terra1gempire683'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO swap_events
            (pair_id, block_height, block_timestamp, tx_hash, sender,
             offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 1, $2, 'tx-llama-gem', 'terra1t', $3, $4, 1000, 950, 0.95)",
    )
    .bind(gem_pair)
    .bind(ts)
    .bind(seed.ust1_id)
    .bind(seed.gem_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO swap_events
            (pair_id, block_height, block_timestamp, tx_hash, sender,
             offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 1, $2, 'tx-llama-cl8y', 'terra1t', $3, $4, 1000, 950, 0.95)",
    )
    .bind(seed.cl8y_pair)
    .bind(ts)
    .bind(seed.ust1_id)
    .bind(seed.cl8y_id)
    .execute(&pool)
    .await
    .unwrap();

    cl8y_dex_indexer::db::queries::defillama::refresh_defillama_daily(&pool)
        .await
        .expect("llama refresh");
    let day = ts.date_naive();
    let fees = cl8y_dex_indexer::db::queries::defillama::get_daily_fees(&pool, day)
        .await
        .unwrap();
    let total: BigDecimal = fees
        .iter()
        .map(|r| r.amount_usd.clone())
        .fold(BigDecimal::from(0), |a, b| a + b);
    assert_eq!(total, bd("0.01298"));
}
