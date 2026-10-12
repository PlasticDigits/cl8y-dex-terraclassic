//! Deterministic concurrent anchor rescaling, using an owned disposable database.
mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::{api::ReferencePrices, db::queries::assets, indexer::hub_usd::HubUsdConfig};
use serial_test::serial;
use std::time::Duration;

#[tokio::test]
#[serial]
async fn issue1479_reference_cohort_cannot_splice_old_anchor_and_new_token_publication() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    let token:i32=sqlx::query_scalar("INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
        VALUES('terra1cohortfixture14790000000000000000',true,'Cohort token','COHORT',6) RETURNING id")
        .fetch_one(&pool).await.unwrap();
    common::reference_marks::seed_reference_marks(&pool, &cfg, &[]).await;
    let indexed = assets::get_all_assets(&pool).await.unwrap();
    let anchor = indexed
        .iter()
        .find(|asset| asset.contract_address.as_deref() == Some(cfg.custc_address.as_str()))
        .unwrap()
        .id;
    let old = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert_eq!(
        old.fair_output(anchor, token, 1_000_000, Utc::now())
            .unwrap()
            .0,
        1_000_000
    );
    // The writer holds only the economic table. The observer can acquire its
    // original hub snapshot, then deterministically waits at the later mark read.
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE economic_token_marks IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *writer)
        .await
        .unwrap();
    let (reader_pool, reader_assets, reader_cfg) = (pool.clone(), indexed.clone(), cfg.clone());
    let observer = tokio::spawn(async move {
        ReferencePrices::observe(&reader_pool, &reader_assets, &reader_cfg).await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_locks l
                JOIN pg_stat_activity a ON a.pid=l.pid
                WHERE l.relation='economic_token_marks'::regclass
                  AND NOT l.granted AND l.mode='AccessShareLock'
                  AND a.datname=current_database()
                  AND a.query LIKE 'SELECT m.contract_address%')",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("observer read the old hub and is blocked before its economic mark read");
    // Same reserves/relative price, new externally certified USD scale. The
    // complete publication updates both tables in one transaction as production.
    sqlx::query("UPDATE oracle_state SET price_usd=2,as_of=clock_timestamp() WHERE ticker='ustc'")
        .execute(&mut *writer)
        .await
        .unwrap();
    sqlx::query("UPDATE hub_prices SET price_usd=2,as_of=clock_timestamp() WHERE ticker='custc'")
        .execute(&mut *writer)
        .await
        .unwrap();
    sqlx::query("UPDATE economic_token_marks SET price_usd=2,tvl_usd=tvl_usd*2,as_of=clock_timestamp() WHERE asset_id=$1")
        .bind(token).execute(&mut *writer).await.unwrap();
    writer.commit().await.unwrap();
    let concurrent = tokio::time::timeout(Duration::from_secs(5), observer)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(concurrent.prices[&anchor].price, BigDecimal::from(1));
    assert_eq!(concurrent.prices[&token].price, BigDecimal::from(1));
    assert_eq!(concurrent.prices[&token].as_of, old.prices[&token].as_of);
    assert_eq!(
        concurrent
            .fair_output(anchor, token, 1_000_000, Utc::now())
            .unwrap()
            .0,
        1_000_000
    );
    assert_eq!(
        concurrent
            .fair_output(token, anchor, 1_000_000, Utc::now())
            .unwrap()
            .0,
        1_000_000
    );
    let new = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert_eq!(new.prices[&anchor].price, BigDecimal::from(2));
    assert_eq!(new.prices[&token].price, BigDecimal::from(2));
    assert_eq!(
        new.fair_output(anchor, token, 1_000_000, Utc::now())
            .unwrap()
            .0,
        1_000_000
    );
    assert_eq!(
        new.fair_output(token, anchor, 1_000_000, Utc::now())
            .unwrap()
            .0,
        1_000_000
    );
}
