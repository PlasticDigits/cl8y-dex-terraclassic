//! GitLab #466 — swap price/volume orientation: two-sided trading must not mix P and 1/P in OHLC.

mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::indexer::swap_orientation;
use serial_test::serial;
use std::str::FromStr;

#[serial]
#[tokio::test]
async fn oriented_two_sided_swaps_high_low_are_not_reciprocals() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
         VALUES ('uluna', false, 'Luna Classic', 'LUNC', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("asset_0");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustctoken', true, 'TerraClassicUSD', 'USTC', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("asset_1");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1orientpair', $1, $2, 'terra1lp', 30)
         RETURNING id",
    )
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair");

    let trader = "terra1traderxyz";

    let offer_base = BigDecimal::from_str("100").unwrap();
    let receive_quote = BigDecimal::from_str("5050").unwrap();
    let forward = swap_orientation::orient_swap_leg(
        asset_0_id,
        asset_0_id,
        &offer_base,
        &receive_quote,
        6,
        6,
    );
    assert_eq!(forward.price, BigDecimal::from_str("50.5").unwrap());

    let offer_quote = receive_quote.clone();
    let receive_base = offer_base.clone();
    let reverse = swap_orientation::orient_swap_leg(
        asset_0_id,
        asset_1_id,
        &offer_quote,
        &receive_base,
        6,
        6,
    );
    assert_eq!(forward.price, reverse.price);

    let now = Utc::now();
    for (tx, offer_id, ask_id, offer_amt, return_amt, price) in [
        (
            "orient_fwd",
            asset_0_id,
            asset_1_id,
            &offer_base,
            &receive_quote,
            &forward.price,
        ),
        (
            "orient_rev",
            asset_1_id,
            asset_0_id,
            &offer_quote,
            &receive_base,
            &reverse.price,
        ),
    ] {
        sqlx::query(
            "INSERT INTO swap_events
             (pair_id, block_height, block_timestamp, tx_hash, sender,
              offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
             VALUES ($1, 4000, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(pair_id)
        .bind(now)
        .bind(tx)
        .bind(trader)
        .bind(offer_id)
        .bind(ask_id)
        .bind(offer_amt)
        .bind(return_amt)
        .bind(price)
        .execute(&pool)
        .await
        .expect("insert oriented swap");
    }

    let stats = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_for_pair(
        &pool, pair_id,
    )
    .await
    .expect("24h stats");

    let high = stats.high.as_ref().expect("high");
    let low = stats.low.as_ref().expect("low");
    assert_eq!(high, low, "high and low must match when both trades share quote-per-base price");
    assert_eq!(high, &forward.price);
    assert!(
        low > &BigDecimal::from(1),
        "low must not be the reciprocal (~0.02); got {low}"
    );

    let reciprocal = BigDecimal::from(1) / high;
    let gap = if low > &reciprocal {
        low - &reciprocal
    } else {
        &reciprocal - low
    };
    assert!(
        gap > BigDecimal::from_str("1").unwrap(),
        "low must not approximate 1/high"
    );
}

#[serial]
#[tokio::test]
async fn oriented_volume_aggregation_sums_base_and_quote_legs() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let offer_base = BigDecimal::from_str("100").unwrap();
    let receive_quote = BigDecimal::from_str("5050").unwrap();
    let forward = swap_orientation::orient_swap_leg(
        seed.asset_0_id,
        seed.asset_0_id,
        &offer_base,
        &receive_quote,
        6,
        6,
    );
    let reverse = swap_orientation::orient_swap_leg(
        seed.asset_0_id,
        seed.asset_1_id,
        &receive_quote,
        &offer_base,
        6,
        6,
    );

    let now = Utc::now();
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 4100, $2, 'vol_fwd', $3, $4, $5, 100, 5050, $6)",
    )
    .bind(seed.pair_id)
    .bind(now)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .bind(&forward.price)
    .execute(&pool)
    .await
    .expect("fwd swap");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 4101, $2, 'vol_rev', $3, $4, $5, 5050, 100, $6)",
    )
    .bind(seed.pair_id)
    .bind(now)
    .bind(&seed.trader_address)
    .bind(seed.asset_1_id)
    .bind(seed.asset_0_id)
    .bind(&reverse.price)
    .execute(&pool)
    .await
    .expect("rev swap");

    let stats = cl8y_dex_indexer::db::queries::swap_events::get_24h_stats_for_pair(
        &pool, seed.pair_id,
    )
    .await
    .expect("stats");

    // seed_db inserts five base->quote swaps of 1000 base each, plus two more above.
    assert_eq!(stats.volume_base, BigDecimal::from(5200));
    assert_eq!(
        stats.volume_quote,
        BigDecimal::from_str("14850").unwrap(),
        "quote volume sums oriented legs, not raw offer/return columns"
    );
}
