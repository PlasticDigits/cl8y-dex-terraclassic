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
        database_url: env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/dex_indexer_test".into()),
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
        rate_limit_rps: 1000,
    }
}

pub async fn setup_pool() -> PgPool {
    init_tracing();
    let config = test_config();
    let pool = match PgPool::connect(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "Skipping test: cannot connect to test database at {}: {}",
                config.database_url, e
            );
            eprintln!("Set TEST_DATABASE_URL to a running PostgreSQL instance to run integration tests.");
            std::process::exit(0);
        }
    };

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

pub async fn clean_db(pool: &PgPool) {
    sqlx::query("DELETE FROM swap_events").execute(pool).await.ok();
    sqlx::query("DELETE FROM candles").execute(pool).await.ok();
    sqlx::query("DELETE FROM liquidity_events").execute(pool).await.ok();
    sqlx::query("DELETE FROM token_volume_stats").execute(pool).await.ok();
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

pub fn build_test_app(pool: PgPool) -> Router {
    let config = test_config();
    let lcd = LcdClient::new(config.lcd_urls.clone(), config.lcd_timeout_ms, config.lcd_cooldown_ms);
    let state = AppState { pool, lcd };
    build_router(state, &config)
}
