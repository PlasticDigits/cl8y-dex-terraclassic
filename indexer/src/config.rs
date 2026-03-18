use std::env;

#[derive(Debug, Clone)]
pub struct Config {
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
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let lcd_raw = env::var("LCD_URLS").unwrap_or_else(|_| {
            "https://terra-classic-lcd.publicnode.com,\
             https://columbus-lcd.terra.dev,\
             https://lcd.terra-classic.hexxagon.io"
                .to_string()
        });
        let lcd_urls: Vec<String> = lcd_raw.split(',').map(|s| s.trim().to_string()).collect();

        let cors_raw = env::var("CORS_ORIGINS").expect("CORS_ORIGINS must be set");
        let cors_origins: Vec<String> =
            cors_raw.split(',').map(|s| s.trim().to_string()).collect();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            lcd_urls,
            factory_address: env::var("FACTORY_ADDRESS").expect("FACTORY_ADDRESS must be set"),
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
            start_block: env::var("START_BLOCK")
                .ok()
                .and_then(|v| v.parse().ok()),
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
        }
    }
}
