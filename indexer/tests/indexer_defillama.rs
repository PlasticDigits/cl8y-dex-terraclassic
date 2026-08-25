//! GitLab #631 — UTC-day DeFiLlama rollup + GET /api/v1/defillama/daily.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, TimeZone, Utc};
use cl8y_dex_indexer::api::reset_defillama_cache;
use cl8y_dex_indexer::db::queries::defillama as daily_q;
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::defillama::{
    utc_day_start, COLUMBUS5_FACTORY, COLUMBUS5_GEM_ADDRESSES,
};
use cl8y_dex_indexer::indexer::protocol_fees::{FeeEventDraft, FeeSource};
use cl8y_dex_indexer::indexer::volume_aggregator;
use common::{build_test_app, seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

const EMBER: &str = COLUMBUS5_GEM_ADDRESSES[0];

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

fn assert_usd(v: &Value, expected: &str) {
    let s = v
        .as_str()
        .unwrap_or_else(|| panic!("expected usd string, got {v}"));
    assert_eq!(bd(s), bd(expected), "{s} != {expected}");
}

fn day0() -> chrono::DateTime<Utc> {
    utc_day_start(Utc::now()) - Duration::days(2)
}

fn day1() -> chrono::DateTime<Utc> {
    utc_day_start(Utc::now()) - Duration::days(1)
}

async fn wipe_events(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM protocol_fee_events")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM limit_order_fills")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM limit_order_placements")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM swap_events")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM defillama_daily_assets")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM defillama_daily_fees")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM defillama_daily_stats")
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_swap(
    pool: &sqlx::PgPool,
    pair_id: i32,
    offer_id: i32,
    ask_id: i32,
    ts: chrono::DateTime<Utc>,
    tx: &str,
    volume_usd: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 6310, $2, $3, 'terra1t', $4, $5, 1000, 950, 0.95, $6)",
    )
    .bind(pair_id)
    .bind(ts)
    .bind(tx)
    .bind(offer_id)
    .bind(ask_id)
    .bind(volume_usd.map(bd))
    .execute(pool)
    .await
    .expect("insert swap");
}

async fn insert_fill(
    pool: &sqlx::PgPool,
    pair_id: i32,
    ts: chrono::DateTime<Utc>,
    tx: &str,
    order_id: i64,
) {
    let swap_id: i64 = sqlx::query_scalar("SELECT id FROM swap_events WHERE tx_hash = $1")
        .bind(tx)
        .fetch_one(pool)
        .await
        .expect("parent swap");
    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, 6311, $3, $4, $5, 'ask', 'terra1maker', 1.0, 50, 50, 1)",
    )
    .bind(pair_id)
    .bind(swap_id)
    .bind(ts)
    .bind(tx)
    .bind(order_id)
    .execute(pool)
    .await
    .expect("insert fill");
}

async fn insert_fee(
    pool: &sqlx::PgPool,
    source: FeeSource,
    asset_id: i32,
    ts: chrono::DateTime<Utc>,
    tx: &str,
    usd: Option<&str>,
    ordinal: i64,
) {
    let draft = FeeEventDraft {
        block_height: 6312,
        block_timestamp: ts,
        tx_hash: tx.to_string(),
        source,
        ordinal,
        asset_id,
        amount_raw: bd("1000000"),
        decimals: 6,
        fee_usd: usd.map(bd),
    };
    fee_q::insert_fee_event(pool, &draft)
        .await
        .expect("insert fee");
}

async fn seed_gem_pair(pool: &sqlx::PgPool, quote_id: i32) -> i32 {
    let gem_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Ember', 'EMBER', 6) RETURNING id",
    )
    .bind(EMBER)
    .fetch_one(pool)
    .await
    .expect("gem asset");
    sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1gempire631', $1, $2, 'terra1gemlp', 30) RETURNING id",
    )
    .bind(gem_id)
    .bind(quote_id)
    .fetch_one(pool)
    .await
    .expect("gem pair")
}

async fn refresh(pool: &sqlx::PgPool) {
    volume_aggregator::refresh_all_volume_windows_with_wrap(pool, true, false).await;
    reset_defillama_cache();
}

#[serial]
#[tokio::test]
async fn daily_volume_excludes_gems_fills_wrap_and_window() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let gem_pair = seed_gem_pair(&pool, seed.asset_1_id).await;
    let d0 = day0() + Duration::hours(3);
    let d1 = day1() + Duration::hours(4);

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d0,
        "tx-econ-0",
        Some("10"),
    )
    .await;
    insert_fill(&pool, seed.pair_id, d0, "tx-econ-0", 99).await;
    insert_swap(
        &pool,
        gem_pair,
        seed.asset_1_id,
        seed.asset_1_id,
        d0,
        "tx-gem-0",
        Some("999"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d1,
        "tx-econ-1",
        Some("4"),
    )
    .await;

    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-econ-0",
        Some("1.5"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::BookTake,
        seed.asset_1_id,
        d0,
        "tx-econ-0",
        Some("0.5"),
        1,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-gem-0",
        Some("80"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_1_id,
        d0,
        "tx-wrap-0",
        Some("0.25"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Ust1Mint,
        seed.asset_1_id,
        d0,
        "tx-ust1-0",
        Some("0.10"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::LimitPlace,
        seed.asset_1_id,
        d1,
        "tx-place-1",
        Some("0.2"),
        0,
    )
    .await;

    refresh(&pool).await;

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let body: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();

    assert_usd(&body["volume_usd"], "10");
    assert_eq!(body["trade_count"], 1, "hybrid fill + gem must not add");
    assert_usd(&body["fees"]["swap_amm"], "1.5");
    assert_usd(&body["fees"]["book_take"], "0.5");
    assert_usd(&body["fees"]["wrap"], "0.25");
    assert_usd(&body["fees"]["ust1_mint"], "0.10");
    assert_usd(&body["fees"]["limit_place"], "0");
    assert_eq!(body["daily_supply_side_revenue_usd"], "0");
    assert_usd(&body["daily_fees_usd"], "2.35");
    assert_usd(&body["daily_revenue_usd"], "2.35");
    assert_eq!(body["methodology"]["factory"], COLUMBUS5_FACTORY);
    assert_eq!(body["assets"]["ust1"]["product"], "unstablecoin");
    assert_eq!(body["assets"]["ustr"]["category"], "reserve");
    assert_usd(&body["assets"]["ust1"]["fees_usd"], "0.10");
    assert_usd(&body["assets"]["ustr"]["volume_usd"], "0");
    assert!(body["methodology"]["tvl"]
        .as_str()
        .unwrap()
        .contains("liquidity_in_usd"));

    let ts1 = day1().timestamp();
    let day1_body: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts1}"))
        .await
        .json();
    assert_usd(&day1_body["volume_usd"], "4");
    assert_usd(&day1_body["fees"]["limit_place"], "0.2");
    assert_usd(&day1_body["fees"]["swap_amm"], "0");
}

#[serial]
#[tokio::test]
async fn unpriced_active_is_null_idle_is_zero() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let d0 = day0() + Duration::hours(2);
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d0,
        "tx-null-vol",
        None,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-null-vol",
        None,
        0,
    )
    .await;
    refresh(&pool).await;

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let body: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert!(body["volume_usd"].is_null(), "{body}");
    assert!(body["fees"]["swap_amm"].is_null());
    assert!(body["daily_fees_usd"].is_null());
    assert_eq!(body["trade_count"], 1);

    let idle_ts = (utc_day_start(Utc::now()) - Duration::days(3)).timestamp();
    daily_q::refresh_defillama_day(&pool, Utc.timestamp_opt(idle_ts, 0).single().unwrap())
        .await
        .unwrap();
    reset_defillama_cache();
    let idle: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={idle_ts}"))
        .await
        .json();
    assert_eq!(idle["volume_usd"], "0");
    assert_eq!(idle["trade_count"], 0);
    assert_eq!(idle["fees"]["swap_amm"], "0");
    assert_eq!(idle["daily_fees_usd"], "0");
}

#[serial]
#[tokio::test]
async fn timestamp_allowlist_and_missing_day() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_events(&pool).await;
    refresh(&pool).await;
    let app = build_test_app(pool).await;
    let server = TestServer::new(app);

    for q in [
        "/api/v1/defillama/daily",
        "/api/v1/defillama/daily?timestamp=",
        "/api/v1/defillama/daily?timestamp=1;drop%20table",
        "/api/v1/defillama/daily?timestamp=-1",
        "/api/v1/defillama/daily?timestamp=86401",
        "/api/v1/defillama/daily?timestamp=window=1;drop%20table",
        "/api/v1/defillama/daily?from=0&to=now",
    ] {
        let res = server.get(q).await;
        assert_eq!(res.status_code(), 400, "{q} → {}", res.status_code());
    }

    let future = (utc_day_start(Utc::now()) + Duration::days(2)).timestamp();
    let res = server
        .get(&format!("/api/v1/defillama/daily?timestamp={future}"))
        .await;
    assert_eq!(res.status_code(), 400);

    let ancient = 86_400i64; // 1970-01-02, aligned, never rolled
    let res = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ancient}"))
        .await;
    assert_eq!(res.status_code(), 404);
}

#[serial]
#[tokio::test]
async fn get_reads_rollup_only() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    refresh(&pool).await;

    let vol_plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT volume_usd, trade_count FROM defillama_daily_stats WHERE utc_day = CURRENT_DATE",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let vol = vol_plan
        .into_iter()
        .map(|(l,)| l)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!vol.contains("swap_events"), "{vol}");
    assert!(!vol.contains("protocol_fee_events"), "{vol}");
    assert!(!vol.contains("limit_order_fills"), "{vol}");

    let fee_plan: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT source, amount_usd FROM defillama_daily_fees WHERE utc_day = CURRENT_DATE",
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
async fn cache_hit_does_not_require_reroll() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let d0 = day0() + Duration::hours(1);
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d0,
        "tx-cache",
        Some("7"),
    )
    .await;
    refresh(&pool).await;
    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let first: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert_usd(&first["volume_usd"], "7");

    sqlx::query("UPDATE defillama_daily_stats SET volume_usd = 1 WHERE utc_day = $1")
        .bind(day0().date_naive())
        .execute(&pool)
        .await
        .unwrap();
    let cached: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert_usd(&cached["volume_usd"], "7");
}

#[serial]
#[tokio::test]
async fn ust1_ustr_asset_volume_fees_and_hub_price() {
    use cl8y_dex_indexer::config::{DEFAULT_HUB_UST1_ADDRESS, DEFAULT_HUB_USTR_ADDRESS};

    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let d0 = day0() + Duration::hours(3);

    let ust1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .bind(DEFAULT_HUB_UST1_ADDRESS)
    .fetch_one(&pool)
    .await
    .expect("ust1 asset");
    let ustr_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'USTR', 'USTR', 18) RETURNING id",
    )
    .bind(DEFAULT_HUB_USTR_ADDRESS)
    .fetch_one(&pool)
    .await
    .expect("ustr asset");

    let ust1_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1ust1pair631', $1, $2, 'terra1ust1lp', 30) RETURNING id",
    )
    .bind(ust1_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("ust1 pair");
    let both_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1ust1ustr631', $1, $2, 'terra1bothlp', 30) RETURNING id",
    )
    .bind(ust1_id)
    .bind(ustr_id)
    .fetch_one(&pool)
    .await
    .expect("ust1/ustr pair");

    insert_swap(
        &pool,
        ust1_pair,
        ust1_id,
        seed.asset_1_id,
        d0,
        "tx-ust1-vol",
        Some("12"),
    )
    .await;
    insert_swap(
        &pool,
        both_pair,
        ust1_id,
        ustr_id,
        d0,
        "tx-both-vol",
        Some("3"),
    )
    .await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d0,
        "tx-other-vol",
        Some("50"),
    )
    .await;

    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        ust1_id,
        d0,
        "tx-ust1-vol",
        Some("0.40"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Ust1Redeem,
        ust1_id,
        d0,
        "tx-ust1-redeem",
        Some("0.05"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-other-vol",
        Some("1.00"),
        0,
    )
    .await;

    sqlx::query(
        "INSERT INTO hub_prices (ticker, price_usd, updated_at) VALUES ('ust1', 0.97, NOW())",
    )
    .execute(&pool)
    .await
    .expect("hub ust1");
    sqlx::query(
        "INSERT INTO hub_prices (ticker, price_usd, updated_at) VALUES ('ustr', 0.015, NOW())",
    )
    .execute(&pool)
    .await
    .expect("hub ustr");

    refresh(&pool).await;

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let body: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();

    assert_usd(&body["volume_usd"], "65");
    assert_eq!(body["trade_count"], 3);
    assert_usd(&body["assets"]["ust1"]["volume_usd"], "15");
    assert_eq!(body["assets"]["ust1"]["trade_count"], 2);
    assert_usd(&body["assets"]["ustr"]["volume_usd"], "3");
    assert_eq!(body["assets"]["ustr"]["trade_count"], 1);
    assert_usd(&body["assets"]["ust1"]["fees_usd"], "0.45");
    assert_usd(&body["assets"]["ustr"]["fees_usd"], "0");
    assert_usd(&body["assets"]["ust1"]["price_usd"], "0.97");
    assert_usd(&body["assets"]["ustr"]["price_usd"], "0.015");
    assert_eq!(body["assets"]["ust1"]["product"], "unstablecoin");
    assert_eq!(body["assets"]["ust1"]["peg_type"], "peggedUSD");
    assert_eq!(body["assets"]["ustr"]["peg_type"], Value::Null);
    assert_eq!(body["assets"]["ustr"]["category"], "reserve");
}
