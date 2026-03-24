mod common;

use axum::http::{header, HeaderValue, StatusCode};
use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn invalid_interval_rejected() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_intervals = &[
        "3h",
        "2m",
        "12h",
        "1M",
        "abc",
        "%27%3B%20DROP%20TABLE%20pairs%3B%20--",
    ];
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
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
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

    let acao = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(
        acao.is_some(),
        "should return ACAO header for allowed origin"
    );
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

    let acao = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
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
async fn cg_ticker_id_attack_matrix_all_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_tickers = [
        "",
        "_",
        "A",
        "A_",
        "_B",
        "A_B_C",
        "LUNC_USTC_EXTRA",
        "SINGLESEGMENT",
    ];
    for t in bad_tickers {
        let url = if t.is_empty() {
            "/cg/historical_trades?ticker_id=".to_string()
        } else {
            format!("/cg/historical_trades?ticker_id={}", t)
        };
        let resp = server.get(&url).await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn oracle_history_limit_capped_at_1000() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history?limit=999999").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let prices = body["prices"].as_array().expect("prices array");
    assert!(prices.len() <= 1000);
}

#[tokio::test]
async fn leaderboard_all_documented_sort_columns_accepted() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let sorts = [
        "total_volume",
        "volume_24h",
        "volume_7d",
        "volume_30d",
        "total_trades",
        "total_realized_pnl",
        "best_trade_pnl",
        "worst_trade_pnl",
        "total_fees_paid",
    ];
    for sort in sorts {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_ok();
    }
}

#[tokio::test]
async fn trader_trades_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?limit=99999",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn rate_limit_returns_429_when_exceeded() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 10;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::new(app);

    let mut saw_429 = false;
    for _ in 0..120 {
        let resp = server.get("/health").await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(saw_429, "expected governor to return 429 after burst");
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
