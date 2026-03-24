mod common;

use bigdecimal::BigDecimal;
use cl8y_dex_indexer::indexer::position_tracker;

#[tokio::test]
async fn position_net_quote_clamped_when_oversold() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let trader = seed.trader_address.as_str();
    let pair_id = seed.pair_id;
    let asset_0 = seed.asset_0_id;
    let asset_1 = seed.asset_1_id;

    // Open/add quote position: offer base (asset_0), receive quote
    position_tracker::update_position_on_swap(
        &pool,
        pair_id,
        asset_0,
        trader,
        asset_0,
        &BigDecimal::from(1000),
        &BigDecimal::from(500),
        None,
        None,
    )
    .await
    .expect("buy quote");

    // Sell more quote than held: offer quote (asset_1) with huge amount
    position_tracker::update_position_on_swap(
        &pool,
        pair_id,
        asset_0,
        trader,
        asset_1,
        &BigDecimal::from(10_000),
        &BigDecimal::from(1),
        None,
        None,
    )
    .await
    .expect("oversell");

    let net: String = sqlx::query_scalar(
        "SELECT net_position_quote::text FROM trader_positions WHERE trader_address = $1 AND pair_id = $2",
    )
    .bind(trader)
    .bind(pair_id)
    .fetch_one(&pool)
    .await
    .expect("select net");

    assert_eq!(net, "0");
}
