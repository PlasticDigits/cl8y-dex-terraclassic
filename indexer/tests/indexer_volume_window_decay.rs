//! Trailing-window decay for token / trader / pair / global rollups (GitLab #577).
//!
//! Decay tests mutate stored `block_timestamp` — never wall-clock sleep of 24h (V3).

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::db::queries::{traders, volume};
use cl8y_dex_indexer::indexer::volume_aggregator;
use common::{seed_db, setup_pool};
use serial_test::serial;
use sqlx::PgPool;

async fn age_pair_swaps(pool: &PgPool, pair_id: i32, ts: chrono::DateTime<Utc>) {
    sqlx::query("UPDATE swap_events SET block_timestamp = $1 WHERE pair_id = $2")
        .bind(ts)
        .bind(pair_id)
        .execute(pool)
        .await
        .expect("age swaps");
}

async fn token_window(pool: &PgPool, asset_id: i32, window: &str) -> Option<(BigDecimal, i64)> {
    let rows = volume::get_token_volume(pool, asset_id)
        .await
        .expect("token volume");
    rows.into_iter()
        .find(|r| r.window == window)
        .map(|r| (r.volume, r.trade_count))
}

async fn trader_rolling(
    pool: &PgPool,
    address: &str,
) -> (BigDecimal, BigDecimal, BigDecimal, BigDecimal) {
    let row = traders::get_trader(pool, address)
        .await
        .expect("trader")
        .expect("trader exists");
    (
        row.volume_24h,
        row.volume_7d,
        row.volume_30d,
        row.total_volume,
    )
}

fn zero() -> BigDecimal {
    BigDecimal::from(0)
}

/// I1: 25h-old offer swaps zero the 24h token window; 7d still counts.
#[serial]
#[tokio::test]
async fn token_24h_window_zeros_when_swaps_aged_past_cutoff() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh tokens");
    let (vol_24h, trades_24h) = token_window(&pool, seed.asset_0_id, "24h")
        .await
        .expect("24h row");
    assert!(vol_24h > zero(), "seed offer swaps must populate 24h");
    assert!(trades_24h > 0);

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;
    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh after age");

    let (vol_24h, trades_24h) = token_window(&pool, seed.asset_0_id, "24h")
        .await
        .expect("24h row");
    assert_eq!(vol_24h.normalized(), zero().normalized());
    assert_eq!(trades_24h, 0);
    let (vol_7d, trades_7d) = token_window(&pool, seed.asset_0_id, "7d")
        .await
        .expect("7d row");
    assert!(vol_7d > zero(), "25h is still inside 7d");
    assert!(trades_7d > 0);
}

/// I2: 8d zeros 7d; 31d zeros 30d.
#[serial]
#[tokio::test]
async fn token_7d_and_30d_windows_zero_past_cutoff() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    volume::refresh_token_volumes(&pool).await.expect("refresh");
    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::days(8)).await;
    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh 8d");

    let (vol_7d, _) = token_window(&pool, seed.asset_0_id, "7d")
        .await
        .expect("7d");
    let (vol_30d, trades_30d) = token_window(&pool, seed.asset_0_id, "30d")
        .await
        .expect("30d");
    assert_eq!(vol_7d.normalized(), zero().normalized());
    assert!(vol_30d > zero());
    assert!(trades_30d > 0);

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::days(31)).await;
    volume::refresh_token_volumes(&pool)
        .await
        .expect("refresh 31d");
    let (vol_30d, trades_30d) = token_window(&pool, seed.asset_0_id, "30d")
        .await
        .expect("30d");
    assert_eq!(vol_30d.normalized(), zero().normalized());
    assert_eq!(trades_30d, 0);
}

/// I3: never-traded asset is not invented; ask-side-only asset stays offer-side empty.
#[serial]
#[tokio::test]
async fn token_refresh_does_not_invent_usd_for_idle_or_ask_only_assets() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    let idle: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1nevertraded', true, 'Idle', 'IDLE', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("idle asset");

    volume::refresh_token_volumes(&pool).await.expect("refresh");

    assert!(
        token_window(&pool, idle, "24h").await.is_none(),
        "never-traded asset must not get a token_volume_stats row"
    );
    let ask_rows = volume::get_token_volume(&pool, seed.asset_1_id)
        .await
        .expect("ask-side");
    assert!(
        ask_rows
            .iter()
            .all(|r| r.volume == zero() && r.trade_count == 0)
            || ask_rows.is_empty(),
        "ask-only asset must not receive offer-side volume (#577 offer-side invariant)"
    );
}

/// I4 / I5 / D2: trader windows decay; lifetime columns stay put.
#[serial]
#[tokio::test]
async fn trader_rolling_windows_decay_without_touching_lifetime() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh traders");
    let (v24, v7, v30, lifetime_before) = trader_rolling(&pool, &seed.trader_address).await;
    assert!(v24 > zero());
    assert!(v7 > zero());
    assert!(v30 > zero());

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;
    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh 25h");
    let (v24, v7, v30, lifetime) = trader_rolling(&pool, &seed.trader_address).await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert!(v7 > zero());
    assert!(v30 > zero());
    assert_eq!(lifetime.normalized(), lifetime_before.normalized());

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::days(31)).await;
    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh 31d");
    let (v24, v7, v30, lifetime) = trader_rolling(&pool, &seed.trader_address).await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert_eq!(v7.normalized(), zero().normalized());
    assert_eq!(v30.normalized(), zero().normalized());
    assert_eq!(lifetime.normalized(), lifetime_before.normalized());

    let usd: Option<BigDecimal> =
        sqlx::query_scalar("SELECT total_volume_usd FROM traders WHERE address = $1")
            .bind(&seed.trader_address)
            .fetch_one(&pool)
            .await
            .expect("lifetime usd");
    let _ = usd;
}

/// I6: trader with no swap_events gets rolling zeros.
#[serial]
#[tokio::test]
async fn trader_with_no_swaps_rolling_columns_zero_after_refresh() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ('terra1ghosttrader', 3, 999, 100, 200, 300, false)",
    )
    .execute(&pool)
    .await
    .expect("ghost trader");

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh");
    let (v24, v7, v30, lifetime) = trader_rolling(&pool, "terra1ghosttrader").await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert_eq!(v7.normalized(), zero().normalized());
    assert_eq!(v30.normalized(), zero().normalized());
    assert_eq!(lifetime.normalized(), BigDecimal::from(999).normalized());
}

/// I7 / D3: pair with only 48h swaps zeros pair_volume_24h and does not rank as live.
#[serial]
#[tokio::test]
async fn pair_idle_48h_volume_quote_zeros_and_does_not_rank_live() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    let live_asset: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1livevol', true, 'Live', 'LIVE', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("live asset");
    let live_pair: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1livepair', $1, $2, 'terra1livelp', 30)
         RETURNING id",
    )
    .bind(seed.asset_0_id)
    .bind(live_asset)
    .fetch_one(&pool)
    .await
    .expect("live pair");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 3000, NOW() - INTERVAL '1 hour', 'txlive', $2, $3, $4, 5000, 4900, 0.98)",
    )
    .bind(live_pair)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(live_asset)
    .execute(&pool)
    .await
    .expect("live swap");

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(48)).await;
    volume::refresh_pair_volumes(&pool)
        .await
        .expect("refresh pairs");

    let idle_vol: BigDecimal =
        sqlx::query_scalar("SELECT volume_quote FROM pair_volume_24h WHERE pair_id = $1")
            .bind(seed.pair_id)
            .fetch_one(&pool)
            .await
            .expect("idle rollup");
    assert_eq!(idle_vol.normalized(), zero().normalized());

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/pairs?sort=volume_24h&order=desc").await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let items = body["items"].as_array().expect("pair list items");
    let first_addr = items[0]["pair_address"].as_str().unwrap_or("");
    assert_eq!(
        first_addr, "terra1livepair",
        "idle 48h pair must not rank above a live 24h pair: {items:?}"
    );
}

/// I8 / D4: aging previously counted 24h swaps decreases global totals to 0.
#[serial]
#[tokio::test]
async fn global_stats_decrease_when_counted_swaps_age_past_24h() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let before = volume::get_global_stats(&pool).await.expect("before");
    assert_eq!(before.total_trades_24h, 5);
    assert!(before.total_volume_24h > zero());

    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;
    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh aged");
    let after = volume::get_global_stats(&pool).await.expect("after");
    assert!(
        after.total_trades_24h < before.total_trades_24h,
        "D4: totals must decrease, not only ignore extra old rows"
    );
    assert_eq!(after.total_trades_24h, 0);
    assert_eq!(after.total_volume_24h.normalized(), zero().normalized());
    assert_eq!(after.total_volume_24h_usd.normalized(), zero().normalized());
}

/// I9: mixed 1h + 25h — only 1h swaps in 24h totals.
#[serial]
#[tokio::test]
async fn global_stats_24h_keeps_only_in_window_swaps() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 4000, $2, 'tx1h', $3, $4, $5, 111, 100, 0.9, 10)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now() - Duration::hours(1))
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("1h swap");

    volume::refresh_global_stats(&pool).await.expect("refresh");
    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_trades_24h, 1);
    assert_eq!(
        stats.total_volume_24h.normalized(),
        BigDecimal::from(111).normalized()
    );
}

/// I10: OVERVIEW_GLOBAL_STATS_LIVE=1 still matches live after decay.
#[serial]
#[tokio::test]
async fn overview_live_env_matches_live_query_after_decay() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");

    struct LiveEnvGuard;
    impl Drop for LiveEnvGuard {
        fn drop(&mut self) {
            unsafe { std::env::remove_var("OVERVIEW_GLOBAL_STATS_LIVE") }
        }
    }
    let _guard = LiveEnvGuard;
    unsafe { std::env::set_var("OVERVIEW_GLOBAL_STATS_LIVE", "1") }

    let via_flag = volume::get_global_stats(&pool).await.expect("flag");
    let live = volume::get_global_stats_live(&pool).await.expect("live");
    assert_eq!(via_flag.total_trades_24h, live.total_trades_24h);
    assert_eq!(via_flag.total_trades_24h, 0);
    assert_eq!(
        via_flag.total_volume_24h.normalized(),
        live.total_volume_24h.normalized()
    );
}

/// I12: empty DB refresh zeros, no error.
#[serial]
#[tokio::test]
async fn empty_db_volume_refresh_is_zero_not_error() {
    let pool = setup_pool().await;
    common::clean_db(&pool).await;

    volume::refresh_token_volumes(&pool)
        .await
        .expect("token empty");
    volume::refresh_pair_volumes(&pool)
        .await
        .expect("pair empty");
    volume::refresh_global_stats(&pool)
        .await
        .expect("global empty");
    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("trader empty");

    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_trades_24h, 0);
}

/// I13: second refresh does not duplicate (asset_id, window) rows.
#[serial]
#[tokio::test]
async fn token_refresh_keeps_unique_asset_window() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    volume::refresh_token_volumes(&pool).await.expect("first");
    volume::refresh_token_volumes(&pool).await.expect("second");
    let n: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM token_volume_stats WHERE asset_id = $1"#)
            .bind(seed.asset_0_id)
            .fetch_one(&pool)
            .await
            .expect("count");
    assert_eq!(n, 3, "exactly one row per window");
}

/// I14 / D5: startup helper refreshes token + trader without waiting 5 min.
#[serial]
#[tokio::test]
async fn startup_refresh_zeros_token_and_trader_windows() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    volume::refresh_token_volumes(&pool)
        .await
        .expect("seed token");
    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("seed trader");
    age_pair_swaps(&pool, seed.pair_id, Utc::now() - Duration::hours(25)).await;

    volume_aggregator::refresh_all_volume_windows(&pool, true).await;

    let (vol_24h, _) = token_window(&pool, seed.asset_0_id, "24h")
        .await
        .expect("24h");
    assert_eq!(vol_24h.normalized(), zero().normalized());
    let (v24, v7, _, _) = trader_rolling(&pool, &seed.trader_address).await;
    assert_eq!(v24.normalized(), zero().normalized());
    assert!(v7 > zero());
    let stats = volume::get_global_stats(&pool).await.expect("global");
    assert_eq!(stats.total_trades_24h, 0);
}

/// D6 / A4: stale non-zero rollup is served; GET must not live-scan new swaps.
#[serial]
#[tokio::test]
async fn stale_nonzero_global_stats_does_not_live_scan() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");
    let before = volume::get_global_stats(&pool).await.expect("before");
    assert_eq!(before.total_trades_24h, 5);

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 5000, NOW() - INTERVAL '30 minutes', 'txstale', $2, $3, $4, 777, 700, 0.9)",
    )
    .bind(seed.pair_id)
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("extra swap");

    sqlx::query(
        "UPDATE global_stats_24h SET updated_at = NOW() - INTERVAL '20 minutes' WHERE id = 1",
    )
    .execute(&pool)
    .await
    .expect("backdate updated_at");

    let updated_at: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT updated_at FROM global_stats_24h WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("updated_at");
    assert!(volume::is_global_stats_stale(updated_at, Utc::now()));

    let served = volume::get_global_stats(&pool).await.expect("stale serve");
    assert_eq!(
        served.total_trades_24h, 5,
        "stale non-zero rollup must not live-scan the extra swap (A4)"
    );
    let live = volume::get_global_stats_live(&pool).await.expect("live");
    assert_eq!(live.total_trades_24h, 6);
}

/// A6: idle 30d+ traders do not occupy volume_30d leaderboard via leftover columns.
#[serial]
#[tokio::test]
async fn leaderboard_volume_30d_excludes_idle_ghosts_after_refresh() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO traders (address, total_trades, total_volume, volume_24h, volume_7d, volume_30d, registered)
         VALUES ('terra1staleghost', 1, 1, 0, 0, 999999, false)",
    )
    .execute(&pool)
    .await
    .expect("ghost");

    traders::refresh_rolling_volumes(&pool)
        .await
        .expect("refresh");

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server
        .get("/api/v1/traders/leaderboard?sort=volume_30d&limit=5")
        .await;
    resp.assert_status_ok();
    let body: Vec<serde_json::Value> = resp.json();
    let ghost = body.iter().find(|t| t["address"] == "terra1staleghost");
    if let Some(row) = ghost {
        let v: f64 = row["volume_30d"]
            .as_str()
            .unwrap_or("0")
            .parse()
            .unwrap_or(-1.0);
        assert_eq!(v, 0.0);
    }
    let seed_row = body
        .iter()
        .find(|t| t["address"] == seed.trader_address)
        .expect("seed trader on board");
    let seed_vol: f64 = seed_row["volume_30d"]
        .as_str()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0.0);
    assert!(seed_vol > 0.0, "active trader keeps 30d volume");
}
