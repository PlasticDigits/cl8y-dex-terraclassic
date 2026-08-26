mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

async fn app_with_seed() -> (TestServer, common::SeedData, sqlx::PgPool) {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 1004)
        .await
        .expect("checkpoint");
    cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
        &pool,
        seed.pair_id,
        &"2000000".parse().unwrap(),
        &"1900000".parse().unwrap(),
        30,
        Some(1004),
    )
    .await
    .expect("reserves");
    let app = common::build_test_app(pool.clone()).await;
    (TestServer::new(app), seed, pool)
}

#[serial]
#[tokio::test]
async fn gt_latest_block_uses_checkpoint() {
    let (server, _, _) = app_with_seed().await;
    let resp = server.get("/gt/latest-block").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["block"]["blockNumber"], 1004);
    assert!(body["block"]["blockTimestamp"].as_i64().unwrap() > 0);
}

#[serial]
#[tokio::test]
async fn gt_asset_native_and_cw20() {
    let (server, seed, pool) = app_with_seed().await;
    let luna = server.get("/gt/asset?id=uluna").await;
    luna.assert_status_ok();
    let body: Value = luna.json();
    assert_eq!(body["asset"]["id"], "uluna");
    assert_eq!(body["asset"]["symbol"], "LUNC");
    assert_eq!(body["asset"]["decimals"], 6);

    let cw20 = server.get("/gt/asset?id=terra1ustctoken").await;
    cw20.assert_status_ok();
    assert_eq!(cw20.json::<Value>()["asset"]["symbol"], "USTC");

    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals, coingecko_id)
         VALUES ($1, true, 'Ceramic Liberty', 'CL8Y', 18, 'ceramicliberty-com')",
    )
    .bind("terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3")
    .execute(&pool)
    .await
    .expect("cl8y");
    let cl8y = server
        .get("/gt/asset?id=terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3")
        .await;
    cl8y.assert_status_ok();
    assert_eq!(
        cl8y.json::<Value>()["asset"]["coinGeckoId"],
        "ceramicliberty-com"
    );
    let _ = seed;
}

#[serial]
#[tokio::test]
async fn gt_pair_and_events() {
    let (server, seed, _) = app_with_seed().await;
    let pair = server
        .get(&format!("/gt/pair?id={}", seed.pair_address))
        .await;
    pair.assert_status_ok();
    let body: Value = pair.json();
    assert_eq!(body["pair"]["id"], seed.pair_address);
    assert_eq!(body["pair"]["dexKey"], "cl8y");
    assert_eq!(body["pair"]["asset0Id"], "uluna");
    assert_eq!(body["pair"]["asset1Id"], "terra1ustctoken");
    assert_eq!(body["pair"]["feeBps"], 30);

    let ev = server.get("/gt/events?fromBlock=1000&toBlock=1004").await;
    ev.assert_status_ok();
    let body: Value = ev.json();
    let events = body["events"].as_array().unwrap();
    assert!(events.iter().any(|e| e["eventType"] == "swap"));
    assert!(events.iter().any(|e| e["eventType"] == "join"));
    let swap = events.iter().find(|e| e["eventType"] == "swap").unwrap();
    assert_eq!(swap["pairId"], seed.pair_address);
    assert_eq!(swap["asset0In"], "0.001");
    assert_eq!(swap["asset1Out"], "0.00095");
    assert_eq!(swap["priceNative"], "0.95");
    assert_eq!(swap["reserves"]["asset0"], "2");
    assert!(swap["block"]["blockNumber"].as_i64().unwrap() >= 1000);
}

#[serial]
#[tokio::test]
async fn gt_events_rejects_bad_range_and_hides_gems() {
    let (server, _, pool) = app_with_seed().await;
    server
        .get("/gt/events?fromBlock=10&toBlock=1")
        .await
        .assert_status_bad_request();
    server
        .get("/gt/events?fromBlock=1&toBlock=3000")
        .await
        .assert_status_bad_request();
    server.get("/gt/asset").await.assert_status_bad_request();
    server
        .get("/gt/pair?id=terra1missing")
        .await
        .assert_status_not_found();

    let gem: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94', true, 'EMBER', 'EMBER', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("gem");
    let lunc: i32 = sqlx::query_scalar("SELECT id FROM assets WHERE denom = 'uluna'")
        .fetch_one(&pool)
        .await
        .expect("lunc");
    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, fee_bps)
         VALUES ('terra1gempairgt', $1, $2, 30)",
    )
    .bind(gem)
    .bind(lunc)
    .execute(&pool)
    .await
    .expect("gem pair");
    server
        .get("/gt/pair?id=terra1gempairgt")
        .await
        .assert_status_not_found();
}
