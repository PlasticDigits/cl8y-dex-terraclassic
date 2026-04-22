//! HTTP API: routing, CORS, rate limits, timeouts, and ticker/orderbook caches.
//! Invariants and threat model: see repository `docs/indexer-invariants.md`.

mod cg;
mod cmc;
pub mod hooks;
mod limit_book_lcd;
mod oracle;
pub mod orderbook_sim;
mod overview;
mod pairs;
mod route_solver;
mod tokens;
mod traders;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use sqlx::PgPool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::db::queries::{assets, pairs as db_pairs};
use crate::indexer::oracle::SharedPrice;
use crate::lcd::LcdClient;

const TICKER_MAP_TTL: Duration = Duration::from_secs(30);

/// Cached `BASE_TARGET` ticker string → pair contract address (refreshed periodically).
#[derive(Clone, Default)]
pub struct TickerMapCache {
    pub(crate) inner:
        std::sync::Arc<tokio::sync::RwLock<Option<(HashMap<String, String>, Instant)>>>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub lcd: LcdClient,
    pub ustc_price: SharedPrice,
    pub ticker_map_cache: TickerMapCache,
    pub orderbook_cache: orderbook_sim::OrderbookCache,
    /// Set when `ROUTER_ADDRESS` is configured (LCD simulation in route solver).
    pub router_address: Option<String>,
}

pub fn internal_err(e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!("Internal error: {}", e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

async fn build_asset_map(pool: &PgPool) -> Result<HashMap<i32, assets::AssetRow>, sqlx::Error> {
    let all = assets::get_all_assets(pool).await?;
    Ok(all.into_iter().map(|a| (a.id, a)).collect())
}

/// CoinGecko `ticker_id` / CMC `market_pair`: exactly `BASE_TARGET` with non-empty symbols.
/// Rejects `A_B_C`, `_`, `A_`, `__`, encoded slashes, etc.
pub(crate) fn cg_ticker_segments(ticker_id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = ticker_id.split('_').collect();
    if parts.len() != 2 {
        return None;
    }
    let (a, b) = (parts[0], parts[1]);
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some((a, b))
}

pub async fn find_pair_by_ticker(
    state: &AppState,
    ticker_id: &str,
) -> Result<String, (StatusCode, String)> {
    if cg_ticker_segments(ticker_id).is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid ticker_id format, expected BASE_TARGET".to_string(),
        ));
    }

    let now = Instant::now();

    {
        let guard = state.ticker_map_cache.inner.read().await;
        if let Some((ref map, exp)) = *guard {
            if now < exp {
                return match map.get(ticker_id) {
                    Some(addr) => Ok(addr.clone()),
                    None => Err((
                        StatusCode::NOT_FOUND,
                        format!("Pair not found for ticker: {}", ticker_id),
                    )),
                };
            }
        }
    }

    let mut write = state.ticker_map_cache.inner.write().await;
    if let Some((ref map, exp)) = *write {
        if now < exp {
            return match map.get(ticker_id) {
                Some(addr) => Ok(addr.clone()),
                None => Err((
                    StatusCode::NOT_FOUND,
                    format!("Pair not found for ticker: {}", ticker_id),
                )),
            };
        }
    }

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut map = HashMap::new();
    for p in &all_pairs {
        if let (Some(a0), Some(a1)) = (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            let key = format!("{}_{}", a0.symbol, a1.symbol);
            map.entry(key).or_insert_with(|| p.contract_address.clone());
        }
    }

    let result = map.get(ticker_id).cloned();
    *write = Some((map, now + TICKER_MAP_TTL));

    result.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Pair not found for ticker: {}", ticker_id),
        )
    })
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "CL8Y DEX Indexer API",
        version = "1.0.0",
        description = "Indexer API for CL8Y DEX on Terra Classic — charting, analytics, CoinGecko and CoinMarketCap integrations.",
    ),
    paths(
        route_solver::solve_route,
        route_solver::solve_route_post,
        pairs::list_pairs,
        pairs::get_pair,
        pairs::get_pair_candles,
        pairs::get_pair_trades,
        pairs::get_pair_liquidity_events,
        pairs::get_pair_limit_placements,
        pairs::get_pair_limit_cancellations,
        pairs::get_pair_limit_fills,
        pairs::get_pair_order_limit_fills,
        pairs::get_pair_order_book_head,
        pairs::get_pair_limit_book_shallow,
        pairs::get_pair_limit_book,
        pairs::get_pair_stats,
        hooks::get_hook_events,
        tokens::list_tokens,
        tokens::get_token,
        tokens::get_token_pairs,
        traders::get_trader_profile,
        traders::get_trader_trades,
        traders::get_trader_positions,
        traders::leaderboard,
        overview::get_overview,
        oracle::get_oracle_price,
        oracle::get_oracle_history,
        cg::cg_pairs,
        cg::cg_tickers,
        cg::cg_orderbook,
        cg::cg_historical_trades,
        cmc::cmc_summary,
        cmc::cmc_assets,
        cmc::cmc_ticker,
        cmc::cmc_orderbook,
        cmc::cmc_trades,
    ),
    components(schemas(
        route_solver::SolveRouteParams,
        route_solver::SolveRoutePostBody,
        route_solver::HybridHopJson,
        route_solver::RouteHop,
        route_solver::RouteSolveResponse,
        pairs::PairResponse,
        pairs::PairListResponse,
        pairs::ListPairsQuery,
        pairs::AssetBrief,
        pairs::CandleResponse,
        pairs::TradeResponse,
        pairs::TradesQuery,
        pairs::LiquidityEventsQuery,
        pairs::LiquidityEventResponse,
        pairs::LimitPlacementResponse,
        pairs::LimitCancellationResponse,
        pairs::LimitFillResponse,
        pairs::OrderBookHeadQuery,
        pairs::OrderBookHeadResponse,
        pairs::LimitBookShallowQuery,
        pairs::LimitBookShallowResponse,
        pairs::LimitBookOrderItem,
        pairs::LimitBookPagedQuery,
        pairs::LimitBookPagedResponse,
        pairs::PairStatsResponse,
        hooks::HookEventsQuery,
        hooks::HookEventResponse,
        tokens::TokenResponse,
        tokens::TokenDetailResponse,
        tokens::VolumeStatResponse,
        traders::TraderResponse,
        traders::PositionResponse,
        overview::OverviewResponse,
        cg::CgPairResponse,
        cg::CgTickerResponse,
        cg::CgOrderbookResponse,
        cg::CgTradeEntry,
        cg::CgHistoricalTradesResponse,
        cmc::CmcSummaryEntry,
        cmc::CmcAssetEntry,
        cmc::CmcTickerEntry,
        cmc::CmcOrderbookResponse,
        cmc::CmcTradeEntry,
        oracle::OraclePriceResponse,
        oracle::OracleSourcePrice,
        oracle::OracleHistoryEntry,
        oracle::OracleHistoryResponse,
    )),
    tags(
        (name = "Routing", description = "Multihop route discovery for swaps"),
        (name = "Pairs", description = "Trading pair endpoints"),
        (name = "Tokens", description = "Token/asset endpoints"),
        (name = "Traders", description = "Trader profile and leaderboard"),
        (name = "Overview", description = "Global DEX statistics"),
        (name = "Oracle", description = "USTC/USD oracle price feeds"),
        (name = "CoinGecko", description = "CoinGecko-compatible endpoints"),
        (name = "CoinMarketCap", description = "CoinMarketCap-compatible endpoints"),
        (name = "Hooks", description = "Post-swap hook execution events"),
    )
)]
struct ApiDoc;

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok"}))
}

async fn prometheus_metrics() -> Response {
    match crate::metrics::gather_text() {
        Ok(body) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
            .body(Body::from(body))
            .unwrap_or_else(|e| {
                tracing::error!("metrics response build: {}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            }),
        Err(e) => {
            tracing::error!("Prometheus encode error: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Internal metrics error"))
                .unwrap()
        }
    }
}

pub fn build_router(state: AppState, config: &Config) -> Router {
    let mut origins = Vec::new();
    for o in &config.cors_origins {
        match o.parse::<HeaderValue>() {
            Ok(v) => origins.push(v),
            Err(_) => {
                tracing::warn!("Skipping invalid CORS origin: {}", o);
                continue;
            }
        }
    }

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let api_router = Router::new()
        .route("/health", get(health))
        .route("/api/v1/pairs", get(pairs::list_pairs))
        .route("/api/v1/pairs/{addr}", get(pairs::get_pair))
        .route("/api/v1/pairs/{addr}/candles", get(pairs::get_pair_candles))
        .route("/api/v1/pairs/{addr}/trades", get(pairs::get_pair_trades))
        .route(
            "/api/v1/pairs/{addr}/liquidity-events",
            get(pairs::get_pair_liquidity_events),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-placements",
            get(pairs::get_pair_limit_placements),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-cancellations",
            get(pairs::get_pair_limit_cancellations),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-fills",
            get(pairs::get_pair_limit_fills),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-orders/{order_id}/fills",
            get(pairs::get_pair_order_limit_fills),
        )
        .route(
            "/api/v1/pairs/{addr}/order-book-head",
            get(pairs::get_pair_order_book_head),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-book-shallow",
            get(pairs::get_pair_limit_book_shallow),
        )
        .route(
            "/api/v1/pairs/{addr}/limit-book",
            get(pairs::get_pair_limit_book),
        )
        .route("/api/v1/pairs/{addr}/stats", get(pairs::get_pair_stats))
        .route("/api/v1/tokens", get(tokens::list_tokens))
        .route("/api/v1/tokens/{addr}", get(tokens::get_token))
        .route("/api/v1/tokens/{addr}/pairs", get(tokens::get_token_pairs))
        .route("/api/v1/traders/leaderboard", get(traders::leaderboard))
        .route("/api/v1/traders/{addr}", get(traders::get_trader_profile))
        .route(
            "/api/v1/traders/{addr}/trades",
            get(traders::get_trader_trades),
        )
        .route(
            "/api/v1/traders/{addr}/positions",
            get(traders::get_trader_positions),
        )
        .route("/api/v1/hooks", get(hooks::get_hook_events))
        .route("/api/v1/overview", get(overview::get_overview))
        .route(
            "/api/v1/route/solve",
            get(route_solver::solve_route).post(route_solver::solve_route_post),
        )
        .route("/api/v1/oracle/price", get(oracle::get_oracle_price))
        .route("/api/v1/oracle/history", get(oracle::get_oracle_history))
        .route("/cg/pairs", get(cg::cg_pairs))
        .route("/cg/tickers", get(cg::cg_tickers))
        .route("/cg/orderbook", get(cg::cg_orderbook))
        .route("/cg/historical_trades", get(cg::cg_historical_trades))
        .route("/cmc/summary", get(cmc::cmc_summary))
        .route("/cmc/assets", get(cmc::cmc_assets))
        .route("/cmc/ticker", get(cmc::cmc_ticker))
        .route("/cmc/orderbook/{market_pair}", get(cmc::cmc_orderbook))
        .route("/cmc/trades/{market_pair}", get(cmc::cmc_trades))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));

    let api_router = if config.rate_limit_rps > 0 {
        let governor_conf = GovernorConfigBuilder::default()
            .per_second(config.rate_limit_rps)
            .burst_size(config.rate_limit_rps as u32 * 2)
            .use_headers()
            .finish()
            .expect("failed to build governor config");

        let governor_limiter = governor_conf.limiter().clone();
        let cleanup_interval = std::time::Duration::from_secs(60);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(cleanup_interval).await;
                governor_limiter.retain_recent();
            }
        });

        api_router.layer(GovernorLayer::new(governor_conf))
    } else {
        api_router
    };

    let router = if config.metrics_enabled {
        Router::new()
            .route("/metrics", get(prometheus_metrics))
            .merge(api_router)
    } else {
        api_router
    };

    router
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .with_state(state)
}

pub async fn serve(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
    ustc_price: SharedPrice,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router_address = config.router_address.clone();
    let state = AppState {
        pool,
        lcd,
        ustc_price,
        ticker_map_cache: TickerMapCache::default(),
        orderbook_cache: orderbook_sim::OrderbookCache::default(),
        router_address,
    };
    let app = build_router(state, &config);

    let addr = format!("{}:{}", config.api_bind, config.api_port);
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod cg_ticker_tests {
    use super::cg_ticker_segments;

    #[test]
    fn valid_base_target_accepted() {
        assert_eq!(cg_ticker_segments("LUNC_USTC"), Some(("LUNC", "USTC")));
    }

    #[test]
    fn attack_and_edge_matrix_rejected() {
        let bad = [
            "",
            "_",
            "A",
            "A_",
            "_B",
            "A_B_C",
            "A__B",
            "__",
            "BASE_TARGET_EXTRA",
            "LUNC_USTC_extra",
            "unicode_test_\u{2603}_bad",
            "SINGLESEGMENT",
        ];
        for s in bad {
            assert!(cg_ticker_segments(s).is_none(), "expected reject: {:?}", s);
        }
    }
}

#[cfg(test)]
mod cg_ticker_proptest {
    use super::cg_ticker_segments;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn valid_base_target_from_two_segments_without_underscore(
            a in "[a-zA-Z0-9]{1,24}",
            b in "[a-zA-Z0-9]{1,24}",
        ) {
            prop_assume!(!a.contains('_') && !b.contains('_'));
            let t = format!("{}_{}", a, b);
            let got = cg_ticker_segments(&t);
            prop_assert_eq!(got, Some((a.as_str(), b.as_str())));
        }

        #[test]
        fn three_segments_rejected(
            a in "[a-z]{1,8}",
            b in "[a-z]{1,8}",
            c in "[a-z]{1,8}",
        ) {
            let t = format!("{}_{}_{}", a, b, c);
            prop_assert!(cg_ticker_segments(&t).is_none());
        }
    }
}
