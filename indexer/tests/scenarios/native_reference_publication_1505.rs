//! Owned disposable SQL: accounting contention cannot stall executable references.
use crate::common;

use chrono::{Duration, Utc};
use cl8y_dex_indexer::{
    api::ReferencePrices,
    config::DEFAULT_HUB_CLUNC_ADDRESS,
    db::queries::{assets, hub_prices},
    indexer::{hub_usd::HubUsdConfig, oracle::OraclePriceHandles},
};
use std::{sync::Arc, time::Duration as Wait};
use tokio::sync::Notify;

#[tokio::test]
#[serial_test::serial]
async fn issue1505_blocked_accounting_does_not_delay_atomic_reference_replacement() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let original = chrono::DateTime::from_timestamp_micros(
        (Utc::now() - Duration::seconds(10)).timestamp_micros(),
    )
    .unwrap();
    let token: i32 = sqlx::query_scalar(
        "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
        VALUES('terra1reference1505xxxxxxxxxxxxxxxxxx',true,'Reference fixture','FIXTURE',6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let anchor: i32 = sqlx::query_scalar(
        "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
        VALUES($1,true,'Canonical LUNC wrap','cLUNC',6) RETURNING id",
    )
    .bind(DEFAULT_HUB_CLUNC_ADDRESS)
    .fetch_one(&pool)
    .await
    .unwrap();
    let pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,fee_bps)
        VALUES('terra1referencepair1505',$1,$2,30) RETURNING id",
    )
    .bind(token)
    .bind(anchor)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO pair_reserves(pair_id,reserve_0,reserve_1,fee_bps,snapshot_at)
        VALUES($1,110044622670234,252903132784,30,$2)",
    )
    .bind(pair)
    .bind(original)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at)
        VALUES('lunc',0.00005,'fresh',ARRAY['mexc','kucoin'],$1,$1)",
    )
    .bind(original)
    .execute(&pool)
    .await
    .unwrap();
    let cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    let oracle = OraclePriceHandles::new();
    *oracle.lunc.write().await = Some("0.00005".parse().unwrap());
    let price = "0.00005".parse().unwrap();
    let initial = hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&price))
        .await
        .unwrap();
    assert!(
        initial.lunc.is_some(),
        "fixture establishes qualified original anchor"
    );
    let initial_mark: Option<chrono::DateTime<Utc>> =
        sqlx::query_scalar("SELECT as_of FROM economic_token_marks WHERE asset_id=$1")
            .bind(token)
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(
        initial_mark,
        Some(original),
        "fixture establishes qualified economic source"
    );
    let signal = Arc::new(Notify::new());
    let mut blocker = pool.begin().await.unwrap();
    // The real accounting helper reads this table after the atomic mark commit.
    sqlx::query("LOCK TABLE protocol_fee_events IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let worker = tokio::spawn(hub_prices::run_hub_usd_refresh_loop_with_signal(
        pool.clone(),
        cfg.clone(),
        oracle.ustc.clone(),
        oracle.lunc.clone(),
        Wait::from_secs(5),
        Some(signal.clone()),
    ));
    tokio::time::timeout(Wait::from_secs(5), async {
        loop {
            let mark: Option<chrono::DateTime<Utc>> =
                sqlx::query_scalar("SELECT as_of FROM economic_token_marks WHERE asset_id=$1")
                    .bind(token)
                    .fetch_optional(&pool)
                    .await
                    .unwrap();
            if mark == Some(original) {
                break;
            }
            tokio::time::sleep(Wait::from_millis(20)).await;
        }
    })
    .await
    .expect("initial atomic reference cohort must commit despite blocked accounting");
    tokio::time::timeout(Wait::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE NOT granted AND locktype='relation'
                 AND database=(SELECT oid FROM pg_database WHERE datname=current_database())
                 AND relation='protocol_fee_events'::regclass)",
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Wait::from_millis(20)).await;
        }
    })
    .await
    .expect("accounting must be observably blocked before the raw replacement");
    let replacement = original + Duration::seconds(2);
    let mut update = pool.begin().await.unwrap();
    sqlx::query("UPDATE pair_reserves SET snapshot_at=$1 WHERE pair_id=$2")
        .bind(replacement)
        .bind(pair)
        .execute(&mut *update)
        .await
        .unwrap();
    sqlx::query("UPDATE oracle_state SET as_of=$1,observed_at=$1 WHERE ticker='lunc'")
        .bind(replacement)
        .execute(&mut *update)
        .await
        .unwrap();
    update.commit().await.unwrap();
    signal.notify_one();
    let updated = tokio::time::timeout(Wait::from_secs(2), async {
        loop {
            let mark: Option<chrono::DateTime<Utc>> =
                sqlx::query_scalar("SELECT as_of FROM economic_token_marks WHERE asset_id=$1")
                    .bind(token)
                    .fetch_optional(&pool)
                    .await
                    .unwrap();
            if mark == Some(replacement) {
                break;
            }
            tokio::time::sleep(Wait::from_millis(20)).await;
        }
    })
    .await;
    worker.abort();
    let _ = worker.await;
    blocker.rollback().await.unwrap();
    updated.expect("pool completion must refresh the next atomic cohort before the5s fallback, while accounting is still blocked");
    let indexed = assets::get_all_assets(&pool).await.unwrap();
    let references = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert_eq!(references.prices[&token].as_of, replacement);
    assert_eq!(references.prices[&anchor].as_of, replacement);
    assert!(references
        .fair_output(
            anchor,
            token,
            1_000_000,
            original + Duration::milliseconds(20_100)
        )
        .is_some());
    assert!(references
        .fair_output(
            anchor,
            token,
            1_000_000,
            replacement + Duration::milliseconds(20_001)
        )
        .is_none());
}

#[tokio::test]
#[serial_test::serial]
async fn issue1505_reference_refresh_preserves_supplied_oracle_qualification() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let original = chrono::DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).unwrap();
    sqlx::query(
        "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
        VALUES($1,true,'Canonical LUNC wrap','cLUNC',6)",
    )
    .bind(DEFAULT_HUB_CLUNC_ADDRESS)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of,observed_at)
        VALUES('lunc',0.00005,'fresh',ARRAY['mexc','kucoin'],$1,$1)",
    )
    .bind(original)
    .execute(&pool)
    .await
    .unwrap();
    let cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    assert!(
        hub_prices::refresh_hub_prices(&pool, &cfg, None, None)
            .await
            .unwrap()
            .lunc
            .is_none(),
        "a certified DB row cannot replace an absent caller observation"
    );
    let mismatched = "0.00006".parse().unwrap();
    assert!(
        hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&mismatched))
            .await
            .unwrap()
            .lunc
            .is_none(),
        "a mismatched caller observation must stay unqualified"
    );
    let matching = "0.00005".parse().unwrap();
    let supported = hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&matching))
        .await
        .unwrap();
    assert_eq!(
        supported.lunc.unwrap().as_of,
        Some(original),
        "matching qualification keeps original oracle time"
    );
    sqlx::query("UPDATE oracle_state SET accepted_sources=ARRAY[]::TEXT[],quality='degraded',price_usd=NULL,as_of=NULL WHERE ticker='lunc'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&matching))
            .await
            .unwrap()
            .lunc
            .is_none(),
        "caller equality cannot waive the source quorum"
    );
}
