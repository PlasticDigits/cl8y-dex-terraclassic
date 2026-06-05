//! Per-endpoint negative/zero `limit` lower-bound regression coverage. GitLab #317, follow-up
//! to #284.
//!
//! #284 was an HTTP-500: a defaulted query limit clamped upper-only
//! (`params.limit.unwrap_or(50).min(200)`) let `limit=-1` / `limit=0` reach Postgres as a
//! negative/zero `LIMIT`, which 500s. The `.min(MAX)` -> `.clamp(1, MAX)` sweep fixed every
//! list handler, but only `hooks.rs` had a dedicated lower-bound test
//! (`hooks_negative_and_zero_limit_clamp_to_one_not_500`). This file mirrors that test for
//! EVERY other SQL-backed list endpoint so a future `.min(MAX)` reintroduction is caught here
//! (not only by the static `limit_clamp_guardrail`).
//!
//! Race-safety (#317): these tests share `dex_indexer_test` with sibling integration tests and
//! do not serialize. The assertion is **status 200, never 500**; for bare-array bodies we also
//! assert `len() <= 1` — the clamp guarantees at most one row regardless of what a concurrent
//! test seeds or truncates. Wrapped/object bodies (pairs list, oracle history,
//! cg/historical_trades) only assert status, since their row counts are nested.
//!
//! LCD-backed routes (`limit-book`, `limit-book-shallow`, `/cg/orderbook`, `/cmc/orderbook/*`)
//! are NOT covered here — they need a wiremock LCD harness: `limit-book` / `limit-book-shallow`
//! in [`api_limit_book_lcd_mock.rs`](api_limit_book_lcd_mock.rs); CG/CMC orderbook `depth` in
//! [`api_orderbook_lcd_mock.rs`](api_orderbook_lcd_mock.rs).

mod common;

use axum_test::TestServer;
use serde_json::Value;

/// GET `{path}` with `limit=-1` then `limit=0`; assert 200 (never 500). When the body is a bare
/// JSON array, also assert the clamp held it to `<= 1` row. Object/wrapped bodies assert status
/// only (race-safe under the shared test DB; see module docs).
async fn assert_limit_low_ok(server: &TestServer, path: &str) {
    let sep = if path.contains('?') { '&' } else { '?' };
    for v in ["-1", "0"] {
        let url = format!("{path}{sep}limit={v}");
        let resp = server.get(&url).await;
        resp.assert_status_ok();
        if let Value::Array(arr) = resp.json::<Value>() {
            assert!(
                arr.len() <= 1,
                "{url} must clamp limit to 1, got {} rows",
                arr.len()
            );
        }
    }
}

#[tokio::test]
async fn pairs_list_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/api/v1/pairs").await;
}

#[tokio::test]
async fn pair_candles_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/pairs/{}/candles?interval=1h", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn pair_trades_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, &format!("/api/v1/pairs/{}/trades", seed.pair_address)).await;
}

#[tokio::test]
async fn pair_liquidity_events_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/pairs/{}/liquidity-events", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn pair_limit_fills_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/pairs/{}/limit-fills", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn pair_limit_placements_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/pairs/{}/limit-placements", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn pair_limit_cancellations_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/pairs/{}/limit-cancellations", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn tokens_list_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/api/v1/tokens").await;
}

#[tokio::test]
async fn traders_leaderboard_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/api/v1/traders/leaderboard").await;
}

#[tokio::test]
async fn trader_trades_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/traders/{}/trades", seed.trader_address),
    )
    .await;
}

#[tokio::test]
async fn trader_limit_fills_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/traders/{}/limit-fills", seed.trader_address),
    )
    .await;
}

#[tokio::test]
async fn trader_limit_placements_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/traders/{}/limit-placements", seed.trader_address),
    )
    .await;
}

#[tokio::test]
async fn trader_limit_cancellations_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(
        &server,
        &format!("/api/v1/traders/{}/limit-cancellations", seed.trader_address),
    )
    .await;
}

#[tokio::test]
async fn oracle_history_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/api/v1/oracle/history").await;
}

#[tokio::test]
async fn cg_pairs_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/cg/pairs").await;
}

#[tokio::test]
async fn cg_historical_trades_negative_and_zero_limit_clamp_not_500() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_limit_low_ok(&server, "/cg/historical_trades?ticker_id=LUNC_USTC").await;
}
