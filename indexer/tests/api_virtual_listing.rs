//! #1437: listing projections derive from real transactions and never become pools.
mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::config::{
    DEFAULT_HUB_CLUNC_ADDRESS as CLUNC, DEFAULT_HUB_CUSTC_ADDRESS as CUSTC,
    DEFAULT_USDT_CW20_ADDRESS as USDT,
};
use serde_json::Value;
use serial_test::serial;
use sqlx::PgPool;
use std::str::FromStr;
use wiremock::MockServer;

const MAPPER: &str = "terra1mapper1437";
const CROSS: &str = "terra1cross1437";
const USDT_POOL: &str = "terra1usdtpool1437";
const IDS: [&str; 5] = [
    "cl8y-virtual:clunc-uluna",
    "cl8y-virtual:custc-uusd",
    "cl8y-virtual:uluna-uusd",
    "cl8y-virtual:uluna-usdt",
    "cl8y-virtual:clunc-uusd",
];
const TICKERS: [&str; 5] = [
    "cLUNC_LUNC",
    "cUSTC_USTC",
    "LUNC_USTC",
    "LUNC_USDT",
    "cLUNC_USTC",
];

fn decimal(value: &Value) -> BigDecimal {
    BigDecimal::from_str(value.as_str().expect("decimal string")).unwrap()
}
fn number(value: &Value, expected: &str) {
    assert_eq!(decimal(value), BigDecimal::from_str(expected).unwrap());
}
fn market<'a>(rows: &'a [Value], id: &str) -> &'a Value {
    rows.iter()
        .find(|v| v["pool_id"] == id)
        .unwrap_or_else(|| panic!("missing {id} in {rows:?}"))
}
fn events(body: &Value, id: &str) -> Vec<Value> {
    body["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["pairId"] == id)
        .cloned()
        .collect()
}

async fn asset(pool: &PgPool, address: &str, symbol: &str, decimals: i16, native: bool) -> i32 {
    sqlx::query_scalar("INSERT INTO assets (contract_address,denom,is_cw20,name,symbol,decimals) VALUES ($1,$2,$3,$4,$4,$5) RETURNING id")
        .bind(if native {None} else {Some(address)}).bind(if native {Some(address)} else {None}).bind(!native).bind(symbol).bind(decimals).fetch_one(pool).await.unwrap()
}
async fn pair(pool: &PgPool, address: &str, a0: i32, a1: i32, r0: &str, r1: &str) -> i32 {
    let id = sqlx::query_scalar("INSERT INTO pairs (contract_address,asset_0_id,asset_1_id,fee_bps,created_at_block) VALUES ($1,$2,$3,30,100) RETURNING id").bind(address).bind(a0).bind(a1).fetch_one(pool).await.unwrap();
    cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
        pool,
        id,
        &r0.parse().unwrap(),
        &r1.parse().unwrap(),
        30,
        Some(105),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO pair_liquidity_usd (pair_id,liquidity_usd) VALUES ($1,10000)")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}
async fn swap(
    pool: &PgPool,
    pair: i32,
    offer: i32,
    ask: i32,
    amounts: (&str, &str, &str),
    tx: &str,
    block: i64,
    reserves: (&str, &str),
) {
    sqlx::query("INSERT INTO swap_events (pair_id,block_height,block_timestamp,tx_hash,sender,offer_asset_id,ask_asset_id,offer_amount,return_amount,price,reserve_0,reserve_1) VALUES ($1,$2,NOW()-interval '1 minute'*(110-$2),$3,'terra1maker1437',$4,$5,$6,$7,$8,$9,$10)")
        .bind(pair).bind(block).bind(tx).bind(offer).bind(ask).bind(BigDecimal::from_str(amounts.0).unwrap()).bind(BigDecimal::from_str(amounts.1).unwrap()).bind(BigDecimal::from_str(amounts.2).unwrap()).bind(BigDecimal::from_str(reserves.0).unwrap()).bind(BigDecimal::from_str(reserves.1).unwrap()).execute(pool).await.unwrap();
}
async fn wrap(pool: &PgPool, denom: &str, direction: &str, tx: &str, block: i64) {
    sqlx::query("INSERT INTO listing_wrap_principal (mapper_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,block_height,block_timestamp) VALUES ($1,$2,$3,5000000,100000000,$4,'terra1maker1437',$5,NOW()-interval '1 minute'*(110-$5))").bind(MAPPER).bind(denom).bind(direction).bind(tx).bind(block).execute(pool).await.unwrap();
}
async fn fixture() -> (PgPool, TestServer, MockServer, i32) {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    cl8y_dex_indexer::api::reset_aggregator_cache();
    cl8y_dex_indexer::api::reset_defillama_cache();
    sqlx::query("TRUNCATE listing_wrap_principal,listing_mapper_custody RESTART IDENTITY")
        .execute(&pool)
        .await
        .unwrap();
    let clunc = asset(&pool, CLUNC, "cLUNC", 6, false).await;
    let custc = asset(&pool, CUSTC, "cUSTC", 6, false).await;
    let usdt = asset(&pool, USDT, "USDT", 18, false).await;
    asset(&pool, "uluna", "LUNC", 6, true).await;
    asset(&pool, "uusd", "USTC", 6, true).await;
    // Both backing pools have reversed asset order, including 18-decimal USDT.
    let cross = pair(&pool, CROSS, custc, clunc, "300000000", "100000000").await;
    let usdt_pair = pair(
        &pool,
        USDT_POOL,
        usdt,
        clunc,
        "500000000000000000000",
        "100000000",
    )
    .await;
    swap(
        &pool,
        cross,
        clunc,
        custc,
        ("5000000", "10000000", "0.5"),
        "cross-sell",
        103,
        ("290000000", "105000000"),
    )
    .await;
    swap(
        &pool,
        cross,
        custc,
        clunc,
        ("12000000", "4000000", "0.333333333333333333333333333333"),
        "cross-buy",
        104,
        ("302000000", "101000000"),
    )
    .await;
    swap(
        &pool,
        usdt_pair,
        clunc,
        usdt,
        ("2000000", "10000000000000000000", "0.2"),
        "usdt-sell",
        105,
        ("490000000000000000000", "102000000"),
    )
    .await;
    wrap(&pool, "uluna", "wrap", "wrap-lunc", 100).await;
    wrap(&pool, "uluna", "unwrap", "unwrap-lunc", 101).await;
    wrap(&pool, "uusd", "wrap", "wrap-ustc", 102).await;
    sqlx::query("INSERT INTO listing_wrap_principal (mapper_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,block_height,block_timestamp) VALUES ('terra1foreignmapper1437','uluna','wrap',999000000,999000000,'forged-wrap','terra1attacker1437',100,NOW())").execute(&pool).await.unwrap();
    for denom in ["uluna", "uusd"] {
        sqlx::query("INSERT INTO listing_mapper_custody (mapper_address,native_denom,custody_raw,block_height,block_timestamp) VALUES ($1,$2,100000000,105,NOW())").bind(MAPPER).bind(denom).execute(&pool).await.unwrap();
    }
    // Captured fees are separate economic ledger rows and must not become principal.
    sqlx::query("INSERT INTO protocol_fee_events (source,asset_id,amount_raw,decimals,block_height,block_timestamp,tx_hash) VALUES ('wrap', $1,10000,6,100,NOW(),'wrap-lunc')").bind(clunc).execute(&pool).await.unwrap();
    let excluded =
        cl8y_dex_indexer::indexer::listing_exclude::listing_excluded_cw20_binds()[0].clone();
    let gem = asset(&pool, &excluded, "GEM", 6, false).await;
    pair(&pool, "terra1gem1437", gem, clunc, "100000000", "100000000").await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 105)
        .await
        .unwrap();
    let mock = MockServer::start().await;
    let mut config = common::test_config();
    config.lcd_urls = vec![mock.uri()];
    config.wrap_mapper_address = Some(MAPPER.into());
    let app = common::build_test_app_with_price_and_config(pool.clone(), None, config).await;
    (pool, TestServer::new(app), mock, cross)
}

#[serial]
#[tokio::test]
async fn five_markets_share_factory_shapes_pagination_and_cached_ticker_stats() {
    let (_pool, server, mock, _cross) = fixture().await;
    let response = server.get("/cg/pairs").await;
    response.assert_status_ok();
    let pairs: Vec<Value> = response.json();
    assert_eq!(pairs.len(), 7);
    assert!(!pairs.iter().any(|v| v["pool_id"] == "terra1gem1437"));
    for (i, id) in IDS.iter().enumerate() {
        assert_eq!(pairs[i + 2]["pool_id"], *id);
        assert_eq!(pairs[i + 2]["ticker_id"], TICKERS[i]);
        assert_eq!(
            pairs[i + 2].as_object().unwrap().len(),
            pairs[0].as_object().unwrap().len()
        );
    }
    assert_eq!(
        server
            .get("/cg/pairs?limit=2&offset=3")
            .await
            .json::<Vec<Value>>(),
        pairs[3..5]
    );
    assert!(server
        .get("/cg/pairs?offset=99")
        .await
        .json::<Vec<Value>>()
        .is_empty());
    server
        .get("/cg/pairs?offset=10001")
        .await
        .assert_status_bad_request();
    let response = server.get("/cg/tickers").await;
    response.assert_status_ok();
    let tickers: Vec<Value> = response.json();
    assert_eq!(
        tickers,
        server.get("/cg/tickers").await.json::<Vec<Value>>()
    );
    for (id, base, target, bv, tv, price, count) in [
        (IDS[0], CLUNC, "uluna", "10000000", "10000000", "1", "2"),
        (IDS[1], CUSTC, "uusd", "5000000", "5000000", "1", "1"),
        (IDS[2], "uluna", "uusd", "9000000", "22000000", "3", "2"),
        (
            IDS[3],
            "uluna",
            USDT,
            "2000000",
            "10000000000000000000",
            "5",
            "1",
        ),
        (IDS[4], CLUNC, "uusd", "9000000", "22000000", "3", "2"),
    ] {
        let row = market(&tickers, id);
        assert_eq!(row["base_currency"], base);
        assert_eq!(row["target_currency"], target);
        number(&row["base_volume"], bv);
        number(&row["target_volume"], tv);
        // Source price is a finite-decimal approximation; inversion must remain precise.
        assert!(
            (decimal(&row["last_price"]) - BigDecimal::from_str(price).unwrap()).abs()
                < BigDecimal::from_str("0.000000000001").unwrap()
        );
        assert_eq!(row["cl8y_extensions"]["consolidated"], true);
        assert_eq!(row["cl8y_extensions"]["pool_only_trade_count_24h"], count);
        assert_eq!(row["cl8y_extensions"]["book_leg_volume_quote_24h"], "0");
        number(&row["cl8y_extensions"]["pool_leg_volume_quote_24h"], tv);
        assert!(row
            .as_object()
            .unwrap()
            .keys()
            .all(|k| k != "virtual" && k != "synthetic"));
    }
    for id in [IDS[2], IDS[4]] {
        let row = market(&tickers, id);
        number(&row["low"], "2");
        assert!(
            (decimal(&row["high"]) - BigDecimal::from(3)).abs()
                < BigDecimal::from_str("0.000000000001").unwrap()
        );
    }
    let source = market(&tickers, CROSS);
    number(&source["base_volume"], "22000000");
    number(&source["target_volume"], "9000000");
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn principal_and_reversed_source_swaps_keep_chain_identity_scale_and_event_indexes() {
    let (pool, server, mock, _cross) = fixture().await;
    let response = server.get("/gt/events?fromBlock=100&toBlock=105").await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(
        body,
        server
            .get("/gt/events?fromBlock=100&toBlock=105")
            .await
            .json::<Value>()
    );
    assert!(!body["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["txnId"] == "forged-wrap"));
    let peg = events(&body, IDS[0]);
    assert_eq!(peg.len(), 2);
    let deposit = peg.iter().find(|v| v["txnId"] == "wrap-lunc").unwrap();
    assert_eq!(deposit["eventType"], "swap");
    assert_eq!(deposit["maker"], "terra1maker1437");
    number(&deposit["asset1In"], "5");
    number(&deposit["asset0Out"], "5");
    number(&deposit["priceNative"], "1");
    let withdraw = peg.iter().find(|v| v["txnId"] == "unwrap-lunc").unwrap();
    number(&withdraw["asset0In"], "5");
    number(&withdraw["asset1Out"], "5");
    for id in [IDS[2], IDS[4]] {
        let cross = events(&body, id);
        assert_eq!(cross.len(), 2);
        let sell = cross.iter().find(|v| v["txnId"] == "cross-sell").unwrap();
        number(&sell["asset0In"], "5");
        number(&sell["asset1Out"], "10");
        number(&sell["priceNative"], "2");
        number(&sell["reserves"]["asset0"], "105");
        number(&sell["reserves"]["asset1"], "290");
    }
    let usdt = events(&body, IDS[3]);
    assert_eq!(usdt.len(), 1);
    number(&usdt[0]["asset0In"], "2");
    number(&usdt[0]["asset1Out"], "10");
    number(&usdt[0]["priceNative"], "5");
    let factory = events(&body, USDT_POOL);
    number(&factory[0]["asset0Out"], "10");
    for tx in ["cross-sell", "cross-buy", "usdt-sell"] {
        let matching: Vec<_> = body["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["txnId"] == tx)
            .collect();
        let mut ids: Vec<_> = matching
            .iter()
            .map(|v| v["eventIndex"].as_i64().unwrap())
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), matching.len());
    }
    assert!(server
        .get("/gt/events?fromBlock=106&toBlock=110")
        .await
        .json::<Value>()["events"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM swap_events")
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn virtual_orderbooks_trades_pair_and_asset_ids_never_call_lcd() {
    let (_pool, server, mock, _cross) = fixture().await;
    for (i, id) in IDS.iter().enumerate() {
        let response = server.get(&format!("/gt/pair?id={id}")).await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["pair"]["id"], *id);
        assert_eq!(body["pair"]["dexKey"], "cl8y");
        let legs = [
            (CLUNC, "uluna"),
            (CUSTC, "uusd"),
            ("uluna", "uusd"),
            ("uluna", USDT),
            (CLUNC, "uusd"),
        ];
        assert_eq!(body["pair"]["asset0Id"], legs[i].0);
        assert_eq!(body["pair"]["asset1Id"], legs[i].1);
        assert!(body["pair"]["createdAtBlockNumber"].as_i64().unwrap() > 0);
        let response = server
            .get(&format!(
                "/cg/orderbook?ticker_id={}&depth=999999",
                TICKERS[i]
            ))
            .await;
        response.assert_status_ok();
        let book: Value = response.json();
        assert!(book["timestamp"].as_i64().unwrap() > 1_000_000_000_000);
        assert_eq!(book["bids"].as_array().unwrap().len(), 1);
        assert_eq!(book["asks"].as_array().unwrap().len(), 1);
        let response = server
            .get(&format!("/cg/historical_trades?ticker_id={}", TICKERS[i]))
            .await;
        response.assert_status_ok();
        let prints: Value = response.json();
        assert!(
            !prints["buy"].as_array().unwrap().is_empty()
                || !prints["sell"].as_array().unwrap().is_empty()
        );
    }
    let peg: Value = server
        .get("/cg/historical_trades?ticker_id=clunc_lunc")
        .await
        .json();
    assert_eq!(peg["buy"].as_array().unwrap().len(), 1);
    assert_eq!(peg["sell"].as_array().unwrap().len(), 1);
    // CG retains factory raw-volume assignment, while GT decimalizes those amounts.
    for side in ["buy", "sell"] {
        number(&peg[side][0]["base_volume"], "5000000");
        number(&peg[side][0]["target_volume"], "5000000");
    }
    let limited: Value = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&limit=1")
        .await
        .json();
    assert_eq!(
        limited["buy"].as_array().unwrap().len() + limited["sell"].as_array().unwrap().len(),
        1
    );
    let buys: Value = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&type=buy")
        .await
        .json();
    assert_eq!(buys["buy"].as_array().unwrap().len(), 1);
    assert!(buys["sell"].as_array().unwrap().is_empty());
    sqlx::query("INSERT INTO listing_wrap_principal (mapper_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,block_height,block_timestamp) SELECT $1,'uluna','wrap',1000000,100000000,'cap-principal-'||n,'terra1maker1437',105,NOW() FROM generate_series(1,600) n").bind(MAPPER).execute(&_pool).await.unwrap();
    let capped: Value = server
        .get("/cg/historical_trades?ticker_id=cLUNC_LUNC&limit=9999")
        .await
        .json();
    let prints: Vec<_> = capped["buy"]
        .as_array()
        .unwrap()
        .iter()
        .chain(capped["sell"].as_array().unwrap())
        .collect();
    assert_eq!(prints.len(), 500);
    let ids: std::collections::HashSet<_> = prints
        .iter()
        .map(|v| v["trade_id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids.len(), 500);
    server
        .get("/cg/orderbook?ticker_id=A_B_C")
        .await
        .assert_status_bad_request();
    server
        .get("/cg/orderbook?ticker_id=A_B")
        .await
        .assert_status_not_found();
    server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC&type=invalid")
        .await
        .assert_status_bad_request();
    for (asset, cg_id) in [
        ("uluna", "terra-luna-classic"),
        ("uusd", "terraclassicusd"),
        (USDT, "tether"),
    ] {
        let response = server.get(&format!("/gt/asset?id={asset}")).await;
        response.assert_status_ok();
        assert_eq!(response.json::<Value>()["asset"]["coinGeckoId"], cg_id);
    }
    assert_ne!(
        server
            .get(&format!("/gt/asset?id={CLUNC}"))
            .await
            .json::<Value>()["asset"]["coinGeckoId"],
        "terra-luna-classic"
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn listing_projections_leave_factory_and_economic_surfaces_unchanged() {
    let (pool, server, mock, _cross) = fixture().await;
    sqlx::query("INSERT INTO defillama_daily_stats (utc_day,volume_usd,trade_count) VALUES (to_timestamp(1791158400)::date,123,3)").execute(&pool).await.unwrap();
    let economic_sql = "SELECT jsonb_build_object('tvl',COALESCE((SELECT SUM(liquidity_usd) FROM pair_liquidity_usd),0),'fees',(SELECT COUNT(*) FROM protocol_fee_events),'positions',(SELECT COUNT(*) FROM trader_positions),'candles',(SELECT COUNT(*) FROM candles),'liquidity',(SELECT COUNT(*) FROM liquidity_events),'volume',(SELECT SUM(total_volume) FROM traders))";
    let economic_before: Value = sqlx::query_scalar(economic_sql)
        .fetch_one(&pool)
        .await
        .unwrap();
    let before: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM pairs),(SELECT count(*) FROM swap_events),(SELECT count(*) FROM pair_liquidity_usd),(SELECT count(*) FROM traders)").fetch_one(&pool).await.unwrap();
    for endpoint in [
        "/cg/pairs",
        "/cg/tickers",
        "/gt/events?fromBlock=100&toBlock=105",
    ] {
        server.get(endpoint).await.assert_status_ok();
    }
    for endpoint in [
        "/api/v1/pairs",
        "/cmc/summary",
        "/cmc/assets",
        "/cmc/ticker",
        "/api/v1/defillama/daily?timestamp=1791158400",
    ] {
        let response = server.get(endpoint).await;
        response.assert_status_ok();
        let body = response.text();
        assert!(!body.contains("cl8y-virtual:"), "{endpoint}");
    }
    for ticker in TICKERS {
        server
            .get(&format!("/cmc/orderbook/{ticker}"))
            .await
            .assert_status_not_found();
    }
    for id in [
        IDS[0],
        "CL8Y-VIRTUAL:clunc-uluna",
        "cl8y-virtual%3Aclunc-uluna",
        "cl8y-virtual:cl8y-virtual:clunc-uluna",
        "cl8y-virtual:%2Fterra1contract",
    ] {
        for suffix in [
            "",
            "/candles",
            "/trades",
            "/stats",
            "/liquidity-events",
            "/limit-fills",
            "/order-book-head",
            "/limit-book-shallow?side=bid",
            "/limit-book?side=bid",
            "/limit-book/insert-hints?side=bid&prices=1",
        ] {
            server
                .get(&format!("/api/v1/pairs/{id}{suffix}"))
                .await
                .assert_status_not_found();
        }
    }
    let after: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM pairs),(SELECT count(*) FROM swap_events),(SELECT count(*) FROM pair_liquidity_usd),(SELECT count(*) FROM traders)").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    assert_eq!(
        economic_before,
        sqlx::query_scalar::<_, Value>(economic_sql)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    let daily: Value = server
        .get("/api/v1/defillama/daily?timestamp=1791158400")
        .await
        .json();
    number(&daily["volume_usd"], "123");
    assert_eq!(daily["trade_count"], 3);
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new(vec![mock.uri()], 1000, 1000, "columbus-5");
    assert!(
        cl8y_dex_indexer::indexer::pair_discovery::get_or_discover_pair(
            &pool,
            &lcd,
            "terra1factory",
            IDS[0]
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn missing_backing_missing_pin_and_symbol_collisions_fail_closed() {
    let (pool, server, _mock, cross) = fixture().await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 106)
        .await
        .unwrap();
    sqlx::query("INSERT INTO listing_mapper_custody (mapper_address,native_denom,custody_raw,block_height,block_timestamp) VALUES ($1,'uluna',0,106,NOW())").bind(MAPPER).execute(&pool).await.unwrap();
    let rows: Vec<Value> = server.get("/cg/pairs").await.json();
    for id in [IDS[0], IDS[2], IDS[3]] {
        assert!(!rows.iter().any(|v| v["pool_id"] == id));
    }
    assert!(rows.iter().any(|v| v["pool_id"] == IDS[1]));
    assert!(rows.iter().any(|v| v["pool_id"] == IDS[4]));
    server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC")
        .await
        .assert_status_not_found();
    let body: Value = server
        .get("/gt/events?fromBlock=100&toBlock=106")
        .await
        .json();
    for id in [IDS[0], IDS[2], IDS[3]] {
        assert!(events(&body, id).is_empty());
    }
    // Restore backing and introduce symbol spoofs without changing the contract pins.
    sqlx::query("DELETE FROM listing_mapper_custody WHERE block_height=106")
        .execute(&pool)
        .await
        .unwrap();
    let fake_lunc = asset(&pool, "terra1fakelunc1437", "LUNC", 6, false).await;
    let fake_ustc = asset(&pool, "terra1fakeustc1437", "USTC", 6, false).await;
    pair(
        &pool,
        "terra1collision1437",
        fake_lunc,
        fake_ustc,
        "100000000",
        "100000000",
    )
    .await;
    let rows: Vec<Value> = server.get("/cg/pairs").await.json();
    assert!(rows.iter().any(|v| v["pool_id"] == "terra1collision1437"));
    assert!(!rows.iter().any(|v| v["pool_id"] == IDS[2]));
    // The real colliding ticker is resolved by CG trades without any virtual prints.
    let body: Value = server
        .get("/cg/historical_trades?ticker_id=LUNC_USTC")
        .await
        .json();
    assert!(body["buy"].as_array().unwrap().is_empty());
    assert!(body["sell"].as_array().unwrap().is_empty());
    sqlx::query("DELETE FROM swap_events WHERE pair_id=$1")
        .bind(cross)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM pairs WHERE id=$1")
        .bind(cross)
        .execute(&pool)
        .await
        .unwrap();
    let rows: Vec<Value> = server.get("/cg/pairs").await.json();
    assert!(!rows.iter().any(|v| v["pool_id"] == IDS[4]));
    let mut config = common::test_config();
    config.wrap_mapper_address = Some(MAPPER.into());
    config.hub_clunc_address = "terra1missinglocalterrapin1437".into();
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool.clone(), None, config).await,
    );
    let response = server.get("/cg/pairs").await;
    response.assert_status_ok();
    let rows: Vec<Value> = response.json();
    assert!(!rows.iter().any(|v| v["pool_id"] == IDS[0]));
    assert!(rows.iter().any(|v| v["pool_id"] == IDS[1]));
    let server = TestServer::new(common::build_test_app(pool).await);
    let rows: Vec<Value> = server.get("/cg/pairs").await.json();
    assert!(!rows
        .iter()
        .any(|v| v["pool_id"].as_str().unwrap().starts_with("cl8y-virtual:")));
}

#[serial]
#[tokio::test]
async fn exact_factory_cap_counts_no_projected_rows_and_never_drops_factory_events() {
    let (pool, server, _mock, cross) = fixture().await;
    cl8y_dex_indexer::db::queries::state::set_last_indexed_height(&pool, 106)
        .await
        .unwrap();
    let (a0, a1): (i32, i32) =
        sqlx::query_as("SELECT asset_0_id,asset_1_id FROM pairs WHERE id=$1")
            .bind(cross)
            .fetch_one(&pool)
            .await
            .unwrap();
    // Two cross rows + one USDT row already exist. 4997 additions make 5000.
    sqlx::query("INSERT INTO swap_events (pair_id,block_height,block_timestamp,tx_hash,sender,offer_asset_id,ask_asset_id,offer_amount,return_amount,price,reserve_0,reserve_1) SELECT $1,105,NOW(),'bulk-'||n,'terra1maker1437',$2,$3,2000000,1000000,0.5,300000000,100000000 FROM generate_series(1,4997) n").bind(cross).bind(a0).bind(a1).execute(&pool).await.unwrap();
    let response = server.get("/gt/events?fromBlock=100&toBlock=105").await;
    response.assert_status_ok();
    let body: Value = response.json();
    let factory = events(&body, CROSS).len() + events(&body, USDT_POOL).len();
    assert_eq!(factory, 5000);
    assert_eq!(events(&body, IDS[2]).len(), 4999);
    assert_eq!(events(&body, IDS[4]).len(), 4999);
    assert_eq!(events(&body, IDS[0]).len(), 2);
    swap(
        &pool,
        cross,
        a0,
        a1,
        ("2000000", "1000000", "0.5"),
        "overcap",
        105,
        ("300000000", "100000000"),
    )
    .await;
    let response = server.get("/gt/events?fromBlock=100&toBlock=105").await;
    response.assert_status_bad_request();
    assert!(!response.text().contains("cl8y-virtual:"));
}

#[tokio::test]
async fn reserved_ids_reject_before_even_connecting_to_postgres_or_lcd() {
    let pool =
        PgPool::connect_lazy("postgres://irrelevant:irrelevant@127.0.0.1:1/irrelevant").unwrap();
    let mock = MockServer::start().await;
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new(vec![mock.uri()], 1000, 1000, "columbus-5");
    for id in [
        IDS[0],
        "CL8Y-VIRTUAL:clunc-uluna",
        "cl8y-virtual:cl8y-virtual:clunc-uluna",
        "cl8y-virtual:%2Fterra1contract",
    ] {
        assert!(
            cl8y_dex_indexer::db::queries::pairs::get_pair_by_address(&pool, id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            cl8y_dex_indexer::indexer::pair_discovery::get_or_discover_pair(
                &pool,
                &lcd,
                "terra1factory",
                id
            )
            .await
            .unwrap()
            .is_none()
        );
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn prices_use_exact_contract_pool_not_symbol_or_oracle_and_zero_reserves_omit_market() {
    let (pool, _server, mock, _cross) = fixture().await;
    let fake_usdt = asset(&pool, "terra1fakeusdt1437", "USDT", 18, false).await;
    let clunc: i32 = sqlx::query_scalar("SELECT id FROM assets WHERE contract_address=$1")
        .bind(CLUNC)
        .fetch_one(&pool)
        .await
        .unwrap();
    let fake_pool = pair(
        &pool,
        "terra1fakesource1437",
        clunc,
        fake_usdt,
        "100000000",
        "99900000000000000000000",
    )
    .await;
    sqlx::query("UPDATE pair_liquidity_usd SET liquidity_usd=99999999 WHERE pair_id=$1")
        .bind(fake_pool)
        .execute(&pool)
        .await
        .unwrap();
    swap(
        &pool,
        fake_pool,
        clunc,
        fake_usdt,
        ("1000000", "999000000000000000000", "999"),
        "fake-usdt",
        105,
        ("100000000", "99900000000000000000000"),
    )
    .await;
    let mut config = common::test_config();
    config.wrap_mapper_address = Some(MAPPER.into());
    config.lcd_urls = vec![mock.uri()];
    // Deliberately disagree with the source pool to detect oracle laundering.
    let server = TestServer::new(
        common::build_test_app_with_oracle_prices_and_config(
            pool.clone(),
            Some("777".parse().unwrap()),
            Some("888".parse().unwrap()),
            config,
        )
        .await,
    );
    let rows: Vec<Value> = server.get("/cg/tickers").await.json();
    let row = market(&rows, IDS[3]);
    assert_eq!(row["target_currency"], USDT);
    number(&row["last_price"], "5");
    let body: Value = server
        .get("/gt/events?fromBlock=100&toBlock=105")
        .await
        .json();
    let projected = events(&body, IDS[3]);
    assert_eq!(projected.len(), 1);
    assert_eq!(projected[0]["txnId"], "usdt-sell");
    sqlx::query("UPDATE pair_reserves SET reserve_0=0 WHERE pair_id=(SELECT id FROM pairs WHERE contract_address=$1)").bind(USDT_POOL).execute(&pool).await.unwrap();
    let response = server.get("/cg/pairs").await;
    response.assert_status_ok();
    assert!(!response
        .json::<Vec<Value>>()
        .iter()
        .any(|v| v["pool_id"] == IDS[3]));
    server
        .get("/cg/historical_trades?ticker_id=LUNC_USDT")
        .await
        .assert_status_not_found();
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn uncommitted_future_custody_and_principal_cannot_publish_prices_or_inflate_prints() {
    let (pool, server, mock, _cross) = fixture().await;
    sqlx::query("INSERT INTO listing_mapper_custody (mapper_address,native_denom,custody_raw,block_height,block_timestamp) VALUES ($1,'uluna',0,9999,NOW())").bind(MAPPER).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO listing_wrap_principal (mapper_address,native_denom,direction,principal_raw,custody_raw,tx_hash,trader_address,block_height,block_timestamp) VALUES ($1,'uluna','wrap',999000000,999000000,'future-wrap','terra1maker1437',9999,NOW())").bind(MAPPER).execute(&pool).await.unwrap();
    let response = server.get("/cg/pairs").await;
    response.assert_status_ok();
    let pairs: Vec<Value> = response.json();
    assert!(pairs.iter().any(|v| v["pool_id"] == IDS[0]));
    let tickers: Vec<Value> = server.get("/cg/tickers").await.json();
    number(&market(&tickers, IDS[0])["base_volume"], "10000000");
    let response = server
        .get("/cg/historical_trades?ticker_id=cLUNC_LUNC")
        .await;
    response.assert_status_ok();
    let prints: Value = response.json();
    assert_eq!(prints["buy"].as_array().unwrap().len(), 1);
    assert_eq!(prints["sell"].as_array().unwrap().len(), 1);
    assert!(mock.received_requests().await.unwrap().is_empty());
}
