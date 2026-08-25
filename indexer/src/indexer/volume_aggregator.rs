use std::time::Duration;

use sqlx::PgPool;

use crate::db::queries::{defillama, traders, volume};

/// Refresh token, pair, global, trader, protocol-fee, and DeFiLlama UTC-day windows.
///
/// `startup = true` logs failures at warn (same as historical pair/global poller init).
/// The 5-minute loop uses `startup = false` (error). GitLab #577 **D5**.
pub async fn refresh_all_volume_windows(pool: &PgPool, startup: bool) {
    refresh_all_volume_windows_with_pins(
        pool,
        startup,
        wrap_mapper_configured_from_env(),
        ust1_window_configured_from_env(),
    )
    .await;
}

fn wrap_mapper_configured_from_env() -> bool {
    std::env::var("WRAP_MAPPER_ADDRESS")
        .ok()
        .and_then(|s| crate::indexer::protocol_fees::parse_wrap_mapper_address(&s))
        .is_some()
}

fn ust1_window_configured_from_env() -> bool {
    std::env::var("UST1_WINDOW_ADDRESS")
        .ok()
        .and_then(|s| crate::indexer::protocol_fees::parse_ust1_window_address(&s))
        .is_some()
}

/// Same as [`refresh_all_volume_windows`] with an explicit wrap-mapper pin (tests).
/// UST1 window pin is read from env so existing #586 tests stay unconfigured.
pub async fn refresh_all_volume_windows_with_wrap(
    pool: &PgPool,
    startup: bool,
    wrap_mapper_configured: bool,
) {
    refresh_all_volume_windows_with_pins(
        pool,
        startup,
        wrap_mapper_configured,
        ust1_window_configured_from_env(),
    )
    .await;
}

/// Same as [`refresh_all_volume_windows_with_wrap`] with an explicit window pin (GitLab #614).
pub async fn refresh_all_volume_windows_with_pins(
    pool: &PgPool,
    startup: bool,
    wrap_mapper_configured: bool,
    ust1_window_configured: bool,
) {
    let fail = |label: &str, e: sqlx::Error| {
        if startup {
            tracing::warn!("Initial {label} refresh failed: {e}");
        } else {
            tracing::error!("Failed to refresh {label}: {e}");
        }
    };

    if let Err(e) = volume::refresh_token_volumes(pool).await {
        fail("token volumes", e);
    }
    if let Err(e) = volume::refresh_pair_volumes(pool).await {
        fail("pair 24h volumes", e);
    }
    if let Err(e) = volume::refresh_global_stats(pool).await {
        fail("global 24h stats", e);
    }
    if let Err(e) =
        volume::refresh_protocol_fee_stats(pool, wrap_mapper_configured, ust1_window_configured)
            .await
    {
        fail("protocol fee stats", e);
    }
    if let Err(e) = traders::refresh_rolling_volumes(pool).await {
        fail("rolling trader volumes", e);
    }
    if let Err(e) = defillama::refresh_defillama_daily(pool).await {
        fail("defillama utc-day stats", e);
    }
}

/// Background refresh for token volumes, pair 24h rollups, and trader rolling windows (~5 min).
pub async fn run_volume_refresh_loop(pool: PgPool) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;

        tracing::info!("Refreshing token volumes and rolling trader volumes...");
        refresh_all_volume_windows(&pool, false).await;
    }
}
