//! GitLab #522 — human quote-per-base + USD of 1 human base.

mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::indexer::{pair_price_usd, swap_orientation};
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[serial]
#[tokio::test]
async fn mixed_decimal_ustr_print_persists_human_and_usd() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1', true, 'UST1', 'UST1', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ust1");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustr', true, 'USTR', 'USTR', 18)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ustr");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1ust1ustr', $1, $2, 'terra1lp522', 30)
         RETURNING id",
    )
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair");

    let offer = bd("116624");
    let ret = bd("9297047794755092035");
    let oriented = swap_orientation::orient_swap_leg(asset_0_id, asset_0_id, &offer, &ret, 6, 18);
    let human = oriented.price.to_string().parse::<f64>().unwrap();
    assert!((human - 79.72).abs() < 0.05, "got {human}");

    let ustc = bd("0.004928");
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ('ustc', $1, 'average', NOW())",
    )
    .bind(&ustc)
    .execute(&pool)
    .await
    .expect("oracle");

    let quote = cl8y_dex_indexer::db::queries::assets::get_asset_by_id(&pool, asset_1_id)
        .await
        .expect("quote row")
        .expect("quote");
    let price_usd = pair_price_usd::price_usd_for_human_quote_per_base(
        &quote,
        &oriented.price,
        Some(&ustc),
        None,
    )
    .expect("usd");
    let usd_f = price_usd.to_string().parse::<f64>().unwrap();
    assert!((usd_f - 0.983).abs() < 0.03, "got {usd_f}");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, 5220, $2, 'tx522ustr', 'terra1t', $3, $4, $5, $6, $7, $8)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(&offer)
    .bind(&ret)
    .bind(&oriented.price)
    .bind(&price_usd)
    .execute(&pool)
    .await
    .expect("insert");

    let stats = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_for_pair(&pool, pair_id)
        .await
        .expect("stats");
    let close_usd = stats.close_price_usd.expect("close_usd");
    let close_usd_f = close_usd.to_string().parse::<f64>().unwrap();
    assert!((close_usd_f - usd_f).abs() < 0.0001);
    assert!(stats.close_price.unwrap() < bd("1000"));
}

#[serial]
#[tokio::test]
async fn same_decimal_custc_print_usd_near_one() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1b', true, 'UST1', 'UST1', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("ust1");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1custc', true, 'cUSTC', 'cUSTC', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("custc");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1ust1custc', $1, $2, 'terra1lp522b', 30)
         RETURNING id",
    )
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair");

    let offer = bd("116651");
    let ret = bd("24102109");
    let oriented = swap_orientation::orient_swap_leg(asset_0_id, asset_0_id, &offer, &ret, 6, 6);
    let human = oriented.price.to_string().parse::<f64>().unwrap();
    assert!((human - 206.62).abs() < 0.02, "got {human}");

    let ustc = bd("0.004928");
    let quote = cl8y_dex_indexer::db::queries::assets::get_asset_by_id(&pool, asset_1_id)
        .await
        .expect("quote row")
        .expect("quote");
    let price_usd = pair_price_usd::price_usd_for_human_quote_per_base(
        &quote,
        &oriented.price,
        Some(&ustc),
        None,
    )
    .expect("usd");
    let usd_f = price_usd.to_string().parse::<f64>().unwrap();
    assert!((usd_f - 1.018).abs() < 0.02, "got {usd_f}");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, price_usd)
         VALUES ($1, 5221, $2, 'tx522custc', 'terra1t', $3, $4, $5, $6, $7, $8)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(asset_0_id)
    .bind(asset_1_id)
    .bind(&offer)
    .bind(&ret)
    .bind(&oriented.price)
    .bind(&price_usd)
    .execute(&pool)
    .await
    .expect("insert");

    let stats = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_for_pair(&pool, pair_id)
        .await
        .expect("stats");
    assert!((stats.close_price.as_ref().unwrap().to_string().parse::<f64>().unwrap() - 206.62).abs() < 0.02);
    assert!(
        (stats
            .close_price_usd
            .as_ref()
            .unwrap()
            .to_string()
            .parse::<f64>()
            .unwrap()
            - 1.018)
            .abs()
            < 0.02
    );
}
