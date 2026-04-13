mod api;
mod config;
mod db;
mod indexer;
mod lcd;
mod metrics;

use config::Config;
use indexer::seed_qa::{self, SeedQaConfig};
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "seed-qa" {
        return run_seed_qa(&args[2..]).await;
    }

    run_server().await
}

fn load_config_or_exit() -> Config {
    match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_seed_qa(args: &[String]) -> anyhow::Result<()> {
    let config = load_config_or_exit();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    if args.iter().any(|a| a == "--clean") {
        seed_qa::clean(&pool).await?;
        return Ok(());
    }

    let mut seed_config = SeedQaConfig::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--weeks" => {
                i += 1;
                seed_config.span_weeks = args
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .expect("--weeks requires a number");
            }
            "--swaps-per-day" => {
                i += 1;
                seed_config.swaps_per_day = args
                    .get(i)
                    .and_then(|v| v.parse().ok())
                    .expect("--swaps-per-day requires a number");
            }
            "--help" | "-h" => {
                print_seed_qa_help();
                return Ok(());
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                print_seed_qa_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    seed_qa::run(&pool, seed_config).await?;
    Ok(())
}

fn print_seed_qa_help() {
    eprintln!(
        "\
Usage: cl8y-dex-indexer seed-qa [OPTIONS]

Insert synthetic swap history with spread-out timestamps so QA can
test 1h, 4h, 1d, and 1w candle intervals on a fresh local chain.

Options:
  --weeks <N>           Time span to cover (default: 4)
  --swaps-per-day <N>   Swaps per pair per day (default: 24)
  --clean               Remove all seeded data and rebuild candles
  -h, --help            Show this help"
    );
}

async fn run_server() -> anyhow::Result<()> {
    let config = load_config_or_exit();
    tracing::info!("Starting CL8Y DEX indexer (RUN_MODE={:?})", config.run_mode);
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

    let cancel = CancellationToken::new();
    let ustc_price = indexer::oracle::new_shared_price();

    let indexer_pool = pool.clone();
    let indexer_lcd = lcd_client.clone();
    let indexer_config = config.clone();
    let indexer_cancel = cancel.clone();
    let indexer_ustc = ustc_price.clone();
    let indexer_handle = tokio::spawn(async move {
        if let Err(e) = indexer::poller::run_indexer(
            indexer_pool,
            indexer_lcd,
            indexer_config,
            indexer_cancel,
            indexer_ustc,
        )
        .await
        {
            tracing::error!("Indexer exited with error: {}", e);
        }
    });

    let api_pool = pool.clone();
    let api_lcd = lcd_client.clone();
    let api_config = config.clone();
    let api_ustc = ustc_price.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::serve(api_pool, api_lcd, api_config, api_ustc).await {
            tracing::error!("API server exited with error: {}", e);
        }
    });

    tokio::select! {
        _ = indexer_handle => tracing::warn!("Indexer task ended"),
        _ = api_handle => tracing::warn!("API task ended"),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received, stopping indexer...");
            cancel.cancel();
        }
    }

    tracing::info!("Shutdown complete");
    Ok(())
}

mod anyhow {
    pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
}
