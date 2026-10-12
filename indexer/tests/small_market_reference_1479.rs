//! Owned disposable SQL publication; no chain requests or synthetic spot admission.
mod common;

use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::{
    api::ReferencePrices,
    config::DEFAULT_HUB_CLUNC_ADDRESS,
    db::queries::{assets, hub_prices},
    indexer::hub_usd::HubUsdConfig,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn issue1479_scheduled_small_market_publication_separates_policy_loss_from_missing_data() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let original = chrono::DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).unwrap();
    let token: i32 = sqlx::query_scalar(
        "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
         VALUES('terra1small1479xxxxxxxxxxxxxxxxxx',true,'Small market','SMALL',6) RETURNING id",
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
         VALUES('terra1smallpair1479',$1,$2,180) RETURNING id",
    )
    .bind(token)
    .bind(anchor)
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO pair_reserves(pair_id,reserve_0,reserve_1,fee_bps,snapshot_at)
         VALUES($1,110044622670234,252903132784,180,$2)",
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
    let mut cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    let lunc: BigDecimal = "0.00005".parse().unwrap();
    let indexed = assets::get_all_assets(&pool).await.unwrap();
    // Reproduce former $100 total-TVL exclusion despite $25.29 valuation.
    cfg.economic_tvl_floor = 100.into();
    hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&lunc))
        .await
        .unwrap();
    let excluded = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert!(excluded.policy_excluded(token, Utc::now()));
    assert!(excluded
        .fair_output(anchor, token, 1_000_000, Utc::now())
        .is_none());
    // Explicit revised $10 economic policy retains the $100 hub trust floor.
    cfg.economic_tvl_floor = 10.into();
    hub_prices::refresh_hub_prices(&pool, &cfg, None, Some(&lunc))
        .await
        .unwrap();
    let ready = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert_eq!(cfg.tvl_floor, BigDecimal::from(100));
    assert!(!ready.policy_excluded(token, Utc::now()));
    assert_eq!(ready.prices[&token].as_of, original);
    assert!(
        ready
            .fair_output(anchor, token, 1_000_000, Utc::now())
            .unwrap()
            .0
            > 400_000_000
    );
    assert!(
        ready
            .fair_output(token, anchor, 1_000_000_000, Utc::now())
            .unwrap()
            .0
            > 2_000_000
    );
    assert_ne!(ready.policy_identity, excluded.policy_identity);
    assert!(ready
        .fair_output(anchor, token, 1_000_000, original + Duration::seconds(21))
        .is_none());
    // A newer mark cannot renew its original qualified anchor's remaining TTL.
    let old_anchor = chrono::DateTime::from_timestamp_micros(
        (Utc::now() - Duration::seconds(19)).timestamp_micros(),
    )
    .unwrap();
    sqlx::query("UPDATE hub_prices SET as_of=$1 WHERE ticker='lunc'")
        .bind(old_anchor)
        .execute(&pool)
        .await
        .unwrap();
    let inherited = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert_eq!(inherited.prices[&token].as_of, old_anchor);
    assert!(inherited
        .fair_output(
            anchor,
            token,
            1_000_000,
            old_anchor + Duration::milliseconds(20_001)
        )
        .is_none());
    sqlx::query("UPDATE hub_prices SET as_of=$1 WHERE ticker='lunc'")
        .bind(original)
        .execute(&pool)
        .await
        .unwrap();
    // A policy increase immediately refuses an old smaller-floor mark, before
    // the scheduled economic producer has replaced its database publication.
    cfg.economic_tvl_floor = 100.into();
    let tightened = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert!(!tightened.prices.contains_key(&token));
    assert!(tightened.policy_excluded(token, Utc::now()));
    cfg.economic_tvl_floor = 10.into();
    // Losing quorum is recoverable missing data, never a permanent floor claim.
    assert!(sqlx::query(
        "UPDATE oracle_state SET accepted_sources=ARRAY['mexc'] WHERE ticker='lunc'"
    )
    .execute(&pool)
    .await
    .is_err());
    sqlx::query("UPDATE oracle_state SET quality='degraded',price_usd=NULL,accepted_sources='{}',as_of=NULL WHERE ticker='lunc'")
        .execute(&pool)
        .await
        .unwrap();
    let missing = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert!(!missing.prices.contains_key(&token));
    assert!(!missing.policy_excluded(token, Utc::now()));
    assert_eq!(
        missing.unavailable_reason(token, Utc::now()),
        Some("missing_observations")
    );
    // Future source provenance cannot make this market ready.
    sqlx::query("UPDATE oracle_state SET quality='fresh',price_usd=0.00005,accepted_sources=ARRAY['mexc','kucoin'],as_of=NOW()+INTERVAL '1 minute' WHERE ticker='lunc'")
        .execute(&pool).await.unwrap();
    let future = ReferencePrices::observe(&pool, &indexed, &cfg)
        .await
        .unwrap();
    assert!(future.prices.is_empty());
    assert!(future.policy_exclusions.is_empty());
}

#[tokio::test]
#[serial]
async fn issue1479_native_anchor_source_identity_is_exact_and_kind_bound() {
    use cl8y_dex_indexer::config::DEFAULT_HUB_CUSTC_ADDRESS;
    let pool = common::setup_pool().await;
    let cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    for (denom, ticker, wrapped, usd, anchor_raw) in [
        (
            "uluna",
            "lunc",
            DEFAULT_HUB_CLUNC_ADDRESS,
            "0.00005",
            "250000000000",
        ),
        (
            "uusd",
            "ustc",
            DEFAULT_HUB_CUSTC_ADDRESS,
            "0.005",
            "2500000000",
        ),
    ] {
        common::clean_db(&pool).await;
        sqlx::query(
            "INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
            VALUES($1,true,'Canonical wrap','untrusted-symbol',6)",
        )
        .bind(wrapped)
        .execute(&pool)
        .await
        .unwrap();
        let token:i32=sqlx::query_scalar("INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
            VALUES('terra1native1479xxxxxxxxxxxxxxxxxxxxx',true,'Small token','SMALL',6) RETURNING id")
            .fetch_one(&pool).await.unwrap();
        let native: i32 = sqlx::query_scalar(
            "INSERT INTO assets(denom,is_cw20,name,symbol,decimals)
            VALUES($1,false,'Native anchor','spoofed-symbol',6) RETURNING id",
        )
        .bind(denom)
        .fetch_one(&pool)
        .await
        .unwrap();
        let pair: i32 = sqlx::query_scalar(
            "INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,fee_bps)
            VALUES('terra1nativepair1479',$1,$2,180) RETURNING id",
        )
        .bind(token)
        .bind(native)
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO pair_reserves(pair_id,reserve_0,reserve_1,fee_bps,snapshot_at)
            VALUES($1,100000000000,$2,180,NOW())",
        )
        .bind(pair)
        .bind(anchor_raw.parse::<BigDecimal>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of)
            VALUES($1,$2,'fresh',ARRAY['mexc','kucoin'],NOW())",
        )
        .bind(ticker)
        .bind(usd.parse::<BigDecimal>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
        let price: BigDecimal = usd.parse().unwrap();
        hub_prices::refresh_hub_prices(
            &pool,
            &cfg,
            (ticker == "ustc").then_some(&price),
            (ticker == "lunc").then_some(&price),
        )
        .await
        .unwrap();
        let indexed = assets::get_all_assets(&pool).await.unwrap();
        let reference = ReferencePrices::observe(&pool, &indexed, &cfg)
            .await
            .unwrap();
        assert!(
            reference.prices.contains_key(&token),
            "canonical {denom} anchor must remain supported"
        );
        let expected: BigDecimal =
            anchor_raw.parse::<BigDecimal>().unwrap() * price * BigDecimal::from(2)
                / BigDecimal::from(1_000_000);
        let stored: BigDecimal =
            sqlx::query_scalar("SELECT tvl_usd FROM economic_token_marks WHERE asset_id=$1")
                .bind(token)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, expected);
        let spoof:i32=sqlx::query_scalar("INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals)
            VALUES('terra1spoof1479xxxxxxxxxxxxxxxxxxxxxx',true,'Spoof token','SMALL',6) RETURNING id")
            .fetch_one(&pool).await.unwrap();
        sqlx::query("UPDATE economic_token_marks SET contract_address='terra1spoof1479xxxxxxxxxxxxxxxxxxxxxx' WHERE asset_id=$1")
            .bind(token).execute(&pool).await.unwrap();
        let with_spoof = assets::get_all_assets(&pool).await.unwrap();
        let poisoned = ReferencePrices::observe(&pool, &with_spoof, &cfg)
            .await
            .unwrap();
        assert!(!poisoned.prices.contains_key(&token));
        assert!(!poisoned.prices.contains_key(&spoof));
        sqlx::query("UPDATE economic_token_marks SET contract_address='terra1native1479xxxxxxxxxxxxxxxxxxxxx' WHERE asset_id=$1")
            .bind(token).execute(&pool).await.unwrap();
        // The mark cannot be borrowed from a different source pair identity.
        sqlx::query("UPDATE economic_token_marks SET source_pair_address='terra1spoofedpair1479' WHERE asset_id=$1")
            .bind(token).execute(&pool).await.unwrap();
        assert!(!ReferencePrices::observe(&pool, &indexed, &cfg)
            .await
            .unwrap()
            .prices
            .contains_key(&token));
        sqlx::query("UPDATE economic_token_marks SET source_pair_address='terra1nativepair1479' WHERE asset_id=$1")
            .bind(token).execute(&pool).await.unwrap();
        // A symbol cannot substitute for exact native denom/kind identity.
        sqlx::query("UPDATE assets SET denom='spoofed-native',symbol=$1 WHERE id=$2")
            .bind(denom)
            .bind(native)
            .execute(&pool)
            .await
            .unwrap();
        assert!(!ReferencePrices::observe(&pool, &indexed, &cfg)
            .await
            .unwrap()
            .prices
            .contains_key(&token));
        sqlx::query("UPDATE assets SET denom=$1,is_cw20=true WHERE id=$2")
            .bind(denom)
            .bind(native)
            .execute(&pool)
            .await
            .unwrap();
        assert!(!ReferencePrices::observe(&pool, &indexed, &cfg)
            .await
            .unwrap()
            .prices
            .contains_key(&token));
    }
}
