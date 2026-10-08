//! Old-day repair must refresh corrections beyond the ordinary eight-day loop.
mod common;

use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::db::queries::defillama as daily;
use cl8y_dex_indexer::indexer::defillama::utc_day_start;
use cl8y_dex_indexer::indexer::defillama_history::{repair_history, HistoryRange};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn old_ledger_corrections_are_repaired_idempotently_without_repricing() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let start = utc_day_start(Utc::now()) - Duration::days(30);
    sqlx::query(
        "INSERT INTO protocol_fee_events
        (tx_hash, source, ordinal, block_height, block_timestamp, asset_id,
         amount_raw, decimals, fee_usd)
        VALUES ('history-repair', 'wrap', 0, 1354, $1, $2, 1000000, 6, 1)",
    )
    .bind(start)
    .bind(seed.asset_0_id)
    .execute(&pool)
    .await
    .unwrap();
    daily::refresh_defillama_day(&pool, start).await.unwrap();
    sqlx::query("UPDATE protocol_fee_events SET fee_usd=7.25 WHERE tx_hash='history-repair'")
        .execute(&pool)
        .await
        .unwrap();
    daily::refresh_defillama_daily(&pool).await.unwrap();
    let fee = daily::get_daily_fees(&pool, start.date_naive())
        .await
        .unwrap();
    assert_eq!(
        fee.iter().find(|f| f.source == "wrap").unwrap().amount_usd,
        BigDecimal::from(1)
    );
    let range = HistoryRange::parse(
        &start.date_naive().to_string(),
        &(start + Duration::days(1)).date_naive().to_string(),
        Utc::now().date_naive(),
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(repair_history(&pool, range).await.unwrap(), 2);
        let fee = daily::get_daily_fees(&pool, start.date_naive())
            .await
            .unwrap();
        assert_eq!(
            fee.iter().find(|f| f.source == "wrap").unwrap().amount_usd,
            "7.25".parse::<BigDecimal>().unwrap()
        );
        assert_eq!(
            daily::get_daily_stat(&pool, (start + Duration::days(1)).date_naive())
                .await
                .unwrap()
                .unwrap()
                .trade_count,
            0
        );
    }
    let stamp: BigDecimal = sqlx::query_scalar(
        "SELECT fee_usd FROM protocol_fee_events WHERE tx_hash='history-repair'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stamp, "7.25".parse::<BigDecimal>().unwrap());
}

#[serial]
#[tokio::test]
async fn history_repair_preserves_auxiliary_snapshots_and_never_invents_missing_marks() {
    use cl8y_dex_indexer::config::DEFAULT_HUB_UST1_ADDRESS;
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let start = utc_day_start(Utc::now()) - Duration::days(30);
    let old_mark_time = start + Duration::hours(15);
    sqlx::query("UPDATE assets SET contract_address = $1, is_cw20 = true WHERE id = $2")
        .bind(DEFAULT_HUB_UST1_ADDRESS)
        .bind(seed.asset_0_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO swap_events (pair_id, block_height, block_timestamp, tx_hash, sender,
        offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
        VALUES ($1,1354,$2,'history-auxiliary','terra1test',$3,$4,1000,950,0.95,10)",
    )
    .bind(seed.pair_id)
    .bind(start)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO protocol_fee_events (tx_hash, source, ordinal, pair_id, block_height,
        block_timestamp, asset_id, amount_raw, decimals, fee_usd)
        VALUES ('history-auxiliary','swap_amm',0,$1,1354,$2,$3,1000000,6,0.25)",
    )
    .bind(seed.pair_id)
    .bind(start)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .unwrap();
    daily::repair_defillama_day(&pool, start).await.unwrap();
    sqlx::query(
        "UPDATE defillama_daily_assets SET price_usd = 0.97, price_as_of = $1,
        circulating_raw = 123456 WHERE utc_day = $2 AND ticker = 'ust1'",
    )
    .bind(old_mark_time)
    .bind(start.date_naive())
    .execute(&pool)
    .await
    .unwrap();
    // Test both an absent ticker row and an entirely missing next-day materialization.
    sqlx::query("DELETE FROM defillama_daily_assets WHERE utc_day = $1 AND ticker = 'ustr'")
        .bind(start.date_naive())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE swap_events SET volume_usd = 15 WHERE tx_hash = 'history-auxiliary'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE protocol_fee_events SET fee_usd = 0.75 WHERE tx_hash = 'history-auxiliary'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO hub_prices(ticker,price_usd,updated_at,as_of,quality)
        VALUES ('ust1',999,NOW(),NOW(),'bounded_spot'), ('ustr',888,NOW(),NOW(),'bounded_spot')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at)
        VALUES ('ustc',0.005,'fresh',ARRAY['mexc','kucoin'],NOW(),NOW())",
    )
    .execute(&pool)
    .await
    .unwrap();
    let range = HistoryRange::parse(
        &start.date_naive().to_string(),
        &(start + Duration::days(1)).date_naive().to_string(),
        Utc::now().date_naive(),
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(repair_history(&pool, range).await.unwrap(), 2);
        let rows = daily::get_daily_assets(&pool, start.date_naive())
            .await
            .unwrap();
        let ust1 = rows.iter().find(|row| row.ticker == "ust1").unwrap();
        let ustr = rows.iter().find(|row| row.ticker == "ustr").unwrap();
        assert_eq!(ust1.price_usd, Some("0.97".parse::<BigDecimal>().unwrap()));
        assert_eq!(ust1.price_as_of, Some(old_mark_time));
        assert_eq!(ust1.circulating_raw, Some(BigDecimal::from(123456)));
        assert_eq!(ust1.volume_usd, BigDecimal::from(15));
        assert_eq!(ust1.fees_usd, "0.75".parse::<BigDecimal>().unwrap());
        assert!(ustr.price_usd.is_none());
        assert!(ustr.price_as_of.is_none());
        assert!(ustr.circulating_raw.is_none());
        for row in daily::get_daily_assets(&pool, (start + Duration::days(1)).date_naive())
            .await
            .unwrap()
        {
            assert!(row.price_usd.is_none());
            assert!(row.price_as_of.is_none());
            assert!(row.circulating_raw.is_none());
        }
        assert_eq!(
            daily::get_daily_stat(&pool, start.date_naive())
                .await
                .unwrap()
                .unwrap()
                .volume_usd,
            BigDecimal::from(15)
        );
        assert_eq!(
            daily::get_daily_fees(&pool, start.date_naive())
                .await
                .unwrap()
                .iter()
                .find(|row| row.source == "swap_amm")
                .unwrap()
                .amount_usd,
            "0.75".parse::<BigDecimal>().unwrap()
        );
    }
}
