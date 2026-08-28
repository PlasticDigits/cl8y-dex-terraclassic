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
    // Seeded rows have NULL reserve_* (pre-#684). GET emits "0", never the live snapshot.
    assert_eq!(swap["reserves"]["asset0"], "0");
    assert_eq!(swap["reserves"]["asset1"], "0");
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

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::swap_events;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[allow(clippy::too_many_arguments)]
async fn put_swap(
    pool: &sqlx::PgPool,
    seed: &common::SeedData,
    height: i64,
    tx: &str,
    swap_index: i32,
    offer_is_asset_0: bool,
    offer: &str,
    ret: &str,
    price: &str,
    pool_ret: Option<&str>,
    book_ret: Option<&str>,
    consumed: Option<&str>,
    commission: Option<&str>,
    r0: Option<&str>,
    r1: Option<&str>,
) {
    let offer_id = if offer_is_asset_0 {
        seed.asset_0_id
    } else {
        seed.asset_1_id
    };
    let ask_id = if offer_is_asset_0 {
        seed.asset_1_id
    } else {
        seed.asset_0_id
    };
    let pool_bd = pool_ret.map(bd);
    let book_bd = book_ret.map(bd);
    let cons_bd = consumed.map(bd);
    let comm_bd = commission.map(bd);
    let r0_bd = r0.map(bd);
    let r1_bd = r1.map(bd);
    swap_events::insert_swap(
        pool,
        seed.pair_id,
        swap_index,
        height,
        Utc::now(),
        tx,
        &seed.trader_address,
        None,
        offer_id,
        ask_id,
        &bd(offer),
        &bd(ret),
        None,
        comm_bd.as_ref(),
        None,
        &bd(price),
        None,
        None,
        pool_bd.as_ref(),
        book_bd.as_ref(),
        cons_bd.as_ref(),
        r0_bd.as_ref(),
        r1_bd.as_ref(),
    )
    .await
    .expect("swap")
    .expect("inserted");
}

fn swap_events<'a>(body: &'a Value, pair: &str) -> Vec<&'a Value> {
    body["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["eventType"] == "swap" && e["pairId"] == pair)
        .collect()
}

#[serial]
#[tokio::test]
async fn gt_events_t1_t2_pool_swaps_ignore_live_snapshot() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    put_swap(
        &pool,
        &seed,
        2001,
        "txa",
        0,
        true,
        "1000000",
        "950000",
        "0.95",
        None,
        None,
        None,
        None,
        Some("185000000"),
        Some("1790000000"),
    )
    .await;
    put_swap(
        &pool,
        &seed,
        2002,
        "txb",
        0,
        true,
        "1000000",
        "940000",
        "0.94",
        None,
        None,
        None,
        None,
        Some("186000000"),
        Some("1789060000"),
    )
    .await;
    cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
        &pool,
        seed.pair_id,
        &bd("999999999"),
        &bd("888888888"),
        30,
        Some(2002),
    )
    .await
    .expect("poison snapshot");
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2002)
        .await
        .unwrap();

    let ev = server.get("/gt/events?fromBlock=2001&toBlock=2002").await;
    ev.assert_status_ok();
    let body: Value = ev.json();
    let swaps = swap_events(&body, &seed.pair_address);
    assert_eq!(swaps.len(), 2);
    assert_eq!(swaps[0]["reserves"]["asset0"], "185");
    assert_eq!(swaps[1]["reserves"]["asset0"], "186");
    assert_ne!(swaps[0]["reserves"], swaps[1]["reserves"]);
    assert_ne!(swaps[0]["reserves"]["asset0"], "999.999999");
}

#[serial]
#[tokio::test]
async fn gt_events_t3_t4_hybrid_and_book_only() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    // Pre 100/200 human (raw 1e8 / 2e8). Hybrid: pool_in 40, gross 31 → 140 / 169.
    put_swap(
        &pool,
        &seed,
        2100,
        "txhyb",
        0,
        true,
        "100",
        "90",
        "0.9",
        Some("30"),
        Some("60"),
        Some("60"),
        Some("1"),
        Some("140"),
        Some("169"),
    )
    .await;
    put_swap(
        &pool,
        &seed,
        2101,
        "txbook",
        0,
        true,
        "100",
        "55",
        "0.55",
        Some("0"),
        Some("55"),
        Some("100"),
        Some("0"),
        Some("140"),
        Some("169"),
    )
    .await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2101)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2100&toBlock=2101")
        .await
        .json();
    let swaps = swap_events(&body, &seed.pair_address);
    assert_eq!(swaps.len(), 2);
    assert_eq!(
        swaps[0]["reserves"]["asset0"],
        swaps[1]["reserves"]["asset0"]
    );
    assert_eq!(
        swaps[0]["reserves"]["asset1"],
        swaps[1]["reserves"]["asset1"]
    );
}

#[serial]
#[tokio::test]
async fn gt_events_t5_join_swap_exit_running_reserves() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query(
        "INSERT INTO liquidity_events
         (pair_id, block_height, block_timestamp, tx_hash, provider, event_type,
          asset_0_amount, asset_1_amount, lp_amount, reserve_0, reserve_1)
         VALUES ($1, 2200, NOW(), 'txjoin', $2, 'add', 1000000, 2000000, 500, 1000000, 2000000)",
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .execute(&pool)
    .await
    .unwrap();
    put_swap(
        &pool,
        &seed,
        2201,
        "txsw",
        0,
        true,
        "100000",
        "180000",
        "1.8",
        None,
        None,
        None,
        None,
        Some("1100000"),
        Some("1820000"),
    )
    .await;
    sqlx::query(
        "INSERT INTO liquidity_events
         (pair_id, block_height, block_timestamp, tx_hash, provider, event_type,
          asset_0_amount, asset_1_amount, lp_amount, reserve_0, reserve_1)
         VALUES ($1, 2202, NOW(), 'txexit', $2, 'remove', 100000, 165455, 50, 1000000, 1654545)",
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .execute(&pool)
    .await
    .unwrap();
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2202)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2200&toBlock=2202")
        .await
        .json();
    let events = body["events"].as_array().unwrap();
    let join = events.iter().find(|e| e["eventType"] == "join").unwrap();
    let swap = events.iter().find(|e| e["eventType"] == "swap").unwrap();
    let exit = events.iter().find(|e| e["eventType"] == "exit").unwrap();
    assert_eq!(join["reserves"]["asset0"], "1");
    assert_eq!(swap["reserves"]["asset0"], "1.1");
    assert_eq!(exit["reserves"]["asset0"], "1");
}

#[serial]
#[tokio::test]
async fn gt_events_t6_same_tx_swap_index_uses_prior_post() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    put_swap(
        &pool,
        &seed,
        2300,
        "txsame",
        0,
        true,
        "10",
        "9",
        "0.9",
        None,
        None,
        None,
        None,
        Some("100"),
        Some("200"),
    )
    .await;
    put_swap(
        &pool,
        &seed,
        2300,
        "txsame",
        1,
        true,
        "10",
        "8",
        "0.8",
        None,
        None,
        None,
        None,
        Some("110"),
        Some("192"),
    )
    .await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2300)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2300&toBlock=2300")
        .await
        .json();
    let swaps = swap_events(&body, &seed.pair_address);
    assert_eq!(swaps.len(), 2);
    assert_eq!(swaps[0]["eventIndex"], 0);
    assert_eq!(swaps[1]["eventIndex"], 1);
    assert_eq!(swaps[0]["reserves"]["asset0"], "0.0001");
    assert_eq!(swaps[1]["reserves"]["asset0"], "0.00011");
}

#[serial]
#[tokio::test]
async fn gt_events_t8_offer_asset1_orientation() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    put_swap(
        &pool,
        &seed,
        2400,
        "txq",
        0,
        false,
        "1000000",
        "500000",
        "2",
        None,
        None,
        None,
        None,
        Some("500000"),
        Some("2000000"),
    )
    .await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2400)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2400&toBlock=2400")
        .await
        .json();
    let swap = swap_events(&body, &seed.pair_address)[0];
    assert_eq!(swap["asset1In"], "1");
    assert_eq!(swap["asset0Out"], "0.5");
    assert_eq!(swap["reserves"]["asset0"], "0.5");
    assert_eq!(swap["reserves"]["asset1"], "2");
}

#[serial]
#[tokio::test]
async fn gt_events_t10_t13_empty_and_future() {
    let (server, _, _) = app_with_seed().await;
    let future = server.get("/gt/events?fromBlock=9000&toBlock=9001").await;
    future.assert_status_ok();
    assert_eq!(
        future.json::<Value>()["events"].as_array().unwrap().len(),
        0
    );

    let empty = server.get("/gt/events?fromBlock=3&toBlock=4").await;
    empty.assert_status_ok();
    assert_eq!(empty.json::<Value>()["events"].as_array().unwrap().len(), 0);
}

#[serial]
#[tokio::test]
async fn gt_events_t11_span_and_injection_400() {
    let (server, _, _) = app_with_seed().await;
    server
        .get("/gt/events?fromBlock=1&toBlock=2001")
        .await
        .assert_status_bad_request();
    server
        .get("/gt/events?fromBlock=10&toBlock=1")
        .await
        .assert_status_bad_request();
    server
        .get("/gt/events?fromBlock=1;DROP&toBlock=2")
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn gt_events_t12_omits_gem_pair_swaps() {
    let (server, seed, pool) = app_with_seed().await;
    let gem_id: i32 = if let Some(id) = sqlx::query_scalar(
        "SELECT id FROM assets WHERE contract_address = 'terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap()
    {
        id
    } else {
        sqlx::query_scalar(
            "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
             VALUES ('terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94', true, 'EMBER', 'EMBER', 6)
             RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .expect("gem insert")
    };
    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, fee_bps)
         VALUES ('terra1gemswapgt', $1, $2, 30)
         ON CONFLICT (contract_address) DO NOTHING",
    )
    .bind(gem_id)
    .bind(seed.asset_0_id)
    .execute(&pool)
    .await
    .ok();
    let gem_pair: i32 =
        sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = 'terra1gemswapgt'")
            .fetch_one(&pool)
            .await
            .expect("gem pair id");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, reserve_0, reserve_1)
         VALUES ($1, 1001, NOW(), 'txgem684', $2, $3, $4, 1000, 900, 0.9, 1, 2)",
    )
    .bind(gem_pair)
    .bind(&seed.trader_address)
    .bind(gem_id)
    .bind(seed.asset_0_id)
    .execute(&pool)
    .await
    .ok();
    let body: Value = server
        .get("/gt/events?fromBlock=1000&toBlock=1004")
        .await
        .json();
    let leaked = body["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["pairId"] == "terra1gemswapgt");
    assert!(!leaked);
}

#[serial]
#[tokio::test]
async fn gt_events_t14_skips_zero_price() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, reserve_0, reserve_1)
         VALUES ($1, 2500, NOW(), 'txzero', $2, $3, $4, 1000, 900, 0, 1, 2)",
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .unwrap();
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2500)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2500&toBlock=2500")
        .await
        .json();
    let swaps = swap_events(&body, &seed.pair_address);
    assert!(swaps.is_empty());
}

#[serial]
#[tokio::test]
async fn gt_events_a8_deleted_height_not_served() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1 AND block_height >= 2600")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    put_swap(
        &pool,
        &seed,
        2600,
        "txreorg",
        0,
        true,
        "1000",
        "900",
        "0.9",
        None,
        None,
        None,
        None,
        Some("50"),
        Some("60"),
    )
    .await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 2600)
        .await
        .unwrap();
    sqlx::query("DELETE FROM swap_events WHERE block_height >= 2600")
        .execute(&pool)
        .await
        .unwrap();
    let body: Value = server
        .get("/gt/events?fromBlock=2600&toBlock=2600")
        .await
        .json();
    assert!(swap_events(&body, &seed.pair_address).is_empty());
}

#[serial]
#[tokio::test]
async fn gt_events_rejects_over_row_cap() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    let over = cl8y_dex_indexer::api::MAX_GT_EVENT_ROWS + 1;
    sqlx::query(
        r#"
        INSERT INTO swap_events
          (pair_id, block_height, block_timestamp, tx_hash, sender,
           offer_asset_id, ask_asset_id, offer_amount, return_amount, price, swap_index)
        SELECT $1, 5000, NOW(), 'txcap694-' || g::text, $2, $3, $4, 1000, 950, 0.95, 0
        FROM generate_series(1, $5) AS g
        "#,
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .bind(over)
    .execute(&pool)
    .await
    .expect("bulk swaps");
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 5000)
        .await
        .unwrap();
    let resp = server.get("/gt/events?fromBlock=5000&toBlock=5000").await;
    resp.assert_status_bad_request();
    assert_eq!(resp.text(), cl8y_dex_indexer::api::GT_EVENT_ROW_CAP_MSG);
}

#[serial]
#[tokio::test]
async fn gt_events_at_row_cap_returns_200() {
    let (server, seed, pool) = app_with_seed().await;
    sqlx::query("DELETE FROM swap_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM liquidity_events WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .ok();
    let cap = cl8y_dex_indexer::api::MAX_GT_EVENT_ROWS;
    sqlx::query(
        r#"
        INSERT INTO swap_events
          (pair_id, block_height, block_timestamp, tx_hash, sender,
           offer_asset_id, ask_asset_id, offer_amount, return_amount, price, swap_index)
        SELECT $1, 5001, NOW(), 'txatcap694-' || g::text, $2, $3, $4, 1000, 950, 0.95, 0
        FROM generate_series(1, $5) AS g
        "#,
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .bind(cap)
    .execute(&pool)
    .await
    .expect("at-cap swaps");
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 5001)
        .await
        .unwrap();
    let resp = server.get("/gt/events?fromBlock=5001&toBlock=5001").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["events"].as_array().unwrap().len() as i64, cap);
}
