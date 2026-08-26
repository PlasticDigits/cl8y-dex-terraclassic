mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

// Shared `dex_indexer_test` DB: serialize seed_db + HTTP assertions (see api_pairs.rs).
#[serial]
#[tokio::test]
async fn get_trader_profile_returns_trader() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/traders/{}", seed.trader_address))
        .await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["address"], seed.trader_address);
    assert!(body["total_trades"].is_i64());
    assert!(body["total_volume"].is_string());
    assert!(
        body.get("total_volume_usd").is_some(),
        "GitLab #553: profile exposes total_volume_usd (string or null)"
    );
}

#[serial]
#[tokio::test]
async fn get_trader_not_found() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/terra1unknown").await;
    resp.assert_status_not_found();
}

#[serial]
#[tokio::test]
async fn get_trader_trades_returns_trades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!("/api/v1/traders/{}/trades", seed.trader_address))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    for trade in &body {
        assert_eq!(trade["sender"], seed.trader_address);
    }
}

#[serial]
#[tokio::test]
async fn leaderboard_default_sort() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/leaderboard").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
}

#[serial]
#[tokio::test]
async fn leaderboard_valid_sort_columns() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for sort in &[
        "total_volume",
        "total_volume_usd",
        "volume_24h",
        "volume_7d",
        "volume_30d",
        "total_trades",
    ] {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_ok();
    }
}

#[serial]
#[tokio::test]
async fn leaderboard_invalid_sort_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/traders/leaderboard?sort=hacked_column")
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn leaderboard_limit_capped() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/traders/leaderboard?limit=999").await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[serial]
#[tokio::test]
async fn get_trader_trades_pair_filter_returns_subset() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    for trade in &body {
        assert_eq!(trade["sender"], seed.trader_address);
        assert_eq!(trade["pair_address"], seed.pair_address);
    }
}

#[serial]
#[tokio::test]
async fn get_trader_trades_unknown_pair_returns_404() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?pair=terra1nosuchpairxxxxxxxx",
            seed.trader_address
        ))
        .await;
    resp.assert_status_not_found();
}

#[serial]
#[tokio::test]
async fn get_trader_trades_csv_returns_text_csv() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?format=csv&limit=3",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let content_type = resp.header("content-type");
    let ct = content_type.to_str().unwrap_or("");
    assert!(ct.contains("text/csv"), "unexpected content-type: {ct}");
    let body = resp.text();
    assert!(body.contains("id,pair_address,block_height"));
    assert!(body.contains("tx_hash"));
}

#[serial]
#[tokio::test]
async fn get_trader_limit_fills_returns_maker_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-fills?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 2);
    for row in &body {
        assert_eq!(row["maker"], seed.trader_address);
        assert_eq!(row["pair_address"], seed.pair_address);
    }
}

#[serial]
#[tokio::test]
async fn get_trader_limit_cancellations_returns_owner_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-cancellations?pair={}",
            seed.trader_address, seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["owner"], seed.trader_address);
    assert_eq!(body[0]["order_id"], 7);
}

#[serial]
#[tokio::test]
async fn get_trader_limit_placements_returns_owner_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-placements",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["owner"], seed.trader_address);
    assert_eq!(body[0]["order_id"], 8);
    assert_eq!(body[0]["pair_address"], seed.pair_address);
    assert_eq!(body[0]["lifecycle_status"], "active");
}

#[serial]
#[tokio::test]
async fn get_trader_limit_placements_bad_status_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/limit-placements?status=invalid",
            seed.trader_address
        ))
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn get_trader_positions_returns_rows() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO trader_positions
         (trader_address, pair_id, net_position_quote, avg_entry_price, total_cost_base, realized_pnl, trade_count)
         VALUES ($1, $2, 100, 0.5, 50, 0, 1)",
    )
    .bind(&seed.trader_address)
    .bind(seed.pair_id)
    .execute(&pool)
    .await
    .expect("insert position");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/positions",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["pair_address"], seed.pair_address);
    assert!(body[0]["net_position_quote"].is_string());
    assert_eq!(body[0]["asset_0_decimals"], 6);
    assert_eq!(body[0]["asset_1_decimals"], 6);
    assert_eq!(body[0]["asset_0_symbol"], "LUNC");
    assert_eq!(body[0]["asset_1_symbol"], "USTC");
    assert_eq!(body[0]["asset_0_denom"], "uluna");
    assert!(body[0]["asset_1_denom"].is_null() || body[0].get("asset_1_denom").is_none());
}

/// GitLab #577 **D2**: trader rolling windows zero after last swap ages past 30d; lifetime intact.
#[serial]
#[tokio::test]
async fn trader_profile_rolling_volume_zeros_after_31d_idle() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let lifetime_before: String =
        sqlx::query_scalar("SELECT total_volume::text FROM traders WHERE address = $1")
            .bind(&seed.trader_address)
            .fetch_one(&pool)
            .await
            .expect("lifetime");

    sqlx::query(
        "UPDATE swap_events SET block_timestamp = NOW() - INTERVAL '31 days' WHERE pair_id = $1",
    )
    .bind(seed.pair_id)
    .execute(&pool)
    .await
    .expect("age swaps");

    cl8y_dex_indexer::db::queries::traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh rolling");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!("/api/v1/traders/{}", seed.trader_address))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(
        body["volume_24h"].as_str().unwrap().parse::<f64>().unwrap(),
        0.0
    );
    assert_eq!(
        body["volume_7d"].as_str().unwrap().parse::<f64>().unwrap(),
        0.0
    );
    assert_eq!(
        body["volume_30d"].as_str().unwrap().parse::<f64>().unwrap(),
        0.0
    );
    let lifetime_api: f64 = body["total_volume"].as_str().unwrap().parse().unwrap();
    let lifetime_db: f64 = lifetime_before.parse().unwrap();
    assert!((lifetime_api - lifetime_db).abs() < 0.001);
    assert!(lifetime_api > 0.0);
}

/// GitLab #666: pair-scoped leaderboard from `swap_events` / `trader_positions`, not `traders.*`.
#[serial]
#[tokio::test]
async fn leaderboard_pair_filter_isolates_volume_and_pnl() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    let wallet_b = "terra1traderbbbbbbbbbbbbbbbbbbbbbbbbb";
    let pair_b = "terra1paircontractbbb";

    sqlx::query("UPDATE swap_events SET volume_usd = 10 WHERE pair_id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .expect("price pair A swaps");

    let pair_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lptokenb', 30)
         RETURNING id",
    )
    .bind(pair_b)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("insert pair B");

    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, total_volume_usd, volume_24h, volume_7d, volume_30d, registered, total_realized_pnl, best_trade_pnl)
         VALUES ($1, 99, 999999, 9999, 1, 1, 1, true, 888, 777)",
    )
    .bind(wallet_b)
    .execute(&pool)
    .await
    .expect("insert whale trader B");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 2000, NOW(), 'txhashb0', $2, $3, $4, 100, 90, 0.9, 1.5)",
    )
    .bind(pair_b_id)
    .bind(wallet_b)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("insert pair B swap");

    sqlx::query(
        "INSERT INTO trader_positions
         (trader_address, pair_id, net_position_quote, avg_entry_price, total_cost_base, realized_pnl, trade_count)
         VALUES ($1, $2, 1, 1, 1, 100, 5),
                ($3, $4, 1, 1, 1, 50, 1)",
    )
    .bind(&seed.trader_address)
    .bind(seed.pair_id)
    .bind(wallet_b)
    .bind(pair_b_id)
    .execute(&pool)
    .await
    .expect("insert positions");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let pair_a: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_volume_usd&pair={}&limit=20",
            seed.pair_address
        ))
        .await
        .json();
    assert_eq!(pair_a.len(), 1);
    assert_eq!(pair_a[0]["address"], seed.trader_address);
    assert_eq!(pair_a[0]["total_trades"], 5);
    let a_usd = pair_a[0]["total_volume_usd"]
        .as_str()
        .expect("pair A usd")
        .parse::<f64>()
        .unwrap();
    assert!((a_usd - 50.0).abs() < 1e-8, "pair A usd was {a_usd}");
    assert!(pair_a.iter().all(|r| r["address"] != wallet_b));

    let pair_b_rows: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_volume_usd&pair={pair_b}&limit=20"
        ))
        .await
        .json();
    assert_eq!(pair_b_rows.len(), 1);
    assert_eq!(pair_b_rows[0]["address"], wallet_b);
    assert_eq!(pair_b_rows[0]["total_trades"], 1);
    assert!(pair_b_rows.iter().all(|r| r["address"] != seed.trader_address));

    let unscoped: Vec<Value> = server
        .get("/api/v1/traders/leaderboard?sort=total_volume_usd&limit=20")
        .await
        .json();
    let addrs: Vec<&str> = unscoped
        .iter()
        .filter_map(|r| r["address"].as_str())
        .collect();
    assert!(addrs.contains(&seed.trader_address.as_str()) || addrs.contains(&wallet_b));

    let pnl_a: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_realized_pnl&pair={}&limit=20",
            seed.pair_address
        ))
        .await
        .json();
    assert_eq!(pnl_a[0]["address"], seed.trader_address);
    assert_eq!(pnl_a[0]["total_realized_pnl"], "100");
    assert_ne!(pnl_a[0]["total_realized_pnl"], "888");

    let pnl_b: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_realized_pnl&pair={pair_b}&limit=20"
        ))
        .await
        .json();
    assert_eq!(pnl_b[0]["address"], wallet_b);
    assert_eq!(pnl_b[0]["total_realized_pnl"], "50");
}

#[serial]
#[tokio::test]
async fn leaderboard_unknown_pair_404_empty_listed_pair_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    let empty_pair = "terra1emptypairxxxxxxxxxxxxxxxxxxxx";
    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lpempty', 30)",
    )
    .bind(empty_pair)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("empty pair");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let missing = server
        .get("/api/v1/traders/leaderboard?pair=terra1nosuchpairxxxxxxxx&sort=total_volume_usd")
        .await;
    missing.assert_status_not_found();
    let body = missing.text();
    assert!(!body.contains("sqlx"));
    assert!(!body.contains("SELECT"));
    assert!(!body.contains("postgres"));

    let empty = server
        .get(&format!(
            "/api/v1/traders/leaderboard?pair={empty_pair}&sort=total_volume_usd"
        ))
        .await;
    empty.assert_status_ok();
    let rows: Vec<Value> = empty.json();
    assert!(rows.is_empty());
}

#[serial]
#[tokio::test]
async fn leaderboard_pair_rejects_best_trade_and_injection_sorts() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for sort in &[
        "best_trade_pnl",
        "hacked_column",
        "id",
        "volume_24h",
        "%27%3B%20DROP%20TABLE%20traders%3B%20--",
    ] {
        let resp = server
            .get(&format!(
                "/api/v1/traders/leaderboard?pair={}&sort={}",
                seed.pair_address, sort
            ))
            .await;
        resp.assert_status_bad_request();
        let body = resp.text();
        assert!(!body.contains("sqlx"));
        assert!(!body.contains("SELECT"));
    }

    let inj = server
        .get(&format!(
            "/api/v1/traders/leaderboard?pair={}&sort=total_volume_usd",
            "%27%3B%20DROP%20TABLE%20traders%3B%20--"
        ))
        .await;
    inj.assert_status_not_found();
    let inj_body = inj.text();
    assert!(!inj_body.contains("sqlx"));
    assert!(!inj_body.contains("postgres"));
}

#[serial]
#[tokio::test]
async fn leaderboard_pair_limit_clamp_and_cache_isolation() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    cl8y_dex_indexer::api::reset_leaderboard_cache();

    let pair_b = "terra1paircachebbbbbbbbbbbbbbbbbbb";
    let wallet_b = "terra1cachebbbbbbbbbbbbbbbbbbbbbbbb";
    let pair_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lpcache', 30)
         RETURNING id",
    )
    .bind(pair_b)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .fetch_one(&pool)
    .await
    .expect("pair B");
    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ($1, 1, 1, 0, 0, 0, false)",
    )
    .bind(wallet_b)
    .execute(&pool)
    .await
    .expect("wallet B");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 3000, NOW(), 'txhashcacheb', $2, $3, $4, 10, 9, 0.9, 3)",
    )
    .bind(pair_b_id)
    .bind(wallet_b)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("swap B");

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let capped = server
        .get(&format!(
            "/api/v1/traders/leaderboard?pair={}&limit=999",
            seed.pair_address
        ))
        .await;
    capped.assert_status_ok();
    let capped_rows: Vec<Value> = capped.json();
    assert!(capped_rows.len() <= 200);

    let zero = server
        .get(&format!(
            "/api/v1/traders/leaderboard?pair={}&limit=0",
            seed.pair_address
        ))
        .await;
    zero.assert_status_ok();
    let zero_rows: Vec<Value> = zero.json();
    assert!(!zero_rows.is_empty());

    let a: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_volume_usd&pair={}&limit=10",
            seed.pair_address
        ))
        .await
        .json();
    let b: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_volume_usd&pair={pair_b}&limit=10"
        ))
        .await
        .json();
    assert_eq!(a[0]["address"], seed.trader_address);
    assert_eq!(b[0]["address"], wallet_b);

    cl8y_dex_indexer::api::reset_leaderboard_cache();
    let a2: Vec<Value> = server
        .get(&format!(
            "/api/v1/traders/leaderboard?sort=total_volume_usd&pair={}&limit=10",
            seed.pair_address
        ))
        .await
        .json();
    assert_eq!(a2[0]["address"], seed.trader_address);
}
