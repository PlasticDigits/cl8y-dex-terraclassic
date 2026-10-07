//! Opt-in actual LocalTerra LP withdrawal, LCD outage and recovery control.
//! Run via scripts/qa/localterra-liquidity-1385.py on a dedicated owned chain.
mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::{
    api::reset_overview_cache,
    db::queries::state,
    indexer::{book_snapshot, oracle::OracleTicker, protocol_tvl},
    lcd::LcdClient,
};
use serde_json::{json, Value};
use std::{env, fs, path::Path, process::Command, str::FromStr, time::Duration};

async fn refresh(pool: &sqlx::PgPool, endpoint: &str) -> Value {
    let lcd = LcdClient::new(vec![endpoint.into()], 1500, 100, "localterra");
    book_snapshot::snapshot_all_pairs(pool, &lcd).await.unwrap();
    protocol_tvl::refresh_protocol_liquidity_with_staleness(pool, Duration::from_secs(20))
        .await
        .unwrap();
    reset_overview_cache();
    let server = TestServer::new(common::build_test_app(pool.clone()).await);
    let response = server.get("/api/v1/overview").await;
    response.assert_status_ok();
    let body: Value = response.json();
    json!({"total": body["total_liquidity_usd"], "quality": body["liquidity_quality"],
           "as_of": body["liquidity_as_of"], "generation": body["liquidity_generation"],
           "identity": body["liquidity_coverage_identity"], "reasons": body["liquidity_reasons"],
           "delta_24h": body["liquidity_change_24h_pct"], "delta_30d": body["liquidity_change_30d_pct"]})
}

async fn reserve(pool: &sqlx::PgPool) -> (BigDecimal, BigDecimal, chrono::DateTime<Utc>) {
    sqlx::query_as("SELECT reserve_0,reserve_1,snapshot_at FROM pair_reserves")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "needs owned funded LocalTerra fixture and dedicated QA database capability"]
async fn actual_lp_withdrawal_outage_recovery() {
    let fixture_path = env::var("LIQUIDITY1385_FIXTURE").unwrap();
    let fixture: Value = serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
    let endpoint = env::var("LIQUIDITY1385_LCD_URL").unwrap();
    let flag = env::var("LIQUIDITY1385_OUTAGE_FLAG").unwrap();
    let output = env::var("LIQUIDITY1385_EVIDENCE").unwrap();
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let mut ids = vec![];
    for symbol in ["CLUNC", "CUSTC"] {
        let id: i32 = sqlx::query_scalar("INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals) VALUES($1,true,$2,$2,6) RETURNING id")
            .bind(fixture["tokens"][symbol].as_str().unwrap()).bind(symbol).fetch_one(&pool).await.unwrap();
        ids.push(id);
    }
    // Actual Pool response defines asset order; preserve it rather than assume sorting.
    let before_assets = fixture["before"]["assets"].as_array().unwrap();
    let first_token = before_assets[0]["info"]["token"]["contract_addr"]
        .as_str()
        .unwrap();
    if first_token == fixture["tokens"]["CUSTC"].as_str().unwrap() {
        ids.swap(0, 1);
    }
    sqlx::query("INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) VALUES($1,$2,$3,$4,30)")
        .bind(fixture["pair"].as_str().unwrap()).bind(ids[0]).bind(ids[1])
        .bind(fixture["lp_token"].as_str().unwrap()).execute(&pool).await.unwrap();
    state::set_state(
        &pool,
        "factory_inventory_observed_at",
        &Utc::now().to_rfc3339(),
    )
    .await
    .unwrap();
    // Fixed prices are explicit synthetic test inputs, independent of chain reserve evidence.
    for (ticker, price) in [(OracleTicker::Lunc, "0.0001"), (OracleTicker::Ustc, "0.01")] {
        cl8y_dex_indexer::db::queries::oracle::record_observation(
            &pool,
            ticker,
            Some(&BigDecimal::from_str(price).unwrap()),
            &["kucoin".into(), "mexc".into()],
            "fresh",
        )
        .await
        .unwrap();
    }
    let initial = refresh(&pool, &endpoint).await;
    let initial_reserves = reserve(&pool).await;
    assert_eq!(initial["quality"], "fresh");
    assert_eq!(
        BigDecimal::from_str(initial["total"].as_str().unwrap()).unwrap(),
        BigDecimal::from_str("10.1").unwrap()
    );
    fs::write(&flag, b"owned LCD transport outage").unwrap();
    // Real elapsed TTL, no database timestamp rewrite or synthetic reserve replacement.
    tokio::time::sleep(Duration::from_secs(21)).await;
    let outage = refresh(&pool, &endpoint).await;
    assert!(outage["total"].is_null());
    assert!(matches!(
        outage["quality"].as_str(),
        Some("stale" | "unavailable")
    ));
    assert!(outage["delta_24h"].is_null() && outage["delta_30d"].is_null());
    assert_eq!(
        reserve(&pool).await,
        initial_reserves,
        "failed LCD refresh retains actual reserves and original timestamp"
    );
    fs::remove_file(&flag).unwrap();
    let recovery = refresh(&pool, &endpoint).await;
    assert_eq!(recovery["quality"], "fresh");
    assert_eq!(recovery["total"], initial["total"]);
    let recovered_reserves = reserve(&pool).await;
    assert_eq!(recovered_reserves.0, initial_reserves.0);
    assert_eq!(recovered_reserves.1, initial_reserves.1);
    assert!(recovered_reserves.2 > initial_reserves.2);
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("scripts/qa/localterra-liquidity-1385.py");
    let status = Command::new("python3")
        .arg(script)
        .arg("--container")
        .arg(fixture["container"].as_str().unwrap())
        .arg("--evidence-dir")
        .arg(Path::new(&fixture_path).parent().unwrap())
        .arg("--withdraw")
        .status()
        .unwrap();
    assert!(status.success());
    let withdrawal = refresh(&pool, &endpoint).await;
    assert_eq!(withdrawal["quality"], "fresh");
    assert_eq!(withdrawal["identity"], initial["identity"]);
    let final_reserves = reserve(&pool).await;
    assert!(final_reserves.0 < initial_reserves.0 / 2 && final_reserves.1 < initial_reserves.1 / 2);
    let total = BigDecimal::from_str(withdrawal["total"].as_str().unwrap()).unwrap();
    assert!(total > BigDecimal::from(0) && total < BigDecimal::from_str("2").unwrap());
    // Quality source slot contains the actual terminal withdrawal rather than last-good smoothing.
    let persisted: (String, Option<BigDecimal>) = sqlx::query_as("SELECT quality,total_liquidity_usd FROM global_liquidity_snapshots ORDER BY sampled_at DESC LIMIT 1")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(persisted.0, "fresh");
    assert_eq!(persisted.1, Some(total));
    fs::write(output, serde_json::to_string_pretty(&json!({"status":"PASS",
        "scope":"actual dedicated LocalTerra reserves and LP withdrawal; synthetic fixed quotes; disposable QA database",
        "initial":initial,"outage":outage,"recovery":recovery,"withdrawal":withdrawal})).unwrap()).unwrap();
    println!("PASS actual LP withdrawal and LCD outage/recovery control");
    pool.close().await;
}
