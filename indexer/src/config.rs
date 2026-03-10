use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub lcd_urls: Vec<String>,
    pub factory_address: String,
    pub fee_discount_address: Option<String>,
    pub poll_interval_ms: u64,
    pub api_port: u16,
    pub lcd_timeout_ms: u64,
    pub lcd_cooldown_ms: u64,
    pub start_block: Option<i64>,
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

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/dex_indexer".into()),
            lcd_urls,
            factory_address: env::var("FACTORY_ADDRESS").unwrap_or_default(),
            fee_discount_address: env::var("FEE_DISCOUNT_ADDRESS").ok(),
            poll_interval_ms: env::var("POLL_INTERVAL_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6000),
            api_port: env::var("API_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3001),
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
        }
    }
}
