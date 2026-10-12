//! Owned socket regressions for the dynamic quote HTTP cache boundary (#1479).
mod common;

use axum::Router;
use cl8y_dex_indexer::{api, lcd::LcdClient};
use reqwest::{header::CACHE_CONTROL, StatusCode};
use serde_json::{json, Value};
use serial_test::serial;
use std::net::SocketAddr;

async fn serve(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    (base, task)
}

async fn quote_json(response: reqwest::Response, expected: StatusCode) -> Value {
    assert_eq!(response.status(), expected);
    assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    response.json().await.unwrap()
}

#[tokio::test]
#[serial]
async fn issue1479_quote_http_no_store_preserves_net_floors_errors_and_zero_rpc() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![upstream.uri()];
    let (base, server) = serve(common::local_quotes::app(pool.clone(), cfg.clone()).await).await;
    let client = reqwest::Client::new();
    let params = [
        ("token_in", seed.token_a.as_str()),
        ("token_out", seed.token_b.as_str()),
        ("amount_in", "1000000"),
        ("max_spread", "0.05"),
        ("pool_only", "true"),
    ];
    // Independent integer pool/fee arithmetic, including a reused result cache.
    let gross = 10_000_000_000_000u128 * 1_000_000 / (10_000_000_000_000 + 1_000_000);
    let net = gross - gross * 30 / 10_000;
    let floor = net - net * 5 / 100;
    for endpoint in [
        "/api/v1/route/solve",
        "/api/v1/route/solve",
        "/api/v1/route/solve/best",
    ] {
        let body = quote_json(
            client
                .get(format!("{base}{endpoint}"))
                .query(&params)
                .send()
                .await
                .unwrap(),
            StatusCode::OK,
        )
        .await;
        assert_eq!(body["estimated_amount_out_net"], net.to_string());
        assert_eq!(
            body["execution_metadata"]["minimum_receive"],
            floor.to_string()
        );
        assert_eq!(
            body["router_operations"][0]["terra_swap"]["min_return"],
            floor.to_string()
        );
    }
    let receive = quote_json(
        client
            .get(format!("{base}/api/v1/route/solve"))
            .query(&[
                ("token_in", seed.token_a.as_str()),
                ("token_out", seed.token_b.as_str()),
                ("amount_out", "1000000"),
                ("max_spread", "0.05"),
            ])
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
    )
    .await;
    assert!(
        receive["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            >= 1_000_000
    );
    assert!(
        receive["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            >= 1_000_000
    );
    let posted = quote_json(
        client
            .post(format!("{base}/api/v1/route/solve"))
            .json(&json!({
                "token_in":seed.token_a,"token_out":seed.token_b,"amount_in":"1000000",
                "max_spread":"0.05","hybrid_by_hop":[null]
            }))
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(posted["estimated_amount_out_net"], net.to_string());
    let progress = quote_json(
        client
            .get(format!("{base}/api/v1/route/solve/progress"))
            .query(&params)
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
    )
    .await;
    assert!(progress["quote_generation"].as_str().is_some());
    assert!(progress["estimated_amount_out_net"].is_null());
    let dust = quote_json(
        client
            .get(format!("{base}/api/v1/route/solve"))
            .query(&[
                ("token_in", seed.token_a.as_str()),
                ("token_out", seed.token_b.as_str()),
                ("amount_in", "1"),
            ])
            .send()
            .await
            .unwrap(),
        StatusCode::UNPROCESSABLE_ENTITY,
    )
    .await;
    assert_eq!(dust["error"], "amount_too_small");
    assert!(dust["router_operations"].is_null());
    let invalid = quote_json(
        client
            .get(format!("{base}/api/v1/route/solve"))
            .query(&[
                ("token_in", seed.token_a.as_str()),
                ("token_out", seed.token_b.as_str()),
                ("amount_in", "0"),
            ])
            .send()
            .await
            .unwrap(),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(invalid["error"], "invalid_request");
    // Body-limit rejections occur before handlers, but have the same boundary.
    let oversized = client
        .post(format!("{base}/api/v1/route/solve"))
        .header("content-type", "application/json")
        .body("x".repeat(cl8y_dex_indexer::config::ROUTE_SOLVE_POST_BODY_LIMIT_BYTES + 1))
        .send()
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(oversized.headers().get(CACHE_CONTROL).unwrap(), "no-store");
    // Catalog/history and health retain their own existing HTTP cache contract.
    for endpoint in ["/health", "/api/v1/pairs"] {
        let response = client
            .get(format!("{base}{endpoint}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_ne!(
            response
                .headers()
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
    }
    assert!(upstream.received_requests().await.unwrap().is_empty());
    server.abort();
    let _ = server.await;
    // A cold namespace has an explicit availability blocker, never HTTP caching.
    let cold = LcdClient::new(vec!["http://127.0.0.1:1".into()], 1000, 0, "columbus-5");
    let cold_state = common::local_quotes::state(pool, &cfg, cold);
    let (cold_base, cold_server) = serve(api::build_router(cold_state, &cfg)).await;
    let missing = quote_json(
        client
            .get(format!("{cold_base}/api/v1/route/solve"))
            .query(&params)
            .send()
            .await
            .unwrap(),
        StatusCode::SERVICE_UNAVAILABLE,
    )
    .await;
    assert_eq!(missing["error"], "missing_observations");
    cold_server.abort();
    let _ = cold_server.await;
}

#[tokio::test]
#[serial]
async fn issue1479_quote_http_no_store_covers_outer_rate_limit_rejections() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let mut cfg = common::test_config();
    cfg.rate_limit_rps = 1;
    let lcd = LcdClient::new(vec!["http://127.0.0.1:1".into()], 1000, 0, "columbus-5");
    let (base, server) = serve(api::build_router(
        common::local_quotes::state(pool, &cfg, lcd),
        &cfg,
    ))
    .await;
    let client = reqwest::Client::new();
    let mut limited = false;
    for _ in 0..8 {
        let response = client
            .get(format!(
                "{base}/api/v1/route/solve/progress?token_in=a&token_out=b&amount_in=0"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-store");
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            limited = true;
            break;
        }
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    assert!(
        limited,
        "real peer-IP governor must reject the bounded burst"
    );
    server.abort();
    let _ = server.await;
}
