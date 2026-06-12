//! GitLab #319: DB-backed hybrid route solver (Phase 1c).
//! GitLab #332: `book_start_hint` on optimized hybrid hops.
//! GitLab #369: zero-reserve pair on a candidate path must not 502 the whole solve.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use cl8y_dex_indexer::db::queries::resting_orders;
use common::lcd_mock;
use serde_json::Value;
use serial_test::serial;
use std::str::FromStr;

fn db_hybrid_config(mock: &wiremock::MockServer) -> cl8y_dex_indexer::config::Config {
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    cfg.route_solver_db_hybrid = true;
    cfg
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_no_pair_level_lcd_calls() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "global_v4");
    let kind = j["quote_kind"].as_str().unwrap_or("");
    assert!(
        kind == "indexer_pool_db" || kind == "indexer_hybrid_db",
        "unexpected quote_kind: {kind} notes={:?}",
        j["hybrid_notes"]
    );
    assert_eq!(j["fidelity_check"], "passed");
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(j["db_hybrid_queries"].as_u64().unwrap_or(0) > 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_fidelity_drift_downgrades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    // Router sim returns far less than mirror grid → drift guard.
    let (mock, _) = lcd_mock::start_router_only_route_mock("1").await;
    let mut cfg = db_hybrid_config(&mock);
    cfg.route_fidelity_drift_bps = 1;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=2000000",
        seed.token_a, seed.token_b
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["fidelity_check"], "drift");
    assert_eq!(j["quote_kind"], "indexer_hybrid_db_degraded");
    assert_eq!(j["estimated_amount_out"], "1");
}

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

async fn pair_id_for_route_pair(pool: &sqlx::PgPool) -> i32 {
    sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = 'terra1pairrouteabc'")
        .fetch_one(pool)
        .await
        .expect("route pair id")
}

/// Resting bids on the A/B route pair so the optimizer picks a non-zero book leg.
async fn seed_route_pair_bids(pool: &sqlx::PgPool, orders: &[resting_orders::RestingOrderInput]) {
    let pair_id = pair_id_for_route_pair(pool).await;
    resting_orders::replace_pair_resting_orders(pool, pair_id, Some(100), orders)
        .await
        .expect("seed resting book");
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_book_start_hint_paths() {
    let pool = common::setup_pool().await;

    // Live bid at head → hint equals that order id.
    {
        let seed = common::seed_route_solve_with_mirror(&pool).await;
        seed_route_pair_bids(
            &pool,
            &[resting_orders::RestingOrderInput {
                order_id: 42,
                side: "bid".to_string(),
                price: bd("5"),
                remaining: bd("50000000000"),
                owner: Some("terra1maker".to_string()),
                expires_at: None,
            }],
        )
        .await;

        let (mock, _) = lcd_mock::start_router_only_route_mock("8888888").await;
        let app =
            common::build_test_app_with_price_and_config(pool.clone(), None, db_hybrid_config(&mock))
                .await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=900000",
            seed.token_a, seed.token_b
        );
        let j: Value = server.get(&url).await.json();
        let hybrid = &j["router_operations"][0]["terra_swap"]["hybrid"];
        assert!(
            hybrid["book_input"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0)
                > 0,
            "expected book leg; hybrid={hybrid:?}"
        );
        assert_eq!(hybrid["book_start_hint"], 42);
    }

    // Expired head filtered from mirror → hint is first live order deeper in the book.
    {
        let seed = common::seed_route_solve_with_mirror(&pool).await;
        let now = chrono::Utc::now().timestamp();
        seed_route_pair_bids(
            &pool,
            &[
                resting_orders::RestingOrderInput {
                    order_id: 1,
                    side: "bid".to_string(),
                    price: bd("6"),
                    remaining: bd("50000000000"),
                    owner: Some("terra1expired".to_string()),
                    expires_at: Some(now - 60),
                },
                resting_orders::RestingOrderInput {
                    order_id: 77,
                    side: "bid".to_string(),
                    price: bd("5"),
                    remaining: bd("50000000000"),
                    owner: Some("terra1live".to_string()),
                    expires_at: None,
                },
            ],
        )
        .await;

        let (mock, _) = lcd_mock::start_router_only_route_mock("8888888").await;
        let app =
            common::build_test_app_with_price_and_config(pool.clone(), None, db_hybrid_config(&mock))
                .await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=800000",
            seed.token_a, seed.token_b
        );
        let j: Value = server.get(&url).await.json();
        let hybrid = &j["router_operations"][0]["terra_swap"]["hybrid"];
        assert!(
            hybrid["book_input"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0)
                > 0
        );
        assert_eq!(hybrid["book_start_hint"], 77);
    }

    // Stale mirror → omit hint (LCD fallback grid); book leg may still be non-zero.
    {
        let seed = common::seed_route_solve_with_mirror(&pool).await;
        let pair_id = pair_id_for_route_pair(&pool).await;
        seed_route_pair_bids(
            &pool,
            &[resting_orders::RestingOrderInput {
                order_id: 42,
                side: "bid".to_string(),
                price: bd("5"),
                remaining: bd("50000000000"),
                owner: Some("terra1maker".to_string()),
                expires_at: None,
            }],
        )
        .await;
        sqlx::query(
            "UPDATE pair_reserves SET snapshot_at = NOW() - INTERVAL '1 hour' WHERE pair_id = $1",
        )
        .bind(pair_id)
        .execute(&pool)
        .await
        .expect("age reserves snapshot");

        let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
        let app =
            common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=700000",
            seed.token_a, seed.token_b
        );
        let j: Value = server.get(&url).await.json();
        assert_eq!(j["quote_kind"], "indexer_hybrid_db_degraded");
        assert!(j["lcd_hybrid_queries"].as_u64().unwrap_or(0) > 0);
        let hybrid = &j["router_operations"][0]["terra_swap"]["hybrid"];
        if hybrid["book_input"]
            .as_str()
            .unwrap_or("0")
            .parse::<u128>()
            .unwrap_or(0)
            > 0
        {
            assert!(hybrid["book_start_hint"].is_null());
        }
    }
}

async fn upsert_pair_reserves(
    pool: &sqlx::PgPool,
    contract: &str,
    reserve_0: &str,
    reserve_1: &str,
) {
    use bigdecimal::BigDecimal;
    use cl8y_dex_indexer::db::queries::pair_reserves;
    use std::str::FromStr;

    let pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = $1")
        .bind(contract)
        .fetch_one(pool)
        .await
        .expect("pair id");
    let bd = |s: &str| BigDecimal::from_str(s).unwrap();
    pair_reserves::upsert_pair_reserves(
        pool,
        pair_id,
        &bd(reserve_0),
        &bd(reserve_1),
        30,
        Some(100),
    )
    .await
    .expect("upsert reserves");
}

/// Direct A↔C path stays viable when a longer candidate path touches a zero-reserve pair (#369).
#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_skips_zero_reserve_candidate_path() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_multi_path(&pool).await;

    upsert_pair_reserves(
        &pool,
        "terra1pairroutempac",
        "10000000000000",
        "10000000000000",
    )
    .await;
    upsert_pair_reserves(
        &pool,
        "terra1pairroutempab",
        "10000000000000",
        "10000000000000",
    )
    .await;
    upsert_pair_reserves(&pool, "terra1pairroutempbc", "0", "0").await;

    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app =
        common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    for (token_in, token_out) in [
        (seed.token_a.as_str(), seed.token_c.as_str()),
        (seed.token_c.as_str(), seed.token_a.as_str()),
    ] {
        let solve_url = format!(
            "/api/v1/route/solve?token_in={token_in}&token_out={token_out}&amount_in=1000000"
        );
        let resp = server.get(&solve_url).await;
        resp.assert_status_ok();
        let j: Value = resp.json();
        assert_eq!(
            j["hops"].as_array().unwrap().len(),
            1,
            "direct pool path; body={j:?}"
        );
        assert!(
            j["estimated_amount_out"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0)
                > 0
        );

        let best_url = solve_url.replace("/route/solve?", "/route/solve/best?");
        server.get(&best_url).await.assert_status_ok();
    }

    assert_eq!(
        hybrid_hits.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "DB hybrid grid must not LCD-fallback on zero-reserve hops"
    );
}
