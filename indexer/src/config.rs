use std::collections::HashSet;
use std::env;
use std::net::IpAddr;
use std::str::FromStr;

use bigdecimal::BigDecimal;
use thiserror::Error;

/// Columbus-5 tokenlist hub CW20s (GitLab #556 / #570). Override with `HUB_*_ADDRESS`.
pub const DEFAULT_HUB_CUSTC_ADDRESS: &str =
    "terra1nap4dxh9tv35v0ynd9m4k6zt6c0dq6weszc4j5m564kjls56hu7qcr56ch";
pub const DEFAULT_HUB_CLUNC_ADDRESS: &str =
    "terra1437qslye72t7qmmahn4t5chz50r8a62g45phwkquwpyu2l62u6ksqssgdg";
pub const DEFAULT_HUB_UST1_ADDRESS: &str =
    "terra1f0eqgy9w7e5e7up97vjudqwx38tesf8ylx75x2lv3nwm0clry0pqmgfy72";
pub const DEFAULT_HUB_USTR_ADDRESS: &str =
    "terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv";
/// Columbus-5 CL8Y CW20 (tokenlist / `VITE_CL8Y_TOKEN_ADDRESS`). Identity is contract, not ticker.
pub const DEFAULT_HUB_CL8Y_ADDRESS: &str =
    "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3";
/// Columbus-5 Terra vFDUSD CW20 — never a fee-USD handle (CEX FDUSD / Venus are not this token).
pub const DEFAULT_VFDUSD_ADDRESS: &str =
    "terra1mnl9azefrqpmu888ar2u6zrcwr80hxlt3avf4300r576cw5ar7esvxsvj3";
pub const DEFAULT_HUB_USD_TVL_FLOOR: &str = "100";

/// Deployment profile. `RUN_MODE=prod` enforces explicit production configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Default: LCD URLs may fall back to public defaults.
    Dev,
    /// Requires explicit `LCD_URLS` and non-empty critical settings.
    Prod,
}

impl RunMode {
    fn from_env() -> Self {
        match env::var("RUN_MODE").as_deref() {
            Ok("prod") | Ok("production") => RunMode::Prod,
            _ => RunMode::Dev,
        }
    }
}

/// Built-in LCD list when `LCD_URLS` is unset (dev/local). Prod must use operator-controlled endpoints.
const DEFAULT_LCD_URLS: &str = "https://terra-classic-lcd.publicnode.com,\
             https://columbus-lcd.terra.dev,\
             https://lcd.terra-classic.hexxagon.io";

/// Default global API rate limit (requests per second per client IP).
pub const DEFAULT_RATE_LIMIT_RPS: u64 = 60;
/// Default LCD-heavy route rate limit (requests per second per client IP).
pub const DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS: u64 = 10;
/// Max JSON body size for `POST /api/v1/route/solve` (GitLab #379 / L-08).
pub const ROUTE_SOLVE_POST_BODY_LIMIT_BYTES: usize = 128 * 1024;

fn normalized_lcd_url_list(s: &str) -> String {
    s.split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

/// Staleness TTL multiplier for mirrored book/reserves (one missed snapshot cycle tolerated).
pub const BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES: u64 = 2;

/// Wall-clock staleness TTL for a given snapshot cadence (Phase 1c consumers).
pub const fn book_snapshot_max_staleness_ms(cadence_ms: u64) -> u64 {
    cadence_ms.saturating_mul(BOOK_SNAPSHOT_STALENESS_TOLERANCE_CYCLES)
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "RUN_MODE=prod requires LCD_URLS to list operator-controlled endpoints (not the built-in public defaults)"
    )]
    ProdRequiresCustomLcdUrls,
    #[error("RUN_MODE=prod requires {0} to be non-empty")]
    ProdEmpty(&'static str),
    #[error("{0}")]
    Missing(&'static str),
    #[error(
        "API_BIND is an IPv6 address but API_IPV6_ENABLED is not set — use an IPv4 bind (e.g. 0.0.0.0 or 127.0.0.1) or set API_IPV6_ENABLED=1"
    )]
    Ipv6BindDisabled,
    #[error(
        "FACTORY_ADDRESS must be non-empty in every RUN_MODE — an empty factory address disables \
         pair provenance verification and would index unverified (possibly spoofed) pairs"
    )]
    EmptyFactoryAddress,
    #[error(
        "RATE_LIMIT_RPS=0 and RATE_LIMIT_LCD_HEAVY_RPS=0 on a non-loopback API_BIND disables all \
         rate protection on a public listener — set a non-zero limit, bind to loopback, or set \
         ALLOW_ZERO_RATE_LIMITS=1 to override"
    )]
    ZeroRateLimitNonLoopbackBind,
}

/// True when `bind` is a loopback address (127.0.0.0/8, ::1) or the `localhost` hostname.
/// Anything else — including `0.0.0.0`, `::`, public IPs, and unrecognized hostnames — is
/// treated as non-loopback so the GitLab #458 dual-zero-rate-limit guard errs toward safety.
fn bind_is_loopback(bind: &str) -> bool {
    match bind.parse::<IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => bind.eq_ignore_ascii_case("localhost"),
    }
}

fn env_hub_addr(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

#[derive(Debug, Clone)]
pub struct Config {
    pub run_mode: RunMode,
    pub database_url: String,
    pub lcd_urls: Vec<String>,
    pub factory_address: String,
    pub fee_discount_address: Option<String>,
    /// Drift-correction full reconcile interval (seconds). Event-driven updates are primary (GitLab #364).
    pub tier_sync_reconcile_interval_secs: u64,
    pub poll_interval_ms: u64,
    pub api_port: u16,
    pub api_bind: String,
    /// When false (default), the API listener binds IPv4-only and rejects IPv6 `API_BIND` values.
    pub api_ipv6_enabled: bool,
    pub lcd_timeout_ms: u64,
    pub lcd_cooldown_ms: u64,
    pub start_block: Option<i64>,
    pub cors_origins: Vec<String>,
    pub rate_limit_rps: u64,
    /// Stricter per-IP limit for LCD-heavy routes (limit-book, route solve). Default **10** RPS.
    pub rate_limit_lcd_heavy_rps: u64,
    pub oracle_poll_interval_ms: u64,
    /// Background mirror snapshot cadence for pair reserves + resting books (GitLab #322).
    pub book_snapshot_interval_ms: u64,
    /// When true, global route solver prices hybrid grids from Postgres mirror (#319); LCD only for per-hop fallback + winning-route router sim.
    pub route_solver_db_hybrid: bool,
    /// Max relative drift (bps) between DB grid output and router `simulate_swap_operations` before fidelity downgrade (#319).
    pub route_fidelity_drift_bps: u32,
    pub ustc_denom: Option<String>,
    /// Router contract for `SimulateSwapOperations` in route solver (optional).
    pub router_address: Option<String>,
    /// LCD `search_txs` page size for block ingestion (GitLab #236).
    pub block_tx_page_limit: u32,
    /// Max pages per block — bounds memory / pagination DoS (default 50 × page_limit txs).
    pub block_tx_max_pages: u32,
    /// Retries before halting catch-up on a failing block (cursor not advanced).
    pub block_process_max_retries: u32,
    /// Base backoff ms multiplied by attempt number between block retries.
    pub block_process_retry_backoff_ms: u64,
    /// Optional webhook URL for reorg halt alerts (GitLab #362). POST JSON `indexer_reorg_halt` event.
    pub reorg_alert_webhook_url: Option<String>,
    /// Hub cUSTC CW20 (tokenlist default; GitLab #556). Identity is contract, not symbol.
    pub hub_custc_address: String,
    /// Hub cLUNC wrap CW20 (tokenlist default; GitLab #570). Identity is contract, not `uluna`.
    pub hub_clunc_address: String,
    /// Hub UST1 CW20 (tokenlist default; GitLab #556).
    pub hub_ust1_address: String,
    /// Hub USTR CW20 (tokenlist default; GitLab #556).
    pub hub_ustr_address: String,
    /// Official CL8Y CW20 pin (columbus-5 default; LocalTerra `HUB_CL8Y_ADDRESS` / TCL8Y).
    /// Fee USD keys this contract — never `symbol=CL8Y` / `CL8Y-cb` (GitLab #683).
    pub hub_cl8y_address: String,
    /// Ignore factory pairs below this USD TVL when picking hub sources (default $100).
    pub hub_usd_tvl_floor: BigDecimal,
    /// Comma-separated BSC JSON-RPC URLs for Venus `eth_call` (GitLab #571). Empty = do not poll.
    pub bsc_rpc_urls: Vec<String>,
    /// Venus vFDUSD poll cadence (default 30s). Bounded; API reads cache only.
    pub venus_vfdusd_poll_interval_ms: u64,
    /// Pinned wrap-mapper bech32 for wrap/unwrap treasury fees (GitLab #586).
    /// Invalid / empty → wrap sources omitted (not fake idle `$0`).
    pub wrap_mapper_address: Option<String>,
    /// Pinned ust1-window bech32 for mint/redeem treasury fees (GitLab #614).
    /// Invalid / empty → ust1_mint / ust1_redeem omitted (not fake idle `$0`).
    pub ust1_window_address: Option<String>,
    /// Community tax template code id (GitLab #594). Unset → catalog `configured: false`.
    pub community_tax_code_id: Option<u64>,
    /// Launcher contract — only this emitter may attest `launcher_tx`.
    pub community_token_launcher: Option<String>,
    /// Expected wasm admin (CMM) for `attested_cmm`.
    pub cmm_governance_addr: Option<String>,
    /// Option-2 community-tax code ids (router hops tax). GitLab #615.
    pub community_tax_option2_code_ids: HashSet<i64>,
    /// Known option-2 wasm data hashes (lowercase hex, no 0x). GitLab #615.
    pub community_tax_option2_data_hashes: HashSet<String>,
}

/// Catalog env snapshot shared by API + probe (GitLab #594).
#[derive(Debug, Clone, Default)]
pub struct CommunityTaxCatalogConfig {
    pub code_id: Option<u64>,
    pub launcher: Option<String>,
    pub cmm_governance: Option<String>,
    /// Post-migrate / new-crate code ids whose hops tax the original trader (#615).
    pub option2_code_ids: HashSet<i64>,
    /// Known option-2 `data_hash` values (lowercase hex). Unmigrated 11611 is never implied.
    pub option2_data_hashes: HashSet<String>,
}

impl CommunityTaxCatalogConfig {
    pub fn from_indexer_config(c: &Config) -> Self {
        Self {
            code_id: c.community_tax_code_id,
            launcher: c.community_token_launcher.clone(),
            cmm_governance: c.cmm_governance_addr.clone(),
            option2_code_ids: c.community_tax_option2_code_ids.clone(),
            option2_data_hashes: c.community_tax_option2_data_hashes.clone(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.code_id.is_some()
            && self.launcher.as_ref().is_some_and(|s| !s.is_empty())
            && self.cmm_governance.as_ref().is_some_and(|s| !s.is_empty())
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let run_mode = RunMode::from_env();

        let lcd_raw = env::var("LCD_URLS").unwrap_or_else(|_| DEFAULT_LCD_URLS.to_string());
        let lcd_urls: Vec<String> = lcd_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let uses_builtin_lcd_defaults =
            normalized_lcd_url_list(&lcd_raw) == normalized_lcd_url_list(DEFAULT_LCD_URLS);

        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;
        let factory_address =
            env::var("FACTORY_ADDRESS").map_err(|_| ConfigError::Missing("FACTORY_ADDRESS"))?;
        let cors_raw =
            env::var("CORS_ORIGINS").map_err(|_| ConfigError::Missing("CORS_ORIGINS"))?;
        let cors_origins: Vec<String> = cors_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // GitLab #451 (SEC-I02 H14): FACTORY_ADDRESS must be non-empty in *every* RUN_MODE.
        // An empty factory address makes `verify_factory_provenance` skip the provenance check
        // (pair_discovery.rs), so any contract emitting swap events — including attacker-controlled
        // clone pairs — would be indexed unverified. Prod already rejected empty below; enforce it
        // unconditionally so staging/QA deployments cannot silently index unverified pairs.
        if factory_address.trim().is_empty() {
            return Err(ConfigError::EmptyFactoryAddress);
        }

        if run_mode == RunMode::Prod {
            if uses_builtin_lcd_defaults {
                return Err(ConfigError::ProdRequiresCustomLcdUrls);
            }
            if database_url.trim().is_empty() {
                return Err(ConfigError::ProdEmpty("DATABASE_URL"));
            }
            if cors_origins.is_empty() {
                return Err(ConfigError::ProdEmpty("CORS_ORIGINS"));
            }
            if lcd_urls.is_empty() {
                return Err(ConfigError::ProdEmpty("LCD_URLS"));
            }
        }

        let api_ipv6_enabled = env::var("API_IPV6_ENABLED")
            .ok()
            .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"));
        let api_bind = env::var("API_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
        if !api_ipv6_enabled {
            if let Ok(ip) = api_bind.parse::<IpAddr>() {
                if ip.is_ipv6() {
                    return Err(ConfigError::Ipv6BindDisabled);
                }
            }
        }

        // GitLab #458 (SEC-I04 F01): refuse to start with all rate governors disabled on a
        // non-loopback bind. RUN_MODE=prod already clamps both limits to safe minimums (below),
        // so this guards the Dev/unset path, where a public deployment with RATE_LIMIT_RPS=0 and
        // RATE_LIMIT_LCD_HEAVY_RPS=0 would otherwise serve unthrottled traffic with only a warning.
        // Explicit opt-out: ALLOW_ZERO_RATE_LIMITS=1 (for deliberate offline/benchmark runs).
        if run_mode != RunMode::Prod {
            let rps_raw = env::var("RATE_LIMIT_RPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(DEFAULT_RATE_LIMIT_RPS);
            let lcd_heavy_raw = env::var("RATE_LIMIT_LCD_HEAVY_RPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS);
            let allow_zero = env::var("ALLOW_ZERO_RATE_LIMITS")
                .ok()
                .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"));
            if rps_raw == 0 && lcd_heavy_raw == 0 && !allow_zero && !bind_is_loopback(&api_bind) {
                return Err(ConfigError::ZeroRateLimitNonLoopbackBind);
            }
        }

        Ok(Self {
            run_mode,
            database_url,
            lcd_urls,
            factory_address,
            fee_discount_address: env::var("FEE_DISCOUNT_ADDRESS").ok(),
            tier_sync_reconcile_interval_secs: env::var("TIER_SYNC_RECONCILE_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(crate::indexer::trader_tracker::DEFAULT_TIER_RECONCILE_INTERVAL_SECS),
            poll_interval_ms: env::var("POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6000),
            api_port: env::var("API_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
            api_bind,
            api_ipv6_enabled,
            lcd_timeout_ms: env::var("LCD_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            lcd_cooldown_ms: env::var("LCD_COOLDOWN_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30000),
            start_block: env::var("START_BLOCK").ok().and_then(|v| v.parse().ok()),
            cors_origins,
            rate_limit_rps: {
                let rate_limit_rps_raw = env::var("RATE_LIMIT_RPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_RATE_LIMIT_RPS);
                let rate_limit_lcd_heavy_raw = env::var("RATE_LIMIT_LCD_HEAVY_RPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS);
                if rate_limit_rps_raw == 0 && rate_limit_lcd_heavy_raw == 0 {
                    tracing::warn!(
                        "RATE_LIMIT_RPS=0 and RATE_LIMIT_LCD_HEAVY_RPS=0: all API rate governors are disabled (DoS risk — GitLab #379)"
                    );
                }
                let mut rps = rate_limit_rps_raw;
                if run_mode == RunMode::Prod && rps == 0 {
                    tracing::warn!(
                        "RUN_MODE=prod: RATE_LIMIT_RPS=0 is not allowed; using {DEFAULT_RATE_LIMIT_RPS} RPS"
                    );
                    rps = DEFAULT_RATE_LIMIT_RPS;
                }
                rps
            },
            rate_limit_lcd_heavy_rps: {
                let mut rps = env::var("RATE_LIMIT_LCD_HEAVY_RPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS);
                if run_mode == RunMode::Prod && rps == 0 {
                    tracing::warn!(
                        "RUN_MODE=prod: RATE_LIMIT_LCD_HEAVY_RPS=0 is not allowed; using {DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS} RPS (GitLab #363)"
                    );
                    rps = DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS;
                }
                rps
            },
            oracle_poll_interval_ms: env::var("ORACLE_POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30000),
            book_snapshot_interval_ms: env::var("BOOK_SNAPSHOT_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
            route_solver_db_hybrid: env::var("ROUTE_SOLVER_DB_HYBRID")
                .ok()
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            route_fidelity_drift_bps: env::var("ROUTE_FIDELITY_DRIFT_BPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            ustc_denom: env::var("USTC_DENOM").ok(),
            router_address: env::var("ROUTER_ADDRESS").ok().filter(|s| !s.is_empty()),
            block_tx_page_limit: env::var("BLOCK_TX_PAGE_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            block_tx_max_pages: env::var("BLOCK_TX_MAX_PAGES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            block_process_max_retries: env::var("BLOCK_PROCESS_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            block_process_retry_backoff_ms: env::var("BLOCK_PROCESS_RETRY_BACKOFF_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000),
            reorg_alert_webhook_url: env::var("REORG_ALERT_WEBHOOK_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            hub_custc_address: env_hub_addr("HUB_CUSTC_ADDRESS", DEFAULT_HUB_CUSTC_ADDRESS),
            hub_clunc_address: env_hub_addr("HUB_CLUNC_ADDRESS", DEFAULT_HUB_CLUNC_ADDRESS),
            hub_ust1_address: env_hub_addr("HUB_UST1_ADDRESS", DEFAULT_HUB_UST1_ADDRESS),
            hub_ustr_address: env_hub_addr("HUB_USTR_ADDRESS", DEFAULT_HUB_USTR_ADDRESS),
            hub_cl8y_address: env_hub_addr("HUB_CL8Y_ADDRESS", DEFAULT_HUB_CL8Y_ADDRESS),
            hub_usd_tvl_floor: env::var("HUB_USD_TVL_FLOOR")
                .ok()
                .and_then(|v| BigDecimal::from_str(v.trim()).ok())
                .filter(|v| *v > BigDecimal::from(0))
                .unwrap_or_else(|| BigDecimal::from_str(DEFAULT_HUB_USD_TVL_FLOOR).unwrap()),
            bsc_rpc_urls: env::var("BSC_RPC_URLS")
                .ok()
                .map(|s| {
                    s.split(',')
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            venus_vfdusd_poll_interval_ms: env::var("VENUS_VFDUSD_POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30_000)
                .max(5_000),
            wrap_mapper_address: env::var("WRAP_MAPPER_ADDRESS")
                .ok()
                .and_then(|s| crate::indexer::protocol_fees::parse_wrap_mapper_address(&s)),
            ust1_window_address: env::var("UST1_WINDOW_ADDRESS")
                .ok()
                .and_then(|s| crate::indexer::protocol_fees::parse_ust1_window_address(&s)),
            community_tax_code_id: env::var("COMMUNITY_TAX_CODE_ID")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse().ok()),
            community_token_launcher: env::var("COMMUNITY_TOKEN_LAUNCHER")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            cmm_governance_addr: env::var("CMM_GOVERNANCE_ADDR")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            community_tax_option2_code_ids: parse_i64_set_env("COMMUNITY_TAX_OPTION2_CODE_IDS"),
            community_tax_option2_data_hashes: parse_hash_set_env(
                "COMMUNITY_TAX_OPTION2_DATA_HASHES",
            ),
        })
    }

    /// Mirror staleness TTL derived from the configured snapshot cadence.
    pub fn book_snapshot_max_staleness_ms(&self) -> u64 {
        book_snapshot_max_staleness_ms(self.book_snapshot_interval_ms)
    }
}

fn parse_i64_set_env(key: &str) -> HashSet<i64> {
    env::var(key)
        .ok()
        .map(|s| s.split(',').filter_map(|p| p.trim().parse().ok()).collect())
        .unwrap_or_default()
}

fn parse_hash_set_env(key: &str) -> HashSet<String> {
    env::var(key)
        .ok()
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().trim_start_matches("0x").to_ascii_lowercase())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_config_env() {
        for key in [
            "RUN_MODE",
            "LCD_URLS",
            "DATABASE_URL",
            "FACTORY_ADDRESS",
            "CORS_ORIGINS",
            "RATE_LIMIT_RPS",
            "RATE_LIMIT_LCD_HEAVY_RPS",
            "API_BIND",
            "API_IPV6_ENABLED",
            "ALLOW_ZERO_RATE_LIMITS",
        ] {
            env::remove_var(key);
        }
    }

    #[test]
    #[serial]
    fn dev_allows_default_lcd_urls() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        let c = Config::from_env().expect("dev config");
        assert_eq!(c.run_mode, RunMode::Dev);
        assert!(!c.lcd_urls.is_empty());
    }

    #[test]
    #[serial]
    fn prod_rejects_builtin_lcd_defaults() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", DEFAULT_LCD_URLS);
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, ConfigError::ProdRequiresCustomLcdUrls));
    }

    #[test]
    #[serial]
    fn prod_accepts_explicit_lcd_urls() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        let c = Config::from_env().expect("prod config");
        assert_eq!(c.run_mode, RunMode::Prod);
        assert_eq!(c.lcd_urls, vec!["https://lcd.example.com".to_string()]);
    }

    #[test]
    #[serial]
    fn dev_dual_zero_rate_limits_still_loads() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        let c = Config::from_env().expect("dev dual-zero");
        assert_eq!(c.rate_limit_rps, 0);
        assert_eq!(c.rate_limit_lcd_heavy_rps, 0);
    }

    #[test]
    #[serial]
    fn prod_forces_nonzero_rate_limits_when_zero() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        let c = Config::from_env().expect("prod config");
        assert_eq!(c.rate_limit_rps, 60);
        assert_eq!(c.rate_limit_lcd_heavy_rps, 10);
    }

    #[test]
    #[serial]
    fn ipv6_bind_rejected_when_ipv6_disabled() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("API_BIND", "::");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, ConfigError::Ipv6BindDisabled));
    }

    #[test]
    #[serial]
    fn ipv6_bind_allowed_when_ipv6_enabled() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("API_BIND", "::");
        env::set_var("API_IPV6_ENABLED", "1");
        let c = Config::from_env().expect("ipv6 bind with flag");
        assert!(c.api_ipv6_enabled);
        assert_eq!(c.api_bind, "::");
    }

    #[test]
    #[serial]
    fn dev_allows_both_rate_limits_zero() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        let c = Config::from_env().expect("dev config with zero limits");
        assert_eq!(c.rate_limit_rps, 0);
        assert_eq!(c.rate_limit_lcd_heavy_rps, 0);
    }

    #[test]
    #[serial]
    fn prod_rejects_empty_cors() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "  ,  ");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, ConfigError::ProdEmpty("CORS_ORIGINS")));
    }

    // GitLab #451 (SEC-I02 H14): empty FACTORY_ADDRESS rejected in every RUN_MODE.
    #[test]
    #[serial]
    fn empty_factory_address_rejected_in_dev() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "   ");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, ConfigError::EmptyFactoryAddress));
    }

    // GitLab #458 (SEC-I04 F01): dual-zero rate limits on a non-loopback bind are rejected.
    #[test]
    #[serial]
    fn dev_dual_zero_rate_limits_nonloopback_bind_rejected() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        env::set_var("API_BIND", "0.0.0.0");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(err, ConfigError::ZeroRateLimitNonLoopbackBind));
    }

    #[test]
    #[serial]
    fn dev_dual_zero_rate_limits_nonloopback_bind_allowed_with_optout() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        env::set_var("API_BIND", "0.0.0.0");
        env::set_var("ALLOW_ZERO_RATE_LIMITS", "1");
        let c = Config::from_env().expect("dual-zero allowed via opt-out");
        assert_eq!(c.rate_limit_rps, 0);
        assert_eq!(c.rate_limit_lcd_heavy_rps, 0);
    }

    #[test]
    #[serial]
    fn dev_dual_zero_rate_limits_loopback_bind_loads() {
        // Explicit loopback bind with dual-zero must still load (Playwright / local-dev path).
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        env::set_var("API_BIND", "127.0.0.1");
        let c = Config::from_env().expect("loopback dual-zero loads");
        assert_eq!(c.rate_limit_rps, 0);
    }

    #[test]
    #[serial]
    fn prod_dual_zero_nonloopback_bind_loads_via_clamp() {
        // In prod the dual-zero limits are clamped to safe minimums, so the #458 guard never
        // fires even on a public bind.
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("RATE_LIMIT_RPS", "0");
        env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
        env::set_var("API_BIND", "0.0.0.0");
        let c = Config::from_env().expect("prod clamps dual-zero");
        assert_eq!(c.rate_limit_rps, 60);
        assert_eq!(c.rate_limit_lcd_heavy_rps, 10);
    }
}
