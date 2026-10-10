//! Initialized synthetic observations for request-local API regression fixtures.
//! Admission is test-only; production uses independently scheduled attested ingestion.
use cl8y_dex_indexer::{
    api::{build_router, db_orderbook_sim, local_quote, AppState, TickerMapCache},
    config::Config,
    indexer::{fee_discount_registry_health::FeeDiscountRegistryHealth, hub_usd::HubUsdConfig},
    lcd::LcdClient,
};
use sqlx::PgPool;

pub async fn app(pool: PgPool, config: Config) -> axum::Router {
    let lcd = LcdClient::new_with_policy(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
        config.expected_chain_id.clone(),
        config.lcd_provider_policy.clone(),
    );
    let namespace = lcd.chain_cache_namespace();
    let pair_rows = cl8y_dex_indexer::db::queries::pairs::get_all_pairs(&pool)
        .await
        .unwrap();
    let assets = cl8y_dex_indexer::db::queries::assets::get_all_assets(&pool)
        .await
        .unwrap();
    let id_to_addr = assets
        .iter()
        .filter_map(|a| {
            a.contract_address
                .as_ref()
                .or(a.denom.as_ref())
                .map(|addr| (a.id, addr.clone()))
        })
        .collect();
    let addresses: Vec<_> = pair_rows
        .iter()
        .map(|p| p.contract_address.clone())
        .collect();
    let mirrors =
        db_orderbook_sim::preload_mirrors_for_pairs(&pool, &addresses, &id_to_addr, 60_000)
            .await
            .unwrap();
    local_quote::initialize_for_test(namespace, pair_rows, id_to_addr, mirrors, 60_000);
    build_router(state(pool, &config, lcd), &config)
}

pub fn state(pool: PgPool, config: &Config, lcd: LcdClient) -> AppState {
    AppState {
        indexer_health: Default::default(),
        pool,
        lcd,
        oracle_prices: cl8y_dex_indexer::indexer::oracle::OraclePriceHandles::new(),
        venus_vfdusd: cl8y_dex_indexer::indexer::venus_vfdusd::new_shared_venus(),
        ticker_map_cache: TickerMapCache::default(),
        orderbook_cache: Default::default(),
        router_address: config.router_address.clone(),
        factory_address: Some(config.factory_address.clone()),
        wrap_mapper_address: config.wrap_mapper_address.clone(),
        fee_discount_address: config.fee_discount_address.clone(),
        fee_discount_registry_health: FeeDiscountRegistryHealth::from_config(
            config.fee_discount_address.as_deref(),
        ),
        route_solver_db_hybrid: config.route_solver_db_hybrid,
        book_snapshot_max_staleness_ms: config.book_snapshot_max_staleness_ms(),
        route_fidelity_drift_bps: config.route_fidelity_drift_bps,
        hub_usd: HubUsdConfig::from_indexer_config(&config),
        community_tax: cl8y_dex_indexer::config::CommunityTaxCatalogConfig::from_indexer_config(
            &config,
        ),
    }
}
