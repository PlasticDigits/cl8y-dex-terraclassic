//! Fixed-shape quote correlation without logging actor identifiers or query URIs.
use std::sync::{
    atomic::{AtomicU64, Ordering},
    OnceLock,
};

use axum::{
    body::Body,
    extract::Request,
    http::{header::HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;

pub(super) const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-quote-request-id");
static NEXT_REQUEST_ID: OnceLock<AtomicU64> = OnceLock::new();

/// Tower's default HTTP span includes the entire URI, including wallet queries.
/// Use a fixed quote label; other routes retain their existing URI diagnostics.
pub(super) fn http_span(request: &Request<Body>) -> tracing::Span {
    if super::quote_cache_control::is_local_quote_path(request.uri().path()) {
        tracing::debug_span!("http", method = %request.method(), route = "local_quote")
    } else {
        tracing::debug_span!("http", method = %request.method(), uri = %request.uri())
    }
}

pub(super) async fn correlate(request: Request, next: Next) -> Response {
    if !super::quote_cache_control::is_local_quote_path(request.uri().path()) {
        return next.run(request).await;
    }
    // Ignore client-provided IDs: no arbitrary or identifying log values and no
    // per-client registry. Random process seed distinguishes restarts and rolling
    // deployments; the counter consumes constant memory.
    let request_id = format!(
        "{:016x}",
        NEXT_REQUEST_ID
            .get_or_init(|| AtomicU64::new(rand::random()))
            .fetch_add(1, Ordering::Relaxed)
    );
    let span = tracing::info_span!("local_quote", request_id = %request_id);
    let mut response = next.run(request).instrument(span).await;
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id).expect("fixed hexadecimal request ID"),
    );
    response
}

pub(super) fn rejection(
    request: &super::route_solver::SolveRouteParams,
    error: (axum::http::StatusCode, String),
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let is_native = |asset: &str| matches!(asset.trim(), "uluna" | "uusd");
    let direction = match (is_native(&request.token_in), is_native(&request.token_out)) {
        (true, true) => "native_both",
        (true, false) => "native_input",
        (false, true) => "native_output",
        _ => "cw20",
    };
    tracing::info_span!("quote_outcome", direction)
        .in_scope(|| super::local_quote::public_error(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::to_bytes, http::StatusCode, routing::get, Router};
    use tower::ServiceExt;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn issue1505_correlation_rejects_client_ids_and_preserves_typed_error() {
        let app = Router::new()
            .route(
                "/api/v1/route/solve",
                get(|| async {
                    super::super::local_quote::public_error((
                        StatusCode::SERVICE_UNAVAILABLE,
                        r#"{"error":"stale_observations"}"#.into(),
                    ))
                }),
            )
            .route("/health", get(|| async { "ok" }))
            .layer(tower_http::trace::TraceLayer::new_for_http().make_span_with(http_span))
            .layer(axum::middleware::from_fn(correlate));
        let mut ids = std::collections::HashSet::new();
        for _ in 0..2 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/route/solve?trader=private-actor&amount_in=42")
                        .header(REQUEST_ID_HEADER, "client-private-value")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let id = response.headers()[REQUEST_ID_HEADER].to_str().unwrap();
            assert_eq!(id.len(), 16);
            assert!(id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
            assert!(ids.insert(id.to_owned()));
            let body = to_bytes(response.into_body(), 2048).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                serde_json::json!({"error":"stale_observations"})
            );
        }
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(!response.headers().contains_key(REQUEST_ID_HEADER));
        assert!(logs_contain("stale_observations"));
        assert!(!logs_contain("private-actor"));
        assert!(!logs_contain("client-private-value"));
        assert!(!logs_contain("amount_in=42"));
    }
}
