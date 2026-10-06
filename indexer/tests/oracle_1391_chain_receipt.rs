//! Replay actual isolated LocalTerra reserves through the production hub processor.
//! Bootstrap timing is advanced in the fixture; no deployed indexer claim is made.
mod common;

use bigdecimal::BigDecimal;
use cl8y_dex_indexer::{db::queries::hub_prices, indexer::hub_usd::HubUsdConfig};
use serde_json::Value;
use sqlx::PgPool;
use std::{collections::HashMap, time::Duration};

async fn apply_stage(
    pool: &PgPool,
    stage: &Value,
    pairs: &HashMap<String, i32>,
    assets: &HashMap<String, i32>,
) {
    for (name, fixture) in stage["pairs"].as_object().expect("pairs") {
        let legs = fixture["pool"]["assets"].as_array().expect("assets");
        let address0 = legs[0]["info"]["token"]["contract_addr"]
            .as_str()
            .expect("CW20");
        let address1 = legs[1]["info"]["token"]["contract_addr"]
            .as_str()
            .expect("CW20");
        sqlx::query("UPDATE pairs SET asset_0_id=$2, asset_1_id=$3 WHERE id=$1")
            .bind(pairs[name])
            .bind(assets[address0])
            .bind(assets[address1])
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO pair_reserves (pair_id,reserve_0,reserve_1,fee_bps,snapshot_at)
            VALUES ($1,$2,$3,30,NOW()) ON CONFLICT (pair_id) DO UPDATE SET
            reserve_0=EXCLUDED.reserve_0,reserve_1=EXCLUDED.reserve_1,snapshot_at=EXCLUDED.snapshot_at")
            .bind(pairs[name])
            .bind(legs[0]["amount"].as_str().unwrap().parse::<BigDecimal>().unwrap())
            .bind(legs[1]["amount"].as_str().unwrap().parse::<BigDecimal>().unwrap())
            .execute(pool).await.unwrap();
    }
}

#[tokio::test]
async fn real_canonical_reserve_shift_and_lp_withdrawal_are_contained() {
    let Ok(path) = std::env::var("ORACLE1391_CHAIN_RECEIPT") else {
        eprintln!("SKIP: set ORACLE1391_CHAIN_RECEIPT to a localterra-oracle-1391.py receipt");
        return;
    };
    let receipt: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("receipt file"))
            .expect("receipt JSON");
    assert!(receipt["transaction_hashes"].as_array().unwrap().len() >= 19);
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let mut cfg = HubUsdConfig::from_indexer_config(&common::test_config());
    cfg.custc_address = receipt["tokens"]["custc"].as_str().unwrap().into();
    cfg.ust1_address = receipt["tokens"]["ust1"].as_str().unwrap().into();
    cfg.ustr_address = receipt["tokens"]["ustr"].as_str().unwrap().into();
    cfg.max_staleness = Duration::from_secs(120);
    let mut assets = HashMap::new();
    for (ticker, address) in receipt["tokens"].as_object().unwrap() {
        let address = address.as_str().unwrap();
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO assets (contract_address,is_cw20,name,symbol,decimals)
            VALUES ($1,true,$2,$2,6) RETURNING id",
        )
        .bind(address)
        .bind(ticker)
        .fetch_one(&pool)
        .await
        .unwrap();
        assets.insert(address.to_owned(), id);
    }
    let mut pairs = HashMap::new();
    let stages = receipt["stages"].as_array().unwrap();
    for (name, fixture) in stages[0]["pairs"].as_object().unwrap() {
        let legs = fixture["pool"]["assets"].as_array().unwrap();
        let id: i32 = sqlx::query_scalar(
            "INSERT INTO pairs (contract_address,asset_0_id,asset_1_id,lp_token,fee_bps)
            VALUES ($1,$2,$3,$4,30) RETURNING id",
        )
        .bind(fixture["address"].as_str().unwrap())
        .bind(assets[legs[0]["info"]["token"]["contract_addr"].as_str().unwrap()])
        .bind(assets[legs[1]["info"]["token"]["contract_addr"].as_str().unwrap()])
        .bind(fixture["lp_token"].as_str().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        pairs.insert(name.clone(), id);
    }
    let oracle: BigDecimal = "0.005".parse().unwrap();
    apply_stage(&pool, &stages[0], &pairs, &assets).await;
    let boot = hub_prices::refresh_hub_prices(&pool, &cfg, Some(&oracle), None)
        .await
        .unwrap();
    assert!(
        boot.ust1.is_none(),
        "bootstrap cannot immediately claim healthy spot"
    );
    // Advance only the reference fixture clock; reserve amounts remain actual chain receipts.
    sqlx::query(
        "UPDATE hub_price_references SET pending_since=pending_since-INTERVAL '61 seconds'",
    )
    .execute(&pool)
    .await
    .unwrap();
    apply_stage(&pool, &stages[0], &pairs, &assets).await;
    let before = hub_prices::refresh_hub_prices(&pool, &cfg, Some(&oracle), None)
        .await
        .unwrap();
    assert!(
        before.ust1.is_some(),
        "healthy control must confirm canonical pool"
    );
    assert!(
        before.ustr.is_some(),
        "healthy direct cUSTC-quoted USTR control must confirm"
    );
    apply_stage(&pool, &stages[1], &pairs, &assets).await;
    let manipulated = hub_prices::refresh_hub_prices(&pool, &cfg, Some(&oracle), None)
        .await
        .unwrap();
    assert!(
        manipulated.ust1.is_none(),
        "actual canonical reserve shift must suppress UST1"
    );
    assert!(
        manipulated.ustr.is_some(),
        "independent cUSTC-quoted USTR remains usable"
    );
    apply_stage(&pool, &stages[2], &pairs, &assets).await;
    let withdrawn = hub_prices::refresh_hub_prices(&pool, &cfg, Some(&oracle), None)
        .await
        .unwrap();
    assert!(withdrawn.ust1.is_none());
    assert!(
        withdrawn.ustr.is_none(),
        "depth loss cannot switch to manipulated UST1 anchor"
    );
}
