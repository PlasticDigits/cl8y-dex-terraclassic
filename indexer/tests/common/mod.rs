pub mod lcd_mock;

use std::env;
use std::sync::Once;

use axum::Router;
use cl8y_dex_indexer::api::{build_router, AppState};
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::lcd::LcdClient;
use sqlx::PgPool;

static INIT: Once = Once::new();

fn init_tracing() {
    INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("error")
            .with_test_writer()
            .try_init()
            .ok();
    });
}

pub fn test_config() -> Config {
    Config {
        run_mode: cl8y_dex_indexer::config::RunMode::Dev,
        deploy_env: cl8y_dex_indexer::config::DeployEnv::Dev,
        database_url: env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://postgres:postgres@localhost:5432/dex_indexer_test".into()
        }),
        lcd_urls: vec!["http://localhost:9999".to_string()],
        factory_address: "terra1factory".to_string(),
        fee_discount_address: None,
        poll_interval_ms: 6000,
        api_port: 0,
        api_bind: "127.0.0.1".to_string(),
        lcd_timeout_ms: 5000,
        lcd_cooldown_ms: 30000,
        start_block: None,
        cors_origins: vec![
            "https://dex.cl8y.com".to_string(),
            "http://localhost:5173".to_string(),
        ],
        rate_limit_rps: 0,
        oracle_poll_interval_ms: 30000,
        ustc_denom: None,
        router_address: None,
        metrics_listen: None,
    }
}

pub async fn setup_pool() -> PgPool {
    init_tracing();
    let config = test_config();
    let pool = PgPool::connect(&config.database_url)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Integration tests require PostgreSQL at {}: {}.\n\
                 Set TEST_DATABASE_URL or run `cargo test --lib` for unit tests only.",
                config.database_url, e
            );
        });

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

pub async fn clean_db(pool: &PgPool) {
    sqlx::query("DELETE FROM ustc_prices")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM hook_events")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM limit_order_placements")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM limit_order_cancellations")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM limit_order_fills")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM swap_events")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM candles").execute(pool).await.ok();
    sqlx::query("DELETE FROM liquidity_events")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM token_volume_stats")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM trader_positions")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM traders").execute(pool).await.ok();
    sqlx::query("DELETE FROM pairs").execute(pool).await.ok();
    sqlx::query("DELETE FROM assets").execute(pool).await.ok();
}

pub struct SeedData {
    pub asset_0_id: i32,
    pub asset_1_id: i32,
    pub pair_id: i32,
    pub pair_address: String,
    pub trader_address: String,
}

pub async fn seed_db(pool: &PgPool) -> SeedData {
    clean_db(pool).await;

    let pair_address = "terra1paircontractabc".to_string();
    let trader_address = "terra1traderxyz".to_string();

    let asset_0_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (denom, is_cw20, name, symbol, decimals)
         VALUES ('uluna', false, 'Luna Classic', 'LUNC', 6)
         RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("insert asset_0");

    let asset_1_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustctoken', true, 'TerraClassicUSD', 'USTC', 6)
         RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("insert asset_1");

    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ($1, $2, $3, 'terra1lptoken', 30)
         RETURNING id",
    )
    .bind(&pair_address)
    .bind(asset_0_id)
    .bind(asset_1_id)
    .fetch_one(pool)
    .await
    .expect("insert pair");

    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ($1, 10, 5000, 500, 2000, 4000, true)",
    )
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert trader");

    for i in 0..5 {
        sqlx::query(
            "INSERT INTO swap_events
             (pair_id, block_height, block_timestamp, tx_hash, sender,
              offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
             VALUES ($1, $2, NOW() - interval '1 hour' * $3, $4, $5, $6, $7, 1000, 950, 0.95)",
        )
        .bind(pair_id)
        .bind(1000i64 + i)
        .bind(i as f64)
        .bind(format!("txhash{}", i))
        .bind(&trader_address)
        .bind(asset_0_id)
        .bind(asset_1_id)
        .execute(pool)
        .await
        .expect("insert swap event");
    }

    let first_swap_id: i64 =
        sqlx::query_scalar("SELECT id FROM swap_events WHERE pair_id = $1 ORDER BY id ASC LIMIT 1")
            .bind(pair_id)
            .fetch_one(pool)
            .await
            .expect("first swap id");

    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, 1000, NOW(), 'txfill0', 42, 'bid', $3, 1.0, 100, 100, 1)",
    )
    .bind(pair_id)
    .bind(first_swap_id)
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert limit fill");

    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, 1001, NOW(), 'txfill1', 43, 'ask', $3, 1.1, 50, 55, 0)",
    )
    .bind(pair_id)
    .bind(first_swap_id)
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert limit fill 2");

    sqlx::query(
        "INSERT INTO liquidity_events
         (pair_id, block_height, block_timestamp, tx_hash, provider, event_type, asset_0_amount, asset_1_amount, lp_amount)
         VALUES ($1, 1000, NOW(), 'txliq0', $2, 'add', 1000, 1000, 500)",
    )
    .bind(pair_id)
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert liquidity event");

    sqlx::query(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id)
         VALUES ($1, 1000, NOW(), 'txplace0', 7)",
    )
    .bind(pair_id)
    .execute(pool)
    .await
    .expect("insert placement");

    sqlx::query(
        "INSERT INTO limit_order_cancellations
         (pair_id, block_height, block_timestamp, tx_hash, order_id)
         VALUES ($1, 1001, NOW(), 'txcancel0', 7)",
    )
    .bind(pair_id)
    .execute(pool)
    .await
    .expect("insert cancellation");

    sqlx::query(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close, volume_base, volume_quote, trade_count)
         VALUES ($1, '1h', NOW() - interval '1 hour', 0.94, 0.96, 0.93, 0.95, 5000, 4750, 5)",
    )
    .bind(pair_id)
    .execute(pool)
    .await
    .expect("insert candle");

    SeedData {
        asset_0_id,
        asset_1_id,
        pair_id,
        pair_address,
        trader_address,
    }
}

/// Two CW20 assets connected by one pair, plus a third asset with no pair (no path A↔C).
pub struct RouteSolveSeed {
    pub token_a: String,
    pub token_b: String,
    pub token_c: String,
}

pub async fn seed_route_solve(pool: &PgPool) -> RouteSolveSeed {
    clean_db(pool).await;

    let token_a = "terra1routesolveaaa".to_string();
    let token_b = "terra1routesolvebbb".to_string();
    let token_c = "terra1routesolveccc".to_string();

    let asset_a_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route A', 'RTEA', 6)
         RETURNING id",
    )
    .bind(&token_a)
    .fetch_one(pool)
    .await
    .expect("insert route asset a");

    let asset_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route B', 'RTEB', 6)
         RETURNING id",
    )
    .bind(&token_b)
    .fetch_one(pool)
    .await
    .expect("insert route asset b");

    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route C', 'RTEC', 6)",
    )
    .bind(&token_c)
    .execute(pool)
    .await
    .expect("insert route asset c");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairrouteabc', $1, $2, 'terra1lproute', 30)",
    )
    .bind(asset_a_id)
    .bind(asset_b_id)
    .execute(pool)
    .await
    .expect("insert route pair");

    RouteSolveSeed {
        token_a,
        token_b,
        token_c,
    }
}

/// A→B→C chain (two hops) for multihop hybrid tests.
pub async fn seed_route_solve_2hop(pool: &PgPool) -> RouteSolveSeed {
    clean_db(pool).await;

    let token_a = "terra1routesolveaaa".to_string();
    let token_b = "terra1routesolvebbb".to_string();
    let token_c = "terra1routesolveccc".to_string();

    let asset_a_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route A', 'RTEA', 6)
         RETURNING id",
    )
    .bind(&token_a)
    .fetch_one(pool)
    .await
    .expect("insert route asset a");

    let asset_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route B', 'RTEB', 6)
         RETURNING id",
    )
    .bind(&token_b)
    .fetch_one(pool)
    .await
    .expect("insert route asset b");

    let asset_c_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route C', 'RTEC', 6)
         RETURNING id",
    )
    .bind(&token_c)
    .fetch_one(pool)
    .await
    .expect("insert route asset c");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairrouteabc', $1, $2, 'terra1lprouteab', 30)",
    )
    .bind(asset_a_id)
    .bind(asset_b_id)
    .execute(pool)
    .await
    .expect("insert route pair ab");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairroutebcd', $1, $2, 'terra1lproutebc', 30)",
    )
    .bind(asset_b_id)
    .bind(asset_c_id)
    .execute(pool)
    .await
    .expect("insert route pair bc");

    RouteSolveSeed {
        token_a,
        token_b,
        token_c,
    }
}

pub async fn build_test_app(pool: PgPool) -> Router {
    build_test_app_with_price_and_config(pool, None, test_config()).await
}

pub async fn build_test_app_with_price(
    pool: PgPool,
    ustc_price: Option<bigdecimal::BigDecimal>,
) -> Router {
    build_test_app_with_price_and_config(pool, ustc_price, test_config()).await
}

pub async fn build_test_app_with_price_and_config(
    pool: PgPool,
    ustc_price: Option<bigdecimal::BigDecimal>,
    config: Config,
) -> Router {
    let lcd = LcdClient::new(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
    );
    let price_handle = cl8y_dex_indexer::indexer::oracle::new_shared_price();
    if let Some(price) = ustc_price {
        *price_handle.write().await = Some(price);
    }
    let state = AppState {
        pool,
        lcd,
        ustc_price: price_handle,
        ticker_map_cache: cl8y_dex_indexer::api::TickerMapCache::default(),
        orderbook_cache: cl8y_dex_indexer::api::orderbook_sim::OrderbookCache::default(),
        router_address: config.router_address.clone(),
    };
    build_router(state, &config)
}
