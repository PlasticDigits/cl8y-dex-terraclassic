//! GitLab #676: 18-decimal raw amounts must persist and count every swap.
mod common;

use std::str::FromStr;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::indexer::position_tracker;
use serde_json::Value;
use serial_test::serial;

/// 100 human 18-dec tokens. Overflows `NUMERIC(38, 18)` (`|x| < 10^20`).
fn raw_100_tokens_18dec() -> BigDecimal {
    BigDecimal::from_str("100000000000000000000").unwrap()
}

fn raw_80_tokens_18dec() -> BigDecimal {
    BigDecimal::from_str("80000000000000000000").unwrap()
}

async fn insert_cw20(
    pool: &sqlx::PgPool,
    addr: &str,
    symbol: &str,
    decimals: i16,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, $2, $2, $3)
         RETURNING id",
    )
    .bind(addr)
    .bind(symbol)
    .bind(decimals)
    .fetch_one(pool)
    .await
    .expect("insert cw20")
}

async fn insert_pair(pool: &sqlx::PgPool, addr: &str, a0: i32, a1: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, $4, 30)
         RETURNING id",
    )
    .bind(addr)
    .bind(a0)
    .bind(a1)
    .bind(format!("{addr}-lp"))
    .fetch_one(pool)
    .await
    .expect("insert pair")
}

async fn insert_swap_row(
    pool: &sqlx::PgPool,
    pair_id: i32,
    height: i64,
    tx: &str,
    sender: &str,
    offer: i32,
    ask: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) {
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, $2, NOW(), $3, $4, $5, $6, $7, $8, 1)",
    )
    .bind(pair_id)
    .bind(height)
    .bind(tx)
    .bind(sender)
    .bind(offer)
    .bind(ask)
    .bind(offer_amount)
    .bind(return_amount)
    .execute(pool)
    .await
    .expect("insert swap");
}

async fn position_trade_count(pool: &sqlx::PgPool, trader: &str, pair_id: i32) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT trade_count FROM trader_positions WHERE trader_address = $1 AND pair_id = $2",
    )
    .bind(trader)
    .bind(pair_id)
    .fetch_optional(pool)
    .await
    .expect("select trade_count")
}

async fn position_net_plain(pool: &sqlx::PgPool, trader: &str, pair_id: i32) -> String {
    sqlx::query_scalar(
        "SELECT net_position_quote::text FROM trader_positions WHERE trader_address = $1 AND pair_id = $2",
    )
    .bind(trader)
    .bind(pair_id)
    .fetch_one(pool)
    .await
    .expect("select net")
}

#[serial]
#[tokio::test]
async fn ust1_ustr_100_quote_persists_and_counts() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let ust1 = insert_cw20(&pool, "terra1ust1-676", "UST1", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr-676", "USTR", 18).await;
    let pair_id = insert_pair(&pool, "terra1ust1ustr-676", ust1, ustr).await;
    let trader = seed.trader_address.as_str();

    position_tracker::update_position_on_swap(
        &pool,
        pair_id,
        ust1,
        trader,
        ust1,
        &BigDecimal::from(1_000_000),
        &raw_100_tokens_18dec(),
        None,
        None,
    )
    .await
    .expect("buy 100 USTR");

    assert_eq!(position_trade_count(&pool, trader, pair_id).await, Some(1));
    let net = BigDecimal::from_str(&position_net_plain(&pool, trader, pair_id).await).unwrap();
    assert_eq!(net, raw_100_tokens_18dec());
}

#[serial]
#[tokio::test]
async fn cl8y_custc_100_base_cost_persists_and_counts() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let cl8y = insert_cw20(&pool, "terra1cl8y-676", "CL8Y-cb", 18).await;
    let custc = insert_cw20(&pool, "terra1custc-676", "cUSTC", 6).await;
    let pair_id = insert_pair(&pool, "terra1cl8ycustc-676", cl8y, custc).await;
    let trader = seed.trader_address.as_str();

    position_tracker::update_position_on_swap(
        &pool,
        pair_id,
        cl8y,
        trader,
        cl8y,
        &raw_100_tokens_18dec(),
        &BigDecimal::from(2_000_000),
        None,
        None,
    )
    .await
    .expect("buy quote with 100 CL8Y");

    let cost: String = sqlx::query_scalar(
        "SELECT total_cost_base::text FROM trader_positions WHERE trader_address = $1 AND pair_id = $2",
    )
    .bind(trader)
    .bind(pair_id)
    .fetch_one(&pool)
    .await
    .expect("cost");
    assert_eq!(
        BigDecimal::from_str(&cost).unwrap(),
        raw_100_tokens_18dec()
    );
    assert_eq!(position_trade_count(&pool, trader, pair_id).await, Some(1));
}

#[serial]
#[tokio::test]
async fn mixed_decimal_pairs_trade_count_matches_every_swap() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let trader = seed.trader_address.as_str();

    let ust1 = insert_cw20(&pool, "terra1ust1-676b", "UST1", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr-676b", "USTR", 18).await;
    let pair_618 = insert_pair(&pool, "terra1pair-618", ust1, ustr).await;

    let cl8y = insert_cw20(&pool, "terra1cl8y-676b", "CL8Y-cb", 18).await;
    let custc = insert_cw20(&pool, "terra1custc-676b", "cUSTC", 6).await;
    let pair_186 = insert_pair(&pool, "terra1pair-186", cl8y, custc).await;

    // Three UST1→USTR buys (issue table: /trades=3, /positions was absent).
    for (i, ret) in [
        raw_100_tokens_18dec(),
        raw_80_tokens_18dec(),
        raw_100_tokens_18dec(),
    ]
    .into_iter()
    .enumerate()
    {
        position_tracker::update_position_on_swap(
            &pool,
            pair_618,
            ust1,
            trader,
            ust1,
            &BigDecimal::from(1_000_000),
            &ret,
            None,
            None,
        )
        .await
        .unwrap_or_else(|_| panic!("UST1/USTR swap {i}"));
    }

    // Seven CL8Y→cUSTC buys (issue table: /trades=7, /positions trade_count=2).
    for i in 0..7 {
        position_tracker::update_position_on_swap(
            &pool,
            pair_186,
            cl8y,
            trader,
            cl8y,
            &raw_100_tokens_18dec(),
            &BigDecimal::from(1_000_000),
            None,
            None,
        )
        .await
        .unwrap_or_else(|_| panic!("CL8Y/cUSTC swap {i}"));
    }

    assert_eq!(position_trade_count(&pool, trader, pair_618).await, Some(3));
    assert_eq!(position_trade_count(&pool, trader, pair_186).await, Some(7));

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/traders/{}/positions", trader))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();

    let p618 = body
        .iter()
        .find(|r| r["pair_address"] == "terra1pair-618")
        .expect("UST1/USTR row present");
    assert_eq!(p618["trade_count"], 3);
    assert_eq!(p618["asset_0_decimals"], 6);
    assert_eq!(p618["asset_1_decimals"], 18);
    assert_eq!(
        p618["net_position_quote"].as_str().unwrap(),
        "280000000000000000000"
    );
    assert!(
        !p618["net_position_quote"]
            .as_str()
            .unwrap()
            .contains('e'),
        "plain integer string, not scientific notation"
    );

    let p186 = body
        .iter()
        .find(|r| r["pair_address"] == "terra1pair-186")
        .expect("CL8Y/cUSTC row present");
    assert_eq!(p186["trade_count"], 7);
    assert_eq!(p186["asset_0_decimals"], 18);
    assert_eq!(p186["asset_1_decimals"], 6);
}

#[serial]
#[tokio::test]
async fn rebuild_repairs_stale_trade_count_after_skipped_upsert() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let trader = seed.trader_address.as_str();

    let ust1 = insert_cw20(&pool, "terra1ust1-676c", "UST1", 6).await;
    let ustr = insert_cw20(&pool, "terra1ustr-676c", "USTR", 18).await;
    let pair_id = insert_pair(&pool, "terra1pair-618-rebuild", ust1, ustr).await;

    for i in 0..3 {
        insert_swap_row(
            &pool,
            pair_id,
            2000 + i,
            &format!("tx-676-ustr-{i}"),
            trader,
            ust1,
            ustr,
            &BigDecimal::from(1_000_000),
            &raw_100_tokens_18dec(),
        )
        .await;
    }

    sqlx::query(
        "INSERT INTO trader_positions
         (trader_address, pair_id, net_position_quote, avg_entry_price, total_cost_base, realized_pnl, trade_count)
         VALUES ($1, $2, 1, 1, 1, 0, 1)",
    )
    .bind(trader)
    .bind(pair_id)
    .execute(&pool)
    .await
    .expect("stale position");

    assert!(
        position_tracker::positions_trade_count_diverges(&pool)
            .await
            .expect("diverge"),
        "stale trade_count=1 vs 3 swaps must diverge"
    );

    let repaired = position_tracker::repair_positions_if_trade_count_mismatch(&pool)
        .await
        .expect("repair");
    assert!(repaired);
    assert_eq!(position_trade_count(&pool, trader, pair_id).await, Some(3));

    let net = BigDecimal::from_str(&position_net_plain(&pool, trader, pair_id).await).unwrap();
    assert_eq!(net, raw_100_tokens_18dec() * 3);

    assert!(
        !position_tracker::positions_trade_count_diverges(&pool)
            .await
            .expect("aligned"),
        "second check must be aligned"
    );
    let again = position_tracker::repair_positions_if_trade_count_mismatch(&pool)
        .await
        .expect("noop");
    assert!(!again);
}

#[serial]
#[tokio::test]
async fn six_six_pair_still_counts_after_rebuild() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let trader = seed.trader_address.as_str();

    position_tracker::rebuild_all_positions_from_swaps(&pool)
        .await
        .expect("rebuild seed swaps");

    // seed_db inserts 5 offer-asset_0 swaps (buy quote).
    assert_eq!(
        position_trade_count(&pool, trader, seed.pair_id).await,
        Some(5)
    );
}
