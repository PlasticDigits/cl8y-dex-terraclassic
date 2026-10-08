//! GitLab #631 — UTC-day DeFiLlama rollup + GET /api/v1/defillama/daily.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, TimeZone, Utc};
use cl8y_dex_indexer::api::reset_defillama_cache;
use cl8y_dex_indexer::db::queries::defillama as daily_q;
use cl8y_dex_indexer::db::queries::protocol_fees as fee_q;
use cl8y_dex_indexer::indexer::defillama::{
    utc_day_start, ADAPTER_START_UTC_DAY, COLUMBUS5_FACTORY, COLUMBUS5_GEM_ADDRESSES,
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
    let swap_id: i64 =
        sqlx::query_scalar("SELECT id FROM swap_events WHERE tx_hash = $1 AND pair_id = $2")
            .bind(tx)
            .bind(pair_id)
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
    insert_scoped_fee(pool, source, None, asset_id, ts, tx, usd, ordinal).await;
}

#[allow(clippy::too_many_arguments)]
async fn insert_scoped_fee(
    pool: &sqlx::PgPool,
    source: FeeSource,
    pair_id: Option<i32>,
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
        pair_id,
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
    assert_eq!(body["priced_trade_count"], 0);
    assert_eq!(body["unpriced_trade_count"], 1);
    assert_eq!(body["volume_complete"], false);

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
    assert_eq!(idle["priced_trade_count"], 0);
    assert_eq!(idle["unpriced_trade_count"], 0);
    assert_eq!(idle["volume_complete"], true);
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
        "INSERT INTO hub_prices (ticker, price_usd, updated_at, as_of, quality) VALUES ('ust1', 0.97, NOW(),NOW(),'bounded_spot')",
    )
    .execute(&pool)
    .await
    .expect("hub ust1");
    sqlx::query(
        "INSERT INTO hub_prices (ticker, price_usd, updated_at, as_of, quality) VALUES ('ustr', 0.015, NOW(),NOW(),'bounded_spot')",
    )
    .execute(&pool)
    .await
    .expect("hub ustr");

    sqlx::query(
        "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at)
       VALUES ('ustc',0.005,'fresh',ARRAY['mexc','kucoin'],NOW(),NOW())
       ON CONFLICT(ticker) DO UPDATE SET quality='fresh', price_usd=0.005,
       accepted_sources=ARRAY['mexc','kucoin'],as_of=NOW(),observed_at=NOW()",
    )
    .execute(&pool)
    .await
    .unwrap();
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

#[serial]
#[tokio::test]
async fn mixed_unpriced_source_does_not_null_headline() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let gem_pair = seed_gem_pair(&pool, seed.asset_1_id).await;
    let d0 = day0() + Duration::hours(2);

    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        d0,
        "tx-econ-mix",
        Some("10"),
    )
    .await;
    insert_swap(
        &pool,
        gem_pair,
        seed.asset_1_id,
        seed.asset_1_id,
        d0,
        "tx-gem-mix",
        Some("999"),
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-econ-mix",
        Some("1.5"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_1_id,
        d0,
        "tx-wrap-mix",
        Some("0.25"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::LimitPlace,
        seed.asset_1_id,
        d0,
        "tx-place-unpriced",
        None,
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        d0,
        "tx-gem-mix",
        Some("80"),
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
    assert_usd(&body["fees"]["swap_amm"], "1.5");
    assert_usd(&body["fees"]["wrap"], "0.25");
    assert!(
        body["fees"]["limit_place"].is_null(),
        "per-source stays fail-closed: {}",
        body["fees"]["limit_place"]
    );
    assert_usd(&body["daily_fees_usd"], "1.75");
    assert_usd(&body["daily_revenue_usd"], "1.75");
    assert_eq!(body["daily_supply_side_revenue_usd"], "0");
}

#[serial]
#[tokio::test]
async fn idle_fees_with_volume_are_zero_not_null() {
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
        "tx-vol-only",
        Some("7"),
    )
    .await;
    refresh(&pool).await;

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let body: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert_usd(&body["volume_usd"], "7");
    assert_eq!(body["daily_fees_usd"], "0");
    assert_eq!(body["fees"]["swap_amm"], "0");
}

#[serial]
#[tokio::test]
async fn adapter_start_day_is_200_prior_is_404() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_events(&pool).await;

    let start = Utc
        .timestamp_opt(ADAPTER_START_UTC_DAY, 0)
        .single()
        .expect("adapter start");
    daily_q::refresh_defillama_day(&pool, start).await.unwrap();
    reset_defillama_cache();

    let app = build_test_app(pool).await;
    let server = TestServer::new(app);
    let ok = server
        .get(&format!(
            "/api/v1/defillama/daily?timestamp={ADAPTER_START_UTC_DAY}"
        ))
        .await;
    assert_eq!(ok.status_code(), 200);
    let body: Value = ok.json();
    assert_eq!(body["daily_fees_usd"], "0");
    assert_eq!(body["volume_usd"], "0");

    let prior = ADAPTER_START_UTC_DAY - 86_400;
    let missing = server
        .get(&format!("/api/v1/defillama/daily?timestamp={prior}"))
        .await;
    assert_eq!(missing.status_code(), 404);
}

#[serial]
#[tokio::test]
async fn null_only_fee_usd_backfill_flips_headline() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let d0 = day0() + Duration::hours(2);

    insert_fee(
        &pool,
        FeeSource::Wrap,
        seed.asset_1_id,
        d0,
        "tx-wrap-stamp",
        Some("0.25"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::LimitPlace,
        seed.asset_1_id,
        d0,
        "tx-place-null",
        None,
        0,
    )
    .await;
    refresh(&pool).await;

    let app = build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let ts0 = day0().timestamp();
    let before: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert_usd(&before["daily_fees_usd"], "0.25");
    assert!(before["fees"]["limit_place"].is_null());

    sqlx::query(
        "UPDATE protocol_fee_events SET fee_usd = 1.0 WHERE fee_usd IS NULL AND tx_hash = 'tx-place-null'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let wrap_stamp: BigDecimal = sqlx::query_scalar(
        "SELECT fee_usd FROM protocol_fee_events WHERE tx_hash = 'tx-wrap-stamp'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(wrap_stamp, bd("0.25"), "C568-1: non-null stamp must stay");

    refresh(&pool).await;
    let after: Value = server
        .get(&format!("/api/v1/defillama/daily?timestamp={ts0}"))
        .await
        .json();
    assert_usd(&after["daily_fees_usd"], "1.25");
    assert_usd(&after["fees"]["limit_place"], "1.0");
    assert_usd(&after["fees"]["wrap"], "0.25");
}

#[serial]
#[tokio::test]
async fn partial_volume_reports_positive_sum_and_exact_completeness() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let ts = day0() + Duration::hours(2);
    // NULL, zero and negative marks are all unpriced, never fabricated dollars.
    for (tx, usd) in [
        ("partial-positive", Some("12.345")),
        ("partial-null", None),
        ("partial-zero", Some("0")),
        ("partial-negative", Some("-3")),
    ] {
        insert_swap(
            &pool,
            seed.pair_id,
            seed.asset_0_id,
            seed.asset_1_id,
            ts,
            tx,
            usd,
        )
        .await;
    }
    let gem_pair = seed_gem_pair(&pool, seed.asset_1_id).await;
    insert_swap(
        &pool,
        gem_pair,
        seed.asset_1_id,
        seed.asset_1_id,
        ts,
        "partial-gem",
        None,
    )
    .await;
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    reset_defillama_cache();
    let server = TestServer::new(build_test_app(pool).await);
    let body: Value = server
        .get(&format!(
            "/api/v1/defillama/daily?timestamp={}",
            day0().timestamp()
        ))
        .await
        .json();
    assert_usd(&body["volume_usd"], "12.345");
    assert_eq!(body["trade_count"], 4);
    assert_eq!(body["priced_trade_count"], 1);
    assert_eq!(body["unpriced_trade_count"], 3);
    assert_eq!(body["volume_complete"], false);
}

#[serial]
#[tokio::test]
async fn mixed_gem_route_preserves_actual_pair_fees_and_reports_legacy_scope() {
    use cl8y_dex_indexer::config::DEFAULT_HUB_UST1_ADDRESS;
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    // Make the ordinary pair UST1-linked to also test asset-specific allocation.
    sqlx::query("UPDATE assets SET contract_address = $1, is_cw20 = true WHERE id = $2")
        .bind(DEFAULT_HUB_UST1_ADDRESS)
        .bind(seed.asset_0_id)
        .execute(&pool)
        .await
        .unwrap();
    let gem_pair = seed_gem_pair(&pool, seed.asset_1_id).await;
    let ts = day0() + Duration::hours(2);
    for (pair, tx, usd) in [
        (seed.pair_id, "mixed-route", Some("10")),
        (gem_pair, "mixed-route", Some("999")),
        (seed.pair_id, "partial-asset", None),
    ] {
        insert_swap(&pool, pair, seed.asset_0_id, seed.asset_1_id, ts, tx, usd).await;
    }
    // Pair IDs override all transaction siblings, even when ordinals collide.
    for source in [
        FeeSource::SwapAmm,
        FeeSource::BookTake,
        FeeSource::LimitPlace,
    ] {
        insert_scoped_fee(
            &pool,
            source,
            Some(seed.pair_id),
            seed.asset_1_id,
            ts,
            "mixed-route",
            Some("1.5"),
            0,
        )
        .await;
        insert_scoped_fee(
            &pool,
            source,
            Some(gem_pair),
            seed.asset_1_id,
            ts,
            "mixed-route",
            Some("80"),
            0,
        )
        .await;
    }
    // A NULL-pair legacy swap has two candidates: preserve it explicitly unresolved,
    // never guess gem exclusion or attribute it to UST1.
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        ts,
        "mixed-route",
        Some("0.25"),
        0,
    )
    .await;
    // A unique gem legacy event excludes only its own fee. A missing candidate is
    // retained with an unresolved count, including an unpriced row.
    insert_swap(
        &pool,
        gem_pair,
        seed.asset_1_id,
        seed.asset_1_id,
        ts,
        "legacy-gem",
        Some("99"),
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SwapAmm,
        seed.asset_1_id,
        ts,
        "legacy-gem",
        Some("9"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::BookTake,
        seed.asset_1_id,
        ts,
        "legacy-missing",
        None,
        0,
    )
    .await;
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    reset_defillama_cache();
    let server = TestServer::new(build_test_app(pool.clone()).await);
    let body: Value = server
        .get(&format!(
            "/api/v1/defillama/daily?timestamp={}",
            day0().timestamp()
        ))
        .await
        .json();
    assert_usd(&body["volume_usd"], "10");
    assert_usd(&body["daily_fees_usd"], "4.75");
    assert_usd(&body["fees"]["swap_amm"], "1.75");
    assert!(body["fees"]["book_take"].is_null());
    assert_usd(&body["fees"]["limit_place"], "1.5");
    assert_eq!(body["unresolved_pair_fee_event_count"], 2);
    assert_usd(&body["assets"]["ust1"]["fees_usd"], "4.5");
    assert_usd(&body["assets"]["ust1"]["volume_usd"], "10");
    assert_eq!(body["assets"]["ust1"]["trade_count"], 2);
    assert_eq!(body["assets"]["ust1"]["priced_trade_count"], 1);
    assert_eq!(body["assets"]["ust1"]["unpriced_trade_count"], 1);
    assert_eq!(body["assets"]["ust1"]["volume_complete"], false);
    // Reaggregation is deterministic and does not mutate stored legacy identity.
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    let rows = daily_q::get_daily_fees(&pool, day0().date_naive())
        .await
        .unwrap();
    let swap = rows.iter().find(|r| r.source == "swap_amm").unwrap();
    assert_eq!(swap.amount_usd, bd("1.75"));
    assert_eq!(swap.unresolved_pair_count, 1);
    let legacy_pair: Option<i32> = sqlx::query_scalar("SELECT pair_id FROM protocol_fee_events WHERE tx_hash = 'mixed-route' AND source = 'swap_amm' AND pair_id IS NULL")
        .fetch_one(&pool).await.unwrap();
    assert!(legacy_pair.is_none());
}

#[serial]
#[tokio::test]
async fn failed_day_refresh_rolls_back_all_materializations() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    insert_swap(
        &pool,
        seed.pair_id,
        seed.asset_0_id,
        seed.asset_1_id,
        day0(),
        "atomic-day",
        Some("7"),
    )
    .await;
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    sqlx::query("UPDATE swap_events SET volume_usd = 9 WHERE tx_hash = 'atomic-day'")
        .execute(&pool)
        .await
        .unwrap();
    // Force a failure after the stats/fees writes, when asset rows are inserted.
    sqlx::query("ALTER TABLE defillama_daily_assets ADD CONSTRAINT fail_refresh_1354 CHECK (trade_count < 0) NOT VALID")
        .execute(&pool).await.unwrap();
    let failed = daily_q::refresh_defillama_day(&pool, day0()).await;
    sqlx::query("ALTER TABLE defillama_daily_assets DROP CONSTRAINT fail_refresh_1354")
        .execute(&pool)
        .await
        .unwrap();
    assert!(failed.is_err());
    let stat = daily_q::get_daily_stat(&pool, day0().date_naive())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stat.volume_usd, bd("7"));
    assert_eq!(
        daily_q::get_daily_assets(&pool, day0().date_naive())
            .await
            .unwrap()
            .len(),
        2
    );
}

#[serial]
#[tokio::test]
async fn legacy_book_and_place_scope_matches_the_actual_order() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let gem_pair = seed_gem_pair(&pool, seed.asset_1_id).await;
    let ts = day0() + Duration::hours(2);
    for pair in [seed.pair_id, gem_pair] {
        insert_swap(
            &pool,
            pair,
            seed.asset_0_id,
            seed.asset_1_id,
            ts,
            "legacy-orders",
            Some("10"),
        )
        .await;
    }
    insert_fill(&pool, seed.pair_id, ts, "legacy-orders", 11).await;
    insert_fill(&pool, gem_pair, ts, "legacy-orders", 22).await;
    for (pair, order) in [(seed.pair_id, 33), (gem_pair, 44)] {
        sqlx::query("INSERT INTO limit_order_placements (pair_id, block_height, block_timestamp, tx_hash, order_id) VALUES ($1,1354,$2,'legacy-orders',$3)")
            .bind(pair).bind(ts).bind(order as i64).execute(&pool).await.unwrap();
    }
    for (source, ordinal, usd) in [
        (FeeSource::BookTake, 11, Some("0.5")),
        (FeeSource::BookTake, 22, Some("90")),
        (FeeSource::BookTake, 999, Some("1")),
        (FeeSource::LimitPlace, 33, Some("0.25")),
        (FeeSource::LimitPlace, 44, Some("90")),
        (FeeSource::LimitPlace, 999, None),
    ] {
        insert_fee(
            &pool,
            source,
            seed.asset_1_id,
            ts,
            "legacy-orders",
            usd,
            ordinal,
        )
        .await;
    }
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    let rows = daily_q::get_daily_fees(&pool, day0().date_naive())
        .await
        .unwrap();
    let book = rows.iter().find(|r| r.source == "book_take").unwrap();
    let place = rows.iter().find(|r| r.source == "limit_place").unwrap();
    assert_eq!(book.amount_usd, bd("1.5"));
    assert_eq!(book.event_count, 2);
    assert_eq!(book.unresolved_pair_count, 1);
    assert_eq!(place.amount_usd, bd("0.25"));
    assert_eq!(place.event_count, 2);
    assert_eq!(place.unpriced_count, 1);
    assert_eq!(place.unresolved_pair_count, 1);
}

#[serial]
#[tokio::test]
async fn newly_supported_fee_sources_omit_unconfigured_and_preserve_null_idle() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    wipe_events(&pool).await;
    let ts = day0() + Duration::hours(2);
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    reset_defillama_cache();
    let server = TestServer::new(build_test_app(pool.clone()).await);
    let url = format!("/api/v1/defillama/daily?timestamp={}", day0().timestamp());
    let unconfigured: Value = server.get(&url).await.json();
    for key in ["pair_creation", "sku_unlock", "settings_fee"] {
        assert!(
            unconfigured["fees"].get(key).is_none(),
            "unconfigured {key} must be omitted"
        );
    }
    assert_usd(&unconfigured["daily_fees_usd"], "0");
    sqlx::query("UPDATE global_stats_24h SET factory_configured = true, community_invoices_configured = true WHERE id = 1")
        .execute(&pool).await.unwrap();
    insert_fee(
        &pool,
        FeeSource::PairCreation,
        seed.asset_1_id,
        ts,
        "new-pair-creation",
        Some("3"),
        0,
    )
    .await;
    insert_fee(
        &pool,
        FeeSource::SkuUnlock,
        seed.asset_1_id,
        ts,
        "new-sku",
        None,
        0,
    )
    .await;
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    reset_defillama_cache();
    let configured: Value = server.get(&url).await.json();
    assert_usd(&configured["fees"]["pair_creation"], "3");
    assert!(configured["fees"].get("sku_unlock").unwrap().is_null());
    assert_usd(&configured["fees"]["settings_fee"], "0");
    assert_usd(&configured["daily_fees_usd"], "3");
    assert_usd(&configured["daily_revenue_usd"], "3");
    assert_eq!(configured["unresolved_pair_fee_event_count"], 0);
    sqlx::query("UPDATE global_stats_24h SET factory_configured = false, community_invoices_configured = false WHERE id = 1")
        .execute(&pool).await.unwrap();
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    reset_defillama_cache();
    let historical: Value = server.get(&url).await.json();
    assert_usd(&historical["daily_fees_usd"], "3");
    assert_usd(&historical["fees"]["pair_creation"], "3");
    assert!(historical["fees"].get("sku_unlock").unwrap().is_null());
    assert!(historical["fees"].get("settings_fee").is_none());
}

#[serial]
#[tokio::test]
async fn refresh_rejects_unaligned_days_and_competing_refreshes() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    wipe_events(&pool).await;
    let invalid = daily_q::refresh_defillama_day(&pool, day0() + Duration::seconds(1)).await;
    assert!(invalid.is_err());
    assert!(daily_q::get_daily_stat(&pool, day0().date_naive())
        .await
        .unwrap()
        .is_none());
    let mut competing = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(1354, ($1::date - DATE '1970-01-01')::integer)")
        .bind(day0().date_naive())
        .execute(&mut *competing)
        .await
        .unwrap();
    let failed = daily_q::refresh_defillama_day(&pool, day0()).await;
    assert!(matches!(failed, Err(sqlx::Error::Protocol(_))));
    assert!(daily_q::get_daily_stat(&pool, day0().date_naive())
        .await
        .unwrap()
        .is_none());
    competing.rollback().await.unwrap();
    daily_q::refresh_defillama_day(&pool, day0()).await.unwrap();
    assert!(daily_q::get_daily_stat(&pool, day0().date_naive())
        .await
        .unwrap()
        .is_some());
}
