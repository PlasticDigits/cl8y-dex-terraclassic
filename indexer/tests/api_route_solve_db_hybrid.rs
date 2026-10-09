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
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "local_v1");
    let kind = j["quote_kind"].as_str().unwrap_or("");
    assert!(
        kind == "indexer_pool_db" || kind == "indexer_hybrid_db",
        "unexpected quote_kind: {kind} notes={:?}",
        j["hybrid_notes"]
    );
    assert_eq!(j["fidelity_check"], "skipped");
    assert!(mock.received_requests().await.unwrap().is_empty());
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(j["db_hybrid_queries"], 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_empty_book_skips_full_grid() {
    let pool = common::setup_pool().await;
    // seed_route_solve_with_mirror upserts reserves only — no resting orders.
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_b
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["solver_version"], "local_v1");
    assert_eq!(j["quote_kind"], "indexer_pool_db");
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    let db_q = j["db_hybrid_queries"].as_u64().unwrap_or(u64::MAX);
    // Full 1-hop grid is 17×3 (+ propagate) ≈ 50+; empty-book short-circuit stays tiny (#493).
    assert!(
        db_q == 0,
        "empty-book cold solve must not run full 17×3 grid; db_hybrid_queries={db_q}"
    );
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_live_book_uses_bounded_local_scoring() {
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
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
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
        db_q == 0,
        "live book uses bounded local scoring without request DB queries; db_hybrid_queries={db_q}"
    );
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_skips_zero_reserve_path_candidate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_zero_reserve_poison(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
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
        assert_eq!(j["solver_version"], "local_v1");
        assert_eq!(
            j["hops"].as_array().unwrap().len(),
            1,
            "direct funded pair must win over poisoned multi-hop path: {j:?}"
        );
        assert!(
            j["estimated_amount_out"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
    // Poisoned multi-hop may trigger LCD fallback grid evals before skip; direct path must not need them.
    let _ = hybrid_hits;
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_never_calls_remote_fidelity() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    // Router sim returns far less than mirror grid → drift guard.
    let (mock, _) = lcd_mock::start_router_only_route_mock("1").await;
    let mut cfg = db_hybrid_config(&mock);
    cfg.route_fidelity_drift_bps = 1;
    let app = common::local_quotes::app(pool, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=2000000",
        seed.token_a, seed.token_b
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["fidelity_check"], "skipped");
    assert_eq!(j["quote_kind"], "indexer_pool_db");
    assert!(
        j["estimated_amount_out"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 1
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// Policy A strip vs fidelity: LCD sim of hop-0-only ops must compare to DB-sim of
/// the same stripped plan, not the joint interior-hybrid `grid_out`.
#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_2hop_live_book_fidelity_uses_stripped_plan() {
    use cl8y_dex_indexer::api::db_orderbook_sim;
    use cl8y_dex_indexer::api::hybrid_route_opt::{
        self, HopDescriptor, HybridSimSource, QuoteTrader,
    };
    use cl8y_dex_indexer::lcd::LcdClient;
    use sqlx::Row;
    use std::collections::HashMap;

    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop_with_mirror(&pool).await;
    let book = [resting_orders::RestingOrderInput {
        order_id: 42,
        side: "bid".to_string(),
        price: bd("5"),
        remaining: bd("50000000000"),
        owner: Some("terra1maker".to_string()),
        expires_at: None,
    }];
    seed_pair_bids_at(&pool, "terra1pairrouteabc", &book).await;
    seed_pair_bids_at(&pool, "terra1pairroutebcd", &book).await;

    let amount_in: u128 = 900_000;
    let hops = [
        HopDescriptor {
            pair: "terra1pairrouteabc".into(),
            offer_token: seed.token_a.clone(),
            ask_token: seed.token_b.clone(),
        },
        HopDescriptor {
            pair: "terra1pairroutebcd".into(),
            offer_token: seed.token_b.clone(),
            ask_token: seed.token_c.clone(),
        },
    ];
    let mut id_to_addr = HashMap::new();
    let asset_rows = sqlx::query("SELECT id, contract_address FROM assets")
        .fetch_all(&pool)
        .await
        .expect("assets");
    for row in asset_rows {
        let id: i32 = row.get("id");
        let addr: String = row.get("contract_address");
        id_to_addr.insert(id, addr);
    }
    let pair_addrs = vec![
        "terra1pairrouteabc".to_string(),
        "terra1pairroutebcd".to_string(),
    ];
    let mirrors =
        db_orderbook_sim::preload_mirrors_for_pairs(&pool, &pair_addrs, &id_to_addr, 60_000)
            .await
            .expect("preload mirrors");
    let lcd = LcdClient::new(vec!["http://127.0.0.1:9".into()], 50, 1_000, "columbus-5");
    let source = HybridSimSource::Db {
        lcd_fallback: &lcd,
        mirrors: &mirrors,
        discount_bps: 0,
    };
    let quote = QuoteTrader::default();
    let mut mirror_meta = db_orderbook_sim::MirrorLoadMeta::default();
    let (plan, _, joint_out) = hybrid_route_opt::optimize_multihop_hybrid_joint(
        &source,
        Some(&mut mirror_meta),
        &hops,
        amount_in,
        8,
        &quote,
    )
    .await
    .expect("joint optimize");
    let hop1_book = plan
        .get(1)
        .and_then(|h| h.as_ref())
        .and_then(|h| h.book_input.parse::<u128>().ok())
        .unwrap_or(0);
    assert!(
        hop1_book > 0,
        "joint grid should declare hop-1 book so unstripped fidelity would drift; plan={plan:?}"
    );
    let stripped = hybrid_route_opt::retail_declared_hybrid_plan_hop0_only(plan);
    let stripped_out = hybrid_route_opt::propagate_offer_through_plan(
        &source,
        Some(&mut mirror_meta),
        &hops,
        &stripped,
        amount_in,
        hops.len(),
        8,
        &quote,
    )
    .await
    .expect("stripped propagate");
    assert!(
        joint_out > stripped_out,
        "interior book must raise joint grid_out; joint={joint_out} stripped={stripped_out}"
    );
    let would_drift_bps = (joint_out - stripped_out).saturating_mul(10_000) / stripped_out.max(1);
    assert!(
        would_drift_bps > 100,
        "test setup must exceed default fidelity bps without the strip recompute; {would_drift_bps}"
    );

    let (mock, hybrid_hits) =
        lcd_mock::start_router_only_route_mock(&stripped_out.to_string()).await;
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={amount_in}",
        seed.token_a, seed.token_c
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["solver_version"], "local_v1");
    assert_eq!(j["hops"].as_array().unwrap().len(), 2);
    let ops = j["router_operations"].as_array().unwrap();
    assert!(ops[1]["terra_swap"]["hybrid"].is_null());
    assert_eq!(j["fidelity_check"], "skipped");
    assert!(mock.received_requests().await.unwrap().is_empty());
    let kind = j["quote_kind"].as_str().unwrap_or("");
    assert!(
        kind == "indexer_hybrid_db" || kind == "indexer_pool_db",
        "healthy stripped quote must not degrade; quote_kind={kind} notes={:?}",
        j["hybrid_notes"]
    );
    assert_eq!(j["estimated_amount_out"], stripped_out.to_string());
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

/// #1399: loading must preserve expired/zero/invalid scan positions, not just unit walk fixtures.
#[serial]
#[tokio::test]
async fn db_hybrid_loader_preserves_legacy_prefix_until_bounded_cleanup() {
    use cl8y_dex_indexer::api::db_orderbook_sim::{
        load_hop_mirror, simulate_hybrid_from_mirror, simulate_pool_only_from_mirror,
    };
    use cl8y_dex_indexer::db::queries::pairs;
    use std::collections::HashMap;

    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let pair = pairs::get_pair_by_address(&pool, "terra1pairrouteabc")
        .await
        .unwrap()
        .unwrap();
    let addresses = HashMap::from([
        (pair.asset_0_id, seed.token_a.clone()),
        (pair.asset_1_id, seed.token_b.clone()),
    ]);
    let now = chrono::Utc::now().timestamp();
    for side in ["bid", "ask"] {
        let bad_price = if side == "bid" { "20" } else { "0.000000001" };
        let mut rows: Vec<_> = (0..501)
            .map(|index| {
                resting_orders::RestingOrderInput {
                    order_id: index + 1,
                    side: side.into(),
                    price: bd(bad_price),
                    remaining: bd(match index % 3 {
                        0 => "1",             // Live but economically unfillable.
                        1 => "1000000000000", // Expired but otherwise fillable.
                        _ => "0",
                    }),
                    owner: Some("terra1syntheticlegacy".into()),
                    expires_at: (index % 3 == 1).then_some(now - 60),
                }
            })
            .collect();
        rows.push(resting_orders::RestingOrderInput {
            order_id: 502,
            side: side.into(),
            price: bd(if side == "bid" { "1" } else { "0.5" }),
            remaining: bd("100"),
            owner: Some("terra1syntheticfillable".into()),
            expires_at: None,
        });
        seed_route_pair_bids(&pool, &rows).await;
        let load = || {
            load_hop_mirror(
                &pool,
                &pair.contract_address,
                &addresses,
                &pair,
                3_600_000,
                now as u64,
            )
        };
        let before = load().await.unwrap();
        let orders = if side == "bid" {
            &before.bids
        } else {
            &before.asks
        };
        assert_eq!(
            orders.len(),
            502,
            "loader must retain every legacy scan step"
        );
        assert_eq!(orders[0].order_id, 1);
        assert_eq!(orders[1].expires_at, Some(now - 60));
        assert_eq!(orders[2].remaining, bd("0"));
        assert_eq!(orders[501].order_id, 502);
        let offer = if side == "bid" {
            &seed.token_a
        } else {
            &seed.token_b
        };
        let pool_only = simulate_pool_only_from_mirror(&before, offer, 10, 10_000).unwrap();
        let blocked =
            simulate_hybrid_from_mirror(&before, offer, 10, 0, 10, 1, 10_000, None).unwrap();
        assert_eq!(
            blocked, pool_only,
            "head scan ceiling must hide the deeper maker"
        );

        // Mirror the next chain snapshot after a bounded cleanup removed two head rows.
        seed_route_pair_bids(&pool, &rows[2..]).await;
        let after = load().await.unwrap();
        let orders = if side == "bid" {
            &after.bids
        } else {
            &after.asks
        };
        assert_eq!(orders.len(), 500);
        assert_eq!(orders[0].order_id, 3);
        let reachable =
            simulate_hybrid_from_mirror(&after, offer, 10, 0, 10, 1, 10_000, None).unwrap();
        assert_eq!(reachable, if side == "bid" { 10 } else { 20 });
        assert!(reachable > pool_only);
    }
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
    seed_pair_bids_at(pool, "terra1pairrouteabc", orders).await;
}

async fn seed_pair_bids_at(
    pool: &sqlx::PgPool,
    pair_addr: &str,
    orders: &[resting_orders::RestingOrderInput],
) {
    let pair_id: i32 = sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address = $1")
        .bind(pair_addr)
        .fetch_one(pool)
        .await
        .expect("pair id");
    resting_orders::replace_pair_resting_orders(pool, pair_id, Some(100), orders)
        .await
        .expect("seed resting book");
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_book_scan_matches_emitted_plan_and_stale_fail_closed() {
    let pool = common::setup_pool().await;

    // Live head is consumed by the ordinary bounded scanner; hints stay absent.
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
        let app = common::local_quotes::app(pool.clone(), db_hybrid_config(&mock)).await;
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
        assert_local_book_plan(&pool, &seed.token_a, 900000, &j).await;
        assert!(mock.received_requests().await.unwrap().is_empty());
    }

    // Expired head counts as scan work and cannot change the executable plan output.
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
        let app = common::local_quotes::app(pool.clone(), db_hybrid_config(&mock)).await;
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
        assert_local_book_plan(&pool, &seed.token_a, 800000, &j).await;
        assert!(mock.received_requests().await.unwrap().is_empty());
    }

    // Stale required mirror fails closed; no request-time LCD fallback.
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
            "UPDATE hybrid_pair_reserves SET snapshot_at = NOW() - INTERVAL '1 hour' WHERE pair_id = $1",
        )
        .bind(pair_id)
        .execute(&pool)
        .await
        .expect("age reserves snapshot");

        let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
        let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=700000",
            seed.token_a, seed.token_b
        );
        let response = server.get(&url).await;
        // A stale required mirror fails closed; no legacy LCD-grid escape is permitted.
        response.assert_status_unprocessable_entity();
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

/// #1218 AC7: five ~100% drain 2-hops must not occupy top-K ahead of an honest 3-hop.
#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_skip_unusable_admits_honest_3hop() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_skip_unusable_drain(&pool).await;
    let (mock, _) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::local_quotes::app(pool, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "local_v1");
    let hops = j["hops"].as_array().expect("hops");
    assert_eq!(
        hops.len(),
        3,
        "honest 3-hop must win over five drain 2-hops: {j:?}"
    );
    let intermediates = j["intermediate_tokens"]
        .as_array()
        .expect("intermediate_tokens");
    let labels: Vec<&str> = intermediates.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        labels.iter().any(|t| t.contains("skipunusableh1"))
            && labels.iter().any(|t| t.contains("skipunusableh2")),
        "winner path must walk the hub: {labels:?}"
    );
    let considered = j["paths_considered"].as_u64().unwrap_or(0);
    assert!(
        considered >= 1 && considered <= 512,
        "paths_considered should be scored usable paths, got {considered}"
    );
    assert!(
        j["optimality_scope"]
            .as_str()
            .unwrap_or("")
            .contains("eligible"),
        "optimality_scope must mention usable-path fill"
    );
}

/// Independent DB simulator checks the emitted split, not an old router mock value.
async fn assert_local_book_plan(pool: &sqlx::PgPool, token: &str, amount: u128, response: &Value) {
    use cl8y_dex_indexer::api::db_orderbook_sim::{load_hop_mirror, simulate_hybrid_from_mirror};
    use cl8y_dex_indexer::db::queries::{assets, pairs};
    let pair = pairs::get_pair_by_address(pool, "terra1pairrouteabc")
        .await
        .unwrap()
        .unwrap();
    let addresses = assets::get_all_assets(pool)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|a| {
            a.contract_address
                .or(a.denom)
                .map(|address| (a.id, address))
        })
        .collect();
    let mirror = load_hop_mirror(
        pool,
        &pair.contract_address,
        &addresses,
        &pair,
        60_000,
        chrono::Utc::now().timestamp() as u64,
    )
    .await
    .unwrap();
    let hybrid = &response["router_operations"][0]["terra_swap"]["hybrid"];
    assert!(hybrid["book_start_hint"].is_null());
    let book = hybrid["book_input"]
        .as_str()
        .unwrap()
        .parse::<u128>()
        .unwrap();
    let pool_input = hybrid["pool_input"]
        .as_str()
        .unwrap()
        .parse::<u128>()
        .unwrap();
    assert_eq!(pool_input + book, amount);
    let expected =
        simulate_hybrid_from_mirror(&mirror, token, amount, pool_input, book, 8, 0, None).unwrap();
    assert_eq!(response["estimated_amount_out"], expected.to_string());
    assert_eq!(
        response["execution_metadata"]["hops"][0]["min_return"],
        response["router_operations"][0]["terra_swap"]["min_return"]
    );
    assert!(
        response["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
}
