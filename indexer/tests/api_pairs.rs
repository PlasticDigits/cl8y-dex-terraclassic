mod common;

use axum_test::TestServer;
use chrono::{Duration, Utc};
use serde_json::Value;
use serial_test::serial;

fn assert_created_at_rfc3339(v: &Value) {
    let s = v
        .as_str()
        .expect("created_at must be an ISO-8601 string (GitLab #662)");
    chrono::DateTime::parse_from_rfc3339(s).expect("created_at RFC3339");
}

#[serial]
#[tokio::test]
async fn list_pairs_returns_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    let items = body["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "should have at least one pair");
    assert!(body["total"].as_i64().unwrap() >= 1);
    assert_eq!(body["limit"].as_i64().unwrap(), 50);
    assert_eq!(body["offset"].as_i64().unwrap(), 0);

    let pair = &items[0];
    assert_eq!(pair["pair_address"], seed.pair_address);
    assert!(pair["asset_0"]["symbol"].is_string());
    assert!(pair["asset_1"]["symbol"].is_string());
    assert!(pair["is_active"].as_bool().unwrap());
    assert_eq!(pair["code_id_frozen"].as_bool().unwrap(), false);
    assert!(pair["volume_quote_24h"].is_string());
    assert_created_at_rfc3339(&pair["created_at"]);
    assert!(
        pair.get("liquidity_usd").is_none() || pair["liquidity_usd"].is_null(),
        "list JOIN is #655 — this ticket must not emit list liquidity_usd yet"
    );

    // Pagination, sort, search (same server / DB to avoid parallel seed conflicts)
    let resp = server
        .get("/api/v1/pairs?limit=1&offset=0&sort=id&order=asc")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert!(body["total"].as_i64().unwrap() >= 1);

    let resp = server.get("/api/v1/pairs?sort=volume_24h&order=desc").await;
    resp.assert_status_ok();

    let resp = server.get("/api/v1/pairs?q=LUNC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(!body["items"].as_array().unwrap().is_empty());

    let resp = server.get("/api/v1/pairs?sort=bad_sort").await;
    resp.assert_status_bad_request();

    let resp = server.get("/api/v1/pairs?offset=99999").await;
    resp.assert_status_bad_request();
}

/// GitLab #662: list JSON `created_at` + `sort=created` uses `pairs.created_at` (newest first by default).
#[serial]
#[tokio::test]
async fn list_pairs_sort_created_orders_by_created_at() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let pair_new = "terra1paircontractxyz";
    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lptoken2', 30)",
    )
    .bind(pair_new)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("insert second pair");

    let old_ts = Utc::now() - Duration::days(400);
    let new_ts = Utc::now();
    sqlx::query("UPDATE pairs SET created_at = $1 WHERE contract_address = $2")
        .bind(old_ts)
        .bind(&seed.pair_address)
        .execute(&pool)
        .await
        .expect("age seed pair");
    sqlx::query("UPDATE pairs SET created_at = $1 WHERE contract_address = $2")
        .bind(new_ts)
        .bind(pair_new)
        .execute(&pool)
        .await
        .expect("stamp newer pair");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs?sort=created&order=desc").await;
    resp.assert_status_ok();
    let desc: Value = resp.json();
    let desc_items = desc["items"].as_array().unwrap();
    assert!(desc_items.len() >= 2);
    assert_eq!(desc_items[0]["pair_address"], pair_new);
    assert_eq!(desc_items[1]["pair_address"], seed.pair_address);
    assert_created_at_rfc3339(&desc_items[0]["created_at"]);
    assert_created_at_rfc3339(&desc_items[1]["created_at"]);

    let resp = server.get("/api/v1/pairs?sort=created&order=asc").await;
    resp.assert_status_ok();
    let asc: Value = resp.json();
    let asc_items = asc["items"].as_array().unwrap();
    assert_eq!(asc_items[0]["pair_address"], seed.pair_address);
    assert_eq!(asc_items[1]["pair_address"], pair_new);

    let resp = server.get("/api/v1/pairs?sort=created").await;
    resp.assert_status_ok();
    let default: Value = resp.json();
    let default_items = default["items"].as_array().unwrap();
    assert_eq!(
        default_items[0]["pair_address"], pair_new,
        "sort=created default order is desc (newest first)"
    );

    let resp = server.get("/api/v1/pairs?sort=created;drop").await;
    resp.assert_status_bad_request();
    let resp = server.get("/api/v1/pairs?sort=created_at").await;
    resp.assert_status_bad_request();
}

// GitLab #459 (SEC-I04 F02): ILIKE wildcard metacharacters in `?q=` must be neutralized so a
// wildcard-only query cannot match every pair (search-amplification / full-scan vector).
#[serial]
#[tokio::test]
async fn search_wildcard_query_does_not_match_all_pairs() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    // Baseline: the seed pair is searchable by its real symbol.
    let resp = server.get("/api/v1/pairs?q=LUNC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        !body["items"].as_array().unwrap().is_empty(),
        "normal symbol search must still return the seeded pair"
    );

    // `?q=%` previously produced ILIKE '%%%' and matched every row. After escaping it matches
    // only fields literally containing '%', of which the seed has none.
    let resp = server.get("/api/v1/pairs?q=%25").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        body["items"].as_array().unwrap().is_empty(),
        "wildcard-only query '%' must not match all pairs, got {}",
        body["total"]
    );

    // `?q=_` (single-char wildcard) must likewise match only literal underscores.
    let resp = server.get("/api/v1/pairs?q=_").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        body["items"].as_array().unwrap().is_empty(),
        "wildcard-only query '_' must not match all pairs, got {}",
        body["total"]
    );

    // Sanity: the seed really does contain searchable rows (so the empties above are meaningful).
    let _ = &seed;
}

#[serial]
#[tokio::test]
async fn get_pair_returns_pair() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["pair_address"], seed.pair_address);
    assert_eq!(body["asset_0"]["symbol"], "LUNC");
    assert_eq!(body["asset_1"]["symbol"], "USTC");
    assert_created_at_rfc3339(&body["created_at"]);
    assert!(
        body.get("liquidity_usd").is_none() || body["liquidity_usd"].is_null(),
        "unstamped single GET must omit liquidity_usd, got {:?}",
        body.get("liquidity_usd")
    );
    assert!(
        body.get("volume_quote_24h").is_none() || body["volume_quote_24h"].is_null(),
        "single GET volume_quote_24h stays None"
    );
}

#[serial]
#[tokio::test]
async fn get_pair_liquidity_usd_from_stamp_not_live_reserves() {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO pair_liquidity_usd (pair_id, liquidity_usd, updated_at)
         VALUES ($1, $2, NOW())",
    )
    .bind(seed.pair_id)
    .bind(BigDecimal::from_str("1234.5").unwrap())
    .execute(&pool)
    .await
    .expect("stamp");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}", seed.pair_address))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["liquidity_usd"], "1234.5");
    assert_created_at_rfc3339(&body["created_at"]);
    assert!(
        body.get("volume_quote_24h").is_none() || body["volume_quote_24h"].is_null(),
        "stamping TVL must not start filling volume_quote_24h on single GET"
    );
}

#[serial]
#[tokio::test]
async fn get_pair_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs/terra1nonexistent").await;
    resp.assert_status_not_found();
}

#[serial]
#[tokio::test]
async fn get_pair_candles_valid_interval() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=1h",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty(), "should have candle data");
    assert!(body[0]["open_time"].is_string());
    assert!(body[0]["open"].is_string());
}

#[serial]
#[tokio::test]
async fn get_pair_candles_invalid_interval_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?interval=3h",
            seed.pair_address
        ))
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn get_pair_candles_default_interval() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/candles", seed.pair_address))
        .await;
    resp.assert_status_ok();
}

#[serial]
#[tokio::test]
async fn get_pair_trades_returns_trades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/trades", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert!(body[0]["tx_hash"].is_string());
    assert!(body[0]["sender"].is_string());
}

#[serial]
#[tokio::test]
async fn get_pair_limit_fills_returns_indexed_fills() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, NULL, 1001, NOW(), $2, 7, 'bid', 'terra1maker', 1.5, 100, 150, 1)",
    )
    .bind(seed.pair_id)
    .bind("lofilltx0001")
    .execute(&pool)
    .await
    .expect("insert limit_order_fills");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/limit-fills", seed.pair_address))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["order_id"], 7);
    assert_eq!(body[0]["side"], "bid");
    assert_eq!(body[0]["maker"], "terra1maker");
}

#[serial]
#[tokio::test]
async fn get_pair_order_limit_fills_filters_by_order_id() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    for (tid, oid) in [("lofilltx_a", 1i64), ("lofilltx_b", 2i64)] {
        sqlx::query(
            "INSERT INTO limit_order_fills
             (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
             VALUES ($1, NULL, 1002, NOW(), $2, $3, 'ask', 'terra1mk', 2, 10, 20, 0)",
        )
        .bind(seed.pair_id)
        .bind(tid)
        .bind(oid)
        .execute(&pool)
        .await
        .expect("insert limit_order_fills");
    }

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-orders/2/fills",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["order_id"], 2);
}

#[serial]
#[tokio::test]
async fn get_pair_trades_with_limit() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=2",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 2);
}

#[serial]
#[tokio::test]
async fn get_pair_trades_pagination() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=2",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let page1: Vec<Value> = resp.json();
    assert_eq!(page1.len(), 2);

    let last_id = page1[1]["id"].as_i64().unwrap();
    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=2&before={}",
            seed.pair_address, last_id
        ))
        .await;
    resp.assert_status_ok();
    let page2: Vec<Value> = resp.json();

    for trade in &page2 {
        assert!(trade["id"].as_i64().unwrap() < last_id);
    }
}

#[serial]
#[tokio::test]
async fn get_pair_stats_returns_stats() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/pairs/{}/stats", seed.pair_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["volume_base"].is_string());
    assert!(body["trade_count"].is_i64());
}

#[serial]
#[tokio::test]
async fn get_pair_liquidity_events_returns_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/liquidity-events",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["event_type"], "add");
    assert!(body[0]["lp_amount"].is_string());
}

#[serial]
#[tokio::test]
async fn get_pair_limit_placements_and_cancellations() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let p: Vec<Value> = resp.json();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0]["order_id"], 8);
    assert_eq!(p[0]["lifecycle_status"], "active");

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-cancellations",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let c: Vec<Value> = resp.json();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0]["order_id"], 7);
}

#[serial]
#[tokio::test]
async fn list_pairs_relevance_ordering() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    // Second LUNC pair with lower 24h volume (same symbol tier, lower liquidity).
    let wbtc_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1wbtctoken', true, 'Wrapped BTC', 'WBTC', 8)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("insert wbtc");

    let low_vol_pair: String = "terra1pairluncwbtc".to_string();
    let low_vol_pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lpwbtc', 30)
         RETURNING id",
    )
    .bind(&low_vol_pair)
    .bind(seed.asset_0_id)
    .bind(wbtc_id)
    .fetch_one(&pool)
    .await
    .expect("insert low vol pair");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 2000, NOW(), 'txlowvol', $2, $3, $4, 10, 9, 0.9)",
    )
    .bind(low_vol_pair_id)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(wbtc_id)
    .execute(&pool)
    .await
    .expect("insert low vol swap");

    cl8y_dex_indexer::db::queries::volume::refresh_pair_volumes(&pool)
        .await
        .expect("refresh volumes");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    // Exact pair address — tier 5.
    let resp = server
        .get(&format!(
            "/api/v1/pairs?q={}&sort=relevance",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items[0]["pair_address"], seed.pair_address);

    // Exact CW20 token address — tier 4.
    let resp = server
        .get("/api/v1/pairs?q=terra1ustctoken&sort=relevance")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert_eq!(items[0]["pair_address"], seed.pair_address);

    // Symbol tier — higher volume LUNC/USTC before LUNC/WBTC.
    let resp = server.get("/api/v1/pairs?q=LUNC&sort=relevance").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert!(items.len() >= 2);
    assert_eq!(items[0]["pair_address"], seed.pair_address);
    assert_eq!(items[1]["pair_address"], low_vol_pair);

    // Token name substring — tier 2.
    let resp = server
        .get("/api/v1/pairs?q=Luna+Classic&sort=relevance")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().any(|p| p["pair_address"] == seed.pair_address));

    // Pair symbol query — exact pair ranks first.
    let resp = server.get("/api/v1/pairs?q=LUNC+USTC&sort=relevance").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items[0]["pair_address"], seed.pair_address);

    // Default sort when q present is relevance.
    let resp = server.get("/api/v1/pairs?q=LUNC").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items[0]["pair_address"], seed.pair_address);

    let resp = server.get("/api/v1/pairs?sort=relevance").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items[0]["pair_address"], seed.pair_address);
}

#[serial]
#[tokio::test]
async fn pair_api_flags_code_id_frozen() {
    use cl8y_dex_indexer::indexer::asset_code_id_freeze::{
        replace_frozen_pair_addresses, snapshot_frozen_pair_addresses,
    };
    use std::collections::HashSet;

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let prev = snapshot_frozen_pair_addresses();
    replace_frozen_pair_addresses(HashSet::from([seed.pair_address.clone()]));

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let list: Value = server.get("/api/v1/pairs").await.json();
    let flagged = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["pair_address"] == seed.pair_address)
        .expect("seed pair");
    assert_eq!(flagged["code_id_frozen"].as_bool().unwrap(), true);

    let one: Value = server
        .get(&format!("/api/v1/pairs/{}", seed.pair_address))
        .await
        .json();
    assert_eq!(one["code_id_frozen"].as_bool().unwrap(), true);

    replace_frozen_pair_addresses(prev);
}
