use std::time::Duration;

use sqlx::PgPool;

use crate::db::queries::{traders, volume};

/// Background refresh for token volumes, pair 24h rollups, and trader rolling windows (~5 min).
pub async fn run_volume_refresh_loop(pool: PgPool) {
    loop {
        tokio::time::sleep(Duration::from_secs(300)).await;

        tracing::info!("Refreshing token volumes and rolling trader volumes...");

        if let Err(e) = volume::refresh_token_volumes(&pool).await {
            tracing::error!("Failed to refresh token volumes: {}", e);
        }

        if let Err(e) = volume::refresh_pair_volumes(&pool).await {
            tracing::error!("Failed to refresh pair 24h volumes: {}", e);
        }

        if let Err(e) = traders::refresh_rolling_volumes(&pool).await {
            tracing::error!("Failed to refresh rolling trader volumes: {}", e);
        }
    }
}
