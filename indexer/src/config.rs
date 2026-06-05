use std::env;
use std::net::IpAddr;

use thiserror::Error;

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
}

#[derive(Debug, Clone)]
pub struct Config {
    pub run_mode: RunMode,
    pub database_url: String,
    pub lcd_urls: Vec<String>,
    pub factory_address: String,
    pub fee_discount_address: Option<String>,
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

        if run_mode == RunMode::Prod {
            if uses_builtin_lcd_defaults {
                return Err(ConfigError::ProdRequiresCustomLcdUrls);
            }
            if database_url.trim().is_empty() {
                return Err(ConfigError::ProdEmpty("DATABASE_URL"));
            }
            if factory_address.trim().is_empty() {
                return Err(ConfigError::ProdEmpty("FACTORY_ADDRESS"));
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

        Ok(Self {
            run_mode,
            database_url,
            lcd_urls,
            factory_address,
            fee_discount_address: env::var("FEE_DISCOUNT_ADDRESS").ok(),
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
                let mut rps = env::var("RATE_LIMIT_RPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                if run_mode == RunMode::Prod && rps == 0 {
                    rps = 60;
                }
                rps
            },
            rate_limit_lcd_heavy_rps: {
                let mut rps = env::var("RATE_LIMIT_LCD_HEAVY_RPS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);
                if run_mode == RunMode::Prod && rps == 0 {
                    rps = 10;
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
        })
    }

    /// Mirror staleness TTL derived from the configured snapshot cadence.
    pub fn book_snapshot_max_staleness_ms(&self) -> u64 {
        book_snapshot_max_staleness_ms(self.book_snapshot_interval_ms)
    }
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
}
