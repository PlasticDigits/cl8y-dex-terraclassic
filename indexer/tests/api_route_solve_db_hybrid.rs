//! GitLab #319: DB-backed hybrid route solver (Phase 1c).
//! GitLab #332: `book_start_hint` on optimized hybrid hops.
//! GitLab #369: skip zero-reserve path candidates instead of 502 on viable direct route.
//! GitLab #493: empty-book hybrid grid short-circuit.

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
async fn route_solve_db_hybrid_empty_book_skips_full_grid() {
    let pool = common::setup_pool().await;
    // seed_route_solve_with_mirror upserts reserves only — no resting orders.
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_b
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["solver_version"], "global_v4");
    assert_eq!(j["quote_kind"], "indexer_pool_db");
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    let db_q = j["db_hybrid_queries"].as_u64().unwrap_or(u64::MAX);
    // Full 1-hop grid is 17×3 (+ propagate) ≈ 50+; empty-book short-circuit stays tiny (#493).
    assert!(
        db_q > 0 && db_q < 10,
        "empty-book cold solve must not run full 17×3 grid; db_hybrid_queries={db_q}"
    );
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_live_book_still_grids() {
    let pool = common::setup_pool().await;
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

    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
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
        "live book must still optimize; hybrid={hybrid:?}"
    );
    let db_q = j["db_hybrid_queries"].as_u64().unwrap_or(0);
    assert!(
        db_q >= 17,
        "live book must run the 17-point grid; db_hybrid_queries={db_q}"
    );
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_skips_zero_reserve_path_candidate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_zero_reserve_poison(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    for path in [
        format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
            seed.token_a, seed.token_c
        ),
        format!(
            "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in=1000000",
            seed.token_c, seed.token_a
        ),
    ] {
        let resp = server.get(&path).await;
        resp.assert_status_ok();
        let j: Value = resp.json();
        assert_eq!(j["solver_version"], "global_v4");
        assert_eq!(
            j["hops"].as_array().unwrap().len(),
            1,
            "direct funded pair must win over poisoned multi-hop path: {j:?}"
        );
        assert_eq!(j["estimated_amount_out"], "8888888");
    }
    // Poisoned multi-hop may trigger LCD fallback grid evals before skip; direct path must not need them.
    let _ = hybrid_hits;
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
