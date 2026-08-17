//! GitLab #543 — additive human OHLC beside factory USD candles (no human fallback).

mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::candles;
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[serial]
#[tokio::test]
async fn rebuild_writes_usd_and_human_for_ust1_ustr() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1543a', true, 'UST1', 'UST1', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ust1");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustr543a', true, 'USTR', 'USTR', 18)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ustr");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1ust1ustr543', $1, $2, 'terra1lp543a', 30)
         RETURNING id",
    )
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair");

    let ts = Utc::now();
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, 5430, $2, 'tx543ustr', 'terra1t', $3, $4, 1000000, 86480000000000000000, $5, $6)",
    )
    .bind(pair_id)
    .bind(ts)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(bd("86.48"))
    .bind(bd("1.06"))
    .execute(&pool)
    .await
    .expect("insert usd swap");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, 5431, $2, 'tx543human', 'terra1t', $3, $4, 1000, 21260000, $5, NULL)",
    )
    .bind(pair_id)
    .bind(ts)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(bd("21260"))
    .execute(&pool)
    .await
    .expect("insert human-only swap");

    candles::rebuild_candles_from_swaps(&pool, pair_id, "1h", ts - chrono::Duration::hours(2))
        .await
        .expect("rebuild");

    let rows = candles::get_candles(
        &pool,
        pair_id,
        "1h",
        ts - chrono::Duration::hours(2),
        ts + chrono::Duration::hours(1),
        10,
    )
    .await
    .expect("get");
    assert_eq!(rows.len(), 1, "human-only swap must not create a USD candle");
    let usd = rows[0].close.to_string().parse::<f64>().unwrap();
    let human = rows[0]
        .close_human
        .as_ref()
        .expect("human")
        .to_string()
        .parse::<f64>()
        .unwrap();
    assert!((usd - 1.06).abs() < 0.001, "got usd {usd}");
    assert!((human - 86.48).abs() < 0.01, "got human {human}");
    assert!(usd < 2.0, "USD column must not be human 21260");
}

#[serial]
#[tokio::test]
async fn rebuild_clunc_ust1_usd_matches_human_scale() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1clunc543', true, 'cLUNC', 'cLUNC', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("clunc");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1543b', true, 'UST1', 'UST1', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ust1");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1cluncust1543', $1, $2, 'terra1lp543b', 30)
         RETURNING id",
    )
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair");

    let ts = Utc::now();
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, 5432, $2, 'tx543clunc', 'terra1t', $3, $4, 1000000, 47, $5, $6)",
    )
    .bind(pair_id)
    .bind(ts)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(bd("0.000047"))
    .bind(bd("0.000047"))
    .execute(&pool)
    .await
    .expect("insert");

    candles::rebuild_candles_from_swaps(&pool, pair_id, "1h", ts - chrono::Duration::hours(2))
        .await
        .expect("rebuild");

    let rows = candles::get_candles(
        &pool,
        pair_id,
        "1h",
        ts - chrono::Duration::hours(2),
        ts + chrono::Duration::hours(1),
        10,
    )
    .await
    .expect("get");
    assert_eq!(rows.len(), 1);
    let usd = rows[0].close.to_string().parse::<f64>().unwrap();
    let human = rows[0]
        .close_human
        .as_ref()
        .expect("human")
        .to_string()
        .parse::<f64>()
        .unwrap();
    assert!((usd - 0.000047).abs() < 1e-9, "got usd {usd}");
    assert!((human - 0.000047).abs() < 1e-9, "got human {human}");
}
