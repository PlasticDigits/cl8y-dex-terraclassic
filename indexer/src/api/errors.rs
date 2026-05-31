//! Sanitized HTTP error responses for upstream failures (GitLab #239 / H6).

use axum::http::StatusCode;

/// Client-visible message for upstream LCD failures (no URLs, endpoints, or `LcdError` text).
pub const LCD_UPSTREAM_GATEWAY_MSG: &str = "Upstream LCD query failed";

/// Log full LCD detail server-side; return generic **502** body to clients.
pub fn lcd_gateway_err(e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::warn!(detail = %e, "LCD upstream error");
    (
        StatusCode::BAD_GATEWAY,
        LCD_UPSTREAM_GATEWAY_MSG.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcd_gateway_err_never_echoes_upstream_detail() {
        let detail = "All LCD endpoints failed: https://lcd.example.internal/cosmwasm/wasm/v1/contract/terra1x/smart/abc";
        let (status, body) = lcd_gateway_err(detail);
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body, LCD_UPSTREAM_GATEWAY_MSG);
        assert!(!body.contains("https://"));
        assert!(!body.contains("All LCD endpoints failed"));
        assert!(!body.contains("cosmwasm"));
    }
}
