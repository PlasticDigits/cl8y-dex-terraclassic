use std::time::Duration;

use sqlx::PgPool;

use crate::db::queries::{traders, volume};

/// Refresh token, pair, global, and trader rolling windows.
///
/// `startup = true` logs failures at warn (same as historical pair/global poller init).
/// The 5-minute loop uses `startup = false` (error). GitLab #577 **D5**.
pub async fn refresh_all_volume_windows(pool: &PgPool, startup: bool) {
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
    if let Err(e) = traders::refresh_rolling_volumes(pool).await {
        fail("rolling trader volumes", e);
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
