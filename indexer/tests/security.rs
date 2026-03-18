mod common;

use axum::http::{header, HeaderValue};
use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn invalid_interval_rejected() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_intervals = &["3h", "2m", "12h", "1M", "abc", "%27%3B%20DROP%20TABLE%20pairs%3B%20--"];
    for interval in bad_intervals {
        let resp = server
            .get(&format!(
                "/api/v1/pairs/{}/candles?interval={}",
                seed.pair_address, interval
            ))
            .await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn all_valid_intervals_accepted() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let valid_intervals = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];
    for interval in valid_intervals {
        let resp = server
            .get(&format!(
                "/api/v1/pairs/{}/candles?interval={}",
                seed.pair_address, interval
            ))
            .await;
        resp.assert_status_ok();
    }
}

#[tokio::test]
async fn invalid_sort_rejected() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_sorts = &[
        "unknown",
        "id",
        "address",
        "%27%3B%20DROP%20TABLE%20traders%3B%20--",
    ];
    for sort in bad_sorts {
        let resp = server
            .get(&format!(
                "/api/v1/traders/leaderboard?sort={}",
                sort
            ))
            .await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn error_responses_do_not_leak_internals() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs/nonexistent/stats").await;
    let body = resp.text();

    assert!(
        !body.contains("sqlx"),
        "error should not contain SQL library names"
    );
    assert!(
        !body.contains("SELECT"),
        "error should not contain SQL fragments"
    );
    assert!(
        !body.contains("postgres"),
        "error should not contain DB details"
    );
}

#[tokio::test]
async fn cors_allowed_origin_gets_headers() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/overview")
        .add_header(
            header::ORIGIN,
            "https://dex.cl8y.com".parse::<HeaderValue>().unwrap(),
        )
        .await;
    resp.assert_status_ok();

    let acao = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(acao.is_some(), "should return ACAO header for allowed origin");
    assert_eq!(acao.unwrap().to_str().unwrap(), "https://dex.cl8y.com");
}

#[tokio::test]
async fn cors_disallowed_origin_no_acao() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/overview")
        .add_header(
            header::ORIGIN,
            "https://evil.com".parse::<HeaderValue>().unwrap(),
        )
        .await;

    let acao = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(
        acao.is_none(),
        "should NOT return ACAO header for disallowed origin"
    );
}

#[tokio::test]
async fn trades_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn candles_limit_capped_at_1000() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 1000);
}

#[tokio::test]
async fn swagger_ui_available() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/swagger-ui/").await;
    resp.assert_status_ok();
    let body = resp.text();
    assert!(body.contains("swagger"), "swagger UI page should load");
}

#[tokio::test]
async fn openapi_spec_available() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api-docs/openapi.json").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["info"]["title"], "CL8Y DEX Indexer API");
    assert!(body["paths"].is_object());
    assert!(body["paths"]["/api/v1/pairs"].is_object());
}
