//! Startup logging for the indexer binary (GitLab #433 / SEC-F13).
//!
//! Safe fields only — never log `database_url`, webhook URLs, or other secrets.
//! Enforced by `make lint-indexer-log-secrets` and unit tests in this module.

use crate::config::Config;

/// Emit non-secret startup configuration at INFO level.
pub fn log_startup_config(config: &Config) {
    tracing::info!("Starting CL8Y DEX indexer (RUN_MODE={:?})", config.run_mode);
    tracing::info!("LCD endpoints: {:?}", config.lcd_urls);
    tracing::info!("Factory: {}", config.factory_address);
    tracing::info!(
        "Rate limits: global={} RPS, LCD-heavy={} RPS",
        config.rate_limit_rps,
        config.rate_limit_lcd_heavy_rps
    );
    tracing::info!(
        "Venus vFDUSD poller: {}",
        if config.bsc_rpc_urls.is_empty() {
            "disabled (BSC_RPC_URLS unset)"
        } else {
            "enabled"
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RunMode;
    use tracing_test::traced_test;

    fn test_config_with_password(password: &str) -> Config {
        Config {
            run_mode: RunMode::Dev,
            database_url: format!(
                "postgres://cl8y_user:{password}@127.0.0.1:5432/dex_indexer_test"
            ),
            lcd_urls: vec!["http://localhost:9999".to_string()],
            factory_address: "terra1factory".to_string(),
            fee_discount_address: None,
            tier_sync_reconcile_interval_secs:
                crate::indexer::trader_tracker::DEFAULT_TIER_RECONCILE_INTERVAL_SECS,
            poll_interval_ms: 6000,
            api_port: 3001,
            api_bind: "127.0.0.1".to_string(),
            api_ipv6_enabled: false,
            lcd_timeout_ms: 5000,
            lcd_cooldown_ms: 30000,
            start_block: None,
            cors_origins: vec!["http://localhost:5173".to_string()],
            rate_limit_rps: 60,
            rate_limit_lcd_heavy_rps: 10,
            oracle_poll_interval_ms: 30000,
            book_snapshot_interval_ms: 10_000,
            route_solver_db_hybrid: false,
            route_fidelity_drift_bps: 100,
            ustc_denom: None,
            router_address: None,
            block_tx_page_limit: 100,
            block_tx_max_pages: 50,
            block_process_max_retries: 5,
            block_process_retry_backoff_ms: 2000,
            reorg_alert_webhook_url: Some(format!("https://hooks.example.com/secret/{password}")),
            hub_custc_address: crate::config::DEFAULT_HUB_CUSTC_ADDRESS.to_string(),
            hub_clunc_address: crate::config::DEFAULT_HUB_CLUNC_ADDRESS.to_string(),
            hub_ust1_address: crate::config::DEFAULT_HUB_UST1_ADDRESS.to_string(),
            hub_ustr_address: crate::config::DEFAULT_HUB_USTR_ADDRESS.to_string(),
            hub_usd_tvl_floor: "100".parse().unwrap(),
            bsc_rpc_urls: vec![],
            venus_vfdusd_poll_interval_ms: 30_000,
            wrap_mapper_address: None,
            community_tax_code_id: None,
            community_token_launcher: None,
            cmm_governance_addr: None,
            community_tax_option2_code_ids: std::collections::HashSet::new(),
            community_tax_option2_data_hashes: std::collections::HashSet::new(),
        }
    }

    /// GitLab #433 (SEC-F13): startup INFO logs must not echo DATABASE_URL credentials.
    #[traced_test]
    #[test]
    fn startup_logs_do_not_contain_database_password() {
        const PASSWORD: &str = "SEC_F13_TEST_PASSWORD_433";

        let config = test_config_with_password(PASSWORD);
        log_startup_config(&config);

        assert!(
            !logs_contain(PASSWORD),
            "startup logs must not contain DATABASE_URL password (SEC-F13)"
        );
    }

    /// GitLab #433 (SEC-F13): webhook URLs are operator secrets — not logged at startup.
    #[traced_test]
    #[test]
    fn startup_logs_do_not_contain_reorg_webhook_url() {
        const PASSWORD: &str = "SEC_F13_WEBHOOK_TOKEN_433";

        let config = test_config_with_password(PASSWORD);
        log_startup_config(&config);

        assert!(
            !logs_contain("hooks.example.com"),
            "startup logs must not contain REORG_ALERT_WEBHOOK_URL (SEC-F13)"
        );
    }
}
