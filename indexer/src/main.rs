mod api;
mod config;
mod database_security;
mod db;
mod hybrid_limits;
mod indexer;
mod lcd;
mod listing_id_guard;
mod qa_guard;
mod startup;

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

    let args: Vec<String> = std::env::args().skip(1).collect();
    // Argument validation must happen before loading credentials or opening a connection.
    match args.first().map(String::as_str) {
        Some("seed-qa") => run_seed_qa(&args[1..]).await,
        Some("backfill-defillama") => run_backfill_defillama(&args[1..]).await,
        Some("migrate") if args.len() == 1 => run_migrations().await,
        Some("rebuild-positions") if args.len() == 1 => run_rebuild_positions().await,
        Some("backfill-gt-event-reserves") if args.len() == 1 => {
            run_backfill_gt_event_reserves().await
        }
        Some("--help" | "-h") if args.len() == 1 => {
            eprintln!("Usage: cl8y-dex-indexer [migrate|seed-qa|rebuild-positions|backfill-gt-event-reserves|backfill-defillama]\nServer mode requires pre-applied migrations and least-privilege DATABASE_URL.");
            Ok(())
        }
        None => run_server().await,
        _ => anyhow::bail!("unknown command or unexpected arguments; use --help"),
    }
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

async fn run_backfill_defillama(args: &[String]) -> anyhow::Result<()> {
    if args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h") {
        eprintln!("Usage: cl8y-dex-indexer backfill-defillama --start YYYY-MM-DD --end YYYY-MM-DD\nReaggregate 1-400 completed UTC days from existing event stamps. Requires a prepared schema and least-privilege DATABASE_URL; never migrates, reprices or reloads Llama.");
        return Ok(());
    }
    anyhow::ensure!(
        args.len() == 4 && args[0] == "--start" && args[2] == "--end",
        "use backfill-defillama --start YYYY-MM-DD --end YYYY-MM-DD"
    );
    let range = indexer::defillama_history::HistoryRange::parse(
        &args[1],
        &args[3],
        chrono::Utc::now().date_naive(),
    )?;
    // Parse and bound the complete request before any credentials or connections.
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required for backfill-defillama"))?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    database_security::verify_runtime(&pool).await?;
    let days = indexer::defillama_history::repair_history(&pool, range).await?;
    tracing::info!(
        days,
        "DeFiLlama history repair complete; upstream reload is separate"
    );
    Ok(())
}

enum SeedArguments {
    Help,
    Clean,
    Check,
    Seed(SeedQaConfig),
}

fn parse_seed_arguments(args: &[String]) -> anyhow::Result<SeedArguments> {
    let mut config = SeedQaConfig::default();
    let mut clean = false;
    let mut check = false;
    let mut help = false;
    let mut has_seed_options = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--clean" => clean = true,
            "--check" => check = true,
            "--help" | "-h" => help = true,
            "--weeks" | "--swaps-per-day" => {
                has_seed_options = true;
                let option = args[i].as_str();
                i += 1;
                let number: u32 = args
                    .get(i)
                    .and_then(|value| value.parse().ok())
                    .ok_or_else(|| anyhow::anyhow!("{option} requires a positive integer"))?;
                let max = if option == "--weeks" { 520 } else { 86400 };
                anyhow::ensure!(
                    number > 0 && number <= max,
                    "{option} must be between 1 and {max}"
                );
                if option == "--weeks" {
                    config.span_weeks = number;
                } else {
                    config.swaps_per_day = number;
                }
            }
            _ => anyhow::bail!("unknown seed-qa argument; use seed-qa --help"),
        }
        i += 1;
    }
    anyhow::ensure!(
        !((clean || check) && has_seed_options) && !(clean && check),
        "--clean and --check cannot be combined with seed options or each other"
    );
    if help {
        Ok(SeedArguments::Help)
    } else if clean {
        Ok(SeedArguments::Clean)
    } else if check {
        Ok(SeedArguments::Check)
    } else {
        Ok(SeedArguments::Seed(config))
    }
}

async fn run_seed_qa(args: &[String]) -> anyhow::Result<()> {
    let operation = parse_seed_arguments(args)?;
    if matches!(operation, SeedArguments::Help) {
        print_seed_qa_help();
        return Ok(());
    }
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required for seed-qa"))?;
    let run_mode = std::env::var("RUN_MODE").ok();
    let qa_database_name = std::env::var("QA_DATABASE_NAME").ok();
    qa_guard::validate_seed_target(
        run_mode.as_deref(),
        &database_url,
        qa_database_name.as_deref(),
    )?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    database_security::verify_qa(&pool, qa_database_name.as_deref().unwrap()).await?;
    match operation {
        SeedArguments::Clean => seed_qa::clean(&pool)
            .await
            .map_err(anyhow::Error::from_boxed)?,
        SeedArguments::Seed(config) => seed_qa::run(&pool, config)
            .await
            .map_err(anyhow::Error::from_boxed)?,
        SeedArguments::Check => (),
        SeedArguments::Help => unreachable!(),
    }
    Ok(())
}

async fn run_migrations() -> anyhow::Result<()> {
    // Deliberately independent of full server configuration and chain connectivity.
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is required for migrate"))?;
    let runtime_role =
        std::env::var("INDEXER_RUNTIME_ROLE").unwrap_or_else(|_| "dex_indexer_runtime".to_owned());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    database_security::migrate_and_grant(&pool, &runtime_role).await?;
    tracing::info!("Database migrations applied and runtime grants reconciled");
    Ok(())
}

async fn run_rebuild_positions() -> anyhow::Result<()> {
    let config = load_config_or_exit();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    database_security::verify_runtime(&pool).await?;
    let n = indexer::position_tracker::rebuild_all_positions_from_swaps(&pool)
        .await
        .map_err(anyhow::Error::from_boxed)?;
    tracing::info!(
        swaps_replayed = n,
        "rebuilt trader_positions from swap_events"
    );
    Ok(())
}

async fn run_backfill_gt_event_reserves() -> anyhow::Result<()> {
    let config = load_config_or_exit();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;
    database_security::verify_runtime(&pool).await?;
    let stats = indexer::gt_event_reserves::backfill_all(&pool).await?;
    tracing::info!(
        pairs = stats.pairs,
        swaps_filled = stats.swaps_filled,
        liqs_filled = stats.liqs_filled,
        skipped_ambiguous = stats.skipped_ambiguous,
        "backfilled /gt/events post-event reserves (GitLab #684)"
    );
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
  --check               Validate isolated QA identity and schema without writes

Requires explicit RUN_MODE=dev/qa/test/local, dedicated dex_indexer_qa role,
QA_DATABASE_NAME matching the isolated local database, and a QA identity marker.
Prepare its schema with the standalone migrate command before seeding.
  -h, --help            Show this help"
    );
}

async fn run_server() -> anyhow::Result<()> {
    let config = load_config_or_exit();
    startup::log_startup_config(&config);

    let lcd_client = lcd::LcdClient::new_with_policy(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
        config.expected_chain_id.clone(),
        config.lcd_provider_policy.clone(),
    );
    let indexer_health = indexer::lifecycle::IndexerHealth::default();
    let attestation = tokio::select! {
        biased;
        _ = shutdown_signal() => return Ok(()),
        result = lcd_client.attest_endpoints() => result,
    };
    if let Err(error) = attestation {
        if indexer::block_indexer::is_retryable_availability(&error) {
            indexer_health.unavailable(None);
            tracing::warn!(
                "Startup provider unavailable; serving unready until scheduled ingestion recovers"
            );
        } else {
            return Err(error.into());
        }
    }

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    database_security::verify_runtime(&pool).await?;
    tracing::info!("Database runtime privileges and schema readiness verified");

    match crate::db::queries::usdt_quote_usd::backfill_null_usdt_quote_usd(
        &pool,
        &config.usdt_cw20_address,
    )
    .await
    {
        Ok(stats) => {
            if stats.price_filled > 0 || stats.volume_filled > 0 {
                tracing::info!(
                    price_filled = stats.price_filled,
                    volume_filled = stats.volume_filled,
                    pairs = stats.pairs,
                    "USDT quote USD NULL-only backfill complete"
                );
            }
        }
        Err(e) => tracing::warn!("USDT quote USD NULL-only backfill failed: {}", e),
    }

    if let Err(e) = indexer::candle_gap_fill::run_once(
        &pool,
        config.ustc_denom.as_deref(),
        config.usdt_cw20_address.as_str(),
    )
    .await
    {
        tracing::warn!("candle gap fill failed: {}", e);
    }

    let cancel = CancellationToken::new();
    let oracle_prices = indexer::oracle::OraclePriceHandles::new();
    let venus_vfdusd = indexer::venus_vfdusd::new_shared_venus();
    let fee_discount_registry_health =
        indexer::fee_discount_registry_health::FeeDiscountRegistryHealth::from_config(
            config.fee_discount_address.as_deref(),
        );

    let poller_health = indexer_health.clone();

    let indexer_pool = pool.clone();
    let indexer_lcd = lcd_client.clone();
    let indexer_config = config.clone();
    let indexer_cancel = cancel.clone();
    let indexer_oracle = oracle_prices.clone();
    let indexer_venus = venus_vfdusd.clone();
    let indexer_fee_discount_health = fee_discount_registry_health.clone();
    let indexer_handle = tokio::spawn(async move {
        if let Err(e) = indexer::poller::run_indexer_with_health(
            indexer_pool,
            indexer_lcd,
            indexer_config,
            indexer_cancel,
            indexer_oracle,
            indexer_venus,
            indexer_fee_discount_health,
            poller_health,
        )
        .await
        {
            tracing::error!("Indexer exited with error: {}", e);
        }
    });

    let api_pool = pool.clone();
    let api_lcd = lcd_client.clone();
    let api_config = config.clone();
    let api_oracle = oracle_prices.clone();
    let api_venus = venus_vfdusd.clone();
    let api_fee_discount_health = fee_discount_registry_health.clone();
    let api_indexer_health = indexer_health.clone();
    let api_cancel = cancel.clone();
    let api_handle = tokio::spawn(async move {
        if let Err(e) = api::serve_with_health(
            api_pool,
            api_lcd,
            api_config,
            api_oracle,
            api_venus,
            api_fee_discount_health,
            api_indexer_health,
            api_cancel,
        )
        .await
        {
            tracing::error!("API server exited with error: {}", e);
        }
    });

    indexer::lifecycle::supervise_tasks(
        indexer_handle,
        api_handle,
        indexer_health,
        cancel,
        shutdown_signal(),
    )
    .await;

    tracing::info!("Shutdown complete");
    Ok(())
}

/// Containers normally send SIGTERM; local terminals send Ctrl-C.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
