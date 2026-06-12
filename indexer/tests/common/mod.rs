pub mod lcd_mock;

use std::env;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::sync::Once;

use axum::Router;
use cl8y_dex_indexer::api::{build_router, AppState};
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::lcd::LcdClient;
use sqlx::PgPool;

static INIT: Once = Once::new();
static ENV: Once = Once::new();

fn load_test_env() {
    ENV.call_once(|| {
        dotenvy::dotenv().ok();
    });
}

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
    load_test_env();
    Config {
        run_mode: cl8y_dex_indexer::config::RunMode::Dev,
        database_url: env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
            "postgres://cl8y_legal:cl8y_legal@127.0.0.1:5432/dex_indexer_test".into()
        }),
        lcd_urls: vec!["http://localhost:9999".to_string()],
        factory_address: "terra1factory".to_string(),
        fee_discount_address: None,
        tier_sync_reconcile_interval_secs:
            cl8y_dex_indexer::indexer::trader_tracker::DEFAULT_TIER_RECONCILE_INTERVAL_SECS,
        poll_interval_ms: 6000,
        api_port: 0,
        api_bind: "127.0.0.1".to_string(),
        api_ipv6_enabled: false,
        lcd_timeout_ms: 5000,
        lcd_cooldown_ms: 30000,
        start_block: None,
        cors_origins: vec![
            "https://dex.cl8y.com".to_string(),
            "http://localhost:5173".to_string(),
        ],
        rate_limit_rps: 0,
        rate_limit_lcd_heavy_rps: 0,
        oracle_poll_interval_ms: 30000,
        book_snapshot_interval_ms: 10_000,
        route_solver_db_hybrid: false,
        route_fidelity_drift_bps: 100,
        ustc_denom: None,
        router_address: None,
        block_tx_page_limit: 100,
        block_tx_max_pages: 50,
        block_process_max_retries: 5,
        block_process_retry_backoff_ms: 2000,
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

/// Cross-process lock for shared `dex_indexer_test` (parallel `cargo test` / agents).
fn acquire_shared_test_db_lock() -> std::fs::File {
    let path = env::var("TEST_DB_LOCK_FILE")
        .unwrap_or_else(|_| "/tmp/cl8y-dex-indexer-test.seed.lock".into());
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("open test db lock file {path}: {e}"));
    let fd = file.as_raw_fd();
    let rc = unsafe { libc::flock(fd, libc::LOCK_EX) };
    assert_eq!(rc, 0, "flock LOCK_EX on {path}");
    file
}

async fn clean_db_tables(pool: &PgPool) {
    // TRUNCATE CASCADE avoids flaky DELETE .ok() when FK rows remain.
    sqlx::query(
        "TRUNCATE TABLE
            ustc_prices,
            hook_events,
            limit_order_cancellations,
            limit_order_placements,
            limit_order_fills,
            swap_events,
            candles,
            liquidity_events,
            token_volume_stats,
            pair_volume_24h,
            global_stats_24h,
            pair_reserves,
            resting_limit_orders,
            trader_positions,
            traders,
            pairs,
            assets
        RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await
    .expect("integration test clean_db TRUNCATE");
}

pub async fn clean_db(pool: &PgPool) {
    let _lock = acquire_shared_test_db_lock();
    clean_db_tables(pool).await;
}

pub struct SeedData {
    pub asset_0_id: i32,
    pub asset_1_id: i32,
    pub pair_id: i32,
    pub pair_address: String,
    pub trader_address: String,
}

pub async fn seed_db(pool: &PgPool) -> SeedData {
    let _lock = acquire_shared_test_db_lock();
    clean_db_tables(pool).await;

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
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner)
         VALUES ($1, 1001, NOW(), 'txcancel0', 7, $2)",
    )
    .bind(pair_id)
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert cancellation");

    sqlx::query(
        "INSERT INTO limit_order_placements
         (pair_id, block_height, block_timestamp, tx_hash, order_id, owner, side, price)
         VALUES ($1, 1002, NOW(), 'txplace1', 8, $2, 'ask', 2.0)",
    )
    .bind(pair_id)
    .bind(&trader_address)
    .execute(pool)
    .await
    .expect("insert active placement");

    sqlx::query(
        "INSERT INTO candles (pair_id, interval, open_time, open, high, low, close, volume_base, volume_quote, trade_count)
         VALUES ($1, '1h', NOW() - interval '1 hour', 0.94, 0.96, 0.93, 0.95, 5000, 4750, 5)",
    )
    .bind(pair_id)
    .execute(pool)
    .await
    .expect("insert candle");

    cl8y_dex_indexer::db::queries::volume::refresh_pair_volumes(pool)
        .await
        .expect("refresh pair_volume_24h for tests");
    cl8y_dex_indexer::db::queries::volume::refresh_global_stats(pool)
        .await
        .expect("refresh global_stats_24h for tests");

    SeedData {
        asset_0_id,
        asset_1_id,
        pair_id,
        pair_address,
        trader_address,
    }
}

/// Two CW20 assets connected by one pair, plus a third asset with no path (no path A↔C).
pub struct RouteSolveSeed {
    pub token_a: String,
    pub token_b: String,
    pub token_c: String,
    /// Present when the seed includes a fourth asset and C/D pair (3-hop A→B→C→D).
    pub token_d: Option<String>,
    /// Present when the seed includes a fifth asset and D/E pair (4-hop A→B→C→D→E).
    pub token_e: Option<String>,
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
        token_d: None,
        token_e: None,
    }
}

/// Route solve seed + fresh `pair_reserves` mirror for Phase 1c DB hybrid tests.
pub async fn seed_route_solve_with_mirror(pool: &PgPool) -> RouteSolveSeed {
    use bigdecimal::BigDecimal;
    use cl8y_dex_indexer::db::queries::pair_reserves;
    use std::str::FromStr;

    let seed = seed_route_solve(pool).await;
    let pair_id: i32 = sqlx::query_scalar(
        "SELECT id FROM pairs WHERE contract_address = 'terra1pairrouteabc'",
    )
    .fetch_one(pool)
    .await
    .expect("route pair id");

    let bd = |s: &str| BigDecimal::from_str(s).unwrap();
    pair_reserves::upsert_pair_reserves(
        pool,
        pair_id,
        &bd("10000000000000"),
        &bd("10000000000000"),
        30,
        Some(100),
    )
    .await
    .expect("upsert reserves");
    seed
}

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
        token_d: None,
        token_e: None,
    }
}

/// Insert or update `traders` rows with explicit `tier_id` (GitLab #306 / #283 HTTP cache tests).
pub async fn seed_traders_with_tiers(pool: &PgPool, traders: &[(&str, i16)]) {
    for (address, tier_id) in traders {
        sqlx::query(
            "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered, tier_id)
             VALUES ($1, 0, 0, 0, 0, 0, true, $2)
             ON CONFLICT (address) DO UPDATE SET tier_id = EXCLUDED.tier_id",
        )
        .bind(*address)
        .bind(*tier_id)
        .execute(pool)
        .await
        .expect("seed trader with tier");
    }
}

/// A↔B, B↔C, and direct A↔C pairs — two simple paths A→C (1 hop) and A→B→C (2 hops) for global solver tests (#209).
pub async fn seed_route_solve_multi_path(pool: &PgPool) -> RouteSolveSeed {
    clean_db(pool).await;

    // Distinct addresses so hybrid route cache keys do not collide with other route_solve seeds.
    let token_a = "terra1routemultipathaa".to_string();
    let token_b = "terra1routemultipathbb".to_string();
    let token_c = "terra1routemultipathcc".to_string();

    let asset_a_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route MP A', 'MPA', 6)
         RETURNING id",
    )
    .bind(&token_a)
    .fetch_one(pool)
    .await
    .expect("insert route asset a");

    let asset_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route MP B', 'MPB', 6)
         RETURNING id",
    )
    .bind(&token_b)
    .fetch_one(pool)
    .await
    .expect("insert route asset b");

    let asset_c_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route MP C', 'MPC', 6)
         RETURNING id",
    )
    .bind(&token_c)
    .fetch_one(pool)
    .await
    .expect("insert route asset c");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairroutempab', $1, $2, 'terra1lproutempab', 30)",
    )
    .bind(asset_a_id)
    .bind(asset_b_id)
    .execute(pool)
    .await
    .expect("insert route pair ab");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairroutempbc', $1, $2, 'terra1lproutempbc', 30)",
    )
    .bind(asset_b_id)
    .bind(asset_c_id)
    .execute(pool)
    .await
    .expect("insert route pair bc");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairroutempac', $1, $2, 'terra1lproutempac', 30)",
    )
    .bind(asset_a_id)
    .bind(asset_c_id)
    .execute(pool)
    .await
    .expect("insert route pair ac");

    RouteSolveSeed {
        token_a,
        token_b,
        token_c,
        token_d: None,
        token_e: None,
    }
}

/// Like [`seed_route_solve_multi_path`] but seeds Postgres mirrors: direct A↔C and A↔B are
/// funded; B↔C is zero-reserve (unfunded). Used for GitLab #369 DB-hybrid path skipping.
pub async fn seed_route_solve_zero_reserve_poison(pool: &PgPool) -> RouteSolveSeed {
    use bigdecimal::BigDecimal;
    use cl8y_dex_indexer::db::queries::pair_reserves;
    use std::str::FromStr;

    let seed = seed_route_solve_multi_path(pool).await;
    let bd = |s: &str| BigDecimal::from_str(s).unwrap();
    let healthy = bd("10000000000000");

    for (addr, r0, r1) in [
        ("terra1pairroutempac", healthy.clone(), healthy.clone()),
        ("terra1pairroutempab", healthy.clone(), healthy.clone()),
        ("terra1pairroutempbc", bd("0"), bd("0")),
    ] {
        let pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = $1")
            .bind(addr)
            .fetch_one(pool)
            .await
            .expect("pair id");
        pair_reserves::upsert_pair_reserves(pool, pair_id, &r0, &r1, 30, Some(100))
            .await
            .expect("upsert reserves");
    }

    seed
}

/// A→B→C→D chain (three hops) for multihop hybrid regression tests (GitLab #192).
pub async fn seed_route_solve_3hop(pool: &PgPool) -> RouteSolveSeed {
    clean_db(pool).await;

    let token_a = "terra1routesolveaaa".to_string();
    let token_b = "terra1routesolvebbb".to_string();
    let token_c = "terra1routesolveccc".to_string();
    let token_d = "terra1routesolvedddd".to_string();

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

    let asset_d_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route D', 'RTED', 6)
         RETURNING id",
    )
    .bind(&token_d)
    .fetch_one(pool)
    .await
    .expect("insert route asset d");

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

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairroutecde', $1, $2, 'terra1lproutecd', 30)",
    )
    .bind(asset_c_id)
    .bind(asset_d_id)
    .execute(pool)
    .await
    .expect("insert route pair cd");

    RouteSolveSeed {
        token_a,
        token_b,
        token_c,
        token_d: Some(token_d),
        token_e: None,
    }
}

/// A→B→C→D→E chain (four hops, no shortcut) for default hybrid GET regression (#323).
pub async fn seed_route_solve_4hop(pool: &PgPool) -> RouteSolveSeed {
    clean_db(pool).await;

    let token_a = "terra1route4hopaaaa".to_string();
    let token_b = "terra1route4hopbbbb".to_string();
    let token_c = "terra1route4hopcccc".to_string();
    let token_d = "terra1route4hopdddd".to_string();
    let token_e = "terra1route4hopeeee".to_string();

    let asset_a_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route 4H A', 'R4A', 6)
         RETURNING id",
    )
    .bind(&token_a)
    .fetch_one(pool)
    .await
    .expect("insert route asset a");

    let asset_b_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route 4H B', 'R4B', 6)
         RETURNING id",
    )
    .bind(&token_b)
    .fetch_one(pool)
    .await
    .expect("insert route asset b");

    let asset_c_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route 4H C', 'R4C', 6)
         RETURNING id",
    )
    .bind(&token_c)
    .fetch_one(pool)
    .await
    .expect("insert route asset c");

    let asset_d_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route 4H D', 'R4D', 6)
         RETURNING id",
    )
    .bind(&token_d)
    .fetch_one(pool)
    .await
    .expect("insert route asset d");

    let asset_e_id: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ($1, true, 'Route 4H E', 'R4E', 6)
         RETURNING id",
    )
    .bind(&token_e)
    .fetch_one(pool)
    .await
    .expect("insert route asset e");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pair4hopab', $1, $2, 'terra1lp4hopab', 30)",
    )
    .bind(asset_a_id)
    .bind(asset_b_id)
    .execute(pool)
    .await
    .expect("insert route pair ab");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pair4hopbc', $1, $2, 'terra1lp4hopbc', 30)",
    )
    .bind(asset_b_id)
    .bind(asset_c_id)
    .execute(pool)
    .await
    .expect("insert route pair bc");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pair4hopcd', $1, $2, 'terra1lp4hopcd', 30)",
    )
    .bind(asset_c_id)
    .bind(asset_d_id)
    .execute(pool)
    .await
    .expect("insert route pair cd");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pair4hopde', $1, $2, 'terra1lp4hopde', 30)",
    )
    .bind(asset_d_id)
    .bind(asset_e_id)
    .execute(pool)
    .await
    .expect("insert route pair de");

    RouteSolveSeed {
        token_a,
        token_b,
        token_c,
        token_d: Some(token_d),
        token_e: Some(token_e),
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
        factory_address: Some(config.factory_address.clone()),
        fee_discount_address: config.fee_discount_address.clone(),
        fee_discount_registry_health:
            cl8y_dex_indexer::indexer::fee_discount_registry_health::FeeDiscountRegistryHealth::new(
                config
                    .fee_discount_address
                    .as_ref()
                    .is_some_and(|a| !a.is_empty()),
            ),
        route_solver_db_hybrid: config.route_solver_db_hybrid,
        book_snapshot_max_staleness_ms: config.book_snapshot_max_staleness_ms(),
        route_fidelity_drift_bps: config.route_fidelity_drift_bps,
    };
    build_router(state, &config)
}
