use std::env;
use std::net::SocketAddr;

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

/// Deployment environment for ops toggles (distinct from `RUN_MODE` LCD strictness).
/// `METRICS_BIND` may use `0.0.0.0` / `::` only in **non-production** deploy profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployEnv {
    Dev,
    Qa,
    Production,
}

impl DeployEnv {
    fn from_env(run_mode: RunMode) -> Self {
        match env::var("DEPLOY_ENV")
            .ok()
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("production" | "prod") => DeployEnv::Production,
            Some("qa" | "staging") => DeployEnv::Qa,
            Some("dev" | "development") => DeployEnv::Dev,
            Some(other) => {
                tracing::warn!(
                    "Unknown DEPLOY_ENV={other} (expected dev, qa, or production); using default"
                );
                Self::default_for_run_mode(run_mode)
            }
            None => Self::default_for_run_mode(run_mode),
        }
    }

    fn default_for_run_mode(run_mode: RunMode) -> Self {
        match run_mode {
            RunMode::Prod => DeployEnv::Production,
            RunMode::Dev => DeployEnv::Dev,
        }
    }

    fn allows_public_metrics_bind(self) -> bool {
        matches!(self, DeployEnv::Dev | DeployEnv::Qa)
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

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error(
        "RUN_MODE=prod requires LCD_URLS to list operator-controlled endpoints (not the built-in public defaults)"
    )]
    ProdRequiresCustomLcdUrls,
    #[error("RUN_MODE=prod requires {0} to be non-empty")]
    ProdEmpty(&'static str),
    #[error(
        "METRICS_BIND must not use 0.0.0.0 or :: in DEPLOY_ENV=production (use loopback or set DEPLOY_ENV=qa for scrapers on test setups; see docs/operator-secrets.md)"
    )]
    MetricsPublicBindNotAllowedInProduction,
    #[error("METRICS_BIND must be a valid host:port (or a legacy non-empty flag without ':' to bind 127.0.0.1:METRICS_PORT): {0}")]
    InvalidMetricsBind(String),
    #[error("{0}")]
    Missing(&'static str),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub run_mode: RunMode,
    pub deploy_env: DeployEnv,
    pub database_url: String,
    pub lcd_urls: Vec<String>,
    pub factory_address: String,
    pub fee_discount_address: Option<String>,
    pub poll_interval_ms: u64,
    pub api_port: u16,
    pub api_bind: String,
    pub lcd_timeout_ms: u64,
    pub lcd_cooldown_ms: u64,
    pub start_block: Option<i64>,
    pub cors_origins: Vec<String>,
    pub rate_limit_rps: u64,
    pub oracle_poll_interval_ms: u64,
    pub ustc_denom: Option<String>,
    /// Router contract for `SimulateSwapOperations` in route solver (optional).
    pub router_address: Option<String>,
    /// When set, Prometheus `GET /metrics` is served on this **dedicated** address (not the public API port).
    pub metrics_listen: Option<SocketAddr>,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let run_mode = RunMode::from_env();
        let deploy_env = DeployEnv::from_env(run_mode);

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

        let metrics_listen = parse_metrics_listen(deploy_env)?;

        Ok(Self {
            run_mode,
            deploy_env,
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
            api_bind: env::var("API_BIND").unwrap_or_else(|_| "127.0.0.1".to_string()),
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
            rate_limit_rps: env::var("RATE_LIMIT_RPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            oracle_poll_interval_ms: env::var("ORACLE_POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30000),
            ustc_denom: env::var("USTC_DENOM").ok(),
            router_address: env::var("ROUTER_ADDRESS").ok().filter(|s| !s.is_empty()),
            metrics_listen,
        })
    }
}

fn parse_metrics_listen(deploy_env: DeployEnv) -> Result<Option<SocketAddr>, ConfigError> {
    let raw = match env::var("METRICS_BIND") {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let port: u16 = env::var("METRICS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9095);

    let addr: SocketAddr = if trimmed.contains(':') {
        trimmed
            .parse()
            .map_err(|_| ConfigError::InvalidMetricsBind(trimmed.to_string()))?
    } else {
        // Legacy: any non-empty token without ':' → loopback on METRICS_PORT (secure default).
        SocketAddr::from(([127, 0, 0, 1], port))
    };

    if !deploy_env.allows_public_metrics_bind() && addr.ip().is_unspecified() {
        return Err(ConfigError::MetricsPublicBindNotAllowedInProduction);
    }

    Ok(Some(addr))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_config_env() {
        for key in [
            "RUN_MODE",
            "DEPLOY_ENV",
            "LCD_URLS",
            "DATABASE_URL",
            "FACTORY_ADDRESS",
            "CORS_ORIGINS",
            "METRICS_BIND",
            "METRICS_PORT",
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

    #[test]
    #[serial]
    fn production_rejects_ipv6_unspecified_metrics_bind() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("METRICS_BIND", "[::]:9095");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::MetricsPublicBindNotAllowedInProduction
        ));
    }

    #[test]
    #[serial]
    fn metrics_legacy_flag_binds_loopback() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("METRICS_BIND", "1");
        let c = Config::from_env().expect("config");
        assert_eq!(
            c.metrics_listen,
            Some(SocketAddr::from(([127, 0, 0, 1], 9095)))
        );
    }

    #[test]
    #[serial]
    fn production_rejects_inaddr_any_metrics_bind() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("METRICS_BIND", "0.0.0.0:9095");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(
            err,
            ConfigError::MetricsPublicBindNotAllowedInProduction
        ));
    }

    #[test]
    #[serial]
    fn production_accepts_loopback_metrics_bind() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("METRICS_BIND", "127.0.0.1:9095");
        let c = Config::from_env().expect("config");
        assert_eq!(
            c.metrics_listen,
            Some(SocketAddr::from(([127, 0, 0, 1], 9095)))
        );
    }

    #[test]
    #[serial]
    fn dev_allows_inaddr_any_metrics_bind() {
        clear_config_env();
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "http://localhost:5173");
        env::set_var("METRICS_BIND", "0.0.0.0:9095");
        let c = Config::from_env().expect("config");
        assert_eq!(c.deploy_env, DeployEnv::Dev);
        assert_eq!(
            c.metrics_listen,
            Some(SocketAddr::from(([0, 0, 0, 0], 9095)))
        );
    }

    #[test]
    #[serial]
    fn prod_run_mode_with_deploy_env_qa_allows_inaddr_any() {
        clear_config_env();
        env::set_var("RUN_MODE", "prod");
        env::set_var("DEPLOY_ENV", "qa");
        env::set_var("LCD_URLS", "https://lcd.example.com");
        env::set_var("DATABASE_URL", "postgres://localhost/db");
        env::set_var("FACTORY_ADDRESS", "terra1factory");
        env::set_var("CORS_ORIGINS", "https://app.example.com");
        env::set_var("METRICS_BIND", "0.0.0.0:9095");
        let c = Config::from_env().expect("config");
        assert_eq!(c.deploy_env, DeployEnv::Qa);
        assert!(c.metrics_listen.is_some());
    }
}
