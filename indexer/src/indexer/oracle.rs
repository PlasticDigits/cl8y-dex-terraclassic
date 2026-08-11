//! External CEX/USD reference oracle (GitLab #515).
//!
//! Polls KuCoin / MEXC / CoinGecko for **USTC/USD** and **LUNC/USD**.
//! These feeds are advisory display/reference prices — not on-chain settlement.

use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::RwLock;

use crate::db::queries::oracle as db_oracle;

pub type SharedPrice = Arc<RwLock<Option<BigDecimal>>>;

pub fn new_shared_price() -> SharedPrice {
    Arc::new(RwLock::new(None))
}

/// In-memory handles for each supported external ticker.
#[derive(Clone)]
pub struct OraclePriceHandles {
    pub ustc: SharedPrice,
    pub lunc: SharedPrice,
}

impl OraclePriceHandles {
    pub fn new() -> Self {
        Self {
            ustc: new_shared_price(),
            lunc: new_shared_price(),
        }
    }

    pub fn for_ticker(&self, ticker: OracleTicker) -> &SharedPrice {
        match ticker {
            OracleTicker::Ustc => &self.ustc,
            OracleTicker::Lunc => &self.lunc,
        }
    }
}

impl Default for OraclePriceHandles {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported external USD reference tickers exposed by the API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OracleTicker {
    Ustc,
    Lunc,
}

impl OracleTicker {
    pub const ALL: [OracleTicker; 2] = [OracleTicker::Ustc, OracleTicker::Lunc];

    pub fn as_str(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "ustc",
            OracleTicker::Lunc => "lunc",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ustc" => Some(OracleTicker::Ustc),
            "lunc" => Some(OracleTicker::Lunc),
            _ => None,
        }
    }

    fn kucoin_symbol(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTC-USDT",
            OracleTicker::Lunc => "LUNC-USDT",
        }
    }

    fn mexc_symbol(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTCUSDT",
            OracleTicker::Lunc => "LUNCUSDT",
        }
    }

    fn coingecko_id(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "terrausd",
            OracleTicker::Lunc => "terra-luna",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            OracleTicker::Ustc => "USTC/USD",
            OracleTicker::Lunc => "LUNC/USD",
        }
    }
}

pub async fn run_oracle_loop(pool: PgPool, poll_interval_ms: u64, prices: OraclePriceHandles) {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("failed to build oracle HTTP client");

    for ticker in OracleTicker::ALL {
        if let Ok(Some(price)) = db_oracle::get_latest_average_price(&pool, ticker).await {
            tracing::info!(
                "Oracle: loaded cached {} price from DB: {}",
                ticker.display_name(),
                price
            );
            *prices.for_ticker(ticker).write().await = Some(price);
        }
    }

    let interval = Duration::from_millis(poll_interval_ms);
    let mut tick_count: u64 = 0;

    loop {
        tick_count += 1;
        let fetch_coingecko = tick_count % 2 == 0;

        for ticker in OracleTicker::ALL {
            poll_ticker(&client, &pool, &prices, ticker, fetch_coingecko).await;
        }

        tokio::time::sleep(interval).await;
    }
}

async fn poll_ticker(
    client: &Client,
    pool: &PgPool,
    prices: &OraclePriceHandles,
    ticker: OracleTicker,
    fetch_coingecko: bool,
) {
    let results = fetch_all_sources(client, ticker, fetch_coingecko).await;

    let mut ok_prices: Vec<f64> = Vec::new();
    for (source, result) in &results {
        match result {
            Ok(price) => {
                tracing::debug!(
                    "Oracle: {} {} returned ${:.8}",
                    ticker.as_str(),
                    source,
                    price
                );
                ok_prices.push(*price);
                if let Err(e) =
                    db_oracle::insert_price(pool, ticker, &f64_to_bd(*price), source).await
                {
                    tracing::error!(
                        "Oracle: failed to store {} {} price: {}",
                        ticker.as_str(),
                        source,
                        e
                    );
                }
            }
            Err(e) => match e {
                OracleError::RateLimited => {
                    tracing::debug!("Oracle: {} {} rate limited", ticker.as_str(), source);
                }
                _ => {
                    tracing::warn!("Oracle: {} {} failed: {}", ticker.as_str(), source, e);
                }
            },
        }
    }

    if !ok_prices.is_empty() {
        let avg = ok_prices.iter().sum::<f64>() / ok_prices.len() as f64;
        let avg_bd = f64_to_bd(avg);

        tracing::info!(
            "Oracle: {} avg ${:.8} from {}/{} sources",
            ticker.display_name(),
            avg,
            ok_prices.len(),
            results.len()
        );

        if let Err(e) = db_oracle::insert_price(pool, ticker, &avg_bd, "average").await {
            tracing::error!(
                "Oracle: failed to store {} average price: {}",
                ticker.as_str(),
                e
            );
        }

        *prices.for_ticker(ticker).write().await = Some(avg_bd);
    } else {
        tracing::warn!(
            "Oracle: all sources failed for {}, retaining last known price",
            ticker.display_name()
        );
    }
}

async fn fetch_all_sources(
    client: &Client,
    ticker: OracleTicker,
    include_coingecko: bool,
) -> Vec<(&'static str, Result<f64, OracleError>)> {
    let kucoin = fetch_kucoin(client, ticker);
    let mexc = fetch_mexc(client, ticker);

    let (kc_res, mx_res) = tokio::join!(kucoin, mexc);

    let mut results = vec![("kucoin", kc_res), ("mexc", mx_res)];

    if include_coingecko {
        let cg_res = fetch_coingecko(client, ticker).await;
        results.push(("coingecko", cg_res));
    }

    results
}

fn f64_to_bd(val: f64) -> BigDecimal {
    use std::str::FromStr;
    BigDecimal::from_str(&format!("{:.18}", val)).unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("rate limited")]
    RateLimited,
    #[error("Parse error: {0}")]
    Parse(String),
}

// --- KuCoin ---

#[derive(Deserialize)]
struct KucoinResponse {
    code: String,
    data: Option<KucoinData>,
}

#[derive(Deserialize)]
struct KucoinData {
    price: Option<String>,
}

async fn fetch_kucoin(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let url = format!(
        "https://api.kucoin.com/api/v1/market/orderbook/level1?symbol={}",
        ticker.kucoin_symbol()
    );
    let resp: KucoinResponse = client.get(&url).send().await?.json().await?;

    if resp.code != "200000" {
        return Err(OracleError::Parse(format!(
            "KuCoin returned code: {}",
            resp.code
        )));
    }

    resp.data
        .and_then(|d| d.price)
        .ok_or_else(|| OracleError::Parse("KuCoin: missing price field".into()))?
        .parse::<f64>()
        .map_err(|e| OracleError::Parse(format!("KuCoin: invalid price: {}", e)))
}

// --- MEXC ---

#[derive(Deserialize)]
struct MexcResponse {
    price: Option<String>,
}

async fn fetch_mexc(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let url = format!(
        "https://api.mexc.com/api/v3/ticker/price?symbol={}",
        ticker.mexc_symbol()
    );
    let resp: MexcResponse = client.get(&url).send().await?.json().await?;

    resp.price
        .ok_or_else(|| OracleError::Parse("MEXC: missing price field".into()))?
        .parse::<f64>()
        .map_err(|e| OracleError::Parse(format!("MEXC: invalid price: {}", e)))
}

// --- CoinGecko (fallback) ---

#[derive(Deserialize)]
struct CoinGeckoUsd {
    usd: Option<f64>,
}

/// CoinGecko free-tier rate-limit body (HTTP 429), e.g.
/// `{"status":{"error_code":429,"error_message":"..."}}`.
#[derive(Deserialize)]
struct CoinGeckoStatusBody {
    status: Option<CoinGeckoStatus>,
}

#[derive(Deserialize)]
struct CoinGeckoStatus {
    error_code: Option<u32>,
}

async fn fetch_coingecko(client: &Client, ticker: OracleTicker) -> Result<f64, OracleError> {
    let id = ticker.coingecko_id();
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={id}&vs_currencies=usd"
    );
    fetch_coingecko_url(client, &url, id).await
}

async fn fetch_coingecko_url(
    client: &Client,
    url: &str,
    coin_id: &str,
) -> Result<f64, OracleError> {
    let resp = client.get(url).send().await?;

    let status = resp.status();
    let body = resp.text().await.map_err(OracleError::Http)?;

    if status.as_u16() == 429 || coingecko_body_is_rate_limited(&body) {
        return Err(OracleError::RateLimited);
    }
    if !status.is_success() {
        return Err(OracleError::Parse(format!(
            "CoinGecko HTTP {status}: {}",
            body.chars().take(120).collect::<String>()
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| OracleError::Parse(format!("CoinGecko JSON: {e}")))?;

    parsed
        .get(coin_id)
        .and_then(|v| serde_json::from_value::<CoinGeckoUsd>(v.clone()).ok())
        .and_then(|t| t.usd)
        .ok_or_else(|| {
            OracleError::Parse(format!("CoinGecko: missing {coin_id}.usd field"))
        })
}

fn coingecko_body_is_rate_limited(body: &str) -> bool {
    serde_json::from_str::<CoinGeckoStatusBody>(body)
        .ok()
        .and_then(|b| b.status)
        .and_then(|s| s.error_code)
        == Some(429)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_parse_accepts_known() {
        assert_eq!(OracleTicker::parse("ustc"), Some(OracleTicker::Ustc));
        assert_eq!(OracleTicker::parse("LUNC"), Some(OracleTicker::Lunc));
        assert_eq!(OracleTicker::parse("unknown"), None);
    }

    #[test]
    fn ticker_symbols_are_distinct() {
        assert_ne!(
            OracleTicker::Ustc.kucoin_symbol(),
            OracleTicker::Lunc.kucoin_symbol()
        );
        assert_ne!(
            OracleTicker::Ustc.mexc_symbol(),
            OracleTicker::Lunc.mexc_symbol()
        );
        assert_ne!(
            OracleTicker::Ustc.coingecko_id(),
            OracleTicker::Lunc.coingecko_id()
        );
    }

    #[test]
    fn lunc_symbols_match_cex_convention() {
        assert_eq!(OracleTicker::Lunc.kucoin_symbol(), "LUNC-USDT");
        assert_eq!(OracleTicker::Lunc.mexc_symbol(), "LUNCUSDT");
        assert_eq!(OracleTicker::Lunc.coingecko_id(), "terra-luna");
    }

    #[test]
    fn ustc_symbols_unchanged() {
        assert_eq!(OracleTicker::Ustc.kucoin_symbol(), "USTC-USDT");
        assert_eq!(OracleTicker::Ustc.mexc_symbol(), "USTCUSDT");
        assert_eq!(OracleTicker::Ustc.coingecko_id(), "terrausd");
    }

    #[test]
    fn test_f64_to_bd() {
        let bd = f64_to_bd(0.00512);
        assert!(bd > BigDecimal::from(0));
        let s = bd.to_string();
        assert!(s.starts_with("0.005"));
    }

    #[test]
    fn test_average_calculation() {
        let prices = vec![0.00510, 0.00512, 0.00514];
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;
        assert!((avg - 0.00512).abs() < 1e-10);
    }

    #[test]
    fn test_average_single_source() {
        let prices = vec![0.00512];
        let avg = prices.iter().sum::<f64>() / prices.len() as f64;
        assert!((avg - 0.00512).abs() < 1e-10);
    }

    #[test]
    fn f64_to_bd_non_finite_defaults_to_zero() {
        let nan_bd = f64_to_bd(f64::NAN);
        assert_eq!(nan_bd, BigDecimal::from(0));
        let inf_bd = f64_to_bd(f64::INFINITY);
        assert_eq!(inf_bd, BigDecimal::from(0));
    }

    #[test]
    fn coingecko_rate_limit_body_detected() {
        let body = r#"{"status":{"error_code":429,"error_message":"You've exceeded the Rate Limit."}}"#;
        assert!(coingecko_body_is_rate_limited(body));
        assert!(!coingecko_body_is_rate_limited(r#"{"terrausd":{"usd":0.005}}"#));
    }

    #[tokio::test]
    async fn fetch_coingecko_maps_429_to_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .respond_with(ResponseTemplate::new(429).set_body_string(
                r#"{"status":{"error_code":429,"error_message":"rate limit"}}"#,
            ))
            .mount(&server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!(
            "{}/api/v3/simple/price?ids=terrausd&vs_currencies=usd",
            server.uri()
        );
        let err = fetch_coingecko_url(&client, &url, "terrausd")
            .await
            .unwrap_err();
        assert!(matches!(err, OracleError::RateLimited), "unexpected: {err}");
    }

    #[tokio::test]
    async fn fetch_coingecko_parses_lunc_id() {
        use wiremock::matchers::{method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v3/simple/price"))
            .and(query_param("ids", "terra-luna"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"terra-luna":{"usd":0.00005024}}"#,
            ))
            .mount(&server)
            .await;

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!(
            "{}/api/v3/simple/price?ids=terra-luna&vs_currencies=usd",
            server.uri()
        );
        let price = fetch_coingecko_url(&client, &url, "terra-luna")
            .await
            .unwrap();
        assert!((price - 0.00005024).abs() < 1e-12);
    }
}
