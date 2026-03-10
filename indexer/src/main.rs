mod api;
mod config;
mod db;
mod indexer;
mod lcd;

use config::Config;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = Config::from_env();
    tracing::info!("Starting CL8Y DEX indexer");
    tracing::info!("LCD endpoints: {:?}", config.lcd_urls);
    tracing::info!("Factory: {}", config.factory_address);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Database migrations applied");

    let lcd_client = lcd::LcdClient::new(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
    );

    let indexer_pool = pool.clone();
    let indexer_lcd = lcd_client.clone();
    let indexer_config = config.clone();
    let indexer_handle = tokio::spawn(async move {
        if let Err(e) =
            indexer::poller::run_indexer(indexer_pool, indexer_lcd, indexer_config).await
        {
            tracing::error!("Indexer exited with error: {}", e);
        }
    });

    let api_pool = pool.clone();
    let api_lcd = lcd_client.clone();
    let api_config = config.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::serve(api_pool, api_lcd, api_config).await {
            tracing::error!("API server exited with error: {}", e);
        }
    });

    tokio::select! {
        _ = indexer_handle => tracing::warn!("Indexer task ended"),
        _ = api_handle => tracing::warn!("API task ended"),
    }

    Ok(())
}

mod anyhow {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
