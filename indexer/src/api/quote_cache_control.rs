//! Dynamic local quote responses carry original observation expiry and context.
//! Browser/proxy HTTP caches must not retain successes or blockers across changes.
//! The independent in-process quote caches and producer budgets remain unchanged.
use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

pub(super) fn is_local_quote_path(path: &str) -> bool {
    matches!(
        path,
        "/api/v1/route/solve" | "/api/v1/route/solve/best" | "/api/v1/route/solve/progress"
    )
}

/// Applied outside all response-producing layers, including rate limits and
/// timeouts. Receive/reference queries and declared POSTs share these routes.
pub(super) async fn no_store(request: Request, next: Next) -> Response {
    let dynamic_quote = is_local_quote_path(request.uri().path());
    let mut response = next.run(request).await;
    if dynamic_quote {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_http::timeout::TimeoutLayer;

    async fn receipt() -> Response {
        (
            [
                (header::CACHE_CONTROL, "public, max-age=60"),
                (header::HeaderName::from_static("x-receipt"), "original"),
            ],
            "unchanged",
        )
            .into_response()
    }

    #[tokio::test]
    async fn issue1479_quote_http_boundary_preserves_other_headers_body_and_timeout_status() {
        let app = Router::new()
            .route("/api/v1/route/solve", get(receipt).post(receipt))
            .route(
                "/api/v1/route/solve/best",
                get(|| async {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    "late"
                }),
            )
            .route("/health", get(receipt))
            .route("/api/v1/route/solve-history", get(receipt))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_millis(1),
            ))
            .layer(axum::middleware::from_fn(no_store));
        for (method, path, dynamic) in [
            ("GET", "/api/v1/route/solve?amount_in=1", true),
            ("POST", "/api/v1/route/solve", true),
            ("GET", "/health", false),
            ("GET", "/api/v1/route/solve-history", false),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["x-receipt"], "original");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                if dynamic {
                    "no-store"
                } else {
                    "public, max-age=60"
                }
            );
            assert_eq!(
                to_bytes(response.into_body(), 100).await.unwrap().as_ref(),
                b"unchanged"
            );
        }
        let timed = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/route/solve/best")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(timed.status(), StatusCode::REQUEST_TIMEOUT);
        assert_eq!(timed.headers()[header::CACHE_CONTROL], "no-store");
    }
}
